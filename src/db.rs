use anyhow::Result;
use chrono::{DateTime, Utc};
use log::debug;
use sqlx::postgres::{PgPool, PgQueryResult};

use crate::models::ReaderResult;

pub async fn save(pool: &PgPool, result: &ReaderResult) -> Result<PgQueryResult> {
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

pub async fn load_checkpoint(pool: &PgPool) -> Result<Option<DateTime<Utc>>> {
    let row = sqlx::query!("SELECT last_sync_at FROM sync_state WHERE id = 1")
        .fetch_one(pool)
        .await?;
    Ok(row.last_sync_at)
}

pub async fn save_checkpoint(pool: &PgPool, ts: &DateTime<Utc>) -> Result<()> {
    sqlx::query!(
        "INSERT INTO sync_state (id, last_sync_at) VALUES (1, $1)
         ON CONFLICT (id) DO UPDATE SET last_sync_at = EXCLUDED.last_sync_at",
        ts
    )
    .execute(pool)
    .await?;
    Ok(())
}
