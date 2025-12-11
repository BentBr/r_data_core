FROM node:22-slim

# Install build dependencies for sharp
RUN apt-get update && apt-get install -y \
    python3 \
    make \
    g++ \
    libvips-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /var/www/html

# Expose port 80
EXPOSE 80

# Install dependencies and start dev server at runtime
# This ensures dependencies are installed after the volume mount
CMD ["sh", "-c", "npm install && npm run dev"]
