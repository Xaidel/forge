//! Host-side driver for the MXC native-containment spike.
//!
//! Builds a throwaway task worktree, a sibling "outside" directory, and a fake
//! credential directory, then runs the `canary` binary inside MXC's native
//! containment tier. The canary reports the six PRD denial checks on stdout.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use mxc_sdk::v1::policy::{
    FilesystemPolicy, NetworkAction, NetworkEgressPolicy, NetworkIngressPolicy, NetworkPolicy,
    UiPolicy,
};
use mxc_sdk::v1::{self, ContainerRequest, Containment, WaitResult};

fn main() {
    if let Err(error) = run() {
        eprintln!("SPIKE ERROR: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let platform = platform_name();
    println!("MXC SPIKE");
    println!("platform: {platform}");
    println!("spike: {}", env!("CARGO_PKG_VERSION"));
    println!("mxc-sdk: 1.0.1");
    println!("host os: {}", env::consts::OS);
    println!("host arch: {}", env::consts::ARCH);
    println!("exe: {}", env::current_exe()?.display());

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
    if !support.is_supported {
        println!(
            "SPIKE RESULT: UNSUPPORTED {}",
            support
                .reason
                .unwrap_or_else(|| "no MXC backend on this host".to_string())
        );
        std::process::exit(3);
    }

    let root = env::current_exe()?
        .parent()
        .ok_or("target directory has no parent")?
        .join("mxc-spike-run");
    let worktree = root.join("worktree");
    let outside = root.join("outside");
    let credentials = root.join("credentials");
    let home = worktree.join("home");
    let temp = worktree.join("tmp");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    for directory in [&worktree, &outside, &credentials, &home, &temp] {
        fs::create_dir_all(directory)?;
    }

    let credential_path = credentials.join("id_rsa");
    fs::write(&credential_path, b"FAKE-PRIVATE-KEY-DO-NOT-READ")?;

    let canary_name = if cfg!(windows) { "canary.exe" } else { "canary" };
    let source_canary = env::current_exe()?
        .parent()
        .ok_or("target directory has no parent")?
        .join(canary_name);
    let canary_path = worktree.join(canary_name);
    fs::copy(&source_canary, &canary_path)?;

    // Host preflight: both targets must be reachable without MXC, otherwise a
    // denial inside the container would be a false positive.
    let outside_probe = outside.join("host-preflight.txt");
    fs::write(&outside_probe, b"host can write here")?;
    assert!(outside_probe.exists(), "host preflight write failed");
    assert!(credential_path.exists(), "credential fixture missing");

    // Create the escape links from the host, before the container starts, so
    // the canary tests traversal rather than link-creation privileges.
    create_escape_links(&worktree, &outside, &credentials)?;

    let escape_write = worktree.join("escape-outside").join("canary-write.txt");
    let escape_read = worktree.join("escape-credentials").join("id_rsa");

    let worker_env = worker_environment(&worktree, &home, &temp);
    println!("worker env keys: {:?}", worker_env.iter().map(|(k, _)| k).collect::<Vec<_>>());

    let filesystem = FilesystemPolicy {
        readwrite_paths: vec![path_string(&worktree)],
        readonly_paths: readonly_paths(),
        denied_paths: vec![path_string(&outside), path_string(&credentials)],
        clear_policy_on_exit: Some(true),
    };

    let command = canary_command(
        &canary_path,
        &worktree,
        &outside,
        &credential_path,
        &escape_write,
        &escape_read,
    );
    println!("command: {command}");
    println!("readwrite_paths: {:?}", filesystem.readwrite_paths);
    println!("readonly_paths: {:?}", filesystem.readonly_paths);
    println!("denied_paths: {:?}", filesystem.denied_paths);

    println!("containment: Process (native default)");
    let request = ContainerRequest {
        containment: Containment::Process,
        filesystem: Some(filesystem),
        network: Some(NetworkPolicy {
            egress: Some(NetworkEgressPolicy {
                default: Some(NetworkAction::Deny),
                ..Default::default()
            }),
            ingress: Some(NetworkIngressPolicy {
                default: Some(NetworkAction::Deny),
                host_loopback: Some(NetworkAction::Deny),
            }),
            ..Default::default()
        }),
        ui: Some(UiPolicy::default()),
        timeout_ms: Some(120_000),
        working_directory: Some(path_string(&worktree)),
        environment: Some(worker_env),
        inherit_default_environment: Some(false),
        ..ContainerRequest::new(command)
    };

    // Windows ProcessContainer exposes a native probe with its isolation tier.
    #[cfg(windows)]
    match v1::probe(Some(&request)) {
        Ok(probe) => println!(
            "probe: tier={:?} needs_dacl_augmentation={:?} warnings={:?} error={:?} facts={{base_container_api_present:{}, native_capture_available:{}, base_container_supports_deny_paths:{}, base_container_supports_enumerate_paths:{}}}",
            probe.tier,
            probe.needs_dacl_augmentation,
            probe.warnings,
            probe.error,
            probe.probes.base_container_api_present,
            probe.probes.native_capture_available,
            probe.probes.base_container_supports_deny_paths,
            probe.probes.base_container_supports_enumerate_paths
        ),
        Err(error) => println!("probe error: {error}"),
    }

    let result = v1::run(request, Default::default())?;
    println!("outcome: {:?}", result.outcome);
    println!("warnings: {:?}", result.warnings);
    print!("--- canary stdout ---\n{}", String::from_utf8_lossy(&result.stdout));
    print!("--- canary stderr ---\n{}", String::from_utf8_lossy(&result.stderr));

    let stdout = String::from_utf8_lossy(&result.stdout);
    let mut passed = 0;
    let mut failed = 0;
    let mut inconclusive = 0;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("CANARY ") {
            let mut parts = rest.splitn(3, ' ');
            let _name = parts.next();
            match parts.next() {
                Some("PASS") => passed += 1,
                Some("FAIL") => failed += 1,
                Some("INCONCLUSIVE") => inconclusive += 1,
                _ => {}
            }
        }
    }
    println!(
        "canary summary: passed={passed} failed={failed} inconclusive={inconclusive}"
    );
    match result.outcome {
        WaitResult::Exited(0) => {}
        WaitResult::Exited(code) => println!("canary process exited with code {code}"),
        WaitResult::TimedOut => println!("canary process timed out"),
    }
    if failed > 0 || inconclusive > 0 || passed < 6 {
        std::process::exit(2);
    }
    println!("SPIKE RESULT: ALL SIX CANARIES PASS");
    Ok(())
}

fn platform_name() -> &'static str {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        "Windows 11 24H2+ x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macOS 15+ arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "macOS 15+ x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "Ubuntu 24.04 LTS x64"
    }
    #[cfg(not(any(
        all(windows, target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    {
        "unexpected platform"
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn readonly_paths() -> Vec<String> {
    let mut paths = Vec::new();
    #[cfg(windows)]
    {
        for key in ["SystemRoot", "windir"] {
            if let Ok(value) = env::var(key) {
                paths.push(value);
            }
        }
        if let Ok(value) = env::var("ProgramFiles") {
            paths.push(value);
        }
        if let Ok(value) = env::var("ProgramFiles(x86)") {
            paths.push(value);
        }
    }
    #[cfg(target_os = "linux")]
    {
        for path in ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc/ld.so.cache"] {
            if Path::new(path).exists() {
                paths.push(path.to_string());
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        for path in ["/usr", "/System", "/Library", "/private/var/db/dyld"] {
            if Path::new(path).exists() {
                paths.push(path.to_string());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn worker_environment(
    worktree: &Path,
    home: &Path,
    temp: &Path,
) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut push = |key: &str, value: String| {
        pairs.push((key.to_string(), value));
    };
    for key in ["PATH", "LANG", "LC_ALL", "TERM", "SystemDrive"] {
        if let Ok(value) = env::var(key) {
            push(key, value);
        }
    }
    #[cfg(windows)]
    {
        for key in [
            "SystemRoot",
            "windir",
            "ComSpec",
            "PATHEXT",
            "PROCESSOR_ARCHITECTURE",
            "OS",
            "LOCALAPPDATA",
            "APPDATA",
            "ProgramData",
        ] {
            if let Ok(value) = env::var(key) {
                push(key, value);
            }
        }
        // MXC's ProcessContainer requires SYSTEMROOT and LOCALAPPDATA.
        if env::var("SystemRoot").is_err() {
            push("SystemRoot", r"C:\Windows".to_string());
        }
        if env::var("LOCALAPPDATA").is_err() {
            push("LOCALAPPDATA", path_string(&home.join("AppData/Local")));
        }
        push("USERPROFILE", path_string(home));
        push("HOMEDRIVE", path_string(&worktree));
    }
    #[cfg(not(windows))]
    {
        push("HOME", path_string(home));
        push("USERPROFILE", path_string(home));
    }
    push("TEMP", path_string(temp));
    push("TMP", path_string(temp));
    push("TMPDIR", path_string(temp));
    push("FORGE_SPIKE_WORKTREE", path_string(worktree));
    pairs.sort();
    pairs.dedup();
    pairs
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn canary_command(
    canary: &Path,
    worktree: &Path,
    outside: &Path,
    credential: &Path,
    escape_write: &Path,
    escape_read: &Path,
) -> String {
    [
        quote(&path_string(canary)),
        "--worktree".to_string(),
        quote(&path_string(worktree)),
        "--outside".to_string(),
        quote(&path_string(outside)),
        "--credential".to_string(),
        quote(&path_string(credential)),
        "--escape-write".to_string(),
        quote(&path_string(escape_write)),
        "--escape-read".to_string(),
        quote(&path_string(escape_read)),
        "--network".to_string(),
        "1.1.1.1:443".to_string(),
        "--self".to_string(),
        quote(&path_string(canary)),
    ]
    .join(" ")
}

fn create_escape_links(worktree: &Path, outside: &Path, credentials: &Path) -> io::Result<()> {
    let outside_link = worktree.join("escape-outside");
    let credentials_link = worktree.join("escape-credentials");
    #[cfg(windows)]
    {
        for (link, target) in [(&outside_link, outside), (&credentials_link, credentials)] {
            let status = Command::new("cmd.exe")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .status()?;
            if !status.success() {
                return Err(io::Error::other(format!(
                    "mklink /J failed for {}",
                    link.display()
                )));
            }
        }
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside, &outside_link)?;
        std::os::unix::fs::symlink(credentials, &credentials_link)?;
    }
    if !outside_link.exists() {
        return Err(io::Error::other("outside escape link missing"));
    }
    if !credentials_link.exists() {
        return Err(io::Error::other("credential escape link missing"));
    }
    let mut stdout = io::stdout();
    stdout.write_all(b"escape links created\n")?;
    Ok(())
}
