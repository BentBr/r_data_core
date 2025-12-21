#!/bin/sh
# Docker entrypoint script for admin frontend
# Generates runtime configuration from environment variables

set -e

CONFIG_FILE="/usr/share/nginx/html/config.js"

# Generate config.js from environment variables
cat > "$CONFIG_FILE" <<EOF
// Runtime configuration injected at container startup
// This file is generated from environment variables by docker-entrypoint.sh
window.__APP_CONFIG__ = {
EOF

# Add VITE_API_BASE_URL (required)
if [ -n "$VITE_API_BASE_URL" ]; then
    echo "    VITE_API_BASE_URL: '${VITE_API_BASE_URL}'," >> "$CONFIG_FILE"
else
    echo "    // VITE_API_BASE_URL not set - will fall back to build-time config or window.location.origin" >> "$CONFIG_FILE"
fi

# Add VITE_ADMIN_BASE_URL (optional)
if [ -n "$VITE_ADMIN_BASE_URL" ]; then
    echo "    VITE_ADMIN_BASE_URL: '${VITE_ADMIN_BASE_URL}'," >> "$CONFIG_FILE"
fi

# Add VITE_DEV_MODE (optional)
if [ -n "$VITE_DEV_MODE" ]; then
    echo "    VITE_DEV_MODE: '${VITE_DEV_MODE}'," >> "$CONFIG_FILE"
fi

# Add VITE_ENABLE_API_LOGGING (optional)
if [ -n "$VITE_ENABLE_API_LOGGING" ]; then
    echo "    VITE_ENABLE_API_LOGGING: '${VITE_ENABLE_API_LOGGING}'," >> "$CONFIG_FILE"
fi

# Add VITE_DEFAULT_PAGE_SIZE (optional)
if [ -n "$VITE_DEFAULT_PAGE_SIZE" ]; then
    echo "    VITE_DEFAULT_PAGE_SIZE: '${VITE_DEFAULT_PAGE_SIZE}'," >> "$CONFIG_FILE"
fi

# Close the config object
echo "};" >> "$CONFIG_FILE"

# Set proper permissions
chmod 644 "$CONFIG_FILE"

# Start nginx
exec nginx -g "daemon off;"

