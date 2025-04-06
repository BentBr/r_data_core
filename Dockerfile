FROM rust:1.76-slim as builder

WORKDIR /usr/src/r_data_core

# Create a blank project
RUN cargo new --bin r_data_core

# Copy over manifests
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Cache dependencies
WORKDIR /usr/src/r_data_core/r_data_core
RUN cargo build --release
RUN rm src/*.rs

# Copy source code
WORKDIR /usr/src/r_data_core
COPY ./src ./src

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
ENV RUST_LOG=info

# Expose the port
EXPOSE 8080

# Run the binary
CMD ["r_data_core"] 