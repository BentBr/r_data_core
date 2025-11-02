FROM rust:1.86-slim AS builder

LABEL org.opencontainers.image.title="R Data Core Backend"
LABEL org.opencontainers.image.description="R Data Core Backend. Easy rust-backed performant MDM"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.authors="Bent Brüggemann <mail@bent-brueggemann.de>"
LABEL org.opencontainers.image.source="https://github.com/BentBr/r_data_core"
LABEL org.opencontainers.image.licenses="MIT"

WORKDIR /usr/src/r_data_core

# Copy source code and manifests
COPY . .

# Build for release
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /usr/src/r_data_core/target/release/r_data_core /usr/local/bin/

# Create a non-root user
RUN groupadd -r rdata && useradd -r -g rdata rdata
USER rdata

# Set environment variables
ENV RUST_LOG=warning

# Expose the port
EXPOSE 80

# Run the binary
CMD ["r_data_core"]
