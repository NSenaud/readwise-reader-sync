mod api;
mod db;
mod models;

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use log::{debug, error, info, warn};
use sqlx::postgres::PgPool;

#[derive(Parser)]
#[command(about = "Sync Readwise Reader documents to PostgreSQL")]
struct Args {
    /// Bypass the checkpoint and re-sync everything from the beginning
    #[arg(long, default_value_t = false)]
    full_sync: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    info!("Connecting to database...");
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;

    info!("Running migrations...");
    sqlx::migrate!().run(&pool).await?;

    let access_token = &dotenvy::var("READWISE_ACCESS_TOKEN")?;

    let updated_after = if args.full_sync {
        info!("Full sync requested — ignoring checkpoint.");
        None
    } else {
        match db::load_checkpoint(&pool).await? {
            Some(ts) => {
                info!("Resuming from checkpoint: {ts}");
                Some(ts)
            }
            None => {
                info!("No checkpoint found — performing full sync.");
                None
            }
        }
    };

    // Record start time before the sync so we don't miss documents
    // updated while the sync is in progress.
    let sync_started_at = Utc::now();

    let mut next_page_cursor: Option<String> = None;

    loop {
        info!("Requesting Readwise API...");
        let url = api::build_url(next_page_cursor.as_deref(), updated_after.as_ref());

        let page = api::get_reading(&url, access_token)?;

        next_page_cursor = page.next_page_cursor;

        info!("{} total items remaining", page.total_remaining);
        info!("Saving {} items to database...", page.results.len());

        let mut failures = 0usize;
        for result in page.results {
            match db::save(&pool, &result).await {
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

    db::save_checkpoint(&pool, &sync_started_at).await?;
    info!("Checkpoint saved: {sync_started_at}");

    Ok(())
}
