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

# Copy built assets and config with correct ownership
COPY --from=builder --chown=nginx:nginx /app/dist /usr/share/nginx/html
COPY .docker/fe/nginx.conf /etc/nginx/conf.d/default.conf

# Switch to non-root user
USER nginx

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]
