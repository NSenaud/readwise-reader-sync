# readwise-reader-sync

Syncs documents from the [Readwise Reader API v3](https://readwise.io/api/v3/list/) into a PostgreSQL database. Runs as a short-lived CLI — invoke it on a schedule (cron, systemd timer, etc.) to keep your database up to date.

## Features

- Full or incremental sync via `--full-sync` flag
- Resumable: records a checkpoint after each successful run and only fetches documents updated since then
- Idempotent upserts — safe to run repeatedly
- Automatic retries on rate-limit (`429`) and server errors (`5xx`) with `Retry-After` header support
- Change history on the `reading` table via a PostgreSQL audit trigger

## Requirements

- Rust (stable)
- PostgreSQL
- A [Readwise access token](https://readwise.io/access_token)

Tool versions are pinned in `mise.toml`. Install with [mise](https://mise.jdx.dev/):

```bash
mise install
```

## Configuration

Create a `.env` file in the project root (or export the variables directly):

```
DATABASE_URL="postgres://postgres:password@localhost/readwise"
READWISE_ACCESS_TOKEN="<your token>"
RUST_LOG=info
```

If you use [direnv](https://direnv.net/), the included `.envrc` loads `.env` automatically.

## Usage

```bash
# Incremental sync (default — uses saved checkpoint)
cargo run

# Full sync — ignore checkpoint and re-fetch everything
cargo run -- --full-sync
```

On first run with an empty database, a full sync is performed automatically regardless of the flag.

## Database Schema

Migrations run automatically at startup. The schema consists of three tables:

| Table | Purpose |
| -- | -- |
| `reading` | One row per Readwise document |
| `sync_state` | Single-row checkpoint storing the last successful sync timestamp |
| `history` | Audit log of all changes to the `reading` table |

The `reading` table uses two PostgreSQL ENUM types: `category` (article, email, epub, highlight, note, pdf, rss, tweet, video) and `location` (archive, feed, later, new, shortlist).

## Development

```bash
# Install dev tooling (one-time)
just dev-install

# Run with live database
cargo run

# Lint (warnings treated as errors, matching CI)
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt

# After modifying any sqlx::query! macro: regenerate the offline cache
cargo sqlx prepare
```

The `.sqlx/` directory contains a pre-built query cache committed to the repository so that Docker builds work without a live database (`SQLX_OFFLINE=true`). Always run `cargo sqlx prepare` and commit the updated cache after changing any SQL query.

Pre-commit hooks (via [pre-commit](https://pre-commit.com/), run on push) enforce `cargo fmt`, `cargo clippy`, conventional commits, and TOML/Markdown formatting.

## Deployment

The project builds to a minimal Debian-based Docker image using a multi-stage build with [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) for layer caching. Images are pushed to Scaleway Container Registry.

```bash
just build   # prepare sqlx cache + build + tag image
just push    # push to registry
just all     # build + push
```

Images are tagged with the current git tag, or the short commit hash if no tag exists.
