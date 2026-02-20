use chrono::{DateTime, NaiveDate, Utc};
use log::warn;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "category", rename_all = "lowercase")]
pub enum Category {
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
pub enum Location {
    Archive,
    Feed,
    Later,
    New,
    Shortlist,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReaderResult {
    pub author: Option<String>,
    pub category: Category,
    pub content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub id: String,
    pub image_url: Option<String>,
    pub location: Option<Location>,
    pub notes: Option<String>,
    pub parent_id: Option<String>,
    #[serde(deserialize_with = "deserialize_published_date")]
    pub published_date: Option<DateTime<Utc>>,
    pub reading_progress: f32,
    pub site_name: Option<String>,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub summary: Option<String>,
    // TODO: import structured tags
    pub tags: Option<Value>,
    #[serde(deserialize_with = "deserialize_title")]
    pub title: String,
    pub updated_at: Option<DateTime<Utc>>,
    #[serde(rename = "url")]
    pub readwise_url: Option<String>,
    #[serde(deserialize_with = "deserialize_word_count")]
    pub word_count: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReaderResponse {
    #[serde(rename = "count")]
    pub total_remaining: usize,
    #[serde(rename = "nextPageCursor")]
    pub next_page_cursor: Option<String>,
    pub results: Vec<ReaderResult>,
}

/// Deserialize `published_date` from the Readwise API.
///
/// The API returns one of:
/// - `null` → `None`
/// - A Unix timestamp integer (seconds) → converted to `DateTime<Utc>`
/// - A full ISO 8601 / RFC 3339 datetime string → parsed directly
/// - A date-only string like `"2026-01-30"` → treated as midnight UTC
pub fn deserialize_published_date<'de, D>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    match &v {
        Value::Null => Ok(None),
        Value::Number(n) => {
            let ts = n.as_i64().ok_or_else(|| {
                serde::de::Error::custom(format!("invalid timestamp number: {n}"))
            })?;
            DateTime::from_timestamp(ts, 0)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom(format!("timestamp out of range: {ts}")))
        }
        Value::String(s) => {
            // Try full datetime first, then fall back to date-only (midnight UTC).
            if let Ok(dt) = s.parse::<DateTime<Utc>>() {
                return Ok(Some(dt));
            }
            if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                return Ok(Some(date.and_hms_opt(0, 0, 0).unwrap().and_utc()));
            }
            warn!("Failed to parse published_date string {s:?}. Defaulting to None.");
            Ok(None)
        }
        other => {
            warn!("Unexpected published_date value: {other:?}. Defaulting to None.");
            Ok(None)
        }
    }
}

/// Deserialize word_count as i32 or default to 0 if the value is null.
pub fn deserialize_word_count<'a, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'a>,
{
    Deserialize::deserialize(deserializer).map(|x: Option<_>| x.unwrap_or(0))
}

/// Deserialize title as String or default to "Untitled".
pub fn deserialize_title<'a, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'a>,
{
    Deserialize::deserialize(deserializer)
        .map(|x: Option<_>| x.unwrap_or_else(|| String::from("Untitled")))
}
