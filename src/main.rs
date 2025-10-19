#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{delay::Delay, prelude::*};

#[entry]
fn main() -> ! {
    let _peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    esp_println::logger::init_logger_from_env();

    log::info!("Hello from ESP32-C6!");

    let mut counter = 0u32;
    loop {
        log::info!("Counter: {}", counter);
        counter = counter.wrapping_add(1);
        delay.delay(1000.millis());
    }
}
