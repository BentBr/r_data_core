#!/bin/bash
set -euo pipefail

# ==============================================================================
# SSH Key Setup Helper Script
# ==============================================================================
# This script helps set up SSH key authentication on the server.
# Run this BEFORE running setup-server.sh to ensure you can access the server
# after password authentication is disabled.
#
# Usage: ./setup-ssh-keys.sh [public-key-file]
# ==============================================================================

echo "=============================================================================="
echo "SSH Key Setup Helper"
echo "=============================================================================="
echo "This script helps you set up SSH key authentication."
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "⚠️  Warning: Not running as root. Some operations may require sudo."
    SUDO_CMD="sudo"
else
    SUDO_CMD=""
fi

# Get the user (root or current user)
if [ "$EUID" -eq 0 ]; then
    TARGET_USER="root"
    TARGET_HOME="/root"
else
    TARGET_USER=$(whoami)
    TARGET_HOME="$HOME"
fi

echo "Target user: $TARGET_USER"
echo "Target home: $TARGET_HOME"
echo ""

# Check if public key file is provided
if [ $# -ge 1 ]; then
    PUBKEY_FILE="$1"
    if [ ! -f "$PUBKEY_FILE" ]; then
        echo "❌ Error: Public key file not found: $PUBKEY_FILE"
        exit 1
    fi
    echo "Using public key file: $PUBKEY_FILE"
    PUBKEY=$(cat "$PUBKEY_FILE")
else
    echo "No public key file provided."
    echo ""
    echo "Options:"
    echo "1. Paste your public key now"
    echo "2. Provide a public key file path"
    echo ""
    read -p "Choose option (1 or 2): " OPTION
    
    if [ "$OPTION" = "1" ]; then
        echo ""
        echo "Paste your public key (ssh-ed25519 or ssh-rsa) and press Enter, then Ctrl+D:"
        PUBKEY=$(cat)
    elif [ "$OPTION" = "2" ]; then
        read -p "Enter path to public key file: " PUBKEY_FILE
        if [ ! -f "$PUBKEY_FILE" ]; then
            echo "❌ Error: File not found: $PUBKEY_FILE"
            exit 1
        fi
        PUBKEY=$(cat "$PUBKEY_FILE")
    else
        echo "❌ Invalid option"
        exit 1
    fi
fi

# Validate public key format
if ! echo "$PUBKEY" | grep -qE "^(ssh-ed25519|ssh-rsa|ecdsa-sha2|ssh-dss)"; then
    echo "❌ Error: Invalid public key format"
    echo "   Expected format: ssh-ed25519 AAAA... or ssh-rsa AAAA..."
    exit 1
fi

echo ""
echo "Public key (first 50 chars): $(echo "$PUBKEY" | cut -c1-50)..."
read -p "Add this key to $TARGET_USER's authorized_keys? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Cancelled"
    exit 0
fi

# Create .ssh directory if it doesn't exist
if [ ! -d "$TARGET_HOME/.ssh" ]; then
    echo "Creating .ssh directory..."
    $SUDO_CMD mkdir -p "$TARGET_HOME/.ssh"
    $SUDO_CMD chmod 700 "$TARGET_HOME/.ssh"
    if [ "$EUID" -ne 0 ]; then
        $SUDO_CMD chown "$TARGET_USER:$TARGET_USER" "$TARGET_HOME/.ssh"
    fi
fi

# Check if key already exists
if [ -f "$TARGET_HOME/.ssh/authorized_keys" ]; then
    if grep -Fxq "$PUBKEY" "$TARGET_HOME/.ssh/authorized_keys"; then
        echo "⚠️  This key already exists in authorized_keys"
        exit 0
    fi
fi

# Add key to authorized_keys
echo "Adding key to authorized_keys..."
echo "$PUBKEY" | $SUDO_CMD tee -a "$TARGET_HOME/.ssh/authorized_keys" > /dev/null

# Set correct permissions
$SUDO_CMD chmod 600 "$TARGET_HOME/.ssh/authorized_keys"
if [ "$EUID" -ne 0 ]; then
    $SUDO_CMD chown "$TARGET_USER:$TARGET_USER" "$TARGET_HOME/.ssh/authorized_keys"
fi

echo ""
echo "✅ SSH key added successfully!"
echo ""
echo "Next steps:"
echo "1. Test SSH key login from your local machine:"
echo "   ssh -i ~/.ssh/your_key $TARGET_USER@$(hostname -I | awk '{print $1}')"
echo ""
echo "2. If login works without password prompt, you can proceed with setup-server.sh"
echo "3. The setup-server.sh script will disable password authentication"

