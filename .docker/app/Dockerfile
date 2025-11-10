# Build arguments
ARG RUST_VERSION=1.88.0
ARG BUILD_MODE=release
ARG BINARY_NAME=r_data_core
ARG RUST_LOG_LEVEL=warning

# Builder stage with optimized caching
FROM rust:${RUST_VERSION}-slim AS builder

# Re-declare build args inside this stage so they are available to RUN steps
ARG BUILD_MODE
ARG BINARY_NAME

LABEL org.opencontainers.image.title="R Data Core"
LABEL org.opencontainers.image.description="R Data Core Backend. Easy rust-backed performant MDM"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.authors="Bent Brüggemann <mail@bent-brueggemann.de>"
LABEL org.opencontainers.image.source="https://github.com/BentBr/r_data_core"
LABEL org.opencontainers.image.licenses="MIT"

WORKDIR /usr/src/r_data_core

# Install build dependencies required by Rust crates (compiler toolchain, OpenSSL, PostgreSQL headers, etc.)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for better caching
COPY Cargo.toml Cargo.lock ./

# Build dependencies only (this layer will be cached unless Cargo files change)
# Use BuildKit cache mounts for faster builds
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/r_data_core/target \
    if [ "${BUILD_MODE}" = "release" ]; then \
        cargo build --release --bin ${BINARY_NAME} || true; \
    else \
        cargo build --bin ${BINARY_NAME} || true; \
    fi

# Copy source code and migrations
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY .sqlx/ ./.sqlx/

# Build the actual binary (do not cache the target dir so artifacts persist in the layer)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    if [ "${BUILD_MODE}" = "release" ]; then \
        cargo build --release --bin ${BINARY_NAME}; \
    else \
        cargo build --bin ${BINARY_NAME}; \
    fi

# Final stage
FROM debian:bookworm-slim

# Redeclare build arguments for this stage
ARG BUILD_MODE
ARG BINARY_NAME
ARG RUST_LOG_LEVEL

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/r_data_core/target/${BUILD_MODE}/${BINARY_NAME} /usr/local/bin/

# Create a non-root user
RUN groupadd -r rdata && useradd -r -g rdata rdata
USER rdata

# Set environment variables
ENV RUST_LOG=${RUST_LOG_LEVEL}
ENV BINARY_NAME=${BINARY_NAME}

# Expose the port
EXPOSE 80

# Run the binary (use shell form to expand ENV variable)
CMD ["sh", "-c", "exec ${BINARY_NAME}"]
