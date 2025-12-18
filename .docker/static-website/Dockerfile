#
# OCI image metadata (used by GHCR package pages)
# Can be overridden by CI via --build-arg.
ARG OCI_SOURCE="https://github.com/bentbr/r_data_core"
ARG OCI_REVISION=""
ARG OCI_CREATED=""

FROM node:22-slim

ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="r-data-core-static-website" \
      org.opencontainers.image.description="Static website dev image for r_data_core (runs the dev server at container start)." \
      org.opencontainers.image.source=$OCI_SOURCE \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.created=$OCI_CREATED

# Install build dependencies for sharp
RUN apt-get update && apt-get install -y \
    python3 \
    make \
    g++ \
    libvips-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /var/www/html

# Change ownership to node user and create node_modules directory
RUN chown -R node:node /var/www/html && mkdir -p /var/www/html/node_modules && chown -R node:node /var/www/html/node_modules

# Switch to non-root user
USER node

# Expose port 8080
EXPOSE 8080

# Install dependencies and start dev server at runtime
# This ensures dependencies are installed after the volume mount
CMD ["sh", "-c", "npm install && npm run dev"]
