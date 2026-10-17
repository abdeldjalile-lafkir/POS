//! Dev helper: print an argon2 PHC hash for a PIN.
//!
//! Usage: `cargo run -p pos-store --example hash_pin -- 1234`

fn main() {
    let pin = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "1234".to_string());
    match pos_store::auth::hash_pin(&pin) {
        Ok(hash) => println!("{hash}"),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
