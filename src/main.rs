use std::thread;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgQueryResult};

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "category", rename_all = "lowercase")]
enum Category {
    Article,
    Email,
    Epub,
    Highlight,
    Note,
    Pdf,
    Rss,
    Tweet,
    Video,
}

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "location", rename_all = "lowercase")]
enum Location {
    Archive,
    Feed,
    Later,
    New,
    Shortlist,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReaderResult {
    author: Option<String>,
    category: Category,
    content: Option<String>,
    created_at: DateTime<Utc>,
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
    // TODO: import structured tags
    tags: Option<Value>,
    #[serde(deserialize_with = "deserialize_title")]
    title: String,
    updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "url")]
    readwise_url: Option<String>,
    #[serde(deserialize_with = "deserialize_word_count")]
    word_count: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReaderResponse {
    #[serde(rename = "count")]
    total_remaining: usize,
    #[serde(rename = "nextPageCursor")]
    next_page_cursor: Option<String>,
    results: Vec<ReaderResult>,
}

// FIXME: handle Unix timestamp integers and ISO 8601 strings explicitly
fn deserialize_published_date<'a, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'a> + Default,
    D: Deserializer<'a>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    Ok(T::deserialize(v.clone()).unwrap_or_else(|e| {
        warn!("Failed to deserialize published_date (value: {v:?}): {e}. Defaulting to None.");
        T::default()
    }))
}

/// Deserialize word_count as i32 or default to 0 if the value is null.
fn deserialize_word_count<'a, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'a>,
{
    Deserialize::deserialize(deserializer).map(|x: Option<_>| x.unwrap_or(0))
}

/// Deserialize title as String or default to "Untitled".
fn deserialize_title<'a, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'a>,
{
    Deserialize::deserialize(deserializer)
        .map(|x: Option<_>| x.unwrap_or_else(|| String::from("Untitled")))
}

async fn save(pool: &PgPool, result: &ReaderResult) -> Result<PgQueryResult> {
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
        ON CONFLICT (id) DO UPDATE SET
            author           = EXCLUDED.author,
            content          = EXCLUDED.content,
            image_url        = EXCLUDED.image_url,
            location         = EXCLUDED.location,
            notes            = EXCLUDED.notes,
            published_date   = EXCLUDED.published_date,
            reading_progress = EXCLUDED.reading_progress,
            site_name        = EXCLUDED.site_name,
            source           = EXCLUDED.source,
            source_url       = EXCLUDED.source_url,
            summary          = EXCLUDED.summary,
            tags             = EXCLUDED.tags,
            title            = EXCLUDED.title,
            updated_at       = EXCLUDED.updated_at,
            word_count       = EXCLUDED.word_count
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
        anyhow::anyhow!(
            "Failed to save '{:?}' (id={:?}, source_url={:?}): {e}",
            result.title,
            result.id,
            result.source_url
        )
    })
}

fn get_reading(url: &str, access_token: &str) -> Result<ureq::Response> {
    loop {
        match ureq::get(url)
            .set("Authorization", &format!("Token {access_token}"))
            .set("Content-Type", "application/json")
            .call()
        {
            Ok(response) => return Ok(response),
            Err(ureq::Error::Status(code, response)) if code == 429 || code >= 500 => {
                let retry_after: u64 = response
                    .header("Retry-After")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| {
                        warn!(
                            "Missing or unparsable Retry-After header for HTTP {code}. Defaulting to 60s."
                        );
                        60
                    });
                warn!("Received HTTP {code}, retrying after {retry_after}s");
                thread::sleep(Duration::from_secs(retry_after));
            }
            Err(ureq::Error::Status(code, _)) => {
                anyhow::bail!("Non-retryable HTTP error {code} from Readwise API");
            }
            Err(ureq::Error::Transport(e)) => {
                error!("Network transport error: {e}. Retrying in 30s.");
                thread::sleep(Duration::from_secs(30));
            }
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

    let mut next_page_cursor: Option<String> = None;

    loop {
        info!("Requesting Readwise API...");
        let url = match &next_page_cursor {
            None => "https://readwise.io/api/v3/list/".to_string(),
            Some(cursor) => format!("https://readwise.io/api/v3/list/?pageCursor={cursor}"),
        };

        let body: String = get_reading(&url, access_token)?.into_string()?;

        let jd = &mut serde_json::Deserializer::from_str(&body);

        let page: ReaderResponse = serde_path_to_error::deserialize(jd).map_err(|err| {
            error!(
                "Failed to deserialize API response at '{}': {err}. Raw body: {body}",
                err.path()
            );
            err
        })?;

        next_page_cursor = page.next_page_cursor;

        info!("{} total items remaining", page.total_remaining);
        info!("Saving {} items to database...", page.results.len());

        let mut failures = 0usize;
        for result in page.results {
            match save(&pool, &result).await {
                Ok(_) => debug!("Synced: {}", result.title),
                Err(e) => {
                    error!("{e}");
                    failures += 1;
                }
            }
        }
        if failures > 0 {
            warn!("{failures} document(s) failed to save on this page");
        }

        if next_page_cursor.is_none() {
            break;
        }
    }

    Ok(())
}
