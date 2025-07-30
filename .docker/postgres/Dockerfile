FROM postgres:16-alpine

LABEL org.opencontainers.image.title="R Data Core Postgres Database"
LABEL org.opencontainers.image.description="R Data Core Postgres Database with UUID v7 support"
LABEL org.opencontainers.image.version="1.0.0"
LABEL org.opencontainers.image.authors="Bent Brüggemann <mail@bent-brueggemann.de>"
LABEL org.opencontainers.image.source="https://github.com/BentBr/r_data_core"
LABEL org.opencontainers.image.licenses="MIT"

# Install build dependencies
RUN apk add --no-cache \
    build-base \
    git \
    postgresql-dev \
    clang \
    llvm

# Copy our custom entrypoint script
COPY docker-entrypoint-custom.sh /docker-entrypoint-custom.sh
RUN chmod +x /docker-entrypoint-custom.sh

# Set user back to postgres
USER postgres

# Use our custom entrypoint
ENTRYPOINT ["/docker-entrypoint-custom.sh"]
CMD ["postgres"]
