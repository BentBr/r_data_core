# Demo Server Deployment Guide

## ⚠️ Important: This is a DEMO Setup

**This is NOT a production deployment.** This setup is for demonstration purposes only. True production deployments should use native Kubernetes (k8s) with proper orchestration, scaling, and high availability.

## Overview

This guide walks you through setting up a demo deployment of RDataCore on a single root server. The deployment includes:

- **API** at `api.rdatacore.eu` (core, worker, maintenance containers)
- **Frontend** at `demo.rdatacore.eu` (admin FE)
- **Static website** at `rdatacore.eu`
- Nginx reverse proxy with Let's Encrypt SSL
- Docker Compose for orchestration
- GitHub Actions for manual deployment

## Prerequisites

- Blank Ubuntu/Debian server (22.04 LTS recommended)
- Root or sudo access
- Minimum 2GB RAM, 20GB disk space
- Three domains configured with DNS:
  - `api.rdatacore.eu` → server IPv4 and IPv6
  - `demo.rdatacore.eu` → server IPv4 and IPv6
  - `rdatacore.eu` → server IPv4 and IPv6
- SSH access to the server
- GitHub repository with Docker images published

## Architecture

```
Internet (IPv4/IPv6)
  ↓
Nginx Reverse Proxy (Port 80/443) - SSL termination
  ├─→ api.rdatacore.eu → Docker: r-data-core (port 8080)
  ├─→ demo.rdatacore.eu → Docker: r-data-core-admin-fe (port 8080)
  └─→ rdatacore.eu → Docker: r-data-core-static (port 8080)
  
Docker Compose Network:
  ├─ r-data-core (API)
  ├─ r-data-core-worker
  ├─ r-data-core-maintenance
  ├─ r-data-core-admin-fe (nginx)
  ├─ r-data-core-static (nginx)
  ├─ postgres
  └─ redis
```

## Step 1: Initial Server Setup

### 1.1 Set Up SSH Key Authentication (IMPORTANT)

**Before running the setup script, you MUST set up SSH key authentication.** The setup script will disable password authentication for security.

#### 1.1.1 Generate SSH Key (if you don't have one)

On your local machine:

```bash
# Generate a new ED25519 key (recommended)
ssh-keygen -t ed25519 -C "your-email@example.com" -f ~/.ssh/demo_server_key

# Or use existing key
ls -la ~/.ssh/id_ed25519.pub
```

#### 1.1.2 Copy Public Key to Server

**Option A: Using ssh-copy-id (recommended)**

```bash
# Copy your public key to the server
ssh-copy-id -i ~/.ssh/demo_server_key.pub root@your-server-ip
```

**Option B: Using the helper script**

```bash
# Upload the helper script first
scp .docker/demo/scripts/setup-ssh-keys.sh root@your-server-ip:/tmp/

# SSH to server and run the script
ssh root@your-server-ip
chmod +x /tmp/setup-ssh-keys.sh
/tmp/setup-ssh-keys.sh ~/.ssh/demo_server_key.pub
```

**Option C: Manual setup**

```bash
# Copy key manually
cat ~/.ssh/demo_server_key.pub | ssh root@your-server-ip "mkdir -p ~/.ssh && chmod 700 ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
```

#### 1.1.3 Test SSH Key Login

```bash
# Test that you can login with key (should NOT prompt for password)
ssh -i ~/.ssh/demo_server_key root@your-server-ip

# If successful, you're ready to proceed
```

**⚠️ CRITICAL:** Do NOT proceed until you can login via SSH key without a password prompt!

### 1.2 Connect to Server

```bash
ssh -i ~/.ssh/demo_server_key root@your-server-ip
```

### 1.3 Upload Setup Scripts

Copy the setup scripts to your server:

```bash
# From your local machine
scp -r .docker/demo/scripts root@your-server-ip:/opt/r-data-core-demo/
scp .docker/demo/docker-compose.demo.yml root@your-server-ip:/opt/r-data-core-demo/
scp .docker/demo/nginx/nginx.conf root@your-server-ip:/opt/r-data-core-demo/nginx/
scp .docker/demo/env.template root@your-server-ip:/opt/r-data-core-demo/
```

**Note:** The `setup-ssh-keys.sh` helper script is included in the scripts directory if you need to add SSH keys later.

### 1.3 Run Server Setup Script

On the server, run the setup script to install all required software:

```bash
cd /opt/r-data-core-demo/scripts
chmod +x setup-server.sh
sudo ./setup-server.sh
```

**⚠️ IMPORTANT SSH Key Warning:**

- The script will disable password authentication for SSH
- **Make sure the user running the script has SSH keys in their authorized_keys**
- If you run as `root` but keys are in `/home/username/.ssh/`, root will be locked out
- **Best practice:** Run the script as the user who has your SSH keys, or add your key to root first:
  ```bash
  # If running as root, add your key first:
  sudo mkdir -p /root/.ssh
  sudo chmod 700 /root/.ssh
  sudo cp ~/.ssh/authorized_keys /root/.ssh/authorized_keys
  sudo chmod 600 /root/.ssh/authorized_keys
  ```

This script will:
- Update system packages
- Install Docker and Docker Compose
- Install nginx
- Install certbot
- Configure firewall (ports 22, 80, 443, 2224) - **safely handles existing rules**
- **Configure SSH for key-only authentication** - **disables password auth**
- Enable IPv4 and IPv6 support
- Create necessary directories

**Important Notes:**

- **SSH Security:** The script will configure SSH to only allow public key authentication and disable password authentication. **You MUST have SSH keys set up before running this script** (see Step 1.1). The script will check for authorized keys and prompt before making changes.

- **Firewall (UFW):** The script checks for existing firewall rules and only adds new ones if they don't exist. If UFW is not active, you'll be prompted before enabling it to prevent locking yourself out. On Ubuntu 24.04, UFW may already be configured, so the script will preserve existing rules.

## Step 2: Configure DNS

### 2.1 Configure Domain DNS Records

Configure DNS for all three domains:

**IPv4 (A records):**
- `api.rdatacore.eu` → Your server IPv4
- `demo.rdatacore.eu` → Your server IPv4
- `rdatacore.eu` → Your server IPv4

**IPv6 (AAAA records):**
- `api.rdatacore.eu` → Your server IPv6
- `demo.rdatacore.eu` → Your server IPv6
- `rdatacore.eu` → Your server IPv6

### 2.2 Validate DNS Configuration

Wait 5-60 minutes for DNS propagation, then validate:

```bash
cd /opt/r-data-core-demo/scripts
chmod +x validate-domains.sh
./validate-domains.sh [YOUR_IPV4] [YOUR_IPV6]
```

The script will check:
- IPv4 A records for all domains
- IPv6 AAAA records for all domains
- Connectivity to domains

If validation fails, fix DNS and wait for propagation before continuing.

## Step 3: Obtain SSL Certificates

### 3.1 Run SSL Setup Script

```bash
cd /opt/r-data-core-demo/scripts
chmod +x setup-ssl.sh
sudo ./setup-ssl.sh
```

The script will:
- Validate domain DNS configuration
- Obtain Let's Encrypt certificates for all domains
- Set up automatic renewal via systemd timer
- Configure renewal hook to reload nginx after certificate updates

You'll be prompted for:
- Email address for Let's Encrypt notifications

**Systemd Timer:** The script creates a systemd timer (`certbot.timer`) that runs twice daily to check for certificate renewal. The timer is automatically enabled and started. See the "SSL Certificate Renewal" section below for more details.

### 3.2 Verify Certificates

```bash
# Check certificate files
ls -la /etc/letsencrypt/live/api.rdatacore.eu/
ls -la /etc/letsencrypt/live/demo.rdatacore.eu/
ls -la /etc/letsencrypt/live/rdatacore.eu/

# Test certificate validity
openssl x509 -in /etc/letsencrypt/live/api.rdatacore.eu/fullchain.pem -text -noout
```

## Step 4: Configure Environment Variables

### 4.1 Create .env File

```bash
cd /opt/r-data-core-demo
cp env.template .env
nano .env  # or use your preferred editor
```

### 4.2 Required Configuration

Edit the `.env` file and set:

```bash
# GitHub repository owner (your GitHub username/organization, lowercase)
GITHUB_REPO_OWNER=your-username

# Database password (use a strong password)
POSTGRES_PASSWORD=your-strong-password

# JWT secret (generate with: openssl rand -base64 32)
JWT_SECRET=your-jwt-secret
```

### 4.3 Generate Secrets

```bash
# Generate JWT secret
openssl rand -base64 32

# Generate database password
openssl rand -base64 24
```

## Step 5: Configure Nginx

### 5.1 Copy Nginx Configuration

The nginx configuration should already be in `/opt/r-data-core-demo/nginx/nginx.conf`.

### 5.2 Install Nginx Configuration

```bash
# Backup existing nginx config
cp /etc/nginx/nginx.conf /etc/nginx/nginx.conf.backup

# Copy our configuration
cp /opt/r-data-core-demo/nginx/nginx.conf /etc/nginx/nginx.conf

# Test nginx configuration
nginx -t

# If test passes, reload nginx
systemctl reload nginx
```

### 5.3 Configure Nginx to Connect to Docker Network

Nginx needs to resolve Docker container hostnames. We'll use Docker's host networking or configure nginx to use Docker's DNS.

**Option 1: Use Docker host network (recommended for demo)**

Modify docker-compose.demo.yml to expose services on host network, or:

**Option 2: Configure nginx with Docker DNS**

Add to `/etc/nginx/nginx.conf` in the `http` block:

```nginx
resolver 127.0.0.11 valid=30s;
```

And update upstream blocks to use variables:

```nginx
upstream r_data_core_api {
    server core:8080 resolve;
    keepalive 32;
}
```

Actually, since nginx runs on the host, we need to either:
1. Run nginx in Docker on the same network, or
2. Expose container ports to host and use localhost

For simplicity in demo, we'll expose ports to host. Update docker-compose.demo.yml to expose ports:

```yaml
core:
    ports:
        - "127.0.0.1:8081:8080"
admin-fe:
    ports:
        - "127.0.0.1:8082:8080"
static-website:
    ports:
        - "127.0.0.1:8083:8080"
```

And update nginx.conf upstreams:

```nginx
upstream r_data_core_api {
    server 127.0.0.1:8081;
}
upstream r_data_core_admin_fe {
    server 127.0.0.1:8082;
}
upstream r_data_core_static {
    server 127.0.0.1:8083;
}
```

Actually, let me update the docker-compose and nginx configs to handle this properly.

## Step 6: Start Services

### 6.1 Login to GitHub Container Registry

```bash
# On the server
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin
```

Or create a GitHub Personal Access Token with `read:packages` permission and use it.

### 6.2 Start All Services

```bash
cd /opt/r-data-core-demo
export GITHUB_REPO_OWNER=your-username
docker compose -f docker-compose.demo.yml up -d
```

### 6.3 Check Service Status

```bash
docker compose -f docker-compose.demo.yml ps
docker compose -f docker-compose.demo.yml logs -f
```

### 6.4 Verify Services

```bash
# Check API
curl http://localhost:8081/health

# Check admin FE
curl http://localhost:8082/health

# Check static website
curl http://localhost:8083/health
```

## Step 7: Start Nginx

### 7.1 Start Nginx

```bash
systemctl start nginx
systemctl enable nginx
systemctl status nginx
```

### 7.2 Test Endpoints

```bash
# Test API (should redirect to HTTPS)
curl -I http://api.rdatacore.eu
curl -I https://api.rdatacore.eu

# Test Admin FE
curl -I https://demo.rdatacore.eu

# Test Static Website
curl -I https://rdatacore.eu
```

## Step 8: Configure GitHub Actions Deployment

### 8.1 Add GitHub Secrets

In your GitHub repository, go to Settings → Secrets and variables → Actions, and add:

- `DEMO_SERVER_SSH_KEY`: Private SSH key for server access
- `DEMO_SERVER_HOST`: Server hostname or IP (optional, can be set in workflow)
- `DEMO_SERVER_USER`: SSH user (default: root)

### 8.2 Generate SSH Key

On your local machine:

```bash
ssh-keygen -t ed25519 -C "github-actions-demo" -f ~/.ssh/demo_server_key
```

Copy the public key to the server:

```bash
ssh-copy-id -i ~/.ssh/demo_server_key.pub root@your-server-ip
```

Add the private key (`~/.ssh/demo_server_key`) to GitHub secrets as `DEMO_SERVER_SSH_KEY`.

### 8.3 Deploy via GitHub Actions

1. Go to Actions tab in GitHub
2. Select "Deploy Backend to Demo Server" (or Frontend/Static Website)
3. Click "Run workflow"
4. Fill in:
   - Server host
   - Server user (default: root)
   - Compose file path (default: `/opt/r-data-core-demo/docker-compose.demo.yml`)
5. Click "Run workflow"

## Maintenance

### View Logs

```bash
# All services
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml logs -f

# Specific service
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml logs -f core
```

### Restart Services

```bash
# All services
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml restart

# Specific service
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml restart core
```

### Update Services

Use GitHub Actions workflows to deploy updates, or manually:

```bash
cd /opt/r-data-core-demo
export GITHUB_REPO_OWNER=your-username
docker compose -f docker-compose.demo.yml pull
docker compose -f docker-compose.demo.yml up -d
```

### Backup Database

```bash
# Create backup
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml exec postgres pg_dump -U postgres rdata > backup_$(date +%Y%m%d_%H%M%S).sql

# Restore backup
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml exec -T postgres psql -U postgres rdata < backup_file.sql
```

### SSL Certificate Renewal

Certificates auto-renew via systemd timer. The timer is automatically set up by the `setup-ssl.sh` script.

#### Systemd Timer Configuration

The certbot renewal timer is configured as follows:

- **Timer file**: `/etc/systemd/system/certbot.timer`
- **Service file**: `/etc/systemd/system/certbot.service`
- **Schedule**: Runs twice daily (00:00 and 12:00 UTC) with randomized delay
- **Deploy hook**: Automatically reloads nginx after successful renewal

#### Check Timer Status

```bash
# Check if timer is active
systemctl status certbot.timer

# List all timers and see next run time
systemctl list-timers certbot.timer

# Check timer logs
journalctl -u certbot.timer -f
journalctl -u certbot.service -f
```

#### Manual Renewal

To manually renew certificates:

```bash
# Test renewal (dry run)
certbot renew --dry-run

# Force renewal
certbot renew --force-renewal
systemctl reload nginx
```

#### Troubleshooting Timer

If the timer is not working:

```bash
# Check if timer is enabled
systemctl is-enabled certbot.timer

# Enable and start timer
systemctl enable certbot.timer
systemctl start certbot.timer

# Check for errors
journalctl -u certbot.service --since "1 hour ago"
```

## Troubleshooting

### Services Not Starting

```bash
# Check logs
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml logs

# Check service status
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml ps

# Check Docker network
docker network ls
docker network inspect rdata_demo_network
```

### Nginx Not Working

```bash
# Check nginx status
systemctl status nginx

# Test nginx configuration
nginx -t

# Check nginx logs
tail -f /var/log/nginx/error.log
tail -f /var/log/nginx/access.log
```

### DNS Issues

```bash
# Test DNS resolution
dig api.rdatacore.eu
dig AAAA api.rdatacore.eu

# Test connectivity
ping api.rdatacore.eu
ping6 api.rdatacore.eu
```

### SSL Certificate Issues

```bash
# Check certificate expiry
openssl x509 -in /etc/letsencrypt/live/api.rdatacore.eu/fullchain.pem -noout -dates

# Test certificate
openssl s_client -connect api.rdatacore.eu:443 -servername api.rdatacore.eu

# Renew certificate manually
certbot renew --force-renewal
```

### Port Conflicts

```bash
# Check what's using ports
netstat -tulpn | grep :80
netstat -tulpn | grep :443
netstat -tulpn | grep :8081

# Or use ss (modern alternative)
ss -tulpn | grep :80
ss -tulpn | grep :443
```

### Firewall (UFW) Issues

```bash
# Check UFW status
ufw status verbose

# Check UFW rules
ufw status numbered

# If you need to remove a rule
ufw delete [rule-number]

# Check UFW logs
tail -f /var/log/ufw.log

# If UFW is blocking something, check logs
journalctl -u ufw -f
```

**Note:** On Ubuntu 24.04, UFW may have existing rules. The setup script preserves existing rules and only adds new ones if needed. If you encounter firewall issues, check existing rules first before modifying.

### SSH Access Issues

If you're locked out of SSH:

**Common cause:** Script was run as root but keys were only in a non-root user's home directory.

**Recovery via Console (VPS provider console/KVM):**

```bash
# 1. Log in via console (not SSH)
# 2. Check SSH config
cat /etc/ssh/sshd_config | grep -E "PasswordAuthentication|PubkeyAuthentication"

# 3. Restore SSH config backup (if available)
ls -la /etc/ssh/sshd_config.backup.*
cp /etc/ssh/sshd_config.backup.* /etc/ssh/sshd_config

# OR manually enable password auth
sed -i 's/PasswordAuthentication no/PasswordAuthentication yes/' /etc/ssh/sshd_config

# 4. Reload SSH (Ubuntu uses 'ssh', not 'sshd')
systemctl reload ssh || systemctl restart ssh

# 5. Now SSH in with password
# 6. Add your key to the user you're logged in as:
mkdir -p ~/.ssh
chmod 700 ~/.ssh
# Copy your public key here
nano ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys

# 7. Test key login, then disable password auth again
```

**Recovery if you can still SSH as another user:**

```bash
# SSH as the user who has keys (e.g., bentbr)
ssh bentbr@your-server

# Add key to root (if you need root access)
sudo mkdir -p /root/.ssh
sudo chmod 700 /root/.ssh
sudo cp ~/.ssh/authorized_keys /root/.ssh/authorized_keys
sudo chmod 600 /root/.ssh/authorized_keys

# Now root can login with keys
```

**Prevention:** Always test SSH key login before running the setup script!

```bash
# Check SSH configuration
sshd -T | grep -E "passwordauthentication|pubkeyauthentication|permitrootlogin"

# Check authorized keys
cat ~/.ssh/authorized_keys

# Test SSH key login (should not prompt for password)
ssh -v root@your-server-ip
```

### SSH Key Management

```bash
# View current authorized keys
cat ~/.ssh/authorized_keys

# Add a new authorized key
echo "ssh-ed25519 AAAAC3..." >> ~/.ssh/authorized_keys
chmod 600 ~/.ssh/authorized_keys

# Remove an authorized key (edit the file)
nano ~/.ssh/authorized_keys

# Check SSH logs for authentication attempts
tail -f /var/log/auth.log
journalctl -u ssh -f
```

## Security Considerations

### Demo Setup Security

The setup script implements basic security measures:

1. **SSH Key-Only Authentication**: Password authentication is disabled
2. **Firewall (UFW)**: Only necessary ports are open (22, 80, 443)
3. **Root Login**: Restricted to key-only authentication
4. **SSL/TLS**: All traffic encrypted with Let's Encrypt certificates

### Additional Security Recommendations

Even for demo, consider:

1. **Fail2Ban**: Install to prevent brute force attacks
   ```bash
   apt-get install fail2ban
   systemctl enable fail2ban
   systemctl start fail2ban
   ```

2. **SSH Port Change**: Consider changing SSH port (requires firewall update)
   ```bash
   # Edit /etc/ssh/sshd_config
   Port 2222
   # Update UFW: ufw allow 2222/tcp
   ```

3. **Regular Updates**: Keep system packages updated
   ```bash
   apt-get update && apt-get upgrade -y
   ```

4. **Monitor Logs**: Regularly check authentication logs
   ```bash
   tail -f /var/log/auth.log
   ```

5. **Disable Unused Services**: Remove unnecessary packages
   ```bash
   apt-get autoremove
   ```

### Production Security

⚠️ **This is a DEMO setup. For production:**

1. Use Kubernetes with proper orchestration
2. Implement proper secrets management (HashiCorp Vault, AWS Secrets Manager, etc.)
3. Use managed databases (RDS, Cloud SQL, etc.)
4. Implement proper monitoring and alerting (Prometheus, Grafana, etc.)
5. Use WAF and DDoS protection (Cloudflare, AWS WAF, etc.)
6. Implement proper backup and disaster recovery
7. Use network policies and firewalls
8. Regular security audits and updates
9. Implement intrusion detection systems (IDS/IPS)
10. Use security scanning tools (Trivy, Clair, etc.)
11. Implement log aggregation and analysis
12. Set up security incident response procedures

## Support

For issues or questions:
1. Check logs first
2. Review this documentation
3. Check GitHub issues
4. Remember: This is demo, not production!

---

**Last Updated:** 2024
**Version:** Demo Deployment v1.0

