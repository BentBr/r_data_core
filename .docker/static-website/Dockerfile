FROM node:22-slim

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
