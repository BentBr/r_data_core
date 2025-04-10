FROM postgres:16-alpine

# Install build dependencies
RUN apk add --no-cache \
    build-base \
    git \
    postgresql-dev \
    clang \
    llvm

# Set environment variables to skip llvm optimizations
ENV NO_LLVM=1
ENV USE_PGXS=1

# Set up environment
ENV PGDATA /var/lib/postgresql/data
ENV PATH $PATH:/usr/local/pgsql/bin

# Try to install pg_uuidv7 extension (but it's OK if it fails)
RUN mkdir -p /tmp/pg_uuidv7 && \
    cd /tmp/pg_uuidv7 && \
    git clone https://github.com/fboulnois/pg_uuidv7.git . && \
    make || echo "Failed to compile extension, will use SQL fallback" && \
    make install || echo "Failed to install extension, will use SQL fallback" && \
    cd / && \
    rm -rf /tmp/pg_uuidv7

# Copy initialization scripts and fallback implementation
COPY init-uuid-extension.sql /docker-entrypoint-initdb.d/
COPY fallback-uuid-v7.sql /docker-entrypoint-initdb.d/
COPY entry-script.sh /docker-entrypoint-initdb.d/

# Make the entry script executable
RUN chmod +x /docker-entrypoint-initdb.d/entry-script.sh

# Set user back to postgres
USER postgres 