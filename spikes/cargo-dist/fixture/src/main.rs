fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("forge {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    println!("forge cargo-dist spike fixture");
}
