FROM node:22-alpine

# Install build dependencies for sharp
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    vips-dev

WORKDIR /var/www/html

# Copy package files
COPY static-website/package*.json ./

# Install dependencies
RUN npm install

# Copy the rest of the application
COPY static-website/ ./

# Build the application
RUN npm run build

# Expose port 80
EXPOSE 80

# Start development server (for production, you'd use nginx to serve dist/)
CMD ["npm", "run", "dev"]
