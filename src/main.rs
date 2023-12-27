#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay, Rng, xtensa_lx::get_processor_id };

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let mut rng = Rng::new(peripherals.RNG);
    // let random = rng.random();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);
    println!("Chip id: {:?}", get_processor_id());
    loop {
        println!("Random number generated: {:?}", rng.random());
        // println!("Loop...");
        delay.delay_ms(500u32);
    }
}
