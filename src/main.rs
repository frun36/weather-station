#![no_std]
#![no_main]

use core::net::Ipv4Addr;
use cyw43::{Control, JoinOptions};
use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use handlers::{write_temperature, write_time, INDEX};
use heapless::Vec;
use http::{HttpResponse, HttpServer, Method, StatusCode};
use rand_core::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

mod devices;
mod handlers;
mod http;

include!("secrets.rs");

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
async fn http_server(stack: Stack<'static>, control: Control<'static>) {
    let http_server: HttpServer<'_, 4096, 4096> = HttpServer::new(stack, control)
        .route("/", Method::GET, |_, _| {
            HttpResponse::from_slice(StatusCode::Ok, INDEX.as_bytes())
                .unwrap_or(HttpResponse::empty(StatusCode::InternalServerError))
        })
        .route("/rtc", Method::GET, |_, _| {
            let mut response_buffer: Vec<u8, 4096> = Vec::new();
            write_time(&mut response_buffer);
            HttpResponse::new(StatusCode::Ok, response_buffer)
        })
        .route("/data", Method::GET, |_, _| {
            let mut response_buffer: Vec<u8, 4096> = Vec::new();
            write_temperature(&mut response_buffer);
            HttpResponse::new(StatusCode::Ok, response_buffer)
        })
        .route("/rtc", Method::POST, |_, content| {
            let status_code = match handlers::set_time(content) {
                Ok(_) => StatusCode::Ok,
                Err(c) => c,
            };
            HttpResponse::from_slice(status_code, "<a href='/'>back</a>".as_bytes()).unwrap()
        });
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

    // Init readout devices
    devices::rtc::init(p.RTC).await;
    devices::dht::init(p.PIN_27.degrade()).await;

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
}
