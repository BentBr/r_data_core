FROM postgres:16-alpine

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