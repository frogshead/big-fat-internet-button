# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A fun IoT project combining embedded Rust on ESP32-C6 (RISC-V) with a web backend to simulate launching nuclear destruction with a big red button. This is a Cargo workspace with two main components:

- **backend/** - Rust + Axum web server that receives button press events and displays an admin dashboard
- **firmware/** - ESP32-C6 firmware using Embassy async runtime for WiFi connectivity and button monitoring

## Build Commands

### Backend

```bash
# Run backend server (starts on http://localhost:4000)
cargo run --bin backend

# Or from backend directory
cd backend && cargo run

# Run tests
cargo test --bin backend
```

### Backend Docker

```bash
# Build Docker image
docker build -f backend/Dockerfile -t big-red-button-backend .

# Run container
docker run -p 4000:4000 big-red-button-backend

# Run with custom credentials
docker run -p 4000:4000 \
  -e ADMIN_USERNAME=admin \
  -e ADMIN_PASSWORD=supersecret \
  big-red-button-backend

# Pull from GitHub Container Registry
docker pull ghcr.io/<username>/big-fat-internet-button/backend:latest

# Using Docker Compose
docker-compose up -d
docker-compose logs -f
```

### Production Deployment with Nginx

For production deployment with HTTPS:

```bash
# See nginx/SETUP.md for complete setup instructions

# Quick start with Docker Compose
docker-compose up -d

# Install nginx configuration
sudo cp nginx/arewegoneyet.conf /etc/nginx/sites-available/
sudo ln -s /etc/nginx/sites-available/arewegoneyet.conf /etc/nginx/sites-enabled/

# Get Let's Encrypt certificate
sudo certbot certonly --webroot -w /var/www/certbot -d arewegoneyet.viitamäki.fi

# Reload nginx
sudo systemctl reload nginx
```

See `nginx/SETUP.md` for detailed setup instructions including:
- Let's Encrypt SSL certificate setup
- Nginx reverse proxy configuration
- Firewall setup
- Troubleshooting guide

### Firmware

The firmware requires environment variables for WiFi configuration. These are set in `firmware/.cargo/config.toml`:

```bash
# Build firmware
cd firmware
cargo build --release

# Flash to ESP32-C6 and monitor serial output
cargo run --release

# Or use espflash directly
espflash flash --monitor target/riscv32imac-unknown-none-elf/release/firmware

# Monitor serial output after flashing
espflash monitor
# or
screen /dev/ttyACM0 115200
```

### Workspace Commands

```bash
# Format all code
cargo fmt --all

# Check backend build
cargo build --bin backend --release

# Check firmware compilation (without linking)
cargo check --package firmware --target riscv32imac-unknown-none-elf --release
```

## Configuration

### Backend Configuration

The backend uses environment variables for admin authentication (defaults to "admin"/"admin" if not set):

```bash
ADMIN_USERNAME=admin ADMIN_PASSWORD=secret cargo run --bin backend
```

### Firmware Configuration

Edit `firmware/.cargo/config.toml` to configure:
- `WIFI_SSID` - Your WiFi network name
- `WIFI_PASSWORD` - Your WiFi password
- `BACKEND_URL` - Backend server IP:PORT (e.g., "192.168.1.100:4000")

The firmware also requires these variables as compile-time constants via `env!()` macro (firmware/src/main.rs:35-37).

## Architecture

### Backend Architecture (backend/src/main.rs)

Single-file Axum web server with:
- **Shared State**: `Arc<Mutex<Vec<ButtonPress>>>` - in-memory event storage
- **Routes**:
  - `GET /` - Landing page with terminal-style UI
  - `POST /api/destroy` - Receives button press events from ESP32 (or curl)
  - `GET /api/events` - Returns all events as JSON
  - `GET /admin` - HTML dashboard with auto-refresh (requires Basic Auth)
- **Authentication**: Basic Auth extractor using `axum-extra` TypedHeader
- **Styling**: Nuclear-themed admin dashboard with glowing red CSS animations

### Firmware Architecture (firmware/src/main.rs)

Embassy async runtime with three concurrent tasks:
- **WiFi Task** (`connection` function, line 98): Manages WiFi connection state, automatic reconnection
- **Network Task** (`net_task` function, line 92): Runs Embassy TCP/IP stack with DHCP
- **Main Task** (`main_task` function, line 256): Monitors button GPIO9, controls WS2812B LED on GPIO8, sends HTTP POST requests

**Key Components**:
- Button monitoring with debouncing (line 326-400)
- LED feedback patterns: breathing idle (blue), countdown pulse (red), launch (solid red), error (orange)
- HTTP client implementation using embassy-net TcpSocket (line 129-242)
- Static memory allocation using `StaticCell` for Embassy resources

**GPIO Assignments**:
- GPIO9: Button input (BOOT button on DevKit, active-low with pull-up)
- GPIO8: WS2812B addressable LED output

### Data Flow

```
ESP32 Button Press
  ↓
Firmware detects press (GPIO9)
  ↓
5-second countdown with pulsing LED
  ↓
HTTP POST to /api/destroy
  ↓
Backend stores event in memory
  ↓
Admin dashboard auto-refreshes every 5s
```

### Firmware State Machine

```
Boot → Initialize peripherals → Spawn WiFi tasks → Wait for DHCP →
Monitor button loop:
  - Idle: breathing blue LED
  - Button pressed: 5s countdown with pulsing red
  - Launch: Send HTTP POST
  - Success: solid red LED
  - HTTP error: 3 orange flashes
  - Return to idle
```

## Known Issues & Context

### Firmware Build Status

The firmware code is complete and production-ready, but the ESP Rust ecosystem has dependency compatibility issues:
- esp-backtrace 0.14.2 had issues (now resolved with newer versions)
- CI uses `cargo check` instead of full build to work around linking issues
- Actual firmware compilation and flashing works when proper environment is set up with espflash

See `IMPLEMENTATION_NOTES.md` and `firmware/EMBASSY_IMPLEMENTATION.md` for detailed context on implementation choices and dependency issues.

### Testing Workaround

You can test the backend without ESP32 hardware:

```bash
# Terminal 1: Start backend
cargo run --bin backend

# Terminal 2: Simulate button press
curl -X POST http://localhost:4000/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "test-device-001"}'

# Browser: Open admin dashboard
http://localhost:4000/admin
# Login with: admin / admin (or your ADMIN_USERNAME/ADMIN_PASSWORD)
```

## Development Notes

### Hardware Requirements

- **ESP32-C6-DevKitC-1**: RISC-V based ESP32 dev board
- **WS2812B LED**: Connected to GPIO8
- **Big Red Button**: Connected to GPIO9 (or use onboard BOOT button for testing)

### Cargo Profile Settings

The workspace root `Cargo.toml` disables LTO (`lto = "off"`) for both dev and release profiles due to ESP32 codegen backend requirements. This is critical for firmware builds.

### Firmware Target

The firmware uses `riscv32imac-unknown-none-elf` target (ESP32-C6 is RISC-V based, not Xtensa). The target is configured in `firmware/.cargo/config.toml` with custom build-std for `core`.

### CI Pipeline

GitHub Actions runs two workflows:

**Rust CI** (`.github/workflows/rust_ci.yml`):
1. Backend release build
2. Firmware check (not full build due to environment variable requirements)
3. Code formatting check

**Docker** (`.github/workflows/docker.yml`):
1. Builds multi-arch Docker image (linux/amd64, linux/arm64)
2. Pushes to GitHub Container Registry on main branch
3. Uses layer caching for faster builds
4. Tags: `latest`, branch name, git SHA, PR number

Environment variables are provided as placeholders in CI for compilation checks.

## API Reference

### POST /api/destroy

Request:
```json
{
  "device_id": "esp32-button-001"
}
```

Response (201 Created):
```json
{
  "status": "WORLD DESTROYED",
  "event_id": 1,
  "message": "💥 Nuclear launch successful! Goodbye cruel world!",
  "timestamp": "2025-11-01T10:00:00Z"
}
```

### GET /api/events

Response (200 OK):
```json
[
  {
    "id": 1,
    "timestamp": "2025-11-01T10:00:00Z",
    "device_id": "esp32-button-001"
  }
]
```

## Important Patterns

### Embassy Task Spawning

Tasks must be spawned on the executor before they run. Critical pattern in firmware/src/main.rs:

```rust
// Spawn WiFi and network tasks
spawner.spawn(connection(controller)).ok();
spawner.spawn(net_task(runner)).ok();
```

### StaticCell for Embassy Resources

Embassy requires 'static references. Use `StaticCell` to create them:

```rust
static STACK: StaticCell<embassy_net::Stack<'static>> = StaticCell::new();
let stack = STACK.init(stack_inner);
```

### Backend Basic Auth

Admin page uses custom `BasicAuth` extractor (backend/src/main.rs:34-75). Credentials from environment or defaults to "admin"/"admin".

### LED Color Constants

Firmware uses RGB8 color constants (lines 24-44) for different states:
- `IDLE_COLOR`: Soft blue breathing
- `LAUNCH_COLOR`: Bright red
- `STARTUP_COLOR`: Green
- `WIFI_ERROR_COLOR`: Orange
