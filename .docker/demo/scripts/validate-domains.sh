#!/bin/bash
set -euo pipefail

# ==============================================================================
# Domain DNS Validation Script
# ==============================================================================
# Validates that all domains are properly configured with DNS (IPv4 and IPv6)
# and pointing to this server.
#
# Usage: ./validate-domains.sh [SERVER_IPV4] [SERVER_IPV6]
# ==============================================================================

# Check if dig is installed
if ! command -v dig &> /dev/null; then
    echo "❌ Error: 'dig' command not found. Please install dnsutils:"
    echo "   sudo apt-get install dnsutils"
    exit 1
fi

DOMAINS=("api.rdatacore.eu" "demo.rdatacore.eu" "rdatacore.eu")

# Get server IPs if not provided
if [ $# -ge 1 ]; then
    SERVER_IPV4="$1"
else
    echo "🔍 Detecting server IPv4 address..."
    SERVER_IPV4=$(curl -s -4 ifconfig.me || curl -s -4 icanhazip.com || echo "")
    if [ -z "$SERVER_IPV4" ]; then
        echo "⚠️  Warning: Could not auto-detect IPv4. Please provide as first argument."
        SERVER_IPV4=""
    fi
fi

if [ $# -ge 2 ]; then
    SERVER_IPV6="$2"
else
    echo "🔍 Detecting server IPv6 address..."
    SERVER_IPV6=$(curl -s -6 ifconfig.me || curl -s -6 icanhazip.com || echo "")
    if [ -z "$SERVER_IPV6" ]; then
        echo "⚠️  Warning: Could not auto-detect IPv6. IPv6 may not be configured."
        SERVER_IPV6=""
    fi
fi

echo ""
echo "=============================================================================="
echo "Domain DNS Validation"
echo "=============================================================================="
echo "Server IPv4: ${SERVER_IPV4:-NOT DETECTED}"
echo "Server IPv6: ${SERVER_IPV6:-NOT DETECTED}"
echo ""

ERRORS=0

for domain in "${DOMAINS[@]}"; do
    echo "Checking domain: $domain"
    echo "----------------------------------------"
    
    # Check IPv4 A record - use external DNS servers for reliability
    echo -n "  IPv4 (A record): "
    # Try external DNS servers first (more reliable, avoids local cache issues)
    IPV4_RECORD=$(dig @8.8.8.8 +short A "$domain" 2>/dev/null | grep -E '^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$' | head -n1 || echo "")
    if [ -z "$IPV4_RECORD" ]; then
        IPV4_RECORD=$(dig @1.1.1.1 +short A "$domain" 2>/dev/null | grep -E '^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$' | head -n1 || echo "")
    fi
    # Fallback to default resolver
    if [ -z "$IPV4_RECORD" ]; then
        IPV4_RECORD=$(dig +short A "$domain" 2>/dev/null | grep -E '^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$' | head -n1 || echo "")
    fi
    
    if [ -z "$IPV4_RECORD" ]; then
        echo "❌ NOT FOUND"
        echo "     Error: No A record found for $domain"
        echo "     Checked external DNS servers (8.8.8.8, 1.1.1.1) - record may not exist"
        ERRORS=$((ERRORS + 1))
    else
        echo "✓ Found: $IPV4_RECORD"
        if [ -n "$SERVER_IPV4" ] && [ "$IPV4_RECORD" != "$SERVER_IPV4" ]; then
            echo "  ⚠️  Warning: A record ($IPV4_RECORD) does not match server IPv4 ($SERVER_IPV4)"
            echo "     This may be intentional if using a load balancer or proxy"
        fi
    fi
    
    # Check IPv6 AAAA record - use external DNS servers for reliability
    echo -n "  IPv6 (AAAA record): "
    # Try external DNS servers first (more reliable, avoids local cache issues)
    IPV6_RECORD=$(dig @8.8.8.8 +short AAAA "$domain" 2>/dev/null | grep -E '^[0-9a-fA-F:]+$' | head -n1 || echo "")
    if [ -z "$IPV6_RECORD" ]; then
        IPV6_RECORD=$(dig @1.1.1.1 +short AAAA "$domain" 2>/dev/null | grep -E '^[0-9a-fA-F:]+$' | head -n1 || echo "")
    fi
    # Fallback to default resolver
    if [ -z "$IPV6_RECORD" ]; then
        IPV6_RECORD=$(dig +short AAAA "$domain" 2>/dev/null | grep -E '^[0-9a-fA-F:]+$' | head -n1 || echo "")
    fi
    
    if [ -z "$IPV6_RECORD" ]; then
        echo "⚠️  NOT FOUND (optional but recommended)"
        if [ -n "$SERVER_IPV6" ]; then
            echo "     Warning: Server has IPv6 but domain does not have AAAA record"
            echo "     Checked external DNS servers (8.8.8.8, 1.1.1.1) - record may not exist"
        fi
    else
        echo "✓ Found: $IPV6_RECORD"
        if [ -n "$SERVER_IPV6" ] && [ "$IPV6_RECORD" != "$SERVER_IPV6" ]; then
            echo "  ⚠️  Warning: AAAA record ($IPV6_RECORD) does not match server IPv6 ($SERVER_IPV6)"
            echo "     This may be intentional if using a load balancer or proxy"
        fi
    fi
    
    # Test connectivity
    echo -n "  Connectivity test (IPv4): "
    if ping -c 1 -W 2 "$domain" > /dev/null 2>&1; then
        echo "✓ Reachable"
    else
        echo "⚠️  Not reachable (may be normal if DNS just updated)"
    fi
    
    echo ""
done

echo "=============================================================================="
if [ $ERRORS -eq 0 ]; then
    echo "✅ All domains have DNS records configured"
    echo ""
    echo "Note: If you just configured DNS, wait a few minutes for propagation."
    echo "You can proceed with SSL certificate setup."
    exit 0
else
    echo "❌ Some domains are missing DNS records"
    echo ""
    echo "Please configure DNS for all domains before proceeding:"
    echo "  - A records pointing to: $SERVER_IPV4"
    if [ -n "$SERVER_IPV6" ]; then
        echo "  - AAAA records pointing to: $SERVER_IPV6"
    fi
    echo ""
    echo "After configuring DNS, wait for propagation (5-60 minutes) and run this script again."
    exit 1
fi

