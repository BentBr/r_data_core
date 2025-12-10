# Deployment Guide

## Static File Serving

### Sitemaps & Robots.txt

The build process generates:
- `dist/sitemap.xml`
- `dist/sitemap_en.xml`
- `dist/sitemap_de.xml`
- `dist/robots.txt`

**Important**: In development, accessing these files directly (e.g., `http://website.rdatacore.docker/sitemap.xml`) will trigger Vue Router's catch-all route and redirect to home.

### Production Setup with Nginx

For production deployment, configure Nginx to serve static files before passing requests to the Vue app:

```nginx
server {
    listen 80;
    server_name rdatacore.eu www.rdatacore.eu;
    root /var/www/html/dist;
    index index.html;

    # Serve static files first
    location ~* \.(xml|txt|webp|png|jpg|jpeg|gif|ico|css|js|svg|woff|woff2|ttf|eot)$ {
        try_files $uri =404;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # Special handling for sitemaps and robots.txt
    location ~ ^/(sitemap.*\.xml|robots\.txt)$ {
        try_files $uri =404;
        add_header Content-Type text/xml;
        expires 1d;
    }

    # Responsive images
    location /images/ {
        try_files $uri =404;
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # Vue Router - all other requests
    location / {
        try_files $uri $uri/ /index.html;
    }
}
```

### Docker Production Setup

Update the Dockerfile to use nginx for production:

```dockerfile
# Build stage
FROM node:22-alpine AS build

RUN apk add --no-cache python3 make g++ vips-dev

WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

# Production stage
FROM nginx:alpine

# Copy built assets
COPY --from=build /app/dist /usr/share/nginx/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

## Language Routes

All internal navigation links must include the language prefix:

```typescript
// Good
<router-link :to="getLocalizedPath('/about')">About</router-link>

// Bad
<router-link to="/about">About</router-link>
```

The `getLocalizedPath` helper ensures language persistence:

```typescript
const getLocalizedPath = (path: string) => {
    const lang = (route.params.lang as string) || currentLanguage.value
    return `/${lang}${path === '/' ? '' : path}`
}
```

## Environment Variables

All environment variables must be prefixed with `VITE_` to be accessible in the client-side code.

### Required Variables

Create a `.env` file in the `static-website` directory:

```env
# Base URL for the website (used in sitemap generation and meta tags)
VITE_BASE_URL=https://rdatacore.eu

# Site name (appears in page titles and branding)
VITE_SITE_NAME=RDataCore

# Demo application URL (opened when clicking "Try Demo" button)
VITE_R_DATA_CORE_DEMO_URL=https://demo.rdatacore.eu

# API Documentation URLs
VITE_API_DOCS_URL=https://rdatacore.eu/api/docs/
VITE_ADMIN_API_DOCS_URL=https://rdatacore.eu/admin/api/docs/
```

### Development (Docker Compose)

For local development with Docker, update `compose.yaml`:

```yaml
node-static:
  # ...
  environment:
    - VITE_BASE_URL=http://website.rdatacore.docker
    - VITE_SITE_NAME=RDataCore
    - VITE_R_DATA_CORE_DEMO_URL=http://admin.rdatacore.docker
    - VITE_API_DOCS_URL=http://rdatacore.docker/api/docs/
    - VITE_ADMIN_API_DOCS_URL=http://rdatacore.docker/admin/api/docs/
```

### Production

For production deployment, set these in your hosting environment or build pipeline:

```bash
export VITE_BASE_URL=https://rdatacore.eu
export VITE_SITE_NAME=RDataCore
export VITE_R_DATA_CORE_DEMO_URL=https://demo.rdatacore.eu
export VITE_API_DOCS_URL=https://rdatacore.eu/api/docs/
export VITE_ADMIN_API_DOCS_URL=https://rdatacore.eu/admin/api/docs/
```

**Note**: Environment variables are embedded during build time. Rebuild the application when changing environment variables.

## Build Process

```bash
npm run build
```

This will:
1. Generate responsive WebP images (`scripts/generate-images.js`)
2. TypeScript compilation check
3. Vite build
4. Generate sitemaps and robots.txt (`scripts/generate-sitemap.js`)

Output: `dist/` directory ready for deployment

## SEO Checklist

- ✅ Sitemaps generated with hreflang tags
- ✅ robots.txt pointing to sitemap
- ✅ Meta tags (title, description) update with language
- ✅ Open Graph tags
- ✅ Twitter Cards
- ✅ Canonical URLs with language
- ✅ hreflang alternates for each page
- ✅ Responsive images with WebP
- ✅ Lazy loading for images

