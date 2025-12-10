FROM node:22-alpine

# Install build dependencies for sharp
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    vips-dev

WORKDIR /var/www/html

# Copy package files first for better caching
COPY static-website/package*.json ./
RUN npm ci --include=dev

# Copy source files
COPY static-website/ ./

# Clean any cached build artifacts to avoid esbuild version mismatches
RUN rm -rf node_modules/.vite node_modules/.cache

# Build the application
RUN npm run build

# Expose port 80
EXPOSE 80

# Start development server (for production, you'd use nginx to serve dist/)
CMD ["npm", "run", "dev"]
