# HTTP Gateway on Raspberry Pi

This guide shows how to run the ESP32 HTTP gateway on a Raspberry Pi, either natively with nginx or in a Docker container.

## Why Raspberry Pi?

✅ **Low power consumption** - Perfect for always-on gateway
✅ **Local network deployment** - ESP32 connects securely over LAN
✅ **Easy setup** - Can run headless
✅ **Cost effective** - Any Pi model works (Zero, 3, 4, 5)
✅ **Portable** - Can be placed anywhere with WiFi/Ethernet

## Architecture

```
┌─────────────┐ WiFi      ┌──────────────────┐ Internet  ┌────────────────┐
│   ESP32     │───────────▶│  Raspberry Pi    │───────────▶│ Remote Backend │
│  Device     │ HTTP:8080  │  (HTTP Gateway)  │ HTTPS:443 │ (VPS/Cloud)    │
└─────────────┘            └──────────────────┘           └────────────────┘
                           Local Network                    arewegoneyet.fi
```

**Data Flow:**
1. ESP32 sends HTTP POST to Raspberry Pi (192.168.1.x:8080)
2. Raspberry Pi forwards to remote backend via HTTPS
3. Remote backend stores event
4. Web users access via HTTPS (secure)

---

## Option 1: Native Installation (Nginx)

### Prerequisites

- Raspberry Pi (any model with WiFi/Ethernet)
- Raspberry Pi OS (Lite or Desktop)
- SD card (8GB minimum)
- Network connection

### 1. Initial Raspberry Pi Setup

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install nginx
sudo apt install nginx -y

# Verify nginx is running
sudo systemctl status nginx
```

### 2. Copy Gateway Configuration

**If you have this repo on the Pi:**
```bash
cd ~/big-fat-internet-button
sudo cp nginx/esp32-http-gateway.conf /etc/nginx/sites-available/
```

**If you don't have the repo, create the file manually:**
```bash
sudo nano /etc/nginx/sites-available/esp32-http-gateway.conf
```

Paste this configuration:
```nginx
server {
    listen 8080;
    listen [::]:8080;

    access_log /var/log/nginx/esp32_gateway_access.log;
    error_log /var/log/nginx/esp32_gateway_error.log;

    client_max_body_size 1M;

    # Proxy to remote backend
    location / {
        proxy_pass https://arewegoneyet.xn--viitamki-5za.fi;

        proxy_set_header Host arewegoneyet.xn--viitamki-5za.fi;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;

        # SSL verification
        proxy_ssl_verify on;
        proxy_ssl_trusted_certificate /etc/ssl/certs/ca-certificates.crt;
        proxy_ssl_protocols TLSv1.2 TLSv1.3;
    }

    location /health {
        access_log off;
        return 200 "Gateway OK\n";
        add_header Content-Type text/plain;
    }
}
```

### 3. Enable the Gateway

```bash
# Enable the configuration
sudo ln -s /etc/nginx/sites-available/esp32-http-gateway.conf /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Reload nginx
sudo systemctl reload nginx
```

### 4. Find Raspberry Pi's IP Address

```bash
# Get IP address
hostname -I
# Example output: 192.168.1.150
```

### 5. Test the Gateway

```bash
# Test health check
curl http://localhost:8080/health

# Test API endpoint
curl -X POST http://localhost:8080/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-from-pi"}'

# Test from another device on network (replace with Pi's IP)
curl -X POST http://192.168.1.150:8080/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-from-network"}'
```

### 6. Configure Auto-Start

Nginx starts automatically on boot by default. Verify:

```bash
sudo systemctl enable nginx
sudo systemctl is-enabled nginx
# Should output: enabled
```

---

## Option 2: Docker Container (Recommended)

Running nginx in Docker provides isolation and easy management.

### Prerequisites

```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add pi user to docker group
sudo usermod -aG docker $USER

# Log out and back in for group changes to take effect
# Or run: newgrp docker

# Verify Docker installation
docker --version
```

### 1. Create Gateway Configuration Directory

```bash
# Create directory for nginx config
mkdir -p ~/esp32-gateway/conf.d
cd ~/esp32-gateway
```

### 2. Create Nginx Configuration

```bash
cat > ~/esp32-gateway/conf.d/gateway.conf <<'EOF'
server {
    listen 8080;
    listen [::]:8080;

    access_log /var/log/nginx/access.log;
    error_log /var/log/nginx/error.log;

    client_max_body_size 1M;

    # Proxy to remote backend
    location / {
        proxy_pass https://arewegoneyet.xn--viitamki-5za.fi;

        proxy_set_header Host arewegoneyet.xn--viitamki-5za.fi;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;

        # SSL verification
        proxy_ssl_verify on;
        proxy_ssl_trusted_certificate /etc/ssl/certs/ca-certificates.crt;
        proxy_ssl_protocols TLSv1.2 TLSv1.3;
    }

    location /health {
        access_log off;
        return 200 "Gateway OK\n";
        add_header Content-Type text/plain;
    }
}
EOF
```

### 3. Create docker-compose.yml

```bash
cat > ~/esp32-gateway/docker-compose.yml <<'EOF'
version: '3.8'

services:
  gateway:
    image: nginx:alpine
    container_name: esp32-gateway
    restart: unless-stopped
    ports:
      - "8080:8080"
    volumes:
      - ./conf.d:/etc/nginx/conf.d:ro
      - gateway-logs:/var/log/nginx
    networks:
      - gateway-network

volumes:
  gateway-logs:

networks:
  gateway-network:
    driver: bridge
EOF
```

### 4. Start the Gateway

```bash
cd ~/esp32-gateway

# Start the container
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f
```

### 5. Test the Gateway

```bash
# Test health check
curl http://localhost:8080/health

# Test API endpoint
curl -X POST http://localhost:8080/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-from-docker"}'

# Find Pi's IP for ESP32 configuration
hostname -I
```

### 6. Docker Management Commands

```bash
# Stop gateway
docker-compose down

# Restart gateway
docker-compose restart

# View logs
docker-compose logs -f gateway

# Update nginx image
docker-compose pull
docker-compose up -d

# Check resource usage
docker stats esp32-gateway
```

---

## ESP32 Configuration

Update `firmware/.cargo/config.toml` with your Raspberry Pi's IP:

```toml
[env]
WIFI_SSID="your-wifi-network"
WIFI_PASSWORD="your-wifi-password"
BACKEND_URL="192.168.1.150:8080"  # Replace with your Pi's IP
```

Flash the firmware and test!

---

## Monitoring & Troubleshooting

### Check Gateway Logs

**Native nginx:**
```bash
# Follow access log
sudo tail -f /var/log/nginx/esp32_gateway_access.log

# Follow error log
sudo tail -f /var/log/nginx/esp32_gateway_error.log

# Count requests today
sudo grep "$(date '+%d/%b/%Y')" /var/log/nginx/esp32_gateway_access.log | wc -l
```

**Docker:**
```bash
# View all logs
docker-compose logs -f

# View last 50 lines
docker-compose logs --tail=50

# View logs for specific time
docker-compose logs --since 30m
```

### Test Connectivity

```bash
# From Raspberry Pi - test local gateway
curl http://localhost:8080/health

# From Raspberry Pi - test remote backend
curl https://arewegoneyet.xn--viitamki-5za.fi

# From ESP32's network - test Pi gateway
curl http://RASPBERRY_PI_IP:8080/health
```

### Common Issues

**Issue: Port 8080 already in use**
```bash
# Check what's using port 8080
sudo netstat -tlnp | grep 8080

# Or with ss
sudo ss -tlnp | grep 8080
```

**Issue: Can't reach Pi from ESP32**
```bash
# On Raspberry Pi - check firewall (usually disabled by default)
sudo iptables -L

# Ping Pi from another device
ping 192.168.1.150

# Check if nginx is listening
sudo netstat -tlnp | grep :8080
```

**Issue: SSL verification fails**
```bash
# Update CA certificates
sudo apt update
sudo apt install ca-certificates
sudo update-ca-certificates
```

---

## Performance & Resources

### Resource Usage

**Native nginx:**
- RAM: ~10-20 MB
- CPU: <1% idle, <5% under load
- Disk: ~500 KB configuration

**Docker nginx:**
- RAM: ~30-50 MB (includes container overhead)
- CPU: <1% idle, <5% under load
- Disk: ~200 MB (nginx:alpine image)

### Raspberry Pi Models

| Model | Performance | Notes |
|-------|-------------|-------|
| Pi Zero / Zero W | ✅ Good | Sufficient for 1-5 ESP32 devices |
| Pi 3 | ✅ Excellent | Handles 10+ devices easily |
| Pi 4 / Pi 5 | ✅ Excellent | Overkill but works great |

**Recommendation:** Pi Zero W is perfect for this use case - low power, WiFi built-in, adequate performance.

---

## Security Recommendations

### 1. Change Default Password

```bash
# Change pi user password
passwd
```

### 2. Enable Firewall (Optional)

```bash
# Install ufw
sudo apt install ufw

# Allow SSH (if you use it)
sudo ufw allow 22/tcp

# Allow gateway port from local network only
sudo ufw allow from 192.168.1.0/24 to any port 8080

# Enable firewall
sudo ufw enable

# Check status
sudo ufw status
```

### 3. Keep System Updated

```bash
# Update regularly
sudo apt update && sudo apt upgrade -y

# Or set up automatic updates
sudo apt install unattended-upgrades
sudo dpkg-reconfigure unattended-upgrades
```

### 4. Disable Unused Services

```bash
# List enabled services
systemctl list-unit-files --state=enabled

# Disable bluetooth if not needed
sudo systemctl disable bluetooth
```

---

## Static IP Configuration

Give your Pi a static IP so ESP32 configuration doesn't need to change:

### Using dhcpcd (Raspberry Pi OS)

```bash
# Edit dhcpcd configuration
sudo nano /etc/dhcpcd.conf

# Add at the end:
interface wlan0  # or eth0 for Ethernet
static ip_address=192.168.1.150/24
static routers=192.168.1.1
static domain_name_servers=192.168.1.1 8.8.8.8

# Save and reboot
sudo reboot
```

### Using Router DHCP Reservation

Alternatively, configure your router to always assign the same IP to the Pi based on its MAC address.

---

## Backup & Recovery

### Backup Configuration

```bash
# Native nginx
sudo cp /etc/nginx/sites-available/esp32-http-gateway.conf ~/esp32-gateway-backup.conf

# Docker
cd ~/esp32-gateway
tar -czf esp32-gateway-backup.tar.gz conf.d docker-compose.yml
```

### SD Card Image

Create a full SD card backup after setup:

**On Linux/Mac:**
```bash
# Find SD card device
lsblk

# Create image (example: /dev/sdb)
sudo dd if=/dev/sdb of=~/raspberry-pi-gateway.img bs=4M status=progress

# Compress to save space
gzip ~/raspberry-pi-gateway.img
```

**On Windows:** Use Win32DiskImager or Raspberry Pi Imager

---

## Advanced: Multiple ESP32 Devices

The gateway handles multiple devices automatically. Each ESP32 can have a unique `device_id`:

**ESP32 #1:**
```toml
BACKEND_URL="192.168.1.150:8080"
# Uses device_id = "esp32-button-001" in firmware
```

**ESP32 #2:**
```toml
BACKEND_URL="192.168.1.150:8080"
# Change device_id = "esp32-button-002" in firmware
```

All devices send to the same gateway, backend differentiates by `device_id`.

---

## Comparison: Native vs Docker

| Feature | Native Nginx | Docker Nginx |
|---------|--------------|--------------|
| RAM Usage | 10-20 MB | 30-50 MB |
| Setup Complexity | Simple | Medium |
| Isolation | No | Yes |
| Updates | apt upgrade | docker-compose pull |
| Portability | No | Yes (move docker-compose.yml) |
| Resource Overhead | Minimal | Slight |
| Recommended For | Production | Development/Testing |

**Verdict:** Both work great! Native is slightly more efficient, Docker is easier to manage and portable.

---

## Summary

✅ **Raspberry Pi makes an excellent HTTP gateway**
✅ **Low power, always-on, local network**
✅ **Both native and Docker options work well**
✅ **ESP32 stays simple with HTTP-only code**
✅ **Remote backend handles HTTPS for public access**

Choose:
- **Native nginx** if you want maximum efficiency
- **Docker** if you want easy management and portability

Both approaches keep your ESP32 firmware simple while maintaining end-to-end security!
