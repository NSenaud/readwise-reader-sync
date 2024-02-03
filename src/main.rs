use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file.
    // Fails if .env file not found, not readable or invalid.
    dotenvy::dotenv()?;

    let access_token = env::var("READWISE_ACCESS_TOKEN")
        .expect("READWISE_ACCESS_TOKEN environment variable must be set");

    let body: String = ureq::get("https://readwise.io/api/v3/list/?location=later")
        .set("Authorization", &format!("Token {access_token}"))
        .set("Content-Type", "application/json")
        .call()?
        .into_string()?;

    println!("{body}");

    Ok(())
}
