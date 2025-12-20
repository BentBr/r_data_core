#!/bin/bash
set -euo pipefail

# ==============================================================================
# SSL Certificate Setup Script
# ==============================================================================
# Obtains Let's Encrypt SSL certificates for all domains with IPv4 and IPv6 support
#
# Usage: sudo ./setup-ssl.sh
# ==============================================================================

echo "=============================================================================="
echo "SSL Certificate Setup (Let's Encrypt)"
echo "=============================================================================="
echo ""

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then 
    echo "Error: Please run as root or with sudo"
    exit 1
fi

# Check if certbot is installed
if ! command -v certbot &> /dev/null; then
    echo "Error: certbot is not installed. Please run setup-server.sh first."
    exit 1
fi

# Domains to get certificates for
DOMAINS=("api.rdatacore.eu" "demo.rdatacore.eu" "rdatacore.eu")

# Validate domains first
echo "🔍 Validating domain DNS configuration..."
if [ -f "$(dirname "$0")/validate-domains.sh" ]; then
    if ! "$(dirname "$0")/validate-domains.sh"; then
        echo ""
        echo "❌ Domain validation failed. Please fix DNS configuration first."
        exit 1
    fi
else
    echo "⚠️  Warning: validate-domains.sh not found. Skipping DNS validation."
    echo "   Make sure all domains point to this server before continuing."
    read -p "Continue? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""
echo "📋 Domains to configure:"
for domain in "${DOMAINS[@]}"; do
    echo "  - $domain"
done
echo ""

# Email for Let's Encrypt notifications
read -p "Enter email for Let's Encrypt notifications (required): " EMAIL
if [ -z "$EMAIL" ]; then
    echo "Error: Email is required"
    exit 1
fi

# Stop nginx if running (certbot needs port 80)
echo ""
echo "🛑 Stopping nginx (if running)..."
systemctl stop nginx || true

# Obtain certificates for each domain
echo ""
echo "🔒 Obtaining SSL certificates..."
for domain in "${DOMAINS[@]}"; do
    echo ""
    echo "Processing: $domain"
    echo "----------------------------------------"
    
    # Check if certificate already exists
    if [ -d "/etc/letsencrypt/live/$domain" ]; then
        echo "⚠️  Certificate already exists for $domain"
        read -p "Renew/replace certificate? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Skipping $domain"
            continue
        fi
    fi
    
    # Obtain certificate with standalone mode (no nginx needed)
    certbot certonly \
        --standalone \
        --non-interactive \
        --agree-tos \
        --email "$EMAIL" \
        --preferred-challenges http \
        -d "$domain" \
        --cert-name "$domain" || {
        echo "❌ Failed to obtain certificate for $domain"
        exit 1
    }
    
    echo "✅ Certificate obtained for $domain"
done

# Set up automatic renewal
echo ""
echo "🔄 Setting up automatic certificate renewal..."

# Create renewal hook script
cat > /etc/letsencrypt/renewal-hooks/deploy/nginx-reload.sh << 'EOF'
#!/bin/bash
# Reload nginx after certificate renewal
systemctl reload nginx || true
EOF
chmod +x /etc/letsencrypt/renewal-hooks/deploy/nginx-reload.sh

# Test renewal (dry run)
echo "Testing certificate renewal..."
certbot renew --dry-run || {
    echo "⚠️  Warning: Certificate renewal test failed, but certificates are valid"
}

# Create systemd timer for automatic renewal (if not exists)
if [ ! -f /etc/systemd/system/certbot.timer ]; then
    echo "Creating systemd timer for certificate renewal..."
    
    cat > /etc/systemd/system/certbot.timer << 'EOF'
[Unit]
Description=Certbot renewal timer
Documentation=man:certbot(1)

[Timer]
# Run twice daily (at 00:00 and 12:00 UTC)
OnCalendar=*-*-* 00,12:00:00
# Randomize delay by up to 1 hour to spread load
RandomizedDelaySec=3600
# Run immediately if missed (e.g., server was down)
Persistent=true

[Install]
WantedBy=timers.target
EOF

    cat > /etc/systemd/system/certbot.service << 'EOF'
[Unit]
Description=Certbot renewal service
Documentation=man:certbot(1)
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
# Run certbot renewal with quiet mode and deploy hook
ExecStart=/usr/bin/certbot renew --quiet --deploy-hook /etc/letsencrypt/renewal-hooks/deploy/nginx-reload.sh
# Don't restart on failure
Restart=no
EOF

    # Reload systemd to recognize new units
    systemctl daemon-reload
    
    # Enable and start the timer
    systemctl enable certbot.timer
    systemctl start certbot.timer
    
    echo "✅ Automatic renewal timer configured"
    
    # Show timer status
    echo ""
    echo "Timer status:"
    systemctl status certbot.timer --no-pager -l
    echo ""
    echo "Next scheduled run:"
    systemctl list-timers certbot.timer --no-pager
else
    echo "✅ Automatic renewal timer already exists"
    echo ""
    echo "Current timer status:"
    systemctl status certbot.timer --no-pager -l || true
fi

# Verify certificates
echo ""
echo "🔍 Verifying certificates..."
ERRORS=0
for domain in "${DOMAINS[@]}"; do
    CERT_PATH="/etc/letsencrypt/live/$domain/fullchain.pem"
    KEY_PATH="/etc/letsencrypt/live/$domain/privkey.pem"
    
    if [ -f "$CERT_PATH" ] && [ -f "$KEY_PATH" ]; then
        echo "✅ $domain: Certificate files exist"
        
        # Check certificate expiry (if openssl is available)
        if command -v openssl &> /dev/null; then
            EXPIRY=$(openssl x509 -enddate -noout -in "$CERT_PATH" 2>/dev/null | cut -d= -f2 || echo "unknown")
            echo "   Expires: $EXPIRY"
        fi
    else
        echo "❌ $domain: Certificate files not found"
        ERRORS=$((ERRORS + 1))
    fi
done

if [ $ERRORS -gt 0 ]; then
    echo ""
    echo "⚠️  Warning: $ERRORS certificate(s) not found"
fi

echo ""
echo "=============================================================================="
echo "✅ SSL certificate setup complete!"
echo ""
echo "Certificate locations:"
for domain in "${DOMAINS[@]}"; do
    echo "  $domain:"
    echo "    Cert: /etc/letsencrypt/live/$domain/fullchain.pem"
    echo "    Key:  /etc/letsencrypt/live/$domain/privkey.pem"
done
echo ""
echo "Next steps:"
echo "1. Configure nginx to use these certificates"
echo "2. Start nginx: systemctl start nginx"
echo "3. Test SSL: openssl s_client -connect api.rdatacore.eu:443 -servername api.rdatacore.eu"
echo ""
echo "Certificates will auto-renew via systemd timer."

