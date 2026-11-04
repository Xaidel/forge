//! Runs inside the MXC container and reports whether the six PRD canaries hold.
//!
//! Output is line-oriented so the host can parse it without a serialization
//! dependency:
//!
//!   CANARY <name> PASS|FAIL|INCONCLUSIVE <detail>

use std::env;
use std::fs;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

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

#[derive(Default)]
struct Args {
    child: bool,
    worktree: PathBuf,
    outside: PathBuf,
    credential: PathBuf,
    escape_write: PathBuf,
    escape_read: PathBuf,
    network: String,
    self_exe: PathBuf,
}

fn parse_args() -> Args {
    let mut args = Args {
        network: "1.1.1.1:443".to_string(),
        ..Args::default()
    };
    let mut iter = env::args().skip(1);
    while let Some(flag) = iter.next() {
        if flag == "--child" {
            args.child = true;
            continue;
        }
        let value = iter.next().unwrap_or_default();
        match flag.as_str() {
            "--worktree" => args.worktree = PathBuf::from(value),
            "--outside" => args.outside = PathBuf::from(value),
            "--credential" => args.credential = PathBuf::from(value),
            "--escape-write" => args.escape_write = PathBuf::from(value),
            "--escape-read" => args.escape_read = PathBuf::from(value),
            "--network" => args.network = value,
            "--self" => args.self_exe = PathBuf::from(value),
            _ => {}
        }
    }
    args
}

fn line(name: &str, verdict: &str, detail: impl AsRef<str>) {
    println!("CANARY {name} {verdict} {}", detail.as_ref());
}

fn describe(error: &io::Error) -> String {
    format!("{:?}: {}", error.kind(), error)
}

fn attempt_write(path: &Path) -> Result<(), io::Error> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(b"mxc-containment-spike")?;
    file.sync_all()
}

fn main() {
    let args = parse_args();
    if args.child {
        child_mode(&args);
        return;
    }

    // 1. Write outside the task worktree.
    match attempt_write(&args.outside.join("canary-write.txt")) {
        Ok(()) => line(
            "write-outside-worktree",
            "FAIL",
            format!("write unexpectedly succeeded: {}", args.outside.display()),
        ),
        Err(error) => line(
            "write-outside-worktree",
            "PASS",
            format!("{}: {}", args.outside.display(), describe(&error)),
        ),
    }

    // 2. Read a credential path.
    match fs::read(&args.credential) {
        Ok(bytes) => line(
            "read-credential-path",
            "FAIL",
            format!(
                "read unexpectedly succeeded ({} bytes): {}",
                bytes.len(),
                args.credential.display()
            ),
        ),
        Err(error) => line(
            "read-credential-path",
            "PASS",
            format!("{}: {}", args.credential.display(), describe(&error)),
        ),
    }

    // 3. Unlisted network host.
    match args.network.parse::<SocketAddr>() {
        Ok(address) => match TcpStream::connect_timeout(&address, Duration::from_secs(5)) {
            Ok(_) => line(
                "unlisted-network-host",
                "FAIL",
                format!("connection unexpectedly succeeded: {address}"),
            ),
            Err(error) => line(
                "unlisted-network-host",
                "PASS",
                format!("{address}: {}", describe(&error)),
            ),
        },
        Err(error) => line(
            "unlisted-network-host",
            "INCONCLUSIVE",
            format!("bad probe address {}: {error}", args.network),
        ),
    }

    // 4. A child process inherits the denials.
    match Command::new(&args.self_exe)
        .arg("--child")
        .arg("--outside")
        .arg(&args.outside)
        .arg("--credential")
        .arg(&args.credential)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let child_write_denied = stdout.contains("CHILD write DENIED");
            let child_read_denied = stdout.contains("CHILD read DENIED");
            let detail = format!(
                "exit={:?} write_denied={child_write_denied} read_denied={child_read_denied} stdout={} stderr={}",
                output.status.code(),
                stdout.trim().replace('\n', " | "),
                String::from_utf8_lossy(&output.stderr).trim().replace('\n', " | ")
            );
            if child_write_denied && child_read_denied {
                line("child-process-inherits-denials", "PASS", detail);
            } else {
                line("child-process-inherits-denials", "FAIL", detail);
            }
        }
        Err(error) => line(
            "child-process-inherits-denials",
            "PASS",
            format!("child spawn denied: {}", describe(&error)),
        ),
    }

    // 5. Symlink or junction escape, both write and read.
    let write_result = attempt_write(&args.escape_write);
    let read_result = fs::read(&args.escape_read);
    let mut verdict = "PASS";
    let mut detail = String::new();
    match write_result {
        Ok(()) => {
            verdict = "FAIL";
            detail.push_str(&format!(
                "write through escape link succeeded: {}; ",
                args.escape_write.display()
            ));
        }
        Err(error) => detail.push_str(&format!(
            "write through escape link denied ({}); ",
            describe(&error)
        )),
    }
    match read_result {
        Ok(bytes) => {
            verdict = "FAIL";
            detail.push_str(&format!(
                "read through escape link succeeded ({} bytes); ",
                bytes.len()
            ));
        }
        Err(error) => detail.push_str(&format!(
            "read through escape link denied ({}); ",
            describe(&error)
        )),
    }
    line("symlink-or-junction-escape", verdict, detail);

    // 6. No model key or cloud secret in the worker environment.
    let leaked: Vec<&str> = SECRET_ENV_VARS
        .iter()
        .copied()
        .filter(|name| env::var_os(name).is_some())
        .collect();
    if leaked.is_empty() {
        let visible = env::vars_os()
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(",");
        line(
            "secret-free-worker-environment",
            "PASS",
            format!("checked {} secret variables; visible env: {visible}", SECRET_ENV_VARS.len()),
        );
    } else {
        line(
            "secret-free-worker-environment",
            "FAIL",
            format!("leaked secret variables: {}", leaked.join(",")),
        );
    }
}

fn child_mode(args: &Args) {
    match attempt_write(&args.outside.join("child-write.txt")) {
        Ok(()) => println!(
            "CHILD write ALLOWED {}",
            args.outside.join("child-write.txt").display()
        ),
        Err(error) => println!("CHILD write DENIED {}", describe(&error)),
    }
    match fs::read(&args.credential) {
        Ok(bytes) => println!("CHILD read ALLOWED {} bytes", bytes.len()),
        Err(error) => println!("CHILD read DENIED {}", describe(&error)),
    }
}
