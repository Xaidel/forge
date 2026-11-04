use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

/// Append one line to the spike log. Logs are intentionally line-oriented so a
/// human can read the evidence file without a parser.
pub fn append_log(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
    }
}

/// Parse an authority in `host`, `host:port`, or `[v6]:port` form.
pub fn parse_host_port(authority: &str) -> Option<(String, u16)> {
    let authority = authority.trim();
    if authority.is_empty() {
        return None;
    }
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        let port = tail.strip_prefix(':').unwrap_or("80");
        return Some((host.to_string(), port.parse().ok()?));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() && !port.is_empty() && !host.contains(':') => {
            Some((host.to_string(), port.parse().ok()?))
        }
        _ => Some((authority.to_string(), 80)),
    }
}

/// True when an address must never be reachable through the proxy.
///
/// This is the proxy's DNS-rebinding and SSRF fence: loopback, private,
/// link-local, carrier-grade NAT, documentation/test ranges, multicast,
/// broadcast, and IPv4-mapped equivalents are all rejected.
pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_v4(v4),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
                || v6.to_ipv4_mapped().is_some_and(is_forbidden_v4)
        }
    }
}

fn is_forbidden_v4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        || octets[0] >= 240
}

/// Hostnames that are local by definition, independent of DNS.
pub fn is_forbidden_hostname(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || host.ends_with(".localhost")
        || host == "metadata"
        || host == "metadata.google.internal"
        || host == "instance-data"
        || host == "metadata.azure.com"
}

/// A short human-readable error chain for evidence logs.
pub fn describe_error(error: &std::io::Error) -> String {
    format!("{:?}: {}", error.kind(), error)
}

/// A token that is stable enough to distinguish runs in logs.
pub fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Convert an IPv6 address to a string suitable for `host:port` targets.
pub fn socket_string(ip: IpAddr, port: u16) -> String {
    match ip {
        IpAddr::V4(v4) => format!("{v4}:{port}"),
        IpAddr::V6(v6) => format!("[{v6}]:{port}"),
    }
}

/// Format a SID as an SDDL string for `icacls`. Kept here so the Windows-only
/// helper does not need to duplicate non-Windows test code.
pub fn sid_grant(sid: &str) -> String {
    format!("*{sid}")
}

/// A tiny DNS query used only for host preflight. It does not validate the
/// answer; it proves the host can send a query and receive a response.
pub fn host_udp_dns_probe(server: &str, timeout: std::time::Duration) -> std::io::Result<()> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(timeout))?;
    // example.com A query, transaction id 0x466f.
    let query: &[u8] = &[
        0x46, 0x6f, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, b'e', b'x',
        b'a', b'm', b'p', b'l', b'e', 0x03, b'c', b'o', b'm', 0x00, 0x00, 0x01, 0x00, 0x01,
    ];
    socket.send_to(query, server)?;
    let mut buffer = [0u8; 512];
    let _ = socket.recv_from(&mut buffer)?;
    Ok(())
}
