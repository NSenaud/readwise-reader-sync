use anyhow::Result;
use chrono::serde::ts_milliseconds_option;
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgPool;

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "category", rename_all = "lowercase")]
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

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "location", rename_all = "lowercase")]
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
    // TODO: import strutured tags
    tags: Value,
    title: String,
    updated_at: Option<DateTime<Local>>,
    #[serde(rename = "url")]
    readwise_url: String,
    word_count: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReaderResponse {
    count: usize,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: String,
    results: Vec<ReaderResult>,
}

async fn save(pool: &PgPool, result: ReaderResult) {
    debug!("Processing: {result:?}");
    sqlx::query!(
        r#"
        INSERT INTO reading (
            id,
            author,
            category,
            content,
            created_at,
            image_url,
            location,
            notes,
            parent_id,
            published_date,
            reading_progress,
            readwise_url,
            site_name,
            source,
            source_url,
            summary,
            tags,
            title,
            updated_at,
            word_count
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
            $12, $13, $14, $15, $16, $17, $18, $19, $20
        )
        ON CONFLICT DO NOTHING
        "#,
        result.id,
        result.author,
        result.category as _,
        result.content,
        result.created_at,
        result.image_url,
        result.location as _,
        result.notes,
        result.parent_id,
        result.published_date,
        result.reading_progress,
        result.readwise_url,
        result.site_name,
        result.source,
        result.source_url,
        result.summary,
        result.tags,
        result.title,
        result.updated_at,
        result.word_count,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        error!("Failed to execute query: {:?}", e);
        e
    })
    .unwrap();
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    info!("Connecting to database...");
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;

    info!("Running migrations...");
    sqlx::migrate!().run(&pool).await?;

    let access_token = &dotenvy::var("READWISE_ACCESS_TOKEN")?;

    info!("Requisting Readwise API...");
    let response: ReaderResponse = ureq::get("https://readwise.io/api/v3/list/?location=later")
        .set("Authorization", &format!("Token {access_token}"))
        .set("Content-Type", "application/json")
        .call()?
        .into_json()?;

    info!("{} items found", response.count);
    info!("Saving {} items to database...", response.results.len());
    // println!("{response:?}");

    for result in response.results {
        save(&pool, result).await;
    }

    Ok(())
}
