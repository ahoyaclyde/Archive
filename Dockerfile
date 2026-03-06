# Build stage - using latest nightly for Rust 1.91+ and edition2024 support
FROM rustlang/rust:nightly AS builder

WORKDIR /app

# Copy everything
COPY . .

# Build for release
RUN cargo build --release

# Runtime stage - using Debian trixie for GLIBC 2.38+
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the built binary
COPY --from=builder /app/target/release/archive /app/archive

# Create directories
RUN mkdir -p /app/data

# Copy static files from builder
COPY --from=builder /app/static /app/static

# Set the binary as the entrypoint
ENTRYPOINT ["/app/archive"]