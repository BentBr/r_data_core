# syntax=docker/dockerfile:1

# ==============================================================================
# Multi-stage Dockerfile for r_data_core binaries
# Supports multi-architecture builds (amd64, arm64)
#
# Build args:
#   BINARY - Which binary to build (r_data_core, r_data_core_worker, r_data_core_maintenance)
#
# Example:
#   docker build --build-arg BINARY=r_data_core -t r-data-core:latest .
# ==============================================================================

ARG RUST_VERSION=1.92.0
ARG DEBIAN_VERSION=bookworm

# OCI image metadata (used by GHCR package pages)
# Can be overridden by CI via --build-arg.
ARG OCI_SOURCE="https://github.com/bentbr/r_data_core"
ARG OCI_REVISION=""
ARG OCI_CREATED=""

# ==============================================================================
# Chef stage - compute recipe for dependency caching
# ==============================================================================
FROM rust:${RUST_VERSION} AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# ==============================================================================
# Planner stage - create dependency recipe
# ==============================================================================
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==============================================================================
# Builder stage - compile binaries
# ==============================================================================
FROM chef AS builder

# Install sqlx-cli for offline mode preparation
RUN cargo install sqlx-cli --no-default-features --features postgres --locked

# Copy and cook dependencies first (cached layer)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY . .

# Build all binaries in release mode with SQLx offline mode
ENV SQLX_OFFLINE=true
RUN cargo build --release --workspace

# ==============================================================================
# Runtime stage - minimal image with single binary
# ==============================================================================
FROM debian:${DEBIAN_VERSION}-slim AS runtime

ARG BINARY=r_data_core
ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="r-data-core-runtime" \
      org.opencontainers.image.description="Generic runtime image for a single r_data_core binary (set via BINARY build-arg)." \
      org.opencontainers.image.source=$OCI_SOURCE \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.created=$OCI_CREATED

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --user-group appuser
USER appuser
WORKDIR /home/appuser

# Copy the specified binary
COPY --from=builder --chown=appuser:appuser /app/target/release/${BINARY} /usr/local/bin/app

# Set default command
ENTRYPOINT ["/usr/local/bin/app"]

# Health check placeholder - override in docker-compose or k8s
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["true"]

# ==============================================================================
# Main app image - includes helper utilities
# ==============================================================================
FROM debian:${DEBIAN_VERSION}-slim AS r_data_core

ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="r-data-core" \
      org.opencontainers.image.description="r_data_core backend API service with utility binaries." \
      org.opencontainers.image.source=$OCI_SOURCE \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.created=$OCI_CREATED

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --user-group appuser
USER appuser
WORKDIR /home/appuser

# Copy main binary
COPY --from=builder --chown=appuser:appuser /app/target/release/r_data_core /usr/local/bin/r_data_core

# Copy utility binaries for operations/maintenance tasks
# These can be run via: docker compose exec core /usr/local/bin/<binary>
COPY --from=builder --chown=appuser:appuser /app/target/release/run_migrations /usr/local/bin/run_migrations
COPY --from=builder --chown=appuser:appuser /app/target/release/clear_cache /usr/local/bin/clear_cache
COPY --from=builder --chown=appuser:appuser /app/target/release/hash_password /usr/local/bin/hash_password

ENTRYPOINT ["/usr/local/bin/r_data_core"]

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["true"]

# ==============================================================================
# Worker image
# ==============================================================================
FROM debian:${DEBIAN_VERSION}-slim AS r_data_core_worker

ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="r-data-core-worker" \
      org.opencontainers.image.description="r_data_core background worker service." \
      org.opencontainers.image.source=$OCI_SOURCE \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.created=$OCI_CREATED

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --user-group appuser
USER appuser
WORKDIR /home/appuser

COPY --from=builder --chown=appuser:appuser /app/target/release/r_data_core_worker /usr/local/bin/r_data_core_worker

ENTRYPOINT ["/usr/local/bin/r_data_core_worker"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["true"]

# ==============================================================================
# Maintenance image
# ==============================================================================
FROM debian:${DEBIAN_VERSION}-slim AS r_data_core_maintenance

ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="r-data-core-maintenance" \
      org.opencontainers.image.description="r_data_core maintenance/ops image for administrative and one-off tasks." \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.created="${OCI_CREATED}"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --user-group appuser
USER appuser
WORKDIR /home/appuser

COPY --from=builder --chown=appuser:appuser /app/target/release/r_data_core_maintenance /usr/local/bin/r_data_core_maintenance

ENTRYPOINT ["/usr/local/bin/r_data_core_maintenance"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["true"]
