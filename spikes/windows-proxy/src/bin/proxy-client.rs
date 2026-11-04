fn main() {
    if let Err(error) = forge_windows_proxy_spike::client::run() {
        eprintln!("CLIENT ERROR: {error}");
        std::process::exit(1);
    }
}
