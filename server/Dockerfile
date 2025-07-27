# === Build Stage ===
FROM rust:1.88.0-alpine AS builder

WORKDIR /app

# Install required build tools
RUN apk add --no-cache musl-dev build-base protobuf

# Pre-cache dependencies to speed up rebuilds
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -r src

# Copy full source
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY build.rs .
COPY proto ./proto

# Compile the application
RUN cargo build --release

COPY hiqlite.toml .


# === Runtime Stage ===
FROM alpine:3.20


# Install only needed runtime dependencies (if any)
RUN apk add --no-cache libstdc++ # required if app links dynamically to libstdc++

# Create a non-root user (optional but recommended)
RUN addgroup -S app && adduser -S app -G app
USER app

WORKDIR /app

COPY --from=builder /app/target/release/idempotency-server .
COPY --from=builder /app/hiqlite.toml .
COPY --from=builder /app/migrations migrations

ENTRYPOINT ["./idempotency-server"]
