#![no_std]
#![no_main]

use bosch_bme680::{AsyncBme680, Configuration, DeviceAddress};
use cyw43::{JoinOptions, State, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::{Config, StackResources};
use embassy_rp::{
    bind_interrupts,
    clocks::RoscRng,
    dma,
    gpio::{Level, Output},
    i2c::{self, I2c},
    peripherals::{DMA_CH0, I2C1, PIO0},
    pio::{InterruptHandler, Pio},
};
use embassy_time::{Delay, Duration, Timer};
use panic_probe as _;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
    I2C1_IRQ => i2c::InterruptHandler<I2C1>;
});

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let fw = aligned_bytes!("../firmware/43439A0.bin");
    let clm = aligned_bytes!("../firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../firmware/nvram_rp2040.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let dma = dma::Channel::new(p.DMA_CH0, Irqs);
    let spi = PioSpi::new(&mut pio.common, pio.sm0, DEFAULT_CLOCK_DIVIDER, pio.irq0, cs, p.PIN_24, p.PIN_29, dma);

    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init(State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(unwrap!(cyw43_task(runner)));

    info!("Initializing I2C for BME680 sensor...");
    let sda = p.PIN_6;
    let scl = p.PIN_7;
    let i2c = I2c::new_async(p.I2C1, scl, sda, Irqs, i2c::Config::default());

    let mut bme = AsyncBme680::new(i2c, DeviceAddress::Primary, Delay, 20);
    let bme_config = Configuration::default();

    if let Err(e) = bme.initialize(&bme_config).await {
        error!("Failed to initialize BME680 sensor: {:?}", Debug2Format(&e));
    } else {
        info!("BME680 sensor initialized successfully.");
    }

    control.init(clm).await;
    control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;

    let config = Config::dhcpv4(Default::default());
    let seed = RoscRng.next_u64();

    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);
    spawner.spawn(unwrap!(net_task(runner)));

    loop {
        info!("Joining Wi-Fi network...");
        match control.join(WIFI_SSID, JoinOptions::new(WIFI_PASSWORD.as_bytes())).await {
            Ok(_) => {
                info!("Successfully joined Wi-Fi network!");
                break;
            }
            Err(err) => {
                error!("Failed to join Wi-Fi network: {:?}", err);
                info!("Retrying in 2 seconds...");
                Timer::after(Duration::from_secs(2)).await;
            }
        }
    }

    info!("Waiting for an IP address...");
    stack.wait_config_up().await;

    if let Some(ipv4_addr) = stack.config_v4() {
        info!("IP address acquired: {}", ipv4_addr.address);
    } else {
        error!("Failed to acquire an IP address.");
    }

    let mut led_state = false;
    loop {
        led_state = !led_state;
        control.gpio_set(0, led_state).await;

        match bme.measure().await {
            Ok(data) => {
                info!("Temperature: {} °C, Pressure: {} hPa, Humidity: {} %, Gas Resistance: {} Ω", data.temperature, data.pressure, data.humidity, data.gas_resistance);
            }
            Err(e) => {
                error!("Failed to read from BME680 sensor: {:?}", Debug2Format(&e));
            }
        }

        Timer::after(Duration::from_secs(2)).await;
    }
}
