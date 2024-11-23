use core::{fmt::Write as _, str::from_utf8};

use cyw43::Control;
use defmt::*;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::Duration;
use embedded_io_async::Write as _;
use heapless::Vec;

use crate::devices;

mod request;

const PORT: u16 = 80;
const INDEX: &str = include_str!("../static/index.html");

pub struct HttpServer<'a, const BUF_SIZE: usize> {
    tx_buffer: [u8; BUF_SIZE],
    rx_buffer: [u8; BUF_SIZE],
    buffer: Vec<u8, BUF_SIZE>,
    stack: Stack<'a>,
    control: Control<'a>,
}

impl<'a, const BUF_SIZE: usize> HttpServer<'a, BUF_SIZE> {
    pub fn new(stack: Stack<'a>, control: Control<'a>) -> Self {
        let rx_buffer = [0; BUF_SIZE];
        let tx_buffer = [0; BUF_SIZE];
        let buffer = Vec::<u8, BUF_SIZE>::new();

        Self {
            tx_buffer,
            rx_buffer,
            buffer,
            stack,
            control,
        }
    }

    pub async fn run(mut self) {
        loop {
            let mut socket = TcpSocket::new(self.stack, &mut self.rx_buffer, &mut self.tx_buffer);
            socket.set_timeout(Some(Duration::from_secs(5)));

            self.control.gpio_set(0, true).await;
            info!("Listening on TCP port {}", PORT);

            if let Err(e) = socket.accept(PORT).await {
                warn!("Accept error: {:?}", e);
                return;
            }

            info!("Received connection from {:?}", socket.remote_endpoint());
            self.control.gpio_set(0, true).await;

            loop {
                unsafe { self.buffer.set_len(BUF_SIZE); }
                let n = match socket.read(self.buffer.as_mut_slice()).await {
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
                self.buffer.truncate(n);

                info!("Received: {}", from_utf8(self.buffer.as_slice()).unwrap());

                self.buffer.clear();

                core::write!(
                    &mut self.buffer,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                )
                .unwrap();

                core::write!(&mut self.buffer, "{}", INDEX).unwrap();

                let now = devices::rtc::now().await;
                if let Some(dt) = now {
                    core::write!(
                        &mut self.buffer,
                        "<p>{}-{}-{} {:02}:{:02}:{:02}</p>",
                        dt.year,
                        dt.month,
                        dt.day,
                        dt.hour,
                        dt.minute,
                        dt.second
                    )
                    .unwrap();
                }

                let reading = devices::dht::read().await;
                if let Some(reading) = reading {
                    core::write!(
                        &mut self.buffer,
                        "<h3>T: {} Rh: {}</h3>",
                        reading.get_temp(),
                        reading.get_hum()
                    )
                    .unwrap();
                }

                match socket.write_all(self.buffer.as_slice()).await {
                    Ok(()) => {}
                    Err(e) => {
                        warn!("Write error: {:?}", e);
                        break;
                    }
                };
            }
        }
    }
}
