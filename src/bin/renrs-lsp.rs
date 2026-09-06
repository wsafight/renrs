fn main() {
    if let Err(error) = renrs_editor::lsp::run_stdio() {
        eprintln!("renrs-lsp: {error}");
        std::process::exit(1);
    }
}
