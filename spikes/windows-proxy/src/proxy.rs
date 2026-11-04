use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::util::{
    append_log, describe_error, is_forbidden_hostname, is_forbidden_ip, parse_host_port,
};

/// The allowlist used by the spike proxy. `example.com` and `httpbin.org` are
/// real public hosts; the `.test` names are synthetic DNS answers used to prove
/// the rebinding and private-address checks without relying on an attacker DNS
/// server.
pub const ALLOWED_HOSTS: &[&str] = &[
    "example.com",
    "www.example.com",
    "httpbin.org",
    "redirect.test",
    "rebind.test",
    "private.test",
    "metadata.test",
];

pub struct ProxyState {
    pub log_path: PathBuf,
    pub allowlist_path: Option<PathBuf>,
    last_allowlist: Mutex<Option<String>>,
}

impl ProxyState {
    pub fn new(log_path: PathBuf, allowlist_path: Option<PathBuf>) -> Self {
        Self {
            log_path,
            allowlist_path,
            last_allowlist: Mutex::new(None),
        }
    }
}

pub fn run(port: u16, log_path: PathBuf, allowlist_path: Option<PathBuf>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let address = listener.local_addr()?;
    let state = Arc::new(ProxyState::new(log_path, allowlist_path));
    append_log(&state.log_path, &format!("PROXY listen={address}"));
    match ("example.com", 80).to_socket_addrs() {
        Ok(mut addresses) => match addresses.next() {
            Some(resolved) => append_log(
                &state.log_path,
                &format!("PROXY dns-probe host=example.com answer={resolved}"),
            ),
            None => append_log(
                &state.log_path,
                "PROXY dns-probe host=example.com answer=none",
            ),
        },
        Err(error) => append_log(
            &state.log_path,
            &format!("PROXY dns-probe host=example.com error={error}"),
        ),
    }

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Err(error) = handle_connection(stream, &state) {
                        append_log(&state.log_path, &format!("PROXY connection-error={error}"));
                    }
                });
            }
            Err(error) => {
                append_log(
                    &state.log_path,
                    &format!("PROXY accept-error={}", describe_error(&error)),
                );
                break;
            }
        }
    }
    Ok(())
}

fn handle_connection(mut client: TcpStream, state: &ProxyState) -> std::io::Result<()> {
    let log = &state.log_path;
    client.set_read_timeout(Some(Duration::from_secs(20)))?;
    client.set_write_timeout(Some(Duration::from_secs(20)))?;

    let header_bytes = read_headers(&mut client)?;
    if header_bytes.is_empty() {
        return Ok(());
    }
    let request = match Request::parse(&header_bytes) {
        Some(request) => request,
        None => {
            append_log(log, "PROXY malformed-request");
            send_error(&mut client, 400, "Bad Request", "malformed request");
            return Ok(());
        }
    };

    if let RequestKind::Http {
        ref url, ref host, ..
    } = request.kind
    {
        if host.eq_ignore_ascii_case("redirect.test") {
            let location = match url.as_str() {
                "http://redirect.test/allowed" => Some("http://example.com/"),
                "http://redirect.test/unlisted" => Some("http://example.org/"),
                "http://redirect.test/private" => Some("http://169.254.169.254/latest/meta-data/"),
                "http://redirect.test/rebind" => Some("http://rebind.test/"),
                _ => None,
            };
            if let Some(location) = location {
                append_log(
                    log,
                    &format!("PROXY synthetic-redirect target={url} location={location}"),
                );
                let response = format!(
                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                client.write_all(response.as_bytes())?;
                return Ok(());
            }
            let response =
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            client.write_all(response.as_bytes())?;
            return Ok(());
        }
    }

    match request.kind {
        RequestKind::Connect { host, port } => {
            let target = format!("{host}:{port}");
            let decision = authorize(&host, port, state);
            append_log(
                log,
                &format!(
                    "PROXY method=CONNECT target={target} decision={} reason={}{}",
                    decision.verdict(),
                    decision.reason,
                    decision.resolved_suffix()
                ),
            );
            let Some(resolved) = decision.resolved else {
                send_error(&mut client, 403, "Forbidden", &decision.reason);
                return Ok(());
            };
            let upstream = match connect_resolved(resolved, port) {
                Ok(stream) => stream,
                Err(error) => {
                    append_log(
                        log,
                        &format!("PROXY connect-error target={target} error={error}"),
                    );
                    send_error(&mut client, 502, "Bad Gateway", &error);
                    return Ok(());
                }
            };
            client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
            tunnel(client, upstream, log);
        }
        RequestKind::Http {
            ref url,
            ref host,
            port,
        } => {
            let decision = authorize(&host, port, state);
            append_log(
                log,
                &format!(
                    "PROXY method={} target={} host={}:{} decision={} reason={}{}",
                    request.method,
                    url,
                    host,
                    port,
                    decision.verdict(),
                    decision.reason,
                    decision.resolved_suffix()
                ),
            );
            let Some(resolved) = decision.resolved else {
                send_error(&mut client, 403, "Forbidden", &decision.reason);
                return Ok(());
            };
            let upstream = match connect_resolved(resolved, port) {
                Ok(stream) => stream,
                Err(error) => {
                    append_log(
                        log,
                        &format!("PROXY connect-error target={url} error={error}"),
                    );
                    send_error(&mut client, 502, "Bad Gateway", &error);
                    return Ok(());
                }
            };
            forward_http(client, upstream, &request, log)?;
        }
    }
    Ok(())
}

fn read_headers(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if bytes.len() > 64 * 1024 {
            break;
        }
    }
    Ok(bytes)
}

struct Request {
    method: String,
    kind: RequestKind,
    headers: Vec<(String, String)>,
}

enum RequestKind {
    Connect {
        host: String,
        port: u16,
    },
    Http {
        url: String,
        host: String,
        port: u16,
    },
}

impl Request {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let text = String::from_utf8_lossy(bytes);
        let mut lines = text.split("\r\n");
        let request_line = lines.next()?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next()?.to_string();
        let target = parts.next()?.to_string();
        let _version = parts.next().unwrap_or("HTTP/1.1");
        let headers: Vec<(String, String)> = lines
            .take_while(|line| !line.is_empty())
            .filter_map(|line| {
                line.split_once(':')
                    .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
            })
            .collect();

        if method.eq_ignore_ascii_case("CONNECT") {
            let (host, port) = parse_host_port(&target)?;
            return Some(Self {
                method,
                kind: RequestKind::Connect { host, port },
                headers,
            });
        }

        if let Some(rest) = target.strip_prefix("http://") {
            let (authority, path) = match rest.find('/') {
                Some(index) => (&rest[..index], &rest[index..]),
                None => (rest, "/"),
            };
            let (host, port) = parse_host_port(authority)?;
            return Some(Self {
                method,
                kind: RequestKind::Http {
                    url: format!("http://{authority}{path}"),
                    host,
                    port,
                },
                headers,
            });
        }

        // Origin-form request through a proxy. The Host header supplies the
        // authority. This is not used by the spike client, but accepting it
        // keeps the proxy honest for curl-style clients.
        let host_header = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("host"))
            .map(|(_, value)| value.clone())?;
        let (host, port) = parse_host_port(&host_header)?;
        Some(Self {
            method,
            kind: RequestKind::Http {
                url: format!("http://{host_header}{target}"),
                host,
                port,
            },
            headers,
        })
    }

    fn host_header(&self) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("host"))
            .map(|(_, value)| value.as_str())
    }

    fn origin_path(&self) -> &str {
        match &self.kind {
            RequestKind::Http { url, .. } => {
                let rest = url.strip_prefix("http://").unwrap_or(url);
                rest.find('/').map_or("/", |index| &rest[index..])
            }
            RequestKind::Connect { .. } => "/",
        }
    }
}

struct Decision {
    allowed: bool,
    reason: String,
    resolved: Option<IpAddr>,
}

impl Decision {
    fn allow(reason: impl Into<String>, resolved: IpAddr) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            resolved: Some(resolved),
        }
    }

    fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            resolved: None,
        }
    }

    fn verdict(&self) -> &'static str {
        if self.allowed { "allow" } else { "deny" }
    }

    fn resolved_suffix(&self) -> String {
        match self.resolved {
            Some(ip) => format!(" resolved={ip}"),
            None => String::new(),
        }
    }
}

fn authorize(host: &str, port: u16, state: &ProxyState) -> Decision {
    let host = host.trim_end_matches('.').to_ascii_lowercase();

    if is_forbidden_hostname(&host) {
        return Decision::deny("loopback-or-local-hostname");
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_forbidden_ip(ip) {
            return Decision::deny("private-loopback-link-local-or-metadata-address");
        }
        if !is_allowed(&host, state) {
            return Decision::deny("host-not-allowlisted");
        }
        return Decision::allow("allowlisted-ip-literal", ip);
    }

    if !is_allowed(&host, state) {
        return Decision::deny("host-not-allowlisted");
    }

    let addresses = match synthetic_dns(&host) {
        Some(ip) => {
            append_log(
                &state.log_path,
                &format!("PROXY dns-synthetic host={host} answer={ip}"),
            );
            vec![ip]
        }
        None => match (host.as_str(), port).to_socket_addrs() {
            Ok(addresses) => addresses.map(|address| address.ip()).collect(),
            Err(error) => {
                return Decision {
                    allowed: false,
                    reason: format!("dns-resolution-failed: {error}"),
                    resolved: None,
                };
            }
        },
    };

    if addresses.is_empty() {
        return Decision::deny("dns-no-addresses");
    }
    if let Some(forbidden) = addresses.iter().copied().find(|ip| is_forbidden_ip(*ip)) {
        append_log(
            &state.log_path,
            &format!("PROXY dns-rebinding-blocked host={host} answer={forbidden}"),
        );
        return Decision::deny("dns-rebinding-private-address");
    }

    let resolved = addresses[0];
    Decision::allow("allowlisted-host", resolved)
}

fn is_allowed(host: &str, state: &ProxyState) -> bool {
    load_allowlist(state)
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(host))
}

fn load_allowlist(state: &ProxyState) -> Vec<String> {
    let Some(path) = state.allowlist_path.as_ref() else {
        return ALLOWED_HOSTS.iter().map(|host| host.to_string()).collect();
    };
    let text = fs::read_to_string(path).unwrap_or_default();
    let changed = {
        let last = state.last_allowlist.lock().unwrap();
        last.as_deref() != Some(text.as_str())
    };
    let hosts: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_ascii_lowercase)
        .collect();
    if changed {
        append_log(
            &state.log_path,
            &format!("PROXY allowlist-updated hosts={}", hosts.join(",")),
        );
        *state.last_allowlist.lock().unwrap() = Some(text);
    }
    hosts
}

fn synthetic_dns(host: &str) -> Option<IpAddr> {
    match host {
        "rebind.test" => Some(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        "private.test" => Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
        "metadata.test" => Some(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))),
        _ => None,
    }
}

fn connect_resolved(ip: IpAddr, port: u16) -> Result<TcpStream, String> {
    let address = SocketAddr::new(ip, port);
    TcpStream::connect_timeout(&address, Duration::from_secs(10))
        .map_err(|error| describe_error(&error))
}

fn forward_http(
    mut client: TcpStream,
    mut upstream: TcpStream,
    request: &Request,
    log: &PathBuf,
) -> std::io::Result<()> {
    upstream.set_read_timeout(Some(Duration::from_secs(20)))?;
    upstream.set_write_timeout(Some(Duration::from_secs(20)))?;

    let mut forwarded = format!("{} {} HTTP/1.1\r\n", request.method, request.origin_path());
    let mut host_written = false;
    for (name, value) in &request.headers {
        if name.eq_ignore_ascii_case("proxy-connection")
            || name.eq_ignore_ascii_case("connection")
            || name.eq_ignore_ascii_case("proxy-authorization")
            || name.eq_ignore_ascii_case("accept-encoding")
        {
            continue;
        }
        if name.eq_ignore_ascii_case("host") {
            host_written = true;
        }
        forwarded.push_str(&format!("{name}: {value}\r\n"));
    }
    if !host_written {
        if let Some(host) = request.host_header() {
            forwarded.push_str(&format!("Host: {host}\r\n"));
        }
    }
    forwarded.push_str("Accept-Encoding: identity\r\nConnection: close\r\n\r\n");
    upstream.write_all(forwarded.as_bytes())?;

    let mut response = Vec::new();
    upstream.read_to_end(&mut response)?;
    client.write_all(&response)?;
    append_log(log, &format!("PROXY forwarded-bytes={}", response.len()));
    Ok(())
}

fn tunnel(mut client: TcpStream, mut upstream: TcpStream, log: &PathBuf) {
    let _ = client.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = upstream.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = client.set_write_timeout(Some(Duration::from_secs(20)));
    let _ = upstream.set_write_timeout(Some(Duration::from_secs(20)));

    let mut client_read = match client.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            append_log(log, &format!("PROXY tunnel-clone-error={error}"));
            return;
        }
    };
    let mut upstream_read = match upstream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            append_log(log, &format!("PROXY tunnel-clone-error={error}"));
            return;
        }
    };

    let to_upstream = thread::spawn(move || {
        let _ = std::io::copy(&mut client_read, &mut upstream);
        let _ = upstream.shutdown(Shutdown::Write);
    });
    let to_client = thread::spawn(move || {
        let _ = std::io::copy(&mut upstream_read, &mut client);
        let _ = client.shutdown(Shutdown::Write);
    });
    let _ = to_upstream.join();
    let _ = to_client.join();
    append_log(log, "PROXY tunnel-closed");
}

fn send_error(stream: &mut TcpStream, code: u16, reason: &str, detail: &str) {
    let body = format!("forge proxy: {reason}: {detail}\n");
    let response = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}
