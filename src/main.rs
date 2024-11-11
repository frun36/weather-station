#![no_std]
#![no_main]

use core::net::Ipv4Addr;
use core::str::from_utf8;

use core::fmt::Write as CoreWrite;
use cyw43::{Control, JoinOptions};
use cyw43_pio::PioSpi;
use defmt::*;
use embassy_dht::dht11::DHT11;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Ipv4Cidr, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Delay, Duration};
use embedded_io_async::Write;
use rand_core::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

include!("secrets.rs");

const PORT: u16 = 80;

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
async fn tcp_server(
    stack: Stack<'static>,
    mut control: Control<'static>,
    mut dht: DHT11<'static, Delay>,
) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut req = [0; 4096];
    let mut res = heapless::String::<4096>::new();

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, true).await;
        info!("Listening on TCP port {}", PORT);
        if let Err(e) = socket.accept(PORT).await {
            warn!("Accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());
        control.gpio_set(0, true).await;

        loop {
            let _n = match socket.read(&mut req).await {
                Ok(0) => {
                    warn!("Read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("Read error: {:?}", e);
                    break;
                }
            };

            info!("Received: {}", from_utf8(&req).unwrap());

            res.clear();
            match dht.read() {
                Ok(r) => {
                    core::write!(&mut res, "HTTP/1.1 200 OK\nContent-Type: text/html\n\n<h3>T: {} Rh: {}</h3>", r.get_temp(), r.get_hum()).unwrap()
                }
                Err(e) => core::write!(&mut res, "HTTP/1/1 500 Internal Server Error\nContent-Type: text/plain\n\nDHT11 error: {}\n", e).unwrap(),
            }

            match socket.write_all(res.as_bytes()).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("Write error: {:?}", e);
                    break;
                }
            };
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello World!");

    let p = embassy_rp::init(Default::default());
    let mut rng = RoscRng;
    let dht = DHT11::new(p.PIN_27, Delay);

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

    spawner.spawn(tcp_server(stack, control, dht)).unwrap();
}
