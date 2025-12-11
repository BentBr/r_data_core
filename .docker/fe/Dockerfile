FROM node:22-alpine

# Install build dependencies for sharp
RUN apk add --no-cache \
    python3 \
    make \
    g++ \
    vips-dev

WORKDIR /var/www/html
COPY fe/package*.json ./
RUN npm ci --include=dev
COPY fe/ ./
RUN rm -rf node_modules/.vite node_modules/.cache
RUN npm run build

EXPOSE 80
CMD ["npm", "run", "dev"]

