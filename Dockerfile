FROM lukemathwalker/cargo-chef:latest-rust-1.76 AS chef
WORKDIR /app


FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin reader-sync


FROM debian:bookworm-slim AS runtime
WORKDIR /app

ENV APP_ENVIRONMENT production

# Install OpenSSL - it is dynamically linked by some of our dependencies
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/reader-sync reader-sync

ENTRYPOINT ["./reader-sync"]
