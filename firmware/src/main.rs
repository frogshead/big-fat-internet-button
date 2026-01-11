#![no_std]
#![no_main]

extern crate alloc;

use embassy_net::{dns::{DnsSocket, DnsQueryType}, tcp::TcpSocket, IpAddress, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, InputConfig, Pull},
    rmt::Rmt,
    time::Rate,
};
use esp_hal_smartled::{smart_led_buffer, SmartLedsAdapter};
use smart_leds::{SmartLedsWrite, RGB8};
use static_cell::StaticCell;

// WiFi and networking imports (using Embassy async with esp-radio)
use esp_radio::wifi::{new as wifi_new, ClientConfig, WifiDevice};

const DEVICE_ID: &str = "esp32-button-001";
const BLINK_DURATION_MS: u32 = 5000;

// LED color definitions
const IDLE_COLOR: RGB8 = RGB8 {
    r: 0,
    g: 50,
    b: 100,
}; // Soft blue
   // const WARNING_COLOR: RGB8 = RGB8 { r: 255, g: 0, b: 0 };  // Red
const LAUNCH_COLOR: RGB8 = RGB8 { r: 255, g: 0, b: 0 }; // Bright red
const STARTUP_COLOR: RGB8 = RGB8 { r: 0, g: 100, b: 0 }; // Green

// WiFi configuration (from environment variables)
const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
const BACKEND_URL: &str = env!("BACKEND_URL");

// WiFi status colors
const WIFI_ERROR_COLOR: RGB8 = RGB8 {
    r: 100,
    g: 50,
    b: 0,
}; // Orange

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

    RGB8 {
        r: intensity,
        g: 0,
        b: 0,
    }
}

// Embassy network task - runs the network stack
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, WifiDevice<'static>>) -> ! {
    runner.run().await
}

// Embassy connection task - manages WiFi connection
#[embassy_executor::task]
async fn connection(mut controller: esp_radio::wifi::WifiController<'static>) -> ! {
    log::info!("Starting WiFi connection task");
    loop {
        if matches!(controller.is_started(), Ok(false)) {
            let client_config = ClientConfig::default()
                .with_ssid(WIFI_SSID.into())
                .with_password(WIFI_PASSWORD.into());

            let config = esp_radio::wifi::ModeConfig::Client(client_config);
            controller.set_config(&config).ok();

            log::info!("Starting WiFi controller...");
            controller.start_async().await.ok();
        }

        if matches!(controller.is_connected(), Ok(false)) {
            log::info!("Connecting to WiFi...");
            match controller.connect_async().await {
                Ok(_) => log::info!("WiFi connected!"),
                Err(e) => {
                    log::error!("Failed to connect to WiFi: {:?}", e);
                    Timer::after(Duration::from_secs(5)).await;
                }
            }
        }

        Timer::after(Duration::from_millis(1000)).await;
    }
}

/// Parse HTTPS URL and resolve hostname to IP address
/// Returns (hostname, port, ip_address) or error
async fn resolve_backend_url(
    stack: &'static embassy_net::Stack<'static>,
    url: &str,
) -> Result<(heapless::String<128>, u16, IpAddress), ()> {
    // Parse HTTPS URL format: https://hostname:port/path
    if !url.starts_with("https://") {
        log::error!("URL must start with https://");
        return Err(());
    }

    let url_without_proto = &url[8..]; // Strip "https://"

    // Split hostname/path
    let (host_port, _path) = url_without_proto
        .split_once('/')
        .unwrap_or((url_without_proto, ""));

    // Parse hostname and port
    let (hostname, port) = if let Some((h, p)) = host_port.split_once(':') {
        (h, p.parse().unwrap_or(443))
    } else {
        (host_port, 443)
    };

    log::info!("Resolving hostname: {}", hostname);

    // Wait for DHCP to configure DNS servers
    stack.wait_config_up().await;

    // Perform DNS query
    let dns = DnsSocket::new(stack);
    match dns.query(hostname, DnsQueryType::A).await {
        Ok(addrs) => {
            if let Some(addr) = addrs.first() {
                log::info!("Resolved {} to {:?}", hostname, addr);
                let mut hostname_str = heapless::String::new();
                use core::fmt::Write;
                write!(hostname_str, "{}", hostname).ok();
                Ok((hostname_str, port, *addr))
            } else {
                log::error!("DNS returned no addresses");
                Err(())
            }
        }
        Err(e) => {
            log::error!("DNS resolution failed: {:?}", e);
            Err(())
        }
    }
}

// Send HTTP POST request to backend using embassy-net
async fn send_destruction_event(
    stack: &'static embassy_net::Stack<'static>,
    device_id: &str,
) -> bool {
    log::info!("Sending destruction event to backend...");

    // Wait for network to be ready
    stack.wait_config_up().await;

    // Parse BACKEND_URL (format: "IP:PORT")
    let parts: heapless::Vec<&str, 2> = BACKEND_URL.split(':').collect();
    if parts.len() < 2 {
        log::error!("Invalid BACKEND_URL format: {}", BACKEND_URL);
        return false;
    }

    let ip_str = parts[0];
    let port_str = parts[1];

    // Parse IP address
    let ip_parts: heapless::Vec<&str, 4> = ip_str.split('.').collect();
    if ip_parts.len() != 4 {
        log::error!("Invalid IP address: {}", ip_str);
        return false;
    }

    let ip_bytes = [
        ip_parts[0].parse().unwrap_or(0),
        ip_parts[1].parse().unwrap_or(0),
        ip_parts[2].parse().unwrap_or(0),
        ip_parts[3].parse().unwrap_or(0),
    ];

    let port: u16 = port_str.parse().unwrap_or(4000);
    let remote_endpoint = (
        embassy_net::Ipv4Address::new(ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]),
        port,
    );

    log::info!(
        "Connecting to backend at {}.{}.{}.{}:{}",
        ip_bytes[0],
        ip_bytes[1],
        ip_bytes[2],
        ip_bytes[3],
        port
    );

    // Create TCP socket
    let mut rx_buffer = [0; 2048];
    let mut tx_buffer = [0; 2048];
    let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    // Connect to backend
    match socket.connect(remote_endpoint).await {
        Ok(_) => {
            log::info!("TCP connected, sending HTTP POST...");

            // Construct HTTP POST request
            let json_body = "{\"device_id\":\"";
            let json_end = "\"}";
            let content_length = json_body.len() + device_id.len() + json_end.len();

            let mut request = heapless::String::<512>::new();
            use core::fmt::Write;

            write!(request, "POST /api/destroy HTTP/1.1\r\n").ok();
            write!(request, "Host: {}\r\n", ip_str).ok();
            write!(request, "Content-Type: application/json\r\n").ok();
            write!(request, "Content-Length: {}\r\n", content_length).ok();
            write!(request, "Connection: close\r\n").ok();
            write!(request, "\r\n").ok();
            write!(request, "{}{}{}", json_body, device_id, json_end).ok();

            // Send HTTP request
            match socket.write(request.as_bytes()).await {
                Ok(_) => {
                    log::info!("HTTP request sent, waiting for response...");

                    // Read response
                    let mut response_buffer = [0u8; 512];
                    match socket.read(&mut response_buffer).await {
                        Ok(len) => {
                            let response_str =
                                core::str::from_utf8(&response_buffer[..len]).unwrap_or("");
                            log::info!(
                                "HTTP Response: {}",
                                response_str.split('\r').next().unwrap_or("")
                            );

                            // Check for "HTTP/1.1 2xx" status code
                            if response_str.starts_with("HTTP/1.1 2")
                                || response_str.starts_with("HTTP/1.0 2")
                            {
                                log::info!("Destruction event sent successfully!");
                                socket.close();
                                return true;
                            } else {
                                log::error!("HTTP request failed with non-2xx status");
                            }
                        }
                        Err(e) => log::error!("Failed to read HTTP response: {:?}", e),
                    }
                }
                Err(e) => log::error!("Failed to send HTTP request: {:?}", e),
            }
        }
        Err(e) => log::error!("Failed to connect to backend: {:?}", e),
    }

    socket.close();
    false
}

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    log::info!("===========================================");
    log::info!("  BIG RED INTERNET BUTTON");
    log::info!("  Device: {}", DEVICE_ID);
    log::info!("===========================================");

    main_task(spawner).await
}

async fn main_task(spawner: embassy_executor::Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    // Initialize heap allocator for WiFi (98 KB for ESP32-C6)
    esp_alloc::heap_allocator!(size: 98304);

    // Initialize esp-rtos for WiFi radio
    let sw_int = unsafe { esp_hal::interrupt::software::SoftwareInterrupt::<0>::steal() };
    let timer = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timer.timer0, sw_int);

    // Initialize WiFi radio
    static RADIO_CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let radio_init =
        RADIO_CONTROLLER.init(esp_radio::init().expect("Failed to initialize WiFi radio"));

    // Create WiFi controller and device
    let (controller, device_result) = wifi_new(radio_init, peripherals.WIFI, Default::default())
        .expect("Failed to create WiFi controller");
    let device = device_result.sta;

    // Initialize embassy-net stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    static STACK: StaticCell<embassy_net::Stack<'static>> = StaticCell::new();

    let (stack_inner, runner) = embassy_net::new(
        device,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        embassy_time::Instant::now().as_millis() as u64,
    );

    let stack = STACK.init(stack_inner);

    // Spawn WiFi tasks
    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    log::info!("WiFi tasks spawned, waiting for connection...");

    // Setup GPIO for button
    let button = Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Up),
    );

    // Setup RMT for WS2812B LED
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("Failed to initialize RMT");
    let mut rmt_buffer = smart_led_buffer!(1); // Buffer for 1 LED
    let mut led = SmartLedsAdapter::new(rmt.channel0, peripherals.GPIO8, &mut rmt_buffer);

    log::info!("Button: GPIO9 (BOOT button)");
    log::info!("LED: GPIO8");
    log::info!("");

    // Startup sequence (3 quick green flashes)
    for _ in 0..3 {
        led.write([STARTUP_COLOR].iter().cloned()).ok();
        Timer::after(Duration::from_millis(200)).await;
        led.write([RGB8::default()].iter().cloned()).ok();
        Timer::after(Duration::from_millis(200)).await;
    }

    log::info!("BUTTON ARMED - Press to launch!");
    log::info!("");

    let mut launch_count = 0u32;
    let mut last_state = button.is_high();
    let mut idle_time_ms = 0u32; // Track time for breathing

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
                Timer::after(Duration::from_millis(100)).await;

                // Log countdown every second (every 10 iterations)
                if i % 10 == 0 {
                    let remaining = 5 - (i / 10);
                    log::info!("T-{} seconds...", remaining);
                }
            }

            log::warn!("LAUNCH!");
            log::warn!("");

            // Send destruction event to backend
            let success = send_destruction_event(stack, DEVICE_ID).await;

            if success {
                // Solid bright red at launch (success)
                led.write([LAUNCH_COLOR].iter().cloned()).ok();
            } else {
                // Orange flash to indicate HTTP error
                for _ in 0..3 {
                    led.write([WIFI_ERROR_COLOR].iter().cloned()).ok();
                    Timer::after(Duration::from_millis(200)).await;
                    led.write([RGB8::default()].iter().cloned()).ok();
                    Timer::after(Duration::from_millis(200)).await;
                }
                led.write([LAUNCH_COLOR].iter().cloned()).ok();
            }

            Timer::after(Duration::from_millis(2000)).await;

            // Reset idle time for smooth breathing restart
            idle_time_ms = 0;

            // Debounce delay
            Timer::after(Duration::from_millis(500)).await;
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
        Timer::after(Duration::from_millis(50)).await;
    }
}
