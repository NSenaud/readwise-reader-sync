use std::time::Duration;
use std::thread;

use anyhow::{bail, Result};
use chrono::{DateTime, Local, Utc};
use log::{debug, error, info, warn};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgQueryResult};

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
    author: Option<String>,
    category: Category,
    content: Option<String>,
    created_at: DateTime<Local>,
    id: String,
    image_url: Option<String>,
    location: Option<Location>,
    notes: Option<String>,
    parent_id: Option<String>,
    #[serde(deserialize_with = "deserialize_published_date")]
    published_date: Option<DateTime<Utc>>,
    reading_progress: f32,
    site_name: Option<String>,
    source: Option<String>,
    source_url: Option<String>,
    summary: Option<String>,
    // TODO: import strutured tags
    tags: Option<Value>,
    title: Option<String>,
    updated_at: Option<DateTime<Local>>,
    #[serde(rename = "url")]
    readwise_url: Option<String>,
    word_count: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReaderResponse {
    count: usize,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: Option<String>,
    results: Vec<ReaderResult>,
}

// FIXME: deserialize timestamp or ISO3339 dates
fn deserialize_published_date<'a, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'a> + Default,
    D: Deserializer<'a>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;

    Ok(T::deserialize(v).unwrap_or_default())
}

async fn save(pool: &PgPool, result: &ReaderResult) -> Result<PgQueryResult> {
    debug!("Processing: {result:?}");
    match sqlx::query!(
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
    }) {
        Ok(r) => Ok(r),
        Err(e) => bail!("Failed to save entry in database: {e}"),
    }
}

fn get_reading(url: &String, access_token: &String) -> Option<ureq::Response> {
    loop {
        let (response, wait_for) = match ureq::get(url)
            .set("Authorization", &format!("Token {access_token}"))
            .set("Content-Type", "application/json")
            .call()
        {
            Ok(r) => (Some(r), 0),
            Err(ureq::Error::Status(code, response)) => {
                warn!(
                    "Received code {code}, wait for {} seconds",
                    response.header("Retry-After").unwrap_or("undefined")
                );
                (None, str::parse(response.header("Retry-After").unwrap_or("0")).expect("Failed to parse Retry-After header"))
            },
            Err(e) => panic!("{}", e),
        };

        match response {
            None => thread::sleep(Duration::from_millis((wait_for * 1000) as u64)),
            _ => return response,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    info!("Connecting to database...");
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;

    info!("Running migrations...");
    sqlx::migrate!().run(&pool).await?;

    let access_token = &dotenvy::var("READWISE_ACCESS_TOKEN")?;

    let mut next_page_cursor = None;

    loop {
        info!("Requisting Readwise API...");
        let url = match next_page_cursor {
            None => "https://readwise.io/api/v3/list/".to_string(),
            Some(v) => format!("https://readwise.io/api/v3/list/?pageCursor={}", v),
        };

        let response: String = get_reading(&url, access_token).expect("Unexpected answer").into_string()?;

        // Some Deserializer.
        let jd = &mut serde_json::Deserializer::from_str(&response);

        let response: ReaderResponse = match serde_path_to_error::deserialize(jd) {
            Ok(v) => v,
            Err(err) => panic!("{} error for path {}", err, err.path()),
        };

        next_page_cursor = response.next_page_cursor;

        info!("{} items found", response.count);
        info!("Saving {} items to database...", response.results.len());

        for result in response.results {
            match save(&pool, &result).await {
                Ok(_) => debug!("{} sync", result.title.unwrap_or("Untitled".to_string())),
                Err(e) => error!(
                    "Failed to sync {}: {}",
                    result.title.unwrap_or("Untitled".to_string()),
                    e,
                ),
            }
        }

        if next_page_cursor.is_none() {
            break;
        }
    }

    Ok(())
}
