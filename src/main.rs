#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use esp_hal::{delay::Delay, peripherals, prelude::*, rng::Rng};

#[entry]
fn main() -> ! {
    let peripherals = unsafe { peripherals::Peripherals::steal() };
    let delay = Delay::new();
    let mut rng = Rng::new(peripherals.RNG);

    println!("ESP32-C6 DevKitC-1 Started");
    loop {
        println!("Random number generated: {}", rng.random());
        delay.delay_millis(500);
    }
}
