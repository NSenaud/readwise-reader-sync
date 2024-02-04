use std::env;
use std::error::Error;

use chrono::serde::ts_milliseconds_option;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
enum Category {
    #[serde(rename = "article")]
    Article,
    #[serde(rename = "email")]
    Email,
    #[serde(rename = "epub")]
    Epub,
    #[serde(rename = "highlight")]
    Highlight,
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "pdf")]
    Pdf,
    #[serde(rename = "rss")]
    Rss,
    #[serde(rename = "tweet")]
    Tweet,
    #[serde(rename = "video")]
    Video,
}

#[derive(Debug, Deserialize, Serialize)]
enum Location {
    #[serde(rename = "archive")]
    Archive,
    #[serde(rename = "feed")]
    Feed,
    #[serde(rename = "later")]
    Later,
    #[serde(rename = "new")]
    New,
    #[serde(rename = "shortlist")]
    Shortlist,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReaderResult {
    author: String,
    category: Category,
    content: Option<String>,
    created_at: DateTime<Local>,
    id: String,
    image_url: Option<String>,
    location: Location,
    notes: String,
    parent_id: Option<String>,
    #[serde(with = "ts_milliseconds_option")]
    published_date: Option<DateTime<Utc>>,
    reading_progress: f32,
    site_name: String,
    source: Option<String>,
    source_url: String,
    summary: Option<String>,
    tags: Value,
    title: String,
    updated_at: Option<DateTime<Local>>,
    url: String,
    word_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReaderResponse {
    count: usize,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: String,
    results: Vec<ReaderResult>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file.
    // Fails if .env file not found, not readable or invalid.
    dotenvy::dotenv()?;

    let access_token = env::var("READWISE_ACCESS_TOKEN")
        .expect("READWISE_ACCESS_TOKEN environment variable must be set");

    let response: ReaderResponse = ureq::get("https://readwise.io/api/v3/list/?location=later")
        .set("Authorization", &format!("Token {access_token}"))
        .set("Content-Type", "application/json")
        .call()?
        .into_json()?;

    println!("{response:?}");

    Ok(())
}
