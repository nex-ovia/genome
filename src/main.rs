// genome — the nexovia CLI. Thin entrypoint; all work lives in the library.
fn main() {
    if let Err(e) = nexovia::cli::run(std::env::args().skip(1)) {
        eprintln!("genome: {e}");
        std::process::exit(1);
    }
}
