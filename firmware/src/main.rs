#![no_std]
#![no_main]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, Io, Level, Output, Pull},
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_wifi::{
    init,
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiInitFor,
};
use static_cell::StaticCell;

// WiFi Configuration - UPDATE THESE!
const SSID: &str = "YOUR_WIFI_SSID";
const PASSWORD: &str = "YOUR_WIFI_PASSWORD";
const BACKEND_IP: &str = "192.168.1.100"; // Your backend server IP
const BACKEND_PORT: u16 = 3000;
const DEVICE_ID: &str = "esp32-button-001";

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();

    log::info!("===========================================");
    log::info!("  BIG RED INTERNET BUTTON (Embassy)");
    log::info!("  Device: {}", DEVICE_ID);
    log::info!("===========================================");

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialize heap allocator
    esp_alloc::heap_allocator!(72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    // Initialize Embassy timer
    esp_hal_embassy::init(timg0.timer0);

    // Initialize WiFi
    let init = init(
        EspWifiInitFor::Wifi,
        timg0.timer1,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    // Setup GPIO for button and LED
    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let button = Input::new(io.pins.gpio9, Pull::Up);
    let led = Output::new(io.pins.gpio8, Level::Low);

    log::info!("Button: GPIO9 (BOOT button)");
    log::info!("LED: GPIO8");
    log::info!("");

    // Configure WiFi
    let wifi_config = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        password: PASSWORD.try_into().unwrap(),
        ..Default::default()
    });

    // Spawn WiFi connection task
    spawner.spawn(wifi_task(controller, wifi_config)).ok();

    // Setup network stack
    static STACK: StaticCell<Stack<WifiDevice<'static, WifiStaDevice>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    let stack = &*STACK.init(Stack::new(
        wifi_interface,
        Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::<3>::new()),
        embassy_time::with_nanos_since_epoch(|| 0).0,
    ));

    // Spawn network task
    spawner.spawn(net_task(stack)).ok();

    // Wait for network to be ready
    log::info!("Waiting for network...");
    while !stack.is_link_up() {
        Timer::after(Duration::from_millis(500)).await;
    }
    log::info!("Network link is up!");

    while !stack.is_config_up() {
        Timer::after(Duration::from_millis(500)).await;
    }

    if let Some(config) = stack.config_v4() {
        log::info!("✅ WiFi connected!");
        log::info!("IP address: {}", config.address);
    }

    // Spawn button monitoring task
    spawner.spawn(button_task(button, led, stack)).ok();

    log::info!("🔴 BUTTON ARMED - Ready to destroy the world!");
    log::info!("Backend: http://{}:{}/api/destroy", BACKEND_IP, BACKEND_PORT);
    log::info!("");

    // Keep the main task alive
    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}

#[embassy_executor::task]
async fn wifi_task(
    mut controller: WifiController<'static>,
    config: Configuration,
) {
    log::info!("Starting WiFi controller...");
    controller.set_configuration(&config).unwrap();
    controller.start().unwrap();

    log::info!("Connecting to WiFi: {}", SSID);
    controller.connect().unwrap();

    loop {
        match controller.wait_for_event().await {
            WifiEvent::StaConnected => {
                log::info!("WiFi connected!");
            }
            WifiEvent::StaDisconnected => {
                log::warn!("WiFi disconnected! Reconnecting...");
                controller.connect().unwrap();
            }
            _ => {}
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}

#[embassy_executor::task]
async fn button_task(
    mut button: Input<'static>,
    mut led: Output<'static>,
    stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>,
) {
    log::info!("Button monitoring task started");

    // Startup blink sequence
    for _ in 0..3 {
        led.set_high();
        Timer::after(Duration::from_millis(200)).await;
        led.set_low();
        Timer::after(Duration::from_millis(200)).await;
    }

    let mut destruction_count = 0u32;
    let mut last_state = button.is_high();

    loop {
        let current_state = button.is_high();

        // Button pressed (active low with pull-up)
        if last_state && !current_state {
            destruction_count += 1;
            led.set_high();

            log::warn!("");
            log::warn!("╔════════════════════════════════════════╗");
            log::warn!("║  💥💥 BUTTON PRESSED! 💥💥          ║");
            log::warn!("║                                        ║");
            log::warn!("║  World Destruction Event #{}           ║", destruction_count);
            log::warn!("║  Device: {}             ║", DEVICE_ID);
            log::warn!("╚════════════════════════════════════════╝");
            log::warn!("");

            // Send HTTP request if network is up
            if stack.is_config_up() {
                match send_destruction_event(stack, destruction_count).await {
                    Ok(_) => {
                        log::info!("✅ Destruction event sent to backend!");
                        // Success blink
                        for _ in 0..5 {
                            led.toggle();
                            Timer::after(Duration::from_millis(100)).await;
                        }
                    }
                    Err(e) => {
                        log::error!("❌ Failed to send event: {}", e);
                        // Error blink
                        for _ in 0..10 {
                            led.toggle();
                            Timer::after(Duration::from_millis(50)).await;
                        }
                    }
                }
            } else {
                log::warn!("⚠️  Network not ready, event logged locally only");
                for _ in 0..3 {
                    led.toggle();
                    Timer::after(Duration::from_millis(100)).await;
                }
            }

            led.set_low();

            // Debounce delay
            Timer::after(Duration::from_millis(500)).await;
        }

        last_state = current_state;
        Timer::after(Duration::from_millis(50)).await;
    }
}

async fn send_destruction_event(
    stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>,
    count: u32,
) -> Result<(), &'static str> {
    use core::fmt::Write;
    use embassy_net::tcp::TcpSocket;
    use embedded_io_async::{Read, Write as AsyncWrite};

    log::info!("📤 Sending HTTP POST to backend...");

    // Parse backend IP address
    let ip_parts: heapless::Vec<u8, 4> = BACKEND_IP
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();

    if ip_parts.len() != 4 {
        return Err("Invalid IP address");
    }

    let remote_addr = embassy_net::IpAddress::v4(
        ip_parts[0], ip_parts[1], ip_parts[2], ip_parts[3]
    );
    let remote_endpoint = (remote_addr, BACKEND_PORT);

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    log::info!("Connecting to {}:{}...", BACKEND_IP, BACKEND_PORT);
    socket
        .connect(remote_endpoint)
        .await
        .map_err(|_| "Connection failed")?;

    log::info!("Connected! Sending request...");

    // Build HTTP POST request
    let mut request = heapless::String::<512>::new();
    write!(
        &mut request,
        "POST /api/destroy HTTP/1.1\r\n\
         Host: {}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {{\"device_id\":\"{}\"}}\r\n",
        BACKEND_IP,
        DEVICE_ID.len() + 17, // Length of JSON: {"device_id":"..."}
        DEVICE_ID
    )
    .map_err(|_| "Failed to format request")?;

    // Send request
    socket
        .write_all(request.as_bytes())
        .await
        .map_err(|_| "Failed to send request")?;

    socket.flush().await.map_err(|_| "Failed to flush")?;

    log::info!("Request sent, waiting for response...");

    // Read response
    let mut response = [0u8; 1024];
    let n = socket
        .read(&mut response)
        .await
        .map_err(|_| "Failed to read response")?;

    let response_str = core::str::from_utf8(&response[..n]).unwrap_or("<invalid utf8>");
    log::info!("Response received ({} bytes)", n);

    // Check if response contains "201" status
    if response_str.contains("201") || response_str.contains("200") {
        log::info!("✓ Backend confirmed destruction!");
        Ok(())
    } else {
        log::warn!("Unexpected response: {}", &response_str[..n.min(200)]);
        Ok(()) // Still return Ok since we got a response
    }
}
