mod error;
mod models;

fn main() {
    if let Err(e) = run() {
        let json = serde_json::json!({
            "error": e.to_string(),
            "code": e.error_code(),
        });
        eprintln!("{}", json);
        std::process::exit(e.exit_code());
    }
}

fn run() -> error::Result<()> {
    Ok(())
}
