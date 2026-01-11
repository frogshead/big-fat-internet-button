# Nginx Reverse Proxy Setup with Let's Encrypt

This guide will help you set up the Big Red Button backend behind an nginx reverse proxy with HTTPS using Let's Encrypt.

## Prerequisites

- A server with a public IP address
- Domain `arewegoneyet.viitamäki.fi` pointing to your server's IP
- Root or sudo access
- Docker installed
- Nginx installed
- Port 80 and 443 open in your firewall

## Domain Note

The domain `viitamäki.fi` contains international characters (IDN - Internationalized Domain Name). The Punycode representation is `xn--viitamki-5za.fi`. The nginx configuration uses Punycode, but you can use the regular domain name in browser URLs.

## Step 1: Install Required Software

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install nginx
sudo apt install nginx -y

# Install certbot for Let's Encrypt
sudo apt install certbot python3-certbot-nginx -y

# Verify installations
nginx -v
certbot --version
```

## Step 2: Configure DNS

Ensure your DNS A record is set:

```
arewegoneyet.viitamäki.fi  →  YOUR_SERVER_IP
```

Verify with:
```bash
dig arewegoneyet.viitamäki.fi
# or
nslookup arewegoneyet.viitamäki.fi
```

## Step 3: Start the Backend Docker Container

```bash
# Pull the image from GitHub Container Registry
docker pull ghcr.io/frogshead/big-fat-internet-button/backend:latest

# Or build locally
# docker build -f backend/Dockerfile -t big-red-button-backend .

# Run the container
docker run -d \
  --name big-red-button \
  --restart unless-stopped \
  -p 127.0.0.1:4000:4000 \
  -e ADMIN_USERNAME=admin \
  -e ADMIN_PASSWORD=YOUR_SECURE_PASSWORD_HERE \
  ghcr.io/frogshead/big-fat-internet-button/backend:latest

# Verify it's running
docker ps
curl http://localhost:4000/
```

**Important**: Note that we bind to `127.0.0.1:4000` so the container is only accessible locally (through nginx).

## Step 4: Install Nginx Configuration

```bash
# Copy the nginx configuration
sudo cp nginx/arewegoneyet.conf /etc/nginx/sites-available/arewegoneyet.conf

# Create certbot webroot directory
sudo mkdir -p /var/www/certbot

# Test nginx configuration (it will fail SSL check initially, that's OK)
sudo nginx -t

# If the test passes (ignore SSL errors for now), create symlink
sudo ln -s /etc/nginx/sites-available/arewegoneyet.conf /etc/nginx/sites-enabled/

# Remove default site if needed
sudo rm /etc/nginx/sites-enabled/default
```

## Step 5: Initial Nginx Setup (Before SSL)

For the initial setup, we need to temporarily modify the config to work without SSL:

```bash
# Backup the original config
sudo cp /etc/nginx/sites-available/arewegoneyet.conf /etc/nginx/sites-available/arewegoneyet.conf.backup

# Edit the config to comment out SSL server block temporarily
sudo nano /etc/nginx/sites-available/arewegoneyet.conf
```

Comment out the entire HTTPS server block (lines starting with `server { listen 443 ...`), or use this temporary config:

```nginx
server {
    listen 80;
    listen [::]:80;
    server_name arewegoneyet.xn--viitamki-5za.fi;

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }

    location / {
        proxy_pass http://localhost:4000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Reload nginx:
```bash
sudo systemctl reload nginx
```

## Step 6: Obtain Let's Encrypt Certificate

```bash
# Request certificate using certbot
sudo certbot certonly \
  --webroot \
  --webroot-path=/var/www/certbot \
  -d arewegoneyet.viitamäki.fi \
  --email your-email@example.com \
  --agree-tos \
  --no-eff-email

# Alternative: Use nginx plugin (easier but may modify your config)
# sudo certbot --nginx -d arewegoneyet.viitamäki.fi
```

**Note**: Certbot should handle the IDN domain name conversion automatically. If you have issues, you can explicitly use Punycode:

```bash
sudo certbot certonly \
  --webroot \
  --webroot-path=/var/www/certbot \
  -d arewegoneyet.xn--viitamki-5za.fi \
  --email your-email@example.com \
  --agree-tos \
  --no-eff-email
```

## Step 7: Enable Full Nginx Configuration

```bash
# Restore the original configuration with SSL
sudo cp /etc/nginx/sites-available/arewegoneyet.conf.backup /etc/nginx/sites-available/arewegoneyet.conf

# Test configuration
sudo nginx -t

# If test passes, reload nginx
sudo systemctl reload nginx
```

## Step 8: Verify HTTPS is Working

```bash
# Test from command line
curl https://arewegoneyet.viitamäki.fi

# Check SSL certificate
openssl s_client -connect arewegoneyet.viitamäki.fi:443 -servername arewegoneyet.viitamäki.fi
```

Open in browser:
- https://arewegoneyet.viitamäki.fi
- https://arewegoneyet.viitamäki.fi/admin (use your admin credentials)

## Step 9: Enable Auto-Renewal

Certbot should automatically set up renewal. Verify:

```bash
# Check certbot timer
sudo systemctl status certbot.timer

# Test renewal (dry run)
sudo certbot renew --dry-run

# View certificate expiry
sudo certbot certificates
```

Certificates will auto-renew 30 days before expiration.

## Step 10: Enable HSTS (Optional, Recommended)

After verifying HTTPS works correctly for a few days, enable HSTS by uncommenting this line in the nginx config:

```nginx
add_header Strict-Transport-Security "max-age=63072000" always;
```

Then reload nginx:
```bash
sudo systemctl reload nginx
```

## Firewall Configuration

Make sure ports 80 and 443 are open:

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 'Nginx Full'
sudo ufw status

# firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --reload
```

## Troubleshooting

### Certificate not found
```bash
# List certificates
sudo certbot certificates

# Check certificate files
sudo ls -la /etc/letsencrypt/live/
```

### Nginx won't start
```bash
# Check nginx error log
sudo tail -f /var/log/nginx/error.log

# Test configuration
sudo nginx -t
```

### Can't reach backend
```bash
# Check if Docker container is running
docker ps

# Check container logs
docker logs big-red-button

# Test backend directly
curl http://localhost:4000/
```

### DNS issues with IDN domain
```bash
# Convert domain to Punycode manually
python3 -c "print('arewegoneyet.viitamäki.fi'.encode('idna').decode('ascii'))"
# Output: arewegoneyet.xn--viitamki-5za.fi
```

### Certificate renewal fails
```bash
# Manual renewal
sudo certbot renew --force-renewal

# Check renewal logs
sudo journalctl -u certbot.timer
```

## Maintenance Commands

```bash
# Restart backend Docker container
docker restart big-red-button

# View backend logs
docker logs -f big-red-button

# Update backend image
docker pull ghcr.io/frogshead/big-fat-internet-button/backend:latest
docker stop big-red-button
docker rm big-red-button
# Then run the docker run command from Step 3 again

# Reload nginx after config changes
sudo systemctl reload nginx

# Check nginx status
sudo systemctl status nginx

# Renew certificates manually
sudo certbot renew
```

## Docker Compose Alternative (Optional)

For easier management, create a `docker-compose.yml`:

```yaml
version: '3.8'

services:
  backend:
    image: ghcr.io/frogshead/big-fat-internet-button/backend:latest
    container_name: big-red-button
    restart: unless-stopped
    ports:
      - "127.0.0.1:4000:4000"
    environment:
      - ADMIN_USERNAME=admin
      - ADMIN_PASSWORD=YOUR_SECURE_PASSWORD_HERE
      - RUST_LOG=info
```

Then use:
```bash
docker-compose up -d
docker-compose logs -f
docker-compose restart
docker-compose pull && docker-compose up -d  # Update
```

## Security Recommendations

1. **Change default admin password** in Docker environment variables
2. **Enable HSTS** after confirming HTTPS works
3. **Keep system updated**: `sudo apt update && sudo apt upgrade -y`
4. **Monitor logs**: `sudo tail -f /var/log/nginx/arewegoneyet_access.log`
5. **Set up fail2ban** to prevent brute force attacks
6. **Use strong passwords** for admin interface
7. **Regularly update Docker images**: `docker pull` and restart

## Testing Button Events

```bash
# Simulate button press from command line
curl -X POST https://arewegoneyet.viitamäki.fi/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-device-001"}'

# View events
curl https://arewegoneyet.viitamäki.fi/api/events

# Access admin dashboard in browser with Basic Auth
# https://arewegoneyet.viitamäki.fi/admin
```

## Support

If you encounter issues:
1. Check nginx error logs: `/var/log/nginx/arewegoneyet_error.log`
2. Check Docker logs: `docker logs big-red-button`
3. Verify DNS resolution: `dig arewegoneyet.viitamäki.fi`
4. Test backend directly: `curl http://localhost:4000/`
5. Check certbot logs: `sudo journalctl -u certbot.timer`
