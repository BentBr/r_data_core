# Stage 1: Build the frontend once on the build machine architecture.
# The output assets are architecture-independent, so we don't need to run the Node build per target arch.
#
# OCI image metadata (used by GHCR package pages)
# Can be overridden by CI via --build-arg.
ARG OCI_SOURCE="https://github.com/bentbr/r_data_core"
ARG OCI_REVISION=""
ARG OCI_CREATED=""

FROM --platform=$BUILDPLATFORM node:22-alpine AS builder

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

ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="r-data-core-admin-fe" \
      org.opencontainers.image.description="Admin frontend for r_data_core, served by nginx." \
      org.opencontainers.image.source=$OCI_SOURCE \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.created=$OCI_CREATED

# Create nginx cache directories with proper permissions
RUN mkdir -p /var/cache/nginx/client_temp \
    /var/cache/nginx/proxy_temp \
    /var/cache/nginx/fastcgi_temp \
    /var/cache/nginx/uwsgi_temp \
    /var/cache/nginx/scgi_temp \
    && chown -R nginx:nginx /var/cache/nginx \
    && chmod -R 755 /var/cache/nginx

# Copy built assets and config with correct ownership
COPY --from=builder --chown=nginx:nginx /app/dist /usr/share/nginx/html
COPY .docker/fe/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]
