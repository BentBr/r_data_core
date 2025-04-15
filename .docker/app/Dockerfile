FROM rust:1.86-slim AS builder

WORKDIR /usr/src/r_data_core

# Copy source code and manifests
COPY . .

# Build for release with sqlx offline mode
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
ENV RUST_LOG=info

# Expose the port
EXPOSE 80

# Run the binary
CMD ["r_data_core"] 