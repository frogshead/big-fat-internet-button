# BIG RED INTERNET BUTTON
Dictator with nuclear war heads simulator

A fun project combining embedded Rust on ESP32-C6 with a web backend to simulate launching nuclear destruction with a big red button!

## Project Structure

This is a Cargo workspace with two components:

- `firmware/` - ESP32-C6 firmware (RISC-V) for the physical button
- `backend/` - Rust + Axum web server for logging destruction events

## Hardware Specification

- **Dev Board**: [ESP32-C6-DevKitC-1](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32c6/esp32-c6-devkitc-1/user_guide.html#getting-started)
- **Button**: [Big Red Button from AliExpress](https://www.aliexpress.com/item/1005008644989428.html?spm=a2g0o.order_list.order_list_main.5.e77a1802nCw0Yu)

## Backend

The backend is a Rust web server built with Axum that provides:

### Features

- **World Destruction API** - Receives button press events from ESP32
- **Event Storage** - In-memory storage of all destruction events
- **Admin Dashboard** - Web interface to monitor all button presses
- **Auto-refresh** - Admin page updates every 5 seconds
- **JSON API** - Programmatic access to event data

### Running the Backend

```bash
# From the project root
cargo run --bin backend

# Or from the backend directory
cd backend
cargo run
```

The server will start on `http://localhost:3000`

### API Endpoints

**POST /api/destroy** - Trigger world destruction
```bash
curl -X POST http://localhost:3000/api/destroy \
  -H "Content-Type: application/json" \
  -d '{"device_id": "esp32-001"}'
```

Response:
```json
{
  "status": "WORLD DESTROYED",
  "event_id": 1,
  "message": "💥 Nuclear launch successful! Goodbye cruel world!",
  "timestamp": "2025-11-01T09:29:12.203061255Z"
}
```

**GET /api/events** - Get all destruction events (JSON)
```bash
curl http://localhost:3000/api/events
```

**GET /admin** - View the admin dashboard in your browser
```
http://localhost:3000/admin
```

**GET /** - Landing page

### Admin Dashboard

The admin dashboard shows:
- Total number of world destructions
- Table of all events with timestamps and device IDs
- Auto-refresh every 5 seconds
- Nuclear-themed styling with glowing red effects

## Firmware (ESP32-C6)

Currently a basic hello world that logs counter values. Will be updated to:
- Connect to WiFi
- Monitor button press
- Send POST requests to backend on button press

### Building and Flashing Firmware

```bash
cd firmware

# Option 1: Build and flash in one command (recommended)
cargo espflash flash --release --monitor

# Option 2: Build first, then flash
cargo build --release
espflash flash --monitor target/riscv32imac-unknown-none-elf/release/firmware

# Specify port explicitly (useful if auto-detect fails)
espflash flash --port /dev/ttyACM0 --monitor target/riscv32imac-unknown-none-elf/release/firmware

# Or with cargo espflash
cargo espflash flash --release --port /dev/ttyACM0 --monitor
```

### Monitoring Serial Output

After flashing, you can monitor the serial output:

```bash
# Using screen (Linux/macOS)
screen /dev/ttyACM0 115200

# Or using espflash monitor
espflash monitor
```

**Exiting screen:**
- `Ctrl+A` then `K` (then press `y` to confirm) - kills the session
- `Ctrl+A` then `\` - quits screen
- `Ctrl+A` then `D` - detaches (session runs in background)

## Future Plans

- [ ] Add WiFi connectivity to ESP32
- [ ] Implement button GPIO monitoring
- [ ] Send HTTP requests to backend on button press
- [ ] Add sound effects/LED animations
- [ ] Persistent storage (database) for backend
- [ ] Authentication for admin page
- [ ] Websocket support for real-time updates

