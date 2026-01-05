#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
};

const DEVICE_ID: &str = "esp32-button-001";
const BLINK_DURATION_MS: u32 = 5000;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    esp_println::logger::init_logger_from_env();

    log::info!("===========================================");
    log::info!("  BIG RED INTERNET BUTTON");
    log::info!("  Device: {}", DEVICE_ID);
    log::info!("===========================================");

    // Setup GPIO for button and LED
    let button = Input::new(peripherals.GPIO9, InputConfig::default().with_pull(Pull::Up));
    let mut led = Output::new(peripherals.GPIO8, Level::Low, OutputConfig::default());

    log::info!("Button: GPIO9 (BOOT button)");
    log::info!("LED: GPIO8");
    log::info!("");

    // Startup blink sequence (3 quick blinks)
    for _ in 0..3 {
        led.set_high();
        delay.delay_millis(200);
        led.set_low();
        delay.delay_millis(200);
    }

    log::info!("BUTTON ARMED - Press to launch!");
    log::info!("");

    let mut launch_count = 0u32;
    let mut last_state = button.is_high();

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

            // Blink LED for 5 seconds (countdown)
            let blink_interval_ms = 500; // Blink every 500ms
            let total_blinks = BLINK_DURATION_MS / blink_interval_ms;

            for i in 0..total_blinks {
                led.toggle();
                delay.delay_millis(blink_interval_ms);

                // Log countdown every second (every 2 blinks)
                if i % 2 == 0 {
                    let remaining = 5 - (i / 2);
                    log::info!("T-{} seconds...", remaining);
                }
            }

            log::warn!("LAUNCH!");
            log::warn!("");

            // Ensure LED is off after countdown
            led.set_low();

            // Debounce delay
            delay.delay_millis(500);
        }

        last_state = current_state;
        delay.delay_millis(50);
    }
}
