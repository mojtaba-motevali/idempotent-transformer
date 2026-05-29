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
COPY flatbuffers ./flatbuffers

RUN cargo build --release

# ── Runtime Stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r app && useradd -r -g app app
USER app

WORKDIR /app

COPY --from=builder /app/target/release/idempotency-server ./idempotency-server
COPY --from=builder /app/migrations ./migrations

ENTRYPOINT ["./idempotency-server"]
