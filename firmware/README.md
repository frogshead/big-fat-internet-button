# Big Red Button Firmware

ESP32-C6 firmware for the nuclear button simulator.

## Current Status

**Basic button monitoring implemented** - The firmware monitors the BOOT button (GPIO9) and logs button presses via USB serial.

**WiFi/HTTP NOT YET IMPLEMENTED** - Due to ESP Rust dependency compatibility issues, WiFi functionality is pending. The current firmware provides a solid foundation for testing hardware and button functionality.

## Hardware

- **Board**: ESP32-C6-DevKitC-1 (RISC-V)
- **Button**: GPIO9 (built-in BOOT button for testing, external big red button can be connected here)
- **LED**: GPIO8 (RGB LED for visual feedback)

## Building

```bash
# From project root
cargo build --release -p firmware

# Or from firmware directory
cd firmware
cargo build --release
```

## Flashing

Make sure the ESP32-C6 is connected via USB:

```bash
cd firmware
cargo run --release
```

Or use espflash directly:

```bash
espflash flash target/riscv32imac-unknown-none-elf/release/firmware --monitor
```

## Monitoring Serial Output

```bash
# Using cargo (builds, flashes, and monitors)
cargo run --release

# Or use espflash monitor
espflash monitor

# Or use screen
screen /dev/ttyUSB0 115200
```

## Testing

1. Flash the firmware to your ESP32-C6
2. Open serial monitor
3. Press the BOOT button (GPIO9)
4. You should see:
   - LED blinks
   - Serial output showing "BUTTON PRESSED" with event counter

## Configuration

Edit `src/main.rs` to configure:
- `DEVICE_ID` - Unique identifier for this button device
- GPIO pins if using different hardware

## Next Steps

To add WiFi functionality:

1. **Resolve esp-wifi dependency issues** - The esp-wifi crate has compatibility problems with current esp-hal versions
2. **Add WiFi credentials** - Create a config file with SSID/password
3. **Implement HTTP client** - Send POST requests to backend on button press
4. **Add connection management** - Handle WiFi reconnection and error states

## Troubleshooting

### Build Fails
- Ensure you have the Rust RISC-V target: `rustup target add riscv32imac-unknown-none-elf`
- Install espflash: `cargo install espflash`
- Clean and rebuild: `cargo clean && cargo build --release`

### Can't Flash
- Check USB connection
- Verify port in `.cargo/config.toml` (default: `/dev/ttyUSB0`)
- Try holding BOOT button while connecting USB
- Check permissions: `sudo chmod 666 /dev/ttyUSB0`

### No Serial Output
- Set log level: `export ESP_LOG=info`
- Check baud rate is 115200
- Try different USB cable (data+power, not charge-only)
