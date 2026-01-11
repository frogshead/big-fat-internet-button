# HTTP Gateway Setup for ESP32 Devices

> **💡 Tip:** Looking to run the gateway on a Raspberry Pi? See [RASPBERRY-PI-GATEWAY.md](RASPBERRY-PI-GATEWAY.md) for native and Docker deployment options!

## Architecture Overview

This setup uses a hybrid architecture to allow ESP32 devices to use simple HTTP while maintaining HTTPS security for public access:

```
┌─────────────┐ HTTP      ┌──────────────┐           ┌─────────────┐
│   ESP32     │───────────▶│ Nginx :8080  │──────────▶│  Backend    │
│  (WiFi)     │  Plain     │ HTTP Gateway │  Local    │  :4000      │
└─────────────┘            └──────────────┘           └─────────────┘
                                  ▲
┌─────────────┐ HTTPS             │
│ Public Web  │───────────────────┘
│  Browsers   │  Port 443
└─────────────┘
```

**Benefits:**
- ✅ ESP32 uses simple HTTP (no TLS overhead, no memory constraints)
- ✅ Public access is still HTTPS (secure, uses Let's Encrypt)
- ✅ Single backend serves both HTTP (ESP32) and HTTPS (public)
- ✅ Easy to maintain and debug
- ✅ No dependency conflicts with embedded TLS libraries

## Installation

### 1. Install the HTTP Gateway Configuration

```bash
# Copy the gateway config
sudo cp nginx/esp32-http-gateway.conf /etc/nginx/sites-available/

# Enable the gateway
sudo ln -s /etc/nginx/sites-available/esp32-http-gateway.conf /etc/nginx/sites-enabled/

# Test nginx configuration
sudo nginx -t

# Reload nginx
sudo systemctl reload nginx
```

### 2. Configure Firewall

**Option A: Internal Network Only (Recommended for Security)**

If your ESP32 is on the same network as the server:
- No firewall changes needed
- Port 8080 only accessible from local network
- Most secure option

**Option B: Allow Remote ESP32 Devices**

If your ESP32 is on a different network and needs to connect over the internet:

```bash
# UFW (Ubuntu/Debian)
sudo ufw allow 8080/tcp comment 'ESP32 HTTP Gateway'

# firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --reload
```

⚠️ **Security Warning**: Exposing port 8080 to the internet sends unencrypted HTTP traffic. Consider:
- Using VPN/WireGuard for remote ESP32 devices
- Implementing API key authentication
- Rate limiting (already configured in nginx config)

### 3. Configure ESP32 Firmware

Update `firmware/.cargo/config.toml`:

```toml
[env]
WIFI_SSID="your-wifi-ssid"
WIFI_PASSWORD="your-wifi-password"
# Use your server's IP address or hostname
BACKEND_URL="YOUR_SERVER_IP:8080"
```

Examples:
- Local network: `BACKEND_URL="192.168.1.100:8080"`
- Public IP: `BACKEND_URL="203.0.113.42:8080"`
- Domain (no HTTPS): `BACKEND_URL="arewegoneyet.viitamäki.fi:8080"`

**Note**: Use IP:PORT format, NOT full URL. The firmware expects "IP:PORT" or "HOSTNAME:PORT".

### 4. Test the Gateway

```bash
# Test from the server itself
curl http://localhost:8080/health
# Should return: Gateway OK

# Test the full API endpoint
curl -X POST http://localhost:8080/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-gateway-001"}'

# Test from ESP32's network (replace with your server IP)
curl -X POST http://YOUR_SERVER_IP:8080/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-from-network"}'
```

## Verifying the Setup

### Check Nginx Logs

```bash
# Watch gateway access log
sudo tail -f /var/log/nginx/esp32_gateway_access.log

# Watch for errors
sudo tail -f /var/log/nginx/esp32_gateway_error.log
```

### Test ESP32 Connection

1. Flash firmware with updated BACKEND_URL
2. Press the button
3. Check serial output for successful HTTP POST
4. Verify event appears in admin dashboard: https://arewegoneyet.viitamäki.fi/admin
5. Check nginx gateway logs for the request

## Security Considerations

### Rate Limiting

The default configuration limits requests to:
- 10 requests/second sustained
- Burst of 20 requests
- Per IP address

Adjust in `esp32-http-gateway.conf`:
```nginx
limit_req_zone $binary_remote_addr zone=esp32_limit:10m rate=10r/s;
limit_req zone=esp32_limit burst=20 nodelay;
```

### IP Whitelisting (Optional)

To only allow specific ESP32 devices:

```nginx
# Add to server block in esp32-http-gateway.conf
# Allow only specific IPs
allow 192.168.1.50;  # ESP32 device 1
allow 192.168.1.51;  # ESP32 device 2
deny all;
```

### API Key Authentication (Advanced)

For additional security, you could:
1. Add a shared secret to ESP32 firmware
2. Include it as an HTTP header
3. Validate in nginx using lua or forward to backend for validation

## Troubleshooting

### ESP32 Can't Connect

**Check DNS/IP Resolution:**
```bash
# From ESP32's network, test connectivity
ping YOUR_SERVER_IP
telnet YOUR_SERVER_IP 8080
```

**Check Firewall:**
```bash
# Verify port 8080 is open
sudo netstat -tlnp | grep 8080
# Should show nginx listening on port 8080
```

**Check nginx Status:**
```bash
sudo systemctl status nginx
sudo nginx -t
```

### Requests Not Reaching Backend

**Check Backend is Running:**
```bash
# Verify backend on port 4000
curl http://localhost:4000/
docker ps | grep big-red-button
```

**Check Proxy Configuration:**
```bash
# Test proxy from server
curl -v http://localhost:8080/api/destroy \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test"}'
```

### Rate Limiting Issues

If legitimate ESP32 requests are being rate-limited:

```bash
# Check nginx error log for rate limit messages
sudo grep "limiting requests" /var/log/nginx/esp32_gateway_error.log

# Increase rate limit in esp32-http-gateway.conf
# Then reload: sudo systemctl reload nginx
```

## Monitoring

### View Gateway Statistics

```bash
# Count requests in last hour
sudo grep "$(date '+%d/%b/%Y:%H')" /var/log/nginx/esp32_gateway_access.log | wc -l

# Show unique ESP32 IPs
sudo awk '{print $1}' /var/log/nginx/esp32_gateway_access.log | sort -u

# Show error rate
sudo grep " 50[0-9] " /var/log/nginx/esp32_gateway_access.log | wc -l
```

### Set Up Alerts (Optional)

Monitor for:
- High error rates (5xx responses)
- Unusual request patterns
- Devices that fail repeatedly

## Architecture Benefits

**For ESP32:**
- Simple HTTP-only code
- No TLS memory overhead (~50 KB saved)
- No dependency conflicts with embedded libraries
- Faster connections (no TLS handshake)
- More reliable (fewer failure points)

**For Production:**
- Public HTTPS endpoint remains secure
- SSL/TLS termination handled by nginx (optimized)
- Easy to scale (add more ESP32s without firmware changes)
- Centralized logging and monitoring
- Can upgrade TLS versions without touching ESP32

**For Development:**
- Easy to debug (HTTP is human-readable)
- Can test with curl/Postman
- Serial logs show actual HTTP traffic
- No certificate issues during development

## Alternative: VPN for Remote ESP32

If your ESP32 is deployed remotely and you want secure communication without exposing port 8080:

1. **Set up WireGuard VPN** on your server
2. **Configure ESP32 to connect to VPN** (requires VPN client on ESP32)
3. **ESP32 communicates over VPN tunnel** (secure even with HTTP)
4. **No need to expose port 8080 publicly**

This is more complex but provides end-to-end encryption without TLS overhead on ESP32.

## Maintenance

### Update Gateway Configuration

```bash
# Edit configuration
sudo nano /etc/nginx/sites-available/esp32-http-gateway.conf

# Test changes
sudo nginx -t

# Apply changes
sudo systemctl reload nginx
```

### Rotate Logs

Logs are automatically rotated by logrotate. Configuration in `/etc/logrotate.d/nginx`.

### Monitor Disk Space

```bash
# Check log size
du -h /var/log/nginx/esp32_gateway_*.log
```

## FAQ

**Q: Why not use HTTPS on ESP32?**
A: ESP32-C6 has limited RAM (~512 KB) and no hardware crypto acceleration. TLS adds 50+ KB memory overhead and complex dependencies that conflict with stable firmware.

**Q: Is HTTP secure enough?**
A: For local network deployment, yes. For remote devices, consider VPN. The public interface (arewegoneyet.viitamäki.fi) uses HTTPS for web users.

**Q: Can I use this setup with multiple ESP32 devices?**
A: Yes! All ESP32s can connect to the same gateway (port 8080). The backend handles multiple device_ids.

**Q: What if my server restarts?**
A: Nginx and the backend auto-start (configured with `restart: unless-stopped` in docker-compose). ESP32 will automatically reconnect on next button press.

**Q: Can I run the gateway on a different port?**
A: Yes, change `listen 8080` to your preferred port in esp32-http-gateway.conf, and update ESP32 firmware BACKEND_URL accordingly.

## Summary

The HTTP Gateway provides a **pragmatic solution** that:
- Keeps ESP32 firmware simple and reliable
- Maintains HTTPS security for public users
- Avoids embedded TLS dependency hell
- Scales easily for multiple devices
- Is easy to monitor and debug

This architecture is used in production IoT deployments worldwide.
