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

### 1.3 Upload Setup Scripts and Repository

**Option A: Clone Repository on Server (Recommended)**

```bash
# On the server
cd /opt/r-data-core-demo
git clone https://github.com/YOUR_USERNAME/r_data_core.git repo
# Or if using SSH:
# git clone git@github.com:YOUR_USERNAME/r_data_core.git repo
```

**Option B: Upload Files Manually**

From your local machine:

```bash
# Upload setup scripts
scp -r .docker/demo/scripts root@your-server-ip:/opt/r-data-core-demo/
scp .docker/demo/docker-compose.demo.yml root@your-server-ip:/opt/r-data-core-demo/
scp .docker/demo/nginx/nginx.conf root@your-server-ip:/opt/r-data-core-demo/nginx/
scp .docker/demo/env.template root@your-server-ip:/opt/r-data-core-demo/

# Upload static website source (needed for building static website)
# Create directory structure on server
ssh root@your-server-ip "mkdir -p /opt/r-data-core-demo/repo/static-website /opt/r-data-core-demo/repo/.docker/static-website"

# Sync static website directory
rsync -avz --delete static-website/ root@your-server-ip:/opt/r-data-core-demo/repo/static-website/

# Sync Dockerfile and nginx config for static website
rsync -avz .docker/static-website/ root@your-server-ip:/opt/r-data-core-demo/repo/.docker/static-website/
```

**Note:** The static website will be built from the synced source. Use rsync for easier updates later.

### 1.4 Run Server Setup Script

**⚠️ CRITICAL:** Before running this script, ensure:
1. You have SSH keys set up (Step 1.1)
2. The user running the script has SSH keys in their `~/.ssh/authorized_keys`
3. If running as root, add your SSH key to `/root/.ssh/authorized_keys` first

On the server, run the setup script:

```bash
cd /opt/r-data-core-demo/scripts
chmod +x setup-server.sh
sudo ./setup-server.sh
```

**What the script does:**
- Updates system packages
**What the script installs:**
- Docker and Docker Compose
- nginx
- certbot
- Required system packages

**What the script configures:**
- Firewall (UFW) - opens ports 22, 80, 443, 2224
- SSH security - disables password authentication (key-only)
- IPv4 and IPv6 support
- Creates deployment directories

**Important:** The script checks if your current user has SSH keys before disabling password auth and warns you if you'll be locked out.

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

Wait 5-60 minutes for DNS propagation, then run the validation script:

```bash
cd /opt/r-data-core-demo/scripts
chmod +x validate-domains.sh
./validate-domains.sh [YOUR_IPV4] [YOUR_IPV6]
```

**What the script checks:**
- IPv4 A records for all domains (uses external DNS servers: 8.8.8.8, 1.1.1.1)
- IPv6 AAAA records for all domains (uses external DNS servers)
- Connectivity to domains

**If validation fails:**
1. Verify DNS records in your DNS provider's panel
2. Wait 5-60 minutes for propagation
3. Run the validation script again

The script uses external DNS servers to avoid local DNS cache issues.

## Step 3: Obtain SSL Certificates

### 3.1 Run SSL Setup Script

```bash
cd /opt/r-data-core-demo/scripts
chmod +x setup-ssl.sh
sudo ./setup-ssl.sh
```

**What the script does:**
1. Validates domain DNS configuration (uses external DNS servers)
2. Stops nginx temporarily (certbot needs port 80)
3. Obtains Let's Encrypt certificates for all domains
4. Sets up automatic renewal via systemd timer
5. Configures renewal hook to reload nginx after certificate updates

**You'll be prompted for:**
- Email address for Let's Encrypt notifications (required)

**Systemd Timer:** The script automatically creates and enables a systemd timer that runs twice daily (00:00 and 12:00 UTC) to renew certificates. See "SSL Certificate Renewal" section for details.

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

### 5.3 Verify Nginx Configuration

The nginx configuration is already set up to connect to Docker containers via localhost ports:
- API container: `127.0.0.1:8081`
- Admin FE container: `127.0.0.1:8082`
- Static website container: `127.0.0.1:8083`

The docker-compose.demo.yml file exposes these ports automatically. No additional configuration needed.

## Step 6: Configure Docker Access

### 6.1 Enable Docker Socket Access for Your User

**Required:** Add your user to the docker group to run Docker commands without sudo:

```bash
# Add your user to docker group
sudo usermod -aG docker $USER

# Apply the changes immediately (without logging out)
newgrp docker

# Verify Docker access works
docker ps
```

**If you get "permission denied" errors:**
- Make sure you ran `newgrp docker` after adding your user to the group
- Or log out and log back in to apply the group changes
- Verify with: `groups` (should show 'docker' in the list)

### 6.2 Start All Services

**Note:** Since this is a public repository, no authentication is needed to pull Docker images from GHCR.

```bash
cd /opt/r-data-core-demo

# Start all services
docker compose -f docker-compose.demo.yml up -d
```

**Note:** Make sure you've synced the static-website directory to `/opt/r-data-core-demo/repo/` before starting services (see Step 1.3).

### 6.3 Check Service Status

```bash
# View all services
docker compose -f docker-compose.demo.yml ps

# View logs (all services)
docker compose -f docker-compose.demo.yml logs -f

# View logs for specific service
docker compose -f docker-compose.demo.yml logs -f core
```

### 6.4 Verify Services Are Running

```bash
# Check API health endpoint
curl http://localhost:8081/api/v1/health

# Check admin FE health endpoint
curl http://localhost:8082/health

# Check static website health endpoint
curl http://localhost:8083/health
```

**Expected output:** All services should return HTTP 200 OK responses.

## Step 7: Start Nginx (Optional: Static Website)

**Note:** The static website service is commented out in docker-compose.demo.yml. If you want to deploy it:
1. Build and publish the static website image to GHCR, or
2. Uncomment the static-website service in docker-compose.demo.yml and build it locally

For now, continue with the API and Admin FE services.

### 7.1 Start Nginx Service

```bash
# Start nginx
sudo systemctl start nginx

# Enable nginx to start on boot
sudo systemctl enable nginx

# Check nginx status
sudo systemctl status nginx
```

### 7.2 Test Endpoints

```bash
# Test API (HTTP should redirect to HTTPS)
curl -I http://api.rdatacore.eu
curl -I https://api.rdatacore.eu

# Test Admin FE
curl -I https://demo.rdatacore.eu

# Test Static Website
curl -I https://rdatacore.eu
```

**Expected:** All endpoints should return HTTP 200 or 301 (redirect) responses.

## Step 8: Configure GitHub Actions Deployment

### 8.1 Generate SSH Key for GitHub Actions

On your local machine, generate a dedicated SSH key:

```bash
ssh-keygen -t ed25519 -C "github-actions-demo" -f ~/.ssh/demo_server_key
```

### 8.2 Add SSH Key to Server

Copy the public key to the server:

```bash
ssh-copy-id -i ~/.ssh/demo_server_key.pub root@your-server-ip
```

Or manually:
```bash
cat ~/.ssh/demo_server_key.pub | ssh root@your-server-ip "mkdir -p ~/.ssh && chmod 700 ~/.ssh && cat >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys"
```

### 8.3 Add GitHub Secrets

In your GitHub repository:
1. Go to **Settings** → **Secrets and variables** → **Actions**
2. Click **New repository secret**
3. Add the following secrets:

   - **Name:** `DEMO_SERVER_SSH_KEY`
   - **Value:** Contents of `~/.ssh/demo_server_key` (the private key)
   
   ```bash
   cat ~/.ssh/demo_server_key
   ```
   
   Copy the entire output and paste it as the secret value.

### 8.4 Deploy via GitHub Actions

1. Go to **Actions** tab in your GitHub repository
2. Select one of these workflows:
   - "Deploy Backend to Demo Server"
   - "Deploy Frontend to Demo Server"
   - "Deploy Static Website to Demo Server"
3. Click **Run workflow**
4. Fill in the inputs:
   - **Server host:** Your server IP or hostname
   - **Server user:** `root` (or your SSH user)
   - **Compose file path:** `/opt/r-data-core-demo/docker-compose.demo.yml`
5. Click **Run workflow**

The workflow will SSH to your server and update the Docker containers.

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

**Option 1: Use GitHub Actions (Recommended)**

Use the deployment workflows in GitHub Actions (see Step 8).

**Option 2: Manual Update**

```bash
cd /opt/r-data-core-demo

# Pull latest images
docker compose -f docker-compose.demo.yml pull

# Update static website source (if needed)
# From your local machine:
rsync -avz --delete static-website/ root@your-server-ip:/opt/r-data-core-demo/repo/static-website/
rsync -avz .docker/static-website/ root@your-server-ip:/opt/r-data-core-demo/repo/.docker/static-website/

# Rebuild static website if source changed
docker compose -f docker-compose.demo.yml build static-website

# Restart services with new images
docker compose -f docker-compose.demo.yml up -d

# Verify services are running
docker compose -f docker-compose.demo.yml ps
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
# Check service status
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml ps

# Check logs for errors
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml logs

# Check specific service logs
docker compose -f /opt/r-data-core-demo/docker-compose.demo.yml logs core

# Check Docker network
docker network ls
docker network inspect rdata_demo_network

# Verify Docker socket permissions
docker ps
# If permission denied, add user to docker group:
sudo usermod -aG docker $USER
newgrp docker
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
# Test DNS resolution (uses external DNS servers)
dig @8.8.8.8 api.rdatacore.eu
dig @8.8.8.8 AAAA api.rdatacore.eu

# Test connectivity
ping api.rdatacore.eu
ping6 api.rdatacore.eu

# Run validation script
cd /opt/r-data-core-demo/scripts
./validate-domains.sh
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

The setup script implements these security measures:

1. **SSH Key-Only Authentication**: Password authentication is disabled
2. **Firewall (UFW)**: Only necessary ports are open (22, 80, 443, 2224)
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

