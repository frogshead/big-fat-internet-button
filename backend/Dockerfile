# Multi-stage build for minimal final image size

# Build stage
FROM rust:1.90-slim AS builder

WORKDIR /build

# Copy only dependency manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/
COPY firmware/Cargo.toml firmware/

# Create dummy source files to cache dependencies
RUN mkdir -p backend/src firmware/src && \
    echo "fn main() {}" > backend/src/main.rs && \
    echo "fn main() {}" > firmware/src/main.rs

# Build dependencies only
RUN cargo build --package backend --release && \
    rm -rf backend/src

# Copy actual source code
COPY backend/src backend/src
COPY backend/tests backend/tests

# Build the actual application
# Touch main.rs to force rebuild of our code (dependencies are cached)
RUN touch backend/src/main.rs && \
    cargo build --package backend --release

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates for HTTPS (if needed in future)
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/backend /app/backend

# Set ownership
RUN chown -R appuser:appuser /app

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 4000

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["/app/backend"]
