#!/bin/bash
set -euo pipefail

# ==============================================================================
# Server Setup Script for Demo Deployment
# ==============================================================================
# This script sets up a blank server with all required software for the
# demo deployment. This is NOT a production setup - only k8s is production.
#
# Usage: sudo ./setup-server.sh
# ==============================================================================

echo "=============================================================================="
echo "⚠️  DEMO SERVER SETUP - NOT FOR PRODUCTION"
echo "=============================================================================="
echo "This script sets up a demo server. True production requires Kubernetes."
echo ""

# Check if running as root or with sudo
if [ "$EUID" -ne 0 ]; then 
    echo "Error: Please run as root or with sudo"
    exit 1
fi

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VER=$VERSION_ID
else
    echo "Error: Cannot detect OS"
    exit 1
fi

echo "Detected OS: $OS $VER"
echo ""

# Update system packages
echo "📦 Updating system packages..."
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get upgrade -y
apt-get install -y \
    curl \
    wget \
    git \
    ca-certificates \
    gnupg \
    lsb-release \
    ufw \
    net-tools \
    dnsutils \
    iputils-ping

# Install Docker
echo ""
echo "🐳 Installing Docker..."
if ! command -v docker &> /dev/null; then
    # Add Docker's official GPG key
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/${OS}/gpg | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    chmod a+r /etc/apt/keyrings/docker.gpg

    # Set up repository
    echo \
      "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/${OS} \
      $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
      tee /etc/apt/sources.list.d/docker.list > /dev/null

    apt-get update
    apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
else
    echo "Docker already installed"
fi

# Enable and start Docker
systemctl enable docker
systemctl start docker

# Verify Docker installation
docker --version
docker compose version

# Install nginx
echo ""
echo "🌐 Installing nginx..."
if ! command -v nginx &> /dev/null; then
    apt-get install -y nginx
else
    echo "nginx already installed"
fi

# Stop nginx initially (we'll configure it later)
systemctl stop nginx || true
systemctl disable nginx || true

# Install certbot
echo ""
echo "🔒 Installing certbot..."
if ! command -v certbot &> /dev/null; then
    apt-get install -y certbot python3-certbot-nginx
else
    echo "certbot already installed"
fi

# Configure firewall
echo ""
echo "🔥 Configuring firewall (ufw)..."

# Check if UFW is installed
if ! command -v ufw &> /dev/null; then
    echo "Installing ufw..."
    apt-get install -y ufw
fi

# Check current UFW status
UFW_STATUS=$(ufw status | head -n1 | awk '{print $2}')
echo "Current UFW status: $UFW_STATUS"

# Check for existing rules (use head -1 to get first match only, avoid multiline issues)
EXISTING_SSH=$(ufw status numbered 2>/dev/null | grep -c "22/tcp" | head -1 || echo "0")
EXISTING_HTTP=$(ufw status numbered 2>/dev/null | grep -c "80/tcp" | head -1 || echo "0")
EXISTING_HTTPS=$(ufw status numbered 2>/dev/null | grep -c "443/tcp" | head -1 || echo "0")
EXISTING_NYDUS=$(ufw status numbered 2>/dev/null | grep -c "2224/tcp" | head -1 || echo "0")

# Convert to integer (handle any whitespace/newlines)
EXISTING_SSH=$((EXISTING_SSH + 0))
EXISTING_HTTP=$((EXISTING_HTTP + 0))
EXISTING_HTTPS=$((EXISTING_HTTPS + 0))
EXISTING_NYDUS=$((EXISTING_NYDUS + 0))

# Only add rules if they don't exist
if [ "$EXISTING_SSH" -eq 0 ]; then
    echo "Adding SSH rule (port 22)..."
    ufw allow 22/tcp comment 'SSH'
else
    echo "⚠️  SSH rule already exists, skipping"
fi

if [ "$EXISTING_HTTP" -eq 0 ]; then
    echo "Adding HTTP rule (port 80)..."
    ufw allow 80/tcp comment 'HTTP'
else
    echo "⚠️  HTTP rule already exists, skipping"
fi

if [ "$EXISTING_HTTPS" -eq 0 ]; then
    echo "Adding HTTPS rule (port 443)..."
    ufw allow 443/tcp comment 'HTTPS'
else
    echo "⚠️  HTTPS rule already exists, skipping"
fi

if [ "$EXISTING_NYDUS" -eq 0 ]; then
    echo "Adding Nydus rule (port 2224)..."
    ufw allow 2224/tcp comment 'Nydus'
else
    echo "⚠️  Nydus rule already exists, skipping"
fi

# Enable UFW if not already enabled (but don't force if it's already active)
if [ "$UFW_STATUS" != "active" ]; then
    echo ""
    echo "⚠️  UFW is not active. Enabling UFW..."
    echo "   Make sure SSH (port 22) is allowed or you may be locked out!"
    read -p "Continue with enabling UFW? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        ufw --force enable
        echo "✅ UFW enabled"
    else
        echo "⚠️  UFW not enabled. Please enable it manually when ready."
    fi
else
    echo ""
    echo "✅ UFW is already active"
fi

# Show firewall status
echo ""
echo "Current firewall rules:"
ufw status numbered

# Configure SSH security
echo ""
echo "🔐 Configuring SSH security..."

# Check if SSH is installed
if [ ! -f /etc/ssh/sshd_config ] && ! systemctl list-unit-files | grep -q "ssh.service\|sshd.service"; then
    echo "⚠️  SSH server not found. Installing openssh-server..."
    apt-get install -y openssh-server
    systemctl enable ssh || systemctl enable sshd
    systemctl start ssh || systemctl start sshd
fi

# Backup SSH config
if [ -f /etc/ssh/sshd_config ]; then
    cp /etc/ssh/sshd_config /etc/ssh/sshd_config.backup.$(date +%Y%m%d_%H%M%S)
    echo "✅ SSH config backed up"
fi

# Check if user has authorized keys
HAS_AUTHORIZED_KEYS=false
CURRENT_USER_HOME=""
CURRENT_USER=$(whoami)

# Check current user's home directory first (most important)
if [ "$CURRENT_USER" = "root" ]; then
    CURRENT_USER_HOME="/root"
else
    CURRENT_USER_HOME="/home/$CURRENT_USER"
fi

# Check if current user has authorized keys
if [ -d "$CURRENT_USER_HOME/.ssh" ] && [ -f "$CURRENT_USER_HOME/.ssh/authorized_keys" ] && [ -s "$CURRENT_USER_HOME/.ssh/authorized_keys" ]; then
    HAS_AUTHORIZED_KEYS=true
    echo "✅ Found authorized_keys for current user ($CURRENT_USER) in $CURRENT_USER_HOME/.ssh/"
    echo "   Key count: $(wc -l < "$CURRENT_USER_HOME/.ssh/authorized_keys")"
else
    # Check other users as fallback
    for user_home in /root /home/*; do
        if [ -d "$user_home/.ssh" ] && [ -f "$user_home/.ssh/authorized_keys" ] && [ -s "$user_home/.ssh/authorized_keys" ]; then
            HAS_AUTHORIZED_KEYS=true
            echo "✅ Found authorized_keys in $user_home/.ssh/"
            echo "   ⚠️  Warning: This is NOT the current user's home directory!"
            break
        fi
    done
fi

# Check current SSH configuration
CURRENT_PASSWORD_AUTH=$(grep -E "^PasswordAuthentication|^#PasswordAuthentication" /etc/ssh/sshd_config | tail -1 | awk '{print $2}' || echo "yes")
CURRENT_PUBKEY_AUTH=$(grep -E "^PubkeyAuthentication|^#PubkeyAuthentication" /etc/ssh/sshd_config | tail -1 | awk '{print $2}' || echo "yes")
CURRENT_PERMIT_ROOT=$(grep -E "^PermitRootLogin|^#PermitRootLogin" /etc/ssh/sshd_config | tail -1 | awk '{print $2}' || echo "yes")

echo "Current SSH configuration:"
echo "  PasswordAuthentication: $CURRENT_PASSWORD_AUTH"
echo "  PubkeyAuthentication: $CURRENT_PUBKEY_AUTH"
echo "  PermitRootLogin: $CURRENT_PERMIT_ROOT"

# Configure SSH for key-only authentication
if [ "$HAS_AUTHORIZED_KEYS" = true ]; then
    echo ""
    echo "⚠️  SECURITY: Configuring SSH for public key authentication only..."
    echo "   This will disable password authentication."
    echo ""
    
    # Warn if current user doesn't have keys
    if [ -z "$CURRENT_USER_HOME" ] || [ ! -f "$CURRENT_USER_HOME/.ssh/authorized_keys" ]; then
        echo "   ⚠️  CRITICAL WARNING: Current user ($CURRENT_USER) does NOT have authorized_keys!"
        echo "   You will be LOCKED OUT if you continue!"
        echo ""
        read -p "Are you ABSOLUTELY SURE you can login as another user with SSH keys? (type 'YES' to continue): " CONFIRM
        if [ "$CONFIRM" != "YES" ]; then
            echo "SSH security hardening cancelled for safety."
            exit 0
        fi
    else
        echo "   Current user ($CURRENT_USER) has authorized_keys - safe to proceed."
    fi
    
    echo ""
    read -p "Continue with SSH security hardening? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        # Create temporary SSH config file
        SSH_CONFIG_TMP=$(mktemp)
        
        # Read current config and modify
        while IFS= read -r line; do
            # Skip existing PasswordAuthentication, PubkeyAuthentication, PermitRootLogin lines
            if [[ "$line" =~ ^[[:space:]]*#?[[:space:]]*PasswordAuthentication ]]; then
                echo "PasswordAuthentication no"
            elif [[ "$line" =~ ^[[:space:]]*#?[[:space:]]*PubkeyAuthentication ]]; then
                echo "PubkeyAuthentication yes"
            elif [[ "$line" =~ ^[[:space:]]*#?[[:space:]]*PermitRootLogin ]]; then
                # Keep PermitRootLogin as-is if it's already set, otherwise set to prohibit-password
                if [[ "$line" =~ ^[[:space:]]*PermitRootLogin ]]; then
                    echo "$line"
                else
                    echo "PermitRootLogin prohibit-password"
                fi
            elif [[ "$line" =~ ^[[:space:]]*#?[[:space:]]*ChallengeResponseAuthentication ]]; then
                echo "ChallengeResponseAuthentication no"
            elif [[ "$line" =~ ^[[:space:]]*#?[[:space:]]*UsePAM ]]; then
                # Keep UsePAM but ensure it's enabled (needed for some systems)
                echo "UsePAM yes"
            else
                echo "$line"
            fi
        done < /etc/ssh/sshd_config > "$SSH_CONFIG_TMP"
        
        # Add security settings if they don't exist
        if ! grep -q "^PasswordAuthentication" "$SSH_CONFIG_TMP"; then
            echo "" >> "$SSH_CONFIG_TMP"
            echo "# Security: Disable password authentication" >> "$SSH_CONFIG_TMP"
            echo "PasswordAuthentication no" >> "$SSH_CONFIG_TMP"
        fi
        
        if ! grep -q "^PubkeyAuthentication" "$SSH_CONFIG_TMP"; then
            echo "PubkeyAuthentication yes" >> "$SSH_CONFIG_TMP"
        fi
        
        if ! grep -q "^PermitRootLogin" "$SSH_CONFIG_TMP"; then
            echo "PermitRootLogin prohibit-password" >> "$SSH_CONFIG_TMP"
        fi
        
        if ! grep -q "^ChallengeResponseAuthentication" "$SSH_CONFIG_TMP"; then
            echo "ChallengeResponseAuthentication no" >> "$SSH_CONFIG_TMP"
        fi
        
        # Test SSH config before applying
        echo "Testing SSH configuration..."
        if sshd -t -f "$SSH_CONFIG_TMP"; then
            cp "$SSH_CONFIG_TMP" /etc/ssh/sshd_config
            echo "✅ SSH configuration updated"
            
            # Reload SSH (don't restart to avoid disconnecting current session)
            echo "Reloading SSH daemon..."
            # Try both service names (Ubuntu uses 'ssh', some systems use 'sshd')
            SSH_RELOADED=false
            if systemctl is-active --quiet ssh 2>/dev/null; then
                if systemctl reload ssh 2>/dev/null; then
                    echo "✅ SSH service reloaded (ssh)"
                    SSH_RELOADED=true
                fi
            elif systemctl is-active --quiet sshd 2>/dev/null; then
                if systemctl reload sshd 2>/dev/null; then
                    echo "✅ SSH service reloaded (sshd)"
                    SSH_RELOADED=true
                fi
            fi
            
            if [ "$SSH_RELOADED" = false ]; then
                echo "⚠️  Warning: Could not reload SSH service automatically"
                echo "   The configuration has been updated but not applied yet."
                echo ""
                echo "   ⚠️  CRITICAL: Do NOT close this session until you:"
                echo "   1. Open a NEW SSH session and test key-based login"
                echo "   2. If that works, then reload SSH: sudo systemctl reload ssh"
                echo "   3. If reload fails, restart: sudo systemctl restart ssh"
                echo ""
                echo "   If you close this session now, you may be locked out!"
                read -p "Press Enter after you've tested SSH login in another session..."
            fi
            
            echo ""
            echo "✅ SSH security hardened:"
            echo "   - Password authentication: DISABLED"
            echo "   - Public key authentication: ENABLED"
            echo "   - Root login: prohibit-password (key only)"
        else
            echo "❌ SSH configuration test failed. Not applying changes."
            echo "   Please check the configuration manually."
            rm -f "$SSH_CONFIG_TMP"
        fi
    else
        echo "⚠️  SSH security hardening skipped"
        echo "   Consider hardening SSH manually for better security"
    fi
else
    echo ""
    echo "⚠️  WARNING: No authorized SSH keys found!"
    echo "   Cannot safely disable password authentication."
    echo ""
    echo "   To set up SSH key authentication:"
    echo "   1. On your local machine, generate a key:"
    echo "      ssh-keygen -t ed25519 -C 'your-email@example.com'"
    echo "   2. Copy your public key to the server:"
    echo "      ssh-copy-id -i ~/.ssh/id_ed25519.pub root@your-server-ip"
    echo "   3. Test SSH key login before running this script again"
    echo ""
    echo "   For now, SSH will remain with password authentication enabled."
    echo "   This is a security risk - please set up key authentication!"
fi

# Enable IPv6 in sysctl
echo ""
echo "🌐 Enabling IPv6 support..."
if ! grep -q "net.ipv6.conf.all.disable_ipv6 = 0" /etc/sysctl.conf; then
    echo "" >> /etc/sysctl.conf
    echo "# Enable IPv6" >> /etc/sysctl.conf
    echo "net.ipv6.conf.all.disable_ipv6 = 0" >> /etc/sysctl.conf
    echo "net.ipv6.conf.default.disable_ipv6 = 0" >> /etc/sysctl.conf
    sysctl -p
fi

# Create necessary directories
echo ""
echo "📁 Creating directories..."
mkdir -p /opt/r-data-core-demo
mkdir -p /opt/r-data-core-demo/nginx
mkdir -p /opt/r-data-core-demo/ssl
mkdir -p /opt/r-data-core-demo/data/postgres
mkdir -p /opt/r-data-core-demo/data/redis
mkdir -p /etc/letsencrypt/live/api.rdatacore.eu
mkdir -p /etc/letsencrypt/live/demo.rdatacore.eu
mkdir -p /etc/letsencrypt/live/rdatacore.eu

# Set permissions
chmod 755 /opt/r-data-core-demo
chmod 755 /opt/r-data-core-demo/nginx

echo ""
echo "✅ Server setup complete!"
echo ""
echo "Next steps:"
echo "1. Configure DNS for your domains (IPv4 and IPv6)"
echo "2. Run ./validate-domains.sh to verify DNS"
echo "3. Run ./setup-ssl.sh to obtain SSL certificates"
echo "4. Copy docker-compose.demo.yml and .env to /opt/r-data-core-demo/"
echo "5. Start services with: docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml up -d"
echo ""
echo "⚠️  Remember: This is a DEMO setup, not production!"

