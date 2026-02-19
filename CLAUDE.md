# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`reader-sync` is a single-binary Rust CLI that fetches all documents from the [Readwise Reader API v3](https://readwise.io/api/v3/list/) and upserts them into a PostgreSQL database. It handles pagination via cursor and respects rate-limit `Retry-After` headers.

The entire application logic lives in **`src/main.rs`** — there are no modules.

## Environment Setup

Requires a `.env` file (loaded automatically via `.envrc` if using direnv):

```
DATABASE_URL="postgres://postgres:password@localhost/readwise"
READWISE_ACCESS_TOKEN="<token>"
RUST_LOG=info
```

Tool versions are managed by **mise** (`mise.toml`): PostgreSQL 18.2, prek 0.3.3.

## Commands

```bash
# Run locally (requires DATABASE_URL and READWISE_ACCESS_TOKEN in env)
cargo run

# Build release binary
cargo build --release

# Lint
cargo clippy

# Run after schema changes: regenerate sqlx offline query cache
cargo sqlx prepare

# Install sqlx-cli (one-time dev setup)
cargo install --version=^0.7 sqlx-cli --no-default-features --features postgres
```

### Docker / Deployment (via Justfile)

```bash
just build   # cargo sqlx prepare + podman build + tag for Scaleway registry
just push    # push to rg.fr-par.scw.cloud/tooling/readwise-sync
just all     # build + push
```

## Key Architecture Details

### sqlx compile-time query verification

`sqlx::query!` macros verify SQL at compile time. This requires either:

- A live `DATABASE_URL` at compile time, **or**
- `SQLX_OFFLINE=true` with the `.sqlx/` directory containing cached query metadata

The Dockerfile sets `SQLX_OFFLINE=true` and uses the committed `.sqlx/` cache. When modifying queries, run `cargo sqlx prepare` to regenerate the cache before committing.

### Database schema

A single `reading` table (see `migrations/20240205220541_add_reading_table.sql`) with two custom PostgreSQL ENUMs:

- `category`: article, email, epub, highlight, note, pdf, rss, tweet, video
- `location`: archive, feed, later, new, shortlist

Migrations run automatically at startup via `sqlx::migrate!()`.

### Sync flow

1. Connect to PostgreSQL, run pending migrations
2. Loop: GET `https://readwise.io/api/v3/list/?pageCursor=<cursor>`
3. For each page of results, `INSERT ... ON CONFLICT DO NOTHING` (idempotent)
4. Follow `nextPageCursor` until exhausted

### Custom deserializers

Three custom serde deserializers handle Readwise API quirks:

- `deserialize_published_date`: accepts timestamp or ISO8601 or null
- `deserialize_word_count`: defaults null to `0`
- `deserialize_title`: defaults null to `"Untitled"`
