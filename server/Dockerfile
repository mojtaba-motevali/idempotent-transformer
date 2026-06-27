# ── Build Stage ───────────────────────────────────────────────────────────────
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# protobuf-compiler  → tonic gRPC codegen
# fjall is pure Rust — no cmake/clang/libclang needed
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Pre-cache Cargo dependencies (layer reused unless Cargo.toml/lock changes)
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# Build the real binary
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY build.rs ./
COPY proto ./proto

RUN cargo build --release

# ── Runtime Stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r app && useradd -r -g app app

WORKDIR /app

COPY --from=builder /app/target/release/idempotency-server ./idempotency-server
COPY --from=builder /app/migrations ./migrations

# Pre-create the data dir owned by `app`. A fresh named volume mounted at
# /app/data inherits this ownership, so the non-root process can write its
# fjall keyspace (raft-log). Without this the volume is root-owned → EACCES.
RUN mkdir -p /app/data && chown -R app:app /app/data

USER app

ENTRYPOINT ["./idempotency-server"]
