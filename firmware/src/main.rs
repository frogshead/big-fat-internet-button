#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Pull},
    rmt::Rmt,
    time::Rate,
};
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{RGB8, SmartLedsWrite};

const DEVICE_ID: &str = "esp32-button-001";
const BLINK_DURATION_MS: u32 = 5000;

// LED color definitions
const IDLE_COLOR: RGB8 = RGB8 { r: 0, g: 50, b: 100 };  // Soft blue
// const WARNING_COLOR: RGB8 = RGB8 { r: 255, g: 0, b: 0 };  // Red
const LAUNCH_COLOR: RGB8 = RGB8 { r: 255, g: 0, b: 0 };  // Bright red
const STARTUP_COLOR: RGB8 = RGB8 { r: 0, g: 100, b: 0 };  // Green

esp_bootloader_esp_idf::esp_app_desc!();

// Calculate breathing brightness (0-255 range, mapped to 20-100)
fn breathing_brightness(time_ms: u32, period_ms: u32) -> u8 {
    let phase = (time_ms % period_ms) as u32;
    let half_period = period_ms / 2;

    let brightness = if phase < half_period {
        (phase * 255) / half_period
    } else {
        ((period_ms - phase) * 255) / half_period
    };

    // Map to 20-100 range (avoid completely off)
    20 + ((brightness as u32 * 80) / 255) as u8
}

// Calculate pulsing countdown color (increasing intensity and frequency)
fn countdown_pulse_color(elapsed_ms: u32, total_duration_ms: u32) -> RGB8 {
    let progress = elapsed_ms as u32 * 255 / total_duration_ms as u32;

    // Pulse faster as countdown progresses (1000ms -> 333ms)
    let pulse_period_ms = 1000 - (progress as u32 * 667 / 255);
    let pulse_phase = (elapsed_ms % pulse_period_ms) as u32;
    let pulse_half = pulse_period_ms / 2;

    // Triangle wave pulse
    let pulse_brightness = if pulse_phase < pulse_half {
        (pulse_phase * 255) / pulse_half
    } else {
        ((pulse_period_ms - pulse_phase) * 255) / pulse_half
    };

    // Increase max intensity from 50% to 100%
    let max_intensity = 128 + (progress as u32 * 127 / 255);
    let intensity = ((pulse_brightness as u32 * max_intensity) / 255) as u8;

    RGB8 { r: intensity, g: 0, b: 0 }
}

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    esp_println::logger::init_logger_from_env();

    log::info!("===========================================");
    log::info!("  BIG RED INTERNET BUTTON");
    log::info!("  Device: {}", DEVICE_ID);
    log::info!("===========================================");

    // Setup GPIO for button
    let button = Input::new(peripherals.GPIO9, InputConfig::default().with_pull(Pull::Up));

    // Setup RMT for WS2812B LED
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("Failed to initialize RMT");
    let mut rmt_buffer = smart_led_buffer!(1);  // Buffer for 1 LED
    let mut led = SmartLedsAdapter::new(
        rmt.channel0,
        peripherals.GPIO8,
        &mut rmt_buffer,
    );

    log::info!("Button: GPIO9 (BOOT button)");
    log::info!("LED: GPIO8");
    log::info!("");

    // Startup sequence (3 quick green flashes)
    for _ in 0..3 {
        led.write([STARTUP_COLOR].iter().cloned()).ok();
        delay.delay_millis(200);
        led.write([RGB8::default()].iter().cloned()).ok();
        delay.delay_millis(200);
    }

    log::info!("BUTTON ARMED - Press to launch!");
    log::info!("");

    let mut launch_count = 0u32;
    let mut last_state = button.is_high();
    let mut idle_time_ms = 0u32;  // Track time for breathing

    loop {
        let current_state = button.is_high();

        // Button pressed (active low with pull-up)
        if last_state && !current_state {
            launch_count += 1;

            log::warn!("");
            log::warn!("╔════════════════════════════════════════╗");
            log::warn!("║     BUTTON PRESSED!          ║");
            log::warn!("║                                        ║");
            log::warn!("║  Launch Event #{}                      ║", launch_count);
            log::warn!("║  Device: {}             ║", DEVICE_ID);
            log::warn!("║                                        ║");
            log::warn!("║  Missile launching in 5 seconds...     ║");
            log::warn!("╚════════════════════════════════════════╝");
            log::warn!("");

            // Countdown with pulsing red LED (50 x 100ms = 5000ms)
            for i in 0..50 {
                let elapsed_ms = i * 100;
                let color = countdown_pulse_color(elapsed_ms, BLINK_DURATION_MS);
                led.write([color].iter().cloned()).ok();
                delay.delay_millis(100);

                // Log countdown every second (every 10 iterations)
                if i % 10 == 0 {
                    let remaining = 5 - (i / 10);
                    log::info!("T-{} seconds...", remaining);
                }
            }

            log::warn!("LAUNCH!");
            log::warn!("");

            // Solid bright red at launch
            led.write([LAUNCH_COLOR].iter().cloned()).ok();
            delay.delay_millis(2000);

            // Reset idle time for smooth breathing restart
            idle_time_ms = 0;

            // Debounce delay
            delay.delay_millis(500);
        } else {
            // Idle breathing effect
            let brightness = breathing_brightness(idle_time_ms, 3000);
            let color = RGB8 {
                r: (IDLE_COLOR.r as u16 * brightness as u16 / 255) as u8,
                g: (IDLE_COLOR.g as u16 * brightness as u16 / 255) as u8,
                b: (IDLE_COLOR.b as u16 * brightness as u16 / 255) as u8,
            };
            led.write([color].iter().cloned()).ok();

            idle_time_ms = idle_time_ms.wrapping_add(50);
        }

        last_state = current_state;
        delay.delay_millis(50);
    }
}
