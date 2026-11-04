use std::env;
use std::fs;
use std::io::Write;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use forge_windows_proxy_spike::appcontainer;
use forge_windows_proxy_spike::client;
use forge_windows_proxy_spike::proxy;
use forge_windows_proxy_spike::util::{self, append_log};

use mxc_sdk::v1::configs::{ProcessContainerConfig, ProcessContainerNetwork};
use mxc_sdk::v1::policy::{
    FilesystemPolicy, NetworkAction, NetworkEgressPolicy, NetworkIngressPolicy, NetworkPolicy,
    NetworkRuntimeConfig, UiPolicy,
};
use mxc_sdk::v1::{self, ContainerRequest, Containment, WaitResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Identityless,
    IdentityScoped,
    IdentityScopedNormalProxy,
    NetworkDenied,
}

impl Variant {
    fn name(self) -> &'static str {
        match self {
            Self::Identityless => "identityless",
            Self::IdentityScoped => "identity-scoped-appcontainer",
            Self::IdentityScopedNormalProxy => "identity-scoped-normal-proxy",
            Self::NetworkDenied => "network-denied",
        }
    }

    fn uses_proxy(self) -> bool {
        self != Self::NetworkDenied
    }

    fn identity_scoped(self) -> bool {
        matches!(self, Self::IdentityScoped | Self::IdentityScopedNormalProxy)
    }

    fn appcontainer_proxy(self) -> bool {
        self == Self::IdentityScoped
    }
}

struct VariantResult {
    name: &'static str,
    status: &'static str,
    detail: String,
    client_pass: usize,
    client_fail: usize,
    client_inconclusive: usize,
    proxy_reachable: bool,
    proxy_http_status: Option<u16>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|argument| argument == "--token-probe") {
        let log_path = PathBuf::from(
            argument_value(&args, "--log").unwrap_or_else(|| "token-probe.log".to_string()),
        );
        token_probe(&log_path);
        return;
    }
    if args.iter().any(|argument| argument == "--proxy") {
        if let Err(error) = run_proxy_child() {
            eprintln!("PROXY FATAL: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = run() {
        eprintln!("SPIKE ERROR: {error}");
        std::process::exit(1);
    }
}

fn token_probe(log_path: &Path) {
    use std::net::ToSocketAddrs;
    let mut lines = Vec::new();
    lines.push("TOKEN-PROBE start".to_string());
    lines.push(format!(
        "TOKEN-PROBE cwd={:?} exe={:?}",
        env::current_dir(),
        env::current_exe()
    ));
    for key in ["SystemRoot", "LOCALAPPDATA", "TEMP", "USERPROFILE"] {
        lines.push(format!("TOKEN-PROBE env {key}={:?}", env::var(key)));
    }
    match ("example.com", 80).to_socket_addrs() {
        Ok(addresses) => {
            let addresses: Vec<String> = addresses.map(|address| address.to_string()).collect();
            lines.push(format!("TOKEN-PROBE dns example.com={addresses:?}"));
        }
        Err(error) => lines.push(format!("TOKEN-PROBE dns example.com error={error}")),
    }
    match ("httpbin.org", 80).to_socket_addrs() {
        Ok(addresses) => {
            let addresses: Vec<String> = addresses.map(|address| address.to_string()).collect();
            lines.push(format!("TOKEN-PROBE dns httpbin.org={addresses:?}"));
        }
        Err(error) => lines.push(format!("TOKEN-PROBE dns httpbin.org error={error}")),
    }
    match "104.20.23.154:80".parse::<std::net::SocketAddr>() {
        Ok(address) => match TcpStream::connect_timeout(&address, Duration::from_secs(8)) {
            Ok(_) => lines.push(format!("TOKEN-PROBE tcp {address}=connected")),
            Err(error) => lines.push(format!("TOKEN-PROBE tcp {address}=error: {error}")),
        },
        Err(error) => lines.push(format!("TOKEN-PROBE tcp parse error={error}")),
    }
    lines.push("TOKEN-PROBE done".to_string());
    for line in lines {
        append_log(log_path, &line);
    }
}

fn run_proxy_child() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let port = argument_value(&args, "--port")
        .ok_or("proxy child requires --port")?
        .parse::<u16>()?;
    let log_path =
        PathBuf::from(argument_value(&args, "--log").ok_or("proxy child requires --log")?);
    let allowlist_path = argument_value(&args, "--allowlist").map(PathBuf::from);
    proxy::run(port, log_path, allowlist_path)?;
    Ok(())
}

fn argument_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|argument| argument == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        env::set_var("FORGE_SPIKE_HOST_SECRET", "leak-me");
        env::set_var("ANTHROPIC_API_KEY", "sk-ant-spike-should-not-leak");
    }

    println!("FORGE WINDOWS PROXY SPIKE");
    println!("date: {}", util::now_millis());
    println!("mxc-sdk: 1.0.1");
    println!("host os: {} {}", env::consts::OS, env::consts::ARCH);
    println!("os version: {}", os_version());

    let support = v1::platform_support();
    println!(
        "platform_support: is_supported={} reason={:?} methods={:?}",
        support.is_supported, support.reason, support.available_methods
    );
    for backend in v1::available_backends() {
        println!(
            "available_backend: backend={} tier={:?} capabilities={:?} warnings={:?}",
            backend.backend, backend.tier, backend.capabilities, backend.warnings
        );
    }

    let exe = env::current_exe()?;
    let exe_dir = exe.parent().ok_or("target directory has no parent")?;
    let client_exe = exe_dir.join(if cfg!(windows) {
        "proxy-client.exe"
    } else {
        "proxy-client"
    });
    if !client_exe.exists() {
        return Err(format!("client binary not found: {}", client_exe.display()).into());
    }

    let root = exe_dir.join("windows-proxy-run");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;

    let (sentinel_port, sentinel_count) = start_loopback_sentinel()?;
    let host_egress = host_tcp_preflight("1.1.1.1:443");
    let host_dns = util::host_udp_dns_probe("8.8.8.8:53", Duration::from_secs(3)).is_ok();
    let host_metadata = host_tcp_preflight("169.254.169.254:80");
    let host_sentinel = host_tcp_preflight(&format!("127.0.0.1:{sentinel_port}"));
    // The listener thread increments asynchronously. Consume the preflight
    // connection before resetting the counter, otherwise the next variant can
    // see a stale hit and fail the loopback test.
    let reset_start = Instant::now();
    while sentinel_count.load(Ordering::SeqCst) == 0
        && reset_start.elapsed() < Duration::from_secs(2)
    {
        thread::sleep(Duration::from_millis(20));
    }
    sentinel_count.store(0, Ordering::SeqCst);
    println!(
        "host preflight: egress_1_1_1_1_443={} dns_8_8_8_8_53={} loopback_sentinel={} metadata_169_254_169_254_80={}",
        yes_no(host_egress),
        yes_no(host_dns),
        yes_no(host_sentinel),
        yes_no(host_metadata)
    );

    let preflights = HostPreflights {
        egress: host_egress,
        dns: host_dns,
        sentinel: host_sentinel,
        metadata: host_metadata,
    };

    let variants = [
        Variant::Identityless,
        Variant::IdentityScoped,
        Variant::IdentityScopedNormalProxy,
        Variant::NetworkDenied,
    ];
    let mut results = Vec::new();
    for variant in variants {
        let result = run_variant(
            variant,
            &root,
            &client_exe,
            &exe,
            sentinel_port,
            &sentinel_count,
            &preflights,
        );
        results.push(result);
    }

    println!("\n=== SUMMARY ===");
    for result in &results {
        println!(
            "variant={} status={} proxy_reachable={} proxy_http_status={:?} client_pass={} client_fail={} client_inconclusive={} detail={}",
            result.name,
            result.status,
            result.proxy_reachable,
            result.proxy_http_status,
            result.client_pass,
            result.client_fail,
            result.client_inconclusive,
            result.detail
        );
    }

    let strong = results
        .iter()
        .find(|result| result.name == Variant::IdentityScoped.name())
        .map(|result| result.status == "PASS")
        .unwrap_or(false);
    println!("\n=== VERDICT ===");
    if strong {
        println!(
            "SPIKE VERDICT: strong peer-scoped proxy path PASSED; Windows can ship Sandboxed with network allowlisting"
        );
    } else {
        println!(
            "SPIKE VERDICT: strong peer-scoped proxy path FAILED; Windows ships `Sandboxed — network denied`; no weak loopback proxy"
        );
    }
    Ok(())
}

struct HostPreflights {
    egress: bool,
    dns: bool,
    sentinel: bool,
    metadata: bool,
}

fn run_variant(
    variant: Variant,
    root: &Path,
    client_exe: &Path,
    host_exe: &Path,
    sentinel_port: u16,
    sentinel_count: &Arc<AtomicUsize>,
    preflights: &HostPreflights,
) -> VariantResult {
    println!("\n--- variant: {} ---", variant.name());
    let variant_root = root.join(variant.name());
    let _ = fs::remove_dir_all(&variant_root);
    let worktree = variant_root.join("worktree");
    let outside = variant_root.join("outside");
    let credentials = variant_root.join("credentials");
    let home = worktree.join("home");
    let temp = worktree.join("tmp");
    for directory in [&worktree, &outside, &credentials, &home, &temp] {
        if let Err(error) = fs::create_dir_all(directory) {
            return failed_result(variant, format!("mkdir {}: {error}", directory.display()));
        }
    }

    let fixture_sentinel = outside.join("host-sentinel.txt");
    let _ = fs::write(&fixture_sentinel, b"HOST-SENTINEL-UNCHANGED");
    let _ = fs::write(credentials.join("id_rsa"), b"FAKE-PRIVATE-KEY-DO-NOT-READ");

    let client_name = if cfg!(windows) {
        "proxy-client.exe"
    } else {
        "proxy-client"
    };
    let client_path = worktree.join(client_name);
    if let Err(error) = fs::copy(client_exe, &client_path) {
        return failed_result(variant, format!("copy client: {error}"));
    }

    // The proxy executable is copied into the run root so an AppContainer
    // profile can be granted read/execute access without touching target/.
    let proxy_name = if cfg!(windows) {
        "proxy-spike.exe"
    } else {
        "proxy-spike"
    };
    let proxy_exe = variant_root.join(proxy_name);
    if let Err(error) = fs::copy(host_exe, &proxy_exe) {
        return failed_result(variant, format!("copy proxy: {error}"));
    }

    let proxy_port = free_loopback_port().unwrap_or(18080);
    let proxy_log = variant_root.join("proxy.log");
    let allowlist_path = variant_root.join("allowlist.txt");
    let initial_allowlist: Vec<String> = [
        "example.com",
        "www.example.com",
        "httpbin.org",
        "redirect.test",
        "rebind.test",
        "private.test",
        "metadata.test",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let _ = write_allowlist(&allowlist_path, &initial_allowlist);
    let proxy_address = format!("127.0.0.1:{proxy_port}");
    let proxy_url = format!("http://{proxy_address}");
    let profile_name = format!("ForgeProxySpike-{}", std::process::id());

    let mut profile: Option<appcontainer::AppContainerProfile> = None;
    let mut proxy_child: Option<Child> = None;
    #[cfg(windows)]
    let mut appcontainer_child: Option<appcontainer::AppContainerChild> = None;

    if variant.uses_proxy() {
        if variant.identity_scoped() {
            match appcontainer::AppContainerProfile::create(&profile_name) {
                Ok(created) => {
                    println!(
                        "appcontainer: name={} sid={}",
                        created.name,
                        created.sid_string().unwrap_or_else(|error| error)
                    );
                    if let Ok(sid) = created.sid_string() {
                        match appcontainer::grant_access(&variant_root, &sid) {
                            Ok(detail) => println!("appcontainer: {detail}"),
                            Err(detail) => println!("appcontainer: ACL grant failed: {detail}"),
                        }
                    }
                    println!(
                        "appcontainer: loopback-exempt: {}",
                        appcontainer::add_loopback_exemption(&created.name)
                    );
                    profile = Some(created);
                }
                Err(error) => {
                    println!("appcontainer: profile creation failed: {error}");
                }
            }
        }

        if variant.appcontainer_proxy() {
            #[cfg(windows)]
            {
                if let Some(ref profile) = profile {
                    let args = vec![
                        "--proxy".to_string(),
                        "--port".to_string(),
                        proxy_port.to_string(),
                        "--log".to_string(),
                        proxy_log.to_string_lossy().into_owned(),
                        "--allowlist".to_string(),
                        allowlist_path.to_string_lossy().into_owned(),
                    ];
                    match appcontainer::launch(&proxy_exe, &args, &variant_root, profile) {
                        Ok(child) => {
                            println!("proxy: launched in AppContainer pid={}", child.pid);
                            appcontainer_child = Some(child);
                        }
                        Err(error) => {
                            println!("proxy: AppContainer launch failed: {error}");
                        }
                    }
                }
            }
            #[cfg(not(windows))]
            {
                println!("proxy: AppContainer launch is Windows-only");
            }
        } else {
            let child = Command::new(&proxy_exe)
                .arg("--proxy")
                .arg("--port")
                .arg(proxy_port.to_string())
                .arg("--log")
                .arg(&proxy_log)
                .arg("--allowlist")
                .arg(&allowlist_path)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            match child {
                Ok(child) => {
                    println!("proxy: launched as normal process pid={}", child.id());
                    proxy_child = Some(child);
                }
                Err(error) => println!("proxy: normal launch failed: {error}"),
            }
        }

        let listening = wait_for_proxy_listen(&proxy_log, Duration::from_secs(15));
        println!("proxy: listening={listening}");
        if !listening {
            println!(
                "proxy: log so far:\n{}",
                read_text(&proxy_log, 4000).unwrap_or_default()
            );
        }
    } else {
        println!("proxy: not started (network denied variant)");
    }

    let (proxy_reachable, proxy_http_status) = if variant.uses_proxy() {
        host_proxy_preflight(&proxy_address)
    } else {
        (false, None)
    };
    println!(
        "proxy: host preflight reachable={} http_status={:?}",
        proxy_reachable, proxy_http_status
    );

    let (network, allowed_proxy_peer) = network_policy(variant, &proxy_url, &profile_name);
    println!(
        "policy: variant={} allowed_proxy_peer={:?} network_proxy={:?}",
        variant.name(),
        allowed_proxy_peer,
        if variant.uses_proxy() {
            Some(proxy_url.as_str())
        } else {
            None
        }
    );

    let mut process_network = ProcessContainerNetwork::default();
    process_network.allowed_proxy_peer = allowed_proxy_peer.clone();
    let mut process_container = ProcessContainerConfig::default();
    process_container.network = Some(process_network);
    let containment = Containment::ProcessContainer(process_container);

    let filesystem = FilesystemPolicy {
        readwrite_paths: vec![path_string(&worktree)],
        readonly_paths: readonly_paths(),
        denied_paths: vec![path_string(&outside), path_string(&credentials)],
        clear_policy_on_exit: Some(true),
    };

    let mut worker_env = worker_environment(&worktree, &home, &temp);
    worker_env.push(("FORGE_PROXY".to_string(), proxy_url.clone()));
    worker_env.push((
        "FORGE_LOOPBACK_SENTINEL".to_string(),
        sentinel_port.to_string(),
    ));
    worker_env.push((
        "FORGE_HOST_PREFLIGHT_EGRESS".to_string(),
        bool_string(preflights.egress),
    ));
    worker_env.push((
        "FORGE_HOST_PREFLIGHT_DNS".to_string(),
        bool_string(preflights.dns),
    ));
    worker_env.push((
        "FORGE_HOST_PREFLIGHT_SENTINEL".to_string(),
        bool_string(preflights.sentinel),
    ));
    worker_env.push((
        "FORGE_HOST_PREFLIGHT_METADATA".to_string(),
        bool_string(preflights.metadata),
    ));
    if variant == Variant::NetworkDenied {
        worker_env.push(("FORGE_EXPECT_DENIED".to_string(), "1".to_string()));
    } else {
        worker_env.push(("FORGE_DYNAMIC_TEST".to_string(), "1".to_string()));
    }

    let command = quote_arg(&path_string(&client_path));
    let request = ContainerRequest {
        containment,
        filesystem: Some(filesystem),
        network: Some(network),
        ui: Some(UiPolicy::default()),
        timeout_ms: Some(180_000),
        working_directory: Some(path_string(&worktree)),
        environment: Some(worker_env),
        inherit_default_environment: Some(false),
        ..ContainerRequest::new(command)
    };

    #[cfg(windows)]
    match v1::probe(Some(&request)) {
        Ok(probe) => println!(
            "probe: tier={:?} needs_dacl_augmentation={:?} warnings={:?} error={:?} facts={{base_container_api_present:{}, native_capture_available:{}, base_container_supports_deny_paths:{}, base_container_supports_enumerate_paths:{}, base_container_supports_identityless_loopback_proxy:{}}}",
            probe.tier,
            probe.needs_dacl_augmentation,
            probe.warnings,
            probe.error,
            probe.probes.base_container_api_present,
            probe.probes.native_capture_available,
            probe.probes.base_container_supports_deny_paths,
            probe.probes.base_container_supports_enumerate_paths,
            probe
                .probes
                .base_container_supports_identityless_loopback_proxy
        ),
        Err(error) => println!("probe error: {error}"),
    }

    let dynamic_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let dynamic_handle = if variant.uses_proxy() {
        let worktree = worktree.clone();
        let allowlist = allowlist_path.clone();
        let stop = Arc::clone(&dynamic_stop);
        Some(thread::spawn(move || {
            watch_dynamic_grants(worktree, allowlist, stop);
        }))
    } else {
        None
    };

    let client_result = v1::run(request, Default::default());
    dynamic_stop.store(true, Ordering::SeqCst);
    if let Some(handle) = dynamic_handle {
        let _ = handle.join();
    }
    let mut client_pass = 0;
    let mut client_fail = 0;
    let mut client_inconclusive = 0;
    let mut run_error = None;
    match client_result {
        Ok(result) => {
            println!("outcome: {:?}", result.outcome);
            println!("warnings: {:?}", result.warnings);
            let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
            println!("--- client stdout ---\n{stdout}");
            println!("--- client stderr ---\n{stderr}");
            for line in stdout.lines() {
                if let Some(rest) = line.strip_prefix("TEST ") {
                    let mut parts = rest.splitn(3, ' ');
                    let name = parts.next().unwrap_or("");
                    let verdict = parts.next().unwrap_or("");
                    let detail = parts.next().unwrap_or("");
                    match verdict {
                        "PASS" => client_pass += 1,
                        "FAIL" => client_fail += 1,
                        "INCONCLUSIVE" => client_inconclusive += 1,
                        _ => {}
                    }
                    println!("client-test: {name} {verdict} {detail}");
                }
            }
            match result.outcome {
                WaitResult::Exited(0) => {}
                WaitResult::Exited(code) => {
                    println!("client process exited with code {code}");
                }
                WaitResult::TimedOut => {
                    println!("client process timed out");
                }
            }
        }
        Err(error) => {
            run_error = Some(error.to_string());
            println!("mxc run error: {error}");
        }
    }

    let sentinel_hits = sentinel_count.swap(0, Ordering::SeqCst);
    let sentinel_unchanged = fs::read(&fixture_sentinel)
        .map(|bytes| bytes == b"HOST-SENTINEL-UNCHANGED")
        .unwrap_or(false);
    println!(
        "host checks: sentinel_connections={sentinel_hits} sentinel_unchanged={sentinel_unchanged}"
    );

    if variant.uses_proxy() {
        println!("--- proxy log ---");
        println!("{}", read_text(&proxy_log, 12_000).unwrap_or_default());
    }

    let status = evaluate_variant(
        variant,
        client_pass,
        client_fail,
        client_inconclusive,
        sentinel_hits,
        sentinel_unchanged,
        proxy_reachable,
        run_error.as_deref(),
    );

    cleanup_proxy(
        &mut proxy_child,
        #[cfg(windows)]
        &mut appcontainer_child,
        &mut profile,
    );

    VariantResult {
        name: variant.name(),
        status: status.0,
        detail: status.1,
        client_pass,
        client_fail,
        client_inconclusive,
        proxy_reachable,
        proxy_http_status,
    }
}

fn network_policy(
    variant: Variant,
    proxy_url: &str,
    profile_name: &str,
) -> (NetworkPolicy, Option<String>) {
    match variant {
        Variant::Identityless => (
            NetworkPolicy {
                egress: Some(NetworkEgressPolicy {
                    default: Some(NetworkAction::Deny),
                    ..Default::default()
                }),
                ingress: Some(NetworkIngressPolicy {
                    default: Some(NetworkAction::Allow),
                    host_loopback: Some(NetworkAction::Allow),
                }),
                runtime_config: Some(NetworkRuntimeConfig {
                    network_proxy: Some(proxy_url.to_string()),
                }),
            },
            None,
        ),
        Variant::IdentityScoped | Variant::IdentityScopedNormalProxy => (
            NetworkPolicy {
                egress: Some(NetworkEgressPolicy {
                    default: Some(NetworkAction::Deny),
                    ..Default::default()
                }),
                ingress: Some(NetworkIngressPolicy {
                    default: Some(NetworkAction::Allow),
                    host_loopback: Some(NetworkAction::Deny),
                }),
                runtime_config: Some(NetworkRuntimeConfig {
                    network_proxy: Some(proxy_url.to_string()),
                }),
            },
            Some(profile_name.to_string()),
        ),
        Variant::NetworkDenied => (
            NetworkPolicy {
                egress: Some(NetworkEgressPolicy {
                    default: Some(NetworkAction::Deny),
                    ..Default::default()
                }),
                ingress: Some(NetworkIngressPolicy {
                    default: Some(NetworkAction::Deny),
                    host_loopback: Some(NetworkAction::Deny),
                }),
                runtime_config: None,
            },
            None,
        ),
    }
}

fn evaluate_variant(
    variant: Variant,
    client_pass: usize,
    client_fail: usize,
    client_inconclusive: usize,
    sentinel_hits: usize,
    sentinel_unchanged: bool,
    proxy_reachable: bool,
    run_error: Option<&str>,
) -> (&'static str, String) {
    if let Some(error) = run_error {
        return ("FAIL", format!("MXC run error: {error}"));
    }
    if !sentinel_unchanged {
        return ("FAIL", "host sentinel changed".to_string());
    }
    if sentinel_hits > 0 {
        return (
            "FAIL",
            format!("client reached the host loopback sentinel {sentinel_hits} time(s)"),
        );
    }

    if variant == Variant::NetworkDenied {
        // Metadata and DNS probes can be inconclusive when the host itself
        // cannot reach the probe target; only an unexpected success is a
        // failure.
        if client_fail == 0 && client_pass + client_inconclusive >= 7 {
            return (
                "PASS",
                "all proxy and direct egress attempts were denied; no loopback mutation"
                    .to_string(),
            );
        }
        return (
            "FAIL",
            format!(
                "network-denied expectations not met: pass={client_pass} fail={client_fail} inconclusive={client_inconclusive}"
            ),
        );
    }

    if variant == Variant::IdentityScopedNormalProxy {
        if client_pass > 0 && client_fail > 0 {
            return (
                "EXPECTED-FAIL",
                format!(
                    "normal proxy was rejected under the peer-scoped policy (client pass={client_pass}, fail={client_fail}); host reachable={proxy_reachable}"
                ),
            );
        }
        return (
            "UNEXPECTED",
            format!(
                "peer-scoped normal-proxy negative test was inconclusive: pass={client_pass} fail={client_fail} reachable={proxy_reachable}"
            ),
        );
    }

    if client_fail == 0 && client_pass + client_inconclusive >= 21 {
        (
            "PASS",
            format!(
                "allowlist, CONNECT, redirects, DNS/private/metadata, direct egress, and loopback checks passed; host preflight reachable={proxy_reachable}"
            ),
        )
    } else {
        (
            "FAIL",
            format!(
                "proxy path expectations not met: pass={client_pass} fail={client_fail} inconclusive={client_inconclusive}"
            ),
        )
    }
}

fn cleanup_proxy(
    normal: &mut Option<Child>,
    #[cfg(windows)] appcontainer: &mut Option<appcontainer::AppContainerChild>,
    profile: &mut Option<appcontainer::AppContainerProfile>,
) {
    if let Some(child) = normal {
        let _ = child.kill();
        let _ = child.wait();
    }
    #[cfg(windows)]
    if let Some(child) = appcontainer {
        child.terminate();
    }
    if let Some(profile) = profile {
        println!(
            "appcontainer: loopback-exempt cleanup: {}",
            appcontainer::remove_loopback_exemption(&profile.name)
        );
        println!("appcontainer: {}", profile.delete());
    }
}

fn read_allowlist(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn write_allowlist(path: &Path, hosts: &[String]) -> std::io::Result<()> {
    fs::write(path, format!("{}\n", hosts.join("\n")))
}

/// Host-side control channel for a mid-task host grant. The proxy is inside an
/// AppContainer, so the host cannot connect to its listener; instead the proxy
/// watches this file and the host writes the updated allowlist. The client
/// signals the host through files in the task worktree, waits for the ack, and
/// retries the request.
fn watch_dynamic_grants(
    worktree: PathBuf,
    allowlist: PathBuf,
    stop: Arc<std::sync::atomic::AtomicBool>,
) {
    let grant_request = worktree.join("dynamic-grant-request.txt");
    let revoke_request = worktree.join("dynamic-revoke-request.txt");
    let grant_ack = worktree.join("dynamic-grant-ack.txt");
    let revoke_ack = worktree.join("dynamic-revoke-ack.txt");
    let mut granted = false;
    while !stop.load(Ordering::SeqCst) {
        if grant_request.exists() && !granted {
            let mut hosts = read_allowlist(&allowlist);
            if !hosts
                .iter()
                .any(|host| host.eq_ignore_ascii_case("example.org"))
            {
                hosts.push("example.org".to_string());
            }
            let _ = write_allowlist(&allowlist, &hosts);
            thread::sleep(Duration::from_millis(200));
            let _ = fs::write(&grant_ack, b"granted\n");
            let _ = fs::remove_file(&grant_request);
            granted = true;
        }
        if revoke_request.exists() && granted {
            let hosts: Vec<String> = read_allowlist(&allowlist)
                .into_iter()
                .filter(|host| !host.eq_ignore_ascii_case("example.org"))
                .collect();
            let _ = write_allowlist(&allowlist, &hosts);
            thread::sleep(Duration::from_millis(200));
            let _ = fs::write(&revoke_ack, b"revoked\n");
            let _ = fs::remove_file(&revoke_request);
            granted = false;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_proxy_listen(log_path: &Path, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(text) = fs::read_to_string(log_path) {
            if text.contains("PROXY listen=") {
                return true;
            }
            if text.contains("PROXY error=") {
                return false;
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

fn host_proxy_preflight(address: &str) -> (bool, Option<u16>) {
    let Ok(socket) = address.parse::<std::net::SocketAddr>() else {
        return (false, None);
    };
    match TcpStream::connect_timeout(&socket, Duration::from_secs(5)) {
        Ok(stream) => {
            drop(stream);
            let status = client::http_get_via_proxy(socket, "http://example.com/", 0)
                .ok()
                .map(|response| response.status);
            (true, status)
        }
        Err(error) => {
            println!("proxy host preflight failed: {error}");
            (false, None)
        }
    }
}

fn start_loopback_sentinel() -> std::io::Result<(u16, Arc<AtomicUsize>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let count = Arc::new(AtomicUsize::new(0));
    let thread_count = Arc::clone(&count);
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    thread_count.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    );
                }
                Err(_) => break,
            }
        }
    });
    Ok((port, count))
}

fn host_tcp_preflight(target: &str) -> bool {
    let Ok(addresses) = target.to_socket_addrs() else {
        return false;
    };
    addresses
        .into_iter()
        .any(|address| TcpStream::connect_timeout(&address, Duration::from_secs(3)).is_ok())
}

fn free_loopback_port() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn os_version() -> String {
    format!(
        "{} {}",
        std::env::var("OS").unwrap_or_else(|_| std::env::consts::OS.to_string()),
        std::env::var("PROCESSOR_ARCHITECTURE")
            .unwrap_or_else(|_| std::env::consts::ARCH.to_string())
    )
}

fn readonly_paths() -> Vec<String> {
    let mut paths = Vec::new();
    for key in ["SystemRoot", "windir", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(value) = env::var(key) {
            paths.push(value);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn worker_environment(worktree: &Path, home: &Path, temp: &Path) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut push = |key: &str, value: String| {
        pairs.push((key.to_string(), value));
    };
    for key in [
        "PATH",
        "LANG",
        "LC_ALL",
        "TERM",
        "SystemRoot",
        "windir",
        "ComSpec",
        "PATHEXT",
        "PROCESSOR_ARCHITECTURE",
        "OS",
        "LOCALAPPDATA",
        "APPDATA",
        "ProgramData",
        "SystemDrive",
    ] {
        if let Ok(value) = env::var(key) {
            push(key, value);
        }
    }
    if env::var("SystemRoot").is_err() {
        push("SystemRoot", r"C:\Windows".to_string());
    }
    if env::var("LOCALAPPDATA").is_err() {
        push("LOCALAPPDATA", path_string(&home.join("AppData/Local")));
    }
    push("USERPROFILE", path_string(home));
    push("HOMEDRIVE", path_string(worktree));
    push("TEMP", path_string(temp));
    push("TMP", path_string(temp));
    push("TMPDIR", path_string(temp));
    push("FORGE_SPIKE_WORKTREE", path_string(worktree));
    pairs.sort();
    pairs.dedup();
    pairs
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn quote_arg(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn bool_string(value: bool) -> String {
    if value { "1" } else { "0" }.to_string()
}

fn yes_no(value: bool) -> &'static str {
    if value { "ok" } else { "fail" }
}

fn read_text(path: &Path, limit: usize) -> Option<String> {
    let mut text = fs::read_to_string(path).ok()?;
    if text.len() > limit {
        text.truncate(limit);
        text.push_str("\n[truncated]");
    }
    Some(text)
}

fn failed_result(variant: Variant, detail: String) -> VariantResult {
    VariantResult {
        name: variant.name(),
        status: "FAIL",
        detail,
        client_pass: 0,
        client_fail: 0,
        client_inconclusive: 0,
        proxy_reachable: false,
        proxy_http_status: None,
    }
}
