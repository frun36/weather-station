#![no_std]
#![no_main]

use core::net::Ipv4Addr;
use core::sync::atomic::{AtomicI8, AtomicU8, Ordering};
use cyw43::{Control, JoinOptions};
use cyw43_pio::PioSpi;
use defmt::*;
use embassy_dht::dht11::DHT11;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_27, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Delay, Timer};
use http::HttpServer;
use rand_core::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod http;

include!("secrets.rs");

static TEMPERATURE: AtomicI8 = AtomicI8::new(0);
static HUMIDITY: AtomicU8 = AtomicU8::new(0);

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn dht11_task(pin: PIN_27) {
    let mut dht = DHT11::new(pin, Delay);

    loop {
        if let Ok(reading) = dht.read() {
            TEMPERATURE.store(reading.get_temp(), Ordering::Relaxed);
            HUMIDITY.store(reading.get_hum(), Ordering::Relaxed);
            Timer::after_secs(3600).await;
        }
        Timer::after_millis(100).await;
    }
}

#[embassy_executor::task]
async fn http_server(stack: Stack<'static>, control: Control<'static>) {
    let http_server: HttpServer<'_, 4096> = HttpServer::new(stack, control);
    http_server.run().await;
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    // Init cyw43
    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    spawner.spawn(cyw43_task(runner)).unwrap();

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    // Init network stack
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: PICO_IP,
        dns_servers: heapless::Vec::new(),
        gateway: Some(GATEWAY_IP),
    });
    let seed = rng.next_u64();
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(net_task(runner)).unwrap();

    // Connect to network
    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => {
                info!("Joined network {}", WIFI_NETWORK);
                break;
            }
            Err(err) => {
                info!(
                    "Joining {} failed with status = {}",
                    WIFI_NETWORK, err.status
                );
            }
        }
    }

    spawner.spawn(http_server(stack, control)).unwrap();
    spawner.spawn(dht11_task(p.PIN_27)).unwrap();
}
