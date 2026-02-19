# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`reader-sync` is a single-binary Rust CLI that fetches all documents from the [Readwise Reader API v3](https://readwise.io/api/v3/list/) and upserts them into a PostgreSQL database. It handles pagination via cursor and respects rate-limit `Retry-After` headers.

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
# One-time dev setup (installs pre-commit, commitlint, sqlx-cli, etc.)
just dev-install

# Run locally (incremental sync from last checkpoint)
cargo run

# Full sync — bypass checkpoint and re-fetch everything
cargo run -- --full-sync

# Build release binary
cargo build --release

# Lint (pre-push hook uses -D warnings — treat warnings as errors)
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt

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

## Module Structure

```
src/
├── main.rs    — Args (clap), main(), sync loop orchestration
├── models.rs  — Category/Location enums, ReaderResult/ReaderResponse structs, custom deserializers
├── api.rs     — build_url(), get_reading() (HTTP + JSON parsing)
└── db.rs      — save(), load_checkpoint(), save_checkpoint()
```

## Key Architecture Details

### sqlx compile-time query verification

`sqlx::query!` macros verify SQL at compile time. This requires either:

- A live `DATABASE_URL` at compile time, **or**
- `SQLX_OFFLINE=true` with the `.sqlx/` directory containing cached query metadata

The Dockerfile and CI both set `SQLX_OFFLINE=true` and use the committed `.sqlx/` cache. When modifying queries, run `cargo sqlx prepare` to regenerate the cache before committing.

### Database schema

Three tables (see `migrations/`):

- `reading` — one row per Readwise document, upserted on `id`
- `sync_state` — single row (`id = 1`) storing `last_sync_at` timestamp for incremental syncs
- `history` — audit log of all changes to `reading`, populated by a PostgreSQL trigger (added in `20240304213214_track_changes.sql`)

The `reading` table uses two custom PostgreSQL ENUMs:

- `category`: article, email, epub, highlight, note, pdf, rss, tweet, video
- `location`: archive, feed, later, new, shortlist

Migrations run automatically at startup via `sqlx::migrate!()`.

### Sync flow

1. Connect to PostgreSQL, run pending migrations
2. Load checkpoint from `sync_state` (skipped on `--full-sync`)
3. Record `sync_started_at = Utc::now()` before fetching (avoids missing updates during sync)
4. Loop: GET `https://readwise.io/api/v3/list/?pageCursor=<cursor>&updatedAfter=<ts>`
5. For each result, `INSERT ... ON CONFLICT (id) DO UPDATE SET ...` (full upsert, not ignore)
6. Follow `nextPageCursor` until exhausted
7. Write `sync_started_at` back to `sync_state`

Individual document save failures are logged and counted but do **not** abort the sync — the loop continues and saves the checkpoint at the end.

### Custom deserializers (src/models.rs)

Three custom serde deserializers handle Readwise API quirks:

- `deserialize_published_date`: accepts Unix timestamp, ISO8601, or null (defaults to `None`). Has a known FIXME — it uses a generic fallback rather than explicitly handling each format.
- `deserialize_word_count`: defaults null to `0`
- `deserialize_title`: defaults null to `"Untitled"`

Also note: `location` on `ReaderResult` is `Option<Location>` (nullable in the API), but the DB column is non-nullable — the `as _` cast in `db.rs` lets sqlx handle the mapping.

The `tags` field is stored as raw `serde_json::Value` (JSONB in the DB) — structured tag import is a known TODO.

### CI/CD

Three GitHub Actions workflows in `.github/workflows/`:

- `ci.yml` — runs on PRs to `master`: parallel `fmt`, `clippy`, `build` jobs. All set `SQLX_OFFLINE=true` and install `mold` before compiling.
- `release.yml` — runs on push to `master`: uses `semantic-release` with `git-cliff` for changelogs and `semantic-release-cargo` to bump `Cargo.toml` version. Commits back `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` with `[skip ci]`.
- `publish.yml` — runs on GitHub release published: builds a static `x86_64-unknown-linux-musl` binary and attaches it to the release. Uses `CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS="-C target-cpu=native"` to override the mold linker (incompatible with musl).

Release config: `.releaserc.json`. Changelog template: `cliff.toml`.

### Pre-commit hooks

Hooks run on `pre-push` (configured in `.pre-commit-config.yaml`):

- `cargo fmt` — enforced on all Rust files
- `cargo clippy --all-targets --all-features -- -D warnings` — warnings are errors
- `commitlint` — enforces conventional commit messages on `commit-msg` stage
- `typos`, `taplo fmt`, and standard file checks also run
