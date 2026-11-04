use std::env;
use std::fs;
use std::time::Duration;

use forge_windows_proxy_spike::appcontainer;

fn main() {
    if let Err(error) = run() {
        eprintln!("PROBE ERROR: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let exe = env::current_exe()?;
    let exe_dir = exe.parent().ok_or("target directory has no parent")?;
    let root = exe_dir.join("appcontainer-probe");
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(&root)?;

    let proxy_exe = root.join("proxy-spike.exe");
    fs::copy(exe_dir.join("proxy-spike.exe"), &proxy_exe)?;
    let log_path = root.join("token-probe.log");

    let profile_name = format!("ForgeProxySpikeProbe-{}", std::process::id());
    let mut profile = appcontainer::AppContainerProfile::create(&profile_name)?;
    let sid = profile.sid_string()?;
    println!("profile: name={} sid={sid}", profile.name);
    println!("acl: {}", appcontainer::grant_access(&root, &sid)?);
    println!(
        "loopback-exempt: {}",
        appcontainer::add_loopback_exemption(&profile.name)
    );

    let child = appcontainer::launch(
        &proxy_exe,
        &[
            "--token-probe".to_string(),
            "--log".to_string(),
            log_path.to_string_lossy().into_owned(),
        ],
        &root,
        &profile,
    )?;
    println!("launched token probe pid={}", child.pid);
    let exited = appcontainer::wait_for_exit(&child, Duration::from_secs(30));
    println!("token probe exited within 30s: {exited}");
    println!("--- token probe log ---");
    match fs::read_to_string(&log_path) {
        Ok(text) => println!("{text}"),
        Err(error) => println!("read failed: {error}"),
    }
    println!(
        "cleanup: {}",
        appcontainer::remove_loopback_exemption(&profile.name)
    );
    println!("profile: {}", profile.delete());
    Ok(())
}
