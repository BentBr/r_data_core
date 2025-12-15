# Stage 1: Build the frontend
FROM node:22-alpine AS builder

# Install build dependencies for sharp
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    vips-dev

WORKDIR /app
COPY fe/package*.json ./
RUN npm ci
COPY fe/ ./
RUN rm -rf node_modules/.vite node_modules/.cache
RUN npm run build

# Stage 2: Serve with nginx
FROM nginx:alpine AS production

# Copy built assets from builder stage
COPY --from=builder /app/dist /usr/share/nginx/html

# Copy nginx configuration
COPY .docker/fe/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
