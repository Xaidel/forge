use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::util::{describe_error, parse_host_port};

const SECRET_ENV_VARS: &[&str] = &[
    "FORGE_SPIKE_HOST_SECRET",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AZURE_CLIENT_SECRET",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "SSH_AUTH_SOCK",
    "DOCKER_HOST",
];

pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub hops: usize,
    pub final_url: String,
}

pub fn run() -> std::io::Result<()> {
    let proxy = env::var("FORGE_PROXY").unwrap_or_default();
    let proxy_address = parse_proxy_url(&proxy).unwrap_or_else(|| {
        "127.0.0.1:1"
            .parse()
            .expect("hard-coded fallback proxy address is valid")
    });

    if env::var("FORGE_EXPECT_DENIED").as_deref() == Ok("1") {
        run_denied_tests(proxy_address);
    } else {
        run_proxy_tests(proxy_address);
    }
    Ok(())
}

fn run_proxy_tests(proxy: SocketAddr) {
    report("proxy-env-injected", check_proxy_env(&proxy));
    report(
        "http-allowed",
        http_get_via_proxy(proxy, "http://example.com/", 0)
            .and_then(|response| expect_status(response, 200)),
    );
    report(
        "http-unlisted",
        http_get_via_proxy(proxy, "http://example.org/", 0)
            .and_then(|response| expect_status(response, 403)),
    );
    report(
        "connect-allowed",
        connect_via_proxy(proxy, "example.com", 443).and_then(|status| {
            if status == 200 {
                Ok("CONNECT example.com:443 accepted".to_string())
            } else {
                Err(format!("expected 200, got {status}"))
            }
        }),
    );
    report(
        "connect-unlisted",
        connect_via_proxy(proxy, "example.org", 443).and_then(|status| {
            if status == 403 {
                Ok("CONNECT example.org:443 denied".to_string())
            } else {
                Err(format!("expected 403, got {status}"))
            }
        }),
    );
    report(
        "redirect-allowed",
        http_get_via_proxy(proxy, "http://redirect.test/allowed", 5).and_then(|response| {
            if response.status == 200 && response.hops >= 1 {
                Ok(format!(
                    "followed {} redirect(s) to {}",
                    response.hops, response.final_url
                ))
            } else {
                Err(format!(
                    "expected 200 after a redirect, got {} at {} after {} hop(s)",
                    response.status, response.final_url, response.hops
                ))
            }
        }),
    );
    report(
        "redirect-unlisted",
        http_get_via_proxy(proxy, "http://redirect.test/unlisted", 5).and_then(|response| {
            if response.status == 403 && response.hops >= 1 {
                Ok(format!(
                    "redirect to an unlisted host denied after {} hop(s)",
                    response.hops
                ))
            } else {
                Err(format!(
                    "expected 403 after a redirect, got {} at {} after {} hop(s)",
                    response.status, response.final_url, response.hops
                ))
            }
        }),
    );
    report(
        "redirect-private",
        http_get_via_proxy(proxy, "http://redirect.test/private", 5).and_then(|response| {
            if response.status == 403 && response.hops >= 1 {
                Ok(format!(
                    "redirect to a metadata address denied after {} hop(s)",
                    response.hops
                ))
            } else {
                Err(format!(
                    "expected 403 after a redirect, got {} at {} after {} hop(s)",
                    response.status, response.final_url, response.hops
                ))
            }
        }),
    );
    report(
        "proxy-loopback",
        http_get_via_proxy(
            proxy,
            &format!(
                "http://127.0.0.1:{}/",
                env::var("FORGE_LOOPBACK_SENTINEL").unwrap_or_default()
            ),
            0,
        )
        .and_then(|response| expect_status(response, 403)),
    );
    report(
        "proxy-metadata",
        http_get_via_proxy(proxy, "http://169.254.169.254/latest/meta-data/", 0)
            .and_then(|response| expect_status(response, 403)),
    );
    report(
        "proxy-rebind",
        http_get_via_proxy(proxy, "http://rebind.test/", 0)
            .and_then(|response| expect_status(response, 403)),
    );
    report(
        "proxy-private",
        http_get_via_proxy(proxy, "http://private.test/", 0)
            .and_then(|response| expect_status(response, 403)),
    );
    if env::var("FORGE_DYNAMIC_TEST").as_deref() == Ok("1") {
        report("dynamic-allowlist", dynamic_allowlist(proxy));
    }
    report_denial(
        "direct-egress",
        direct_tcp("1.1.1.1:443"),
        "FORGE_HOST_PREFLIGHT_EGRESS",
    );
    report_denial("direct-dns", direct_dns(), "FORGE_HOST_PREFLIGHT_DNS");
    report_denial(
        "direct-loopback",
        direct_tcp(&format!(
            "127.0.0.1:{}",
            env::var("FORGE_LOOPBACK_SENTINEL").unwrap_or_default()
        )),
        "FORGE_HOST_PREFLIGHT_SENTINEL",
    );
    report_denial(
        "direct-metadata",
        direct_tcp("169.254.169.254:80"),
        "FORGE_HOST_PREFLIGHT_METADATA",
    );
    report_optional(
        "real-client-http",
        curl_status("http://example.com/").and_then(|status| {
            if status == "200" {
                Ok("curl.exe returned 200 through the injected proxy".to_string())
            } else {
                Err(format!("expected 200, got {status}"))
            }
        }),
    );
    report_optional(
        "real-client-https",
        curl_status("https://example.com/").and_then(|status| {
            if status == "200" {
                Ok("curl.exe returned 200 over CONNECT".to_string())
            } else {
                Err(format!("expected 200, got {status}"))
            }
        }),
    );
    report_optional(
        "real-client-unlisted",
        curl_status("http://example.org/").and_then(|status| {
            if status == "403" {
                Ok("curl.exe received 403 for the unlisted host".to_string())
            } else {
                Err(format!("expected 403, got {status}"))
            }
        }),
    );
    report("no-secret-env", check_no_secret_env());
}

fn run_denied_tests(proxy: SocketAddr) {
    report_denial(
        "proxy-http-denied",
        http_get_via_proxy(proxy, "http://example.com/", 0).map(|response| {
            format!(
                "request unexpectedly succeeded with status {}",
                response.status
            )
        }),
        "FORGE_HOST_PREFLIGHT_EGRESS",
    );
    report_denial(
        "proxy-connect-denied",
        connect_via_proxy(proxy, "example.com", 443)
            .map(|status| format!("CONNECT unexpectedly returned {status}")),
        "FORGE_HOST_PREFLIGHT_EGRESS",
    );
    report_denial(
        "direct-egress",
        direct_tcp("1.1.1.1:443"),
        "FORGE_HOST_PREFLIGHT_EGRESS",
    );
    report_denial("direct-dns", direct_dns(), "FORGE_HOST_PREFLIGHT_DNS");
    report_denial(
        "direct-loopback",
        direct_tcp(&format!(
            "127.0.0.1:{}",
            env::var("FORGE_LOOPBACK_SENTINEL").unwrap_or_default()
        )),
        "FORGE_HOST_PREFLIGHT_SENTINEL",
    );
    report_denial(
        "direct-metadata",
        direct_tcp("169.254.169.254:80"),
        "FORGE_HOST_PREFLIGHT_METADATA",
    );
    report("no-secret-env", check_no_secret_env());
}

pub fn http_get_via_proxy(
    proxy: SocketAddr,
    url: &str,
    max_redirects: usize,
) -> Result<HttpResponse, String> {
    let mut current = url.to_string();
    for hop in 0..=max_redirects {
        let (host, port, _path) = parse_http_url(&current)?;
        let response = send_http_request(proxy, &current, &host, port)?;
        if (300..400).contains(&response.status) {
            if let Some(location) = response
                .headers
                .iter()
                .find(|(name, _)| name == "location")
                .map(|(_, value)| value)
            {
                if hop == max_redirects {
                    return Err(format!("too many redirects at {current}"));
                }
                current = resolve_location(&current, location)?;
                continue;
            }
        }
        return Ok(HttpResponse {
            status: response.status,
            body: response.body,
            hops: hop,
            final_url: current,
        });
    }
    Err("redirect loop".to_string())
}

pub fn connect_via_proxy(proxy: SocketAddr, host: &str, port: u16) -> Result<u16, String> {
    let mut stream = TcpStream::connect_timeout(&proxy, Duration::from_secs(5))
        .map_err(|error| format!("proxy connect failed: {}", describe_error(&error)))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|error| describe_error(&error))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(|error| describe_error(&error))?;
    let request = format!(
        "CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\nProxy-Connection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| describe_error(&error))?;
    let mut response = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| describe_error(&error))?;
        if read == 0 {
            break;
        }
        response.extend_from_slice(&chunk[..read]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    parse_status(&String::from_utf8_lossy(&response))
        .ok_or_else(|| "proxy returned a malformed CONNECT response".to_string())
}

struct RawResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn send_http_request(
    proxy: SocketAddr,
    url: &str,
    host: &str,
    port: u16,
) -> Result<RawResponse, String> {
    let mut stream = TcpStream::connect_timeout(&proxy, Duration::from_secs(5))
        .map_err(|error| format!("proxy connect failed: {}", describe_error(&error)))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(|error| describe_error(&error))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(20)))
        .map_err(|error| describe_error(&error))?;

    let authority = if port == 80 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let request = format!(
        "GET {url} HTTP/1.1\r\nHost: {authority}\r\nProxy-Connection: close\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| describe_error(&error))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| describe_error(&error))?;
    let text = String::from_utf8_lossy(&response).into_owned();
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| "proxy response had no header terminator".to_string())?;
    let status = parse_status(head).ok_or_else(|| {
        format!(
            "malformed proxy status line: {}",
            head.lines().next().unwrap_or("")
        )
    })?;
    let headers = head
        .lines()
        .skip(1)
        .filter_map(|line| {
            line.split_once(':')
                .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();
    Ok(RawResponse {
        status,
        headers,
        body: body.to_string(),
    })
}

fn parse_proxy_url(url: &str) -> Option<SocketAddr> {
    let rest = url.strip_prefix("http://")?;
    let (host, port) = parse_host_port(rest)?;
    if host == "127.0.0.1" || host == "localhost" {
        format!("127.0.0.1:{port}").parse().ok()
    } else {
        format!("{host}:{port}").parse().ok()
    }
}

fn parse_http_url(url: &str) -> Result<(String, u16, String), String> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| format!("only http:// URLs are supported: {url}"))?;
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    let (host, port) =
        parse_host_port(authority).ok_or_else(|| format!("invalid URL authority: {authority}"))?;
    Ok((host, port, path.to_string()))
}

fn resolve_location(current: &str, location: &str) -> Result<String, String> {
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }
    if let Some(rest) = location.strip_prefix('/') {
        let authority = current
            .strip_prefix("http://")
            .and_then(|rest| rest.split('/').next())
            .ok_or_else(|| format!("cannot resolve relative redirect from {current}"))?;
        return Ok(format!("http://{authority}/{rest}"));
    }
    Err(format!("unsupported relative redirect: {location}"))
}

fn parse_status(text: &str) -> Option<u16> {
    let first = text.lines().next()?;
    let mut parts = first.split_whitespace();
    let _version = parts.next()?;
    parts.next()?.parse().ok()
}

fn expect_status(response: HttpResponse, expected: u16) -> Result<String, String> {
    if response.status == expected {
        Ok(format!(
            "status {expected} at {} after {} hop(s)",
            response.final_url, response.hops
        ))
    } else {
        let body = response.body.chars().take(200).collect::<String>();
        Err(format!(
            "expected {expected}, got {} at {}: {}",
            response.status,
            response.final_url,
            body.replace('\n', " ")
        ))
    }
}

fn check_proxy_env(proxy: &SocketAddr) -> Result<String, String> {
    let expected = proxy.to_string();
    let observed: Vec<String> = ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"]
        .into_iter()
        .filter_map(|name| env::var(name).ok().map(|value| format!("{name}={value}")))
        .collect();
    if observed.iter().any(|value| value.contains(&expected)) {
        Ok(format!("proxy variables observed: {}", observed.join(", ")))
    } else {
        Err(format!(
            "expected a proxy variable containing {expected}; observed {}",
            observed.join(", ")
        ))
    }
}

fn check_no_secret_env() -> Result<String, String> {
    let leaked: Vec<&str> = SECRET_ENV_VARS
        .iter()
        .copied()
        .filter(|name| env::var_os(name).is_some())
        .collect();
    if leaked.is_empty() {
        Ok(format!(
            "checked {} secret variables; none present",
            SECRET_ENV_VARS.len()
        ))
    } else {
        Err(format!("leaked secret variables: {}", leaked.join(",")))
    }
}

fn direct_tcp(target: &str) -> Result<String, String> {
    let address = target
        .to_socket_addrs()
        .map_err(|error| format!("resolve {target}: {}", describe_error(&error)))?
        .next()
        .ok_or_else(|| format!("no address for {target}"))?;
    match TcpStream::connect_timeout(&address, Duration::from_secs(5)) {
        Ok(_) => Ok(format!("connected to {address}")),
        Err(error) => Err(format!("{address}: {}", describe_error(&error))),
    }
}

fn direct_dns() -> Result<String, String> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|error| describe_error(&error))?;
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|error| describe_error(&error))?;
    let query: &[u8] = &[
        0x46, 0x6f, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'e', b'x',
        b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00, 0x01,
    ];
    socket
        .send_to(query, "8.8.8.8:53")
        .map_err(|error| format!("send: {}", describe_error(&error)))?;
    let mut buffer = [0u8; 512];
    match socket.recv_from(&mut buffer) {
        Ok((read, from)) => Ok(format!("DNS response ({read} bytes) from {from}")),
        Err(error) => Err(format!("8.8.8.8:53: {}", describe_error(&error))),
    }
}

fn report(name: &str, result: Result<String, String>) {
    match result {
        Ok(detail) => println!("TEST {name} PASS {detail}"),
        Err(detail) => println!("TEST {name} FAIL {detail}"),
    }
}

fn report_optional(name: &str, result: Result<String, String>) {
    match result {
        Ok(detail) => println!("TEST {name} PASS {detail}"),
        Err(detail) if detail.contains("not found") => {
            println!("TEST {name} INCONCLUSIVE {detail}");
        }
        Err(detail) => println!("TEST {name} FAIL {detail}"),
    }
}

fn dynamic_allowlist(proxy: SocketAddr) -> Result<String, String> {
    let worktree = PathBuf::from(
        env::var("FORGE_SPIKE_WORKTREE")
            .map_err(|_| "FORGE_SPIKE_WORKTREE is not set".to_string())?,
    );
    let grant_request = worktree.join("dynamic-grant-request.txt");
    let revoke_request = worktree.join("dynamic-revoke-request.txt");
    let grant_ack = worktree.join("dynamic-grant-ack.txt");
    let revoke_ack = worktree.join("dynamic-revoke-ack.txt");
    let _ = fs::remove_file(&grant_ack);
    let _ = fs::remove_file(&revoke_ack);

    let before = http_get_via_proxy(proxy, "http://example.org/", 0)?;
    if before.status != 403 {
        return Err(format!(
            "before grant: expected 403 for example.org, got {}",
            before.status
        ));
    }

    fs::write(&grant_request, b"grant\n")
        .map_err(|error| format!("write grant request: {error}"))?;
    wait_for_path(&grant_ack, Duration::from_secs(5))?;
    let after = http_get_via_proxy(proxy, "http://example.org/", 0)?;
    if after.status != 200 {
        return Err(format!(
            "after grant: expected 200 for example.org, got {}",
            after.status
        ));
    }

    fs::write(&revoke_request, b"revoke\n")
        .map_err(|error| format!("write revoke request: {error}"))?;
    wait_for_path(&revoke_ack, Duration::from_secs(5))?;
    let revoked = http_get_via_proxy(proxy, "http://example.org/", 0)?;
    if revoked.status != 403 {
        return Err(format!(
            "after revoke: expected 403 for example.org, got {}",
            revoked.status
        ));
    }

    Ok("403 -> 200 after a mid-task grant -> 403 after revoke".to_string())
}

fn wait_for_path(path: &Path, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!("timed out waiting for {}", path.display()))
}

fn curl_status(url: &str) -> Result<String, String> {
    let output = Command::new("curl.exe")
        .args([
            "-sS",
            "-o",
            "NUL",
            "-w",
            "%{http_code}",
            "--max-time",
            "20",
            url,
        ])
        .output();
    match output {
        Ok(output) => {
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if output.status.success() {
                Ok(status)
            } else {
                Err(format!(
                    "curl.exe exit {:?}: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err("curl.exe not found".to_string())
        }
        Err(error) => Err(format!("failed to run curl.exe: {error}")),
    }
}

fn report_denial(name: &str, result: Result<String, String>, preflight_name: &str) {
    let preflight_ok = env::var(preflight_name).as_deref() == Ok("1");
    match (result, preflight_ok) {
        (Err(detail), true) => println!("TEST {name} PASS denied: {detail}"),
        (Err(detail), false) => println!(
            "TEST {name} INCONCLUSIVE denied, but host preflight {preflight_name} was false: {detail}"
        ),
        (Ok(detail), _) => println!("TEST {name} FAIL {detail}"),
    }
}
