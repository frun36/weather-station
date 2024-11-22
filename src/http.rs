use core::{fmt::Write as _, str::from_utf8, sync::atomic::Ordering};

use cyw43::Control;
use defmt::*;
use embassy_net::{tcp::TcpSocket, Stack};
use embassy_time::Duration;
use embedded_io_async::Write as _;
use heapless::String;

use crate::{HUMIDITY, RTC, TEMPERATURE};

const PORT: u16 = 80;

pub struct HttpServer<'a, const BUF_SIZE: usize> {
    tx_buffer: [u8; BUF_SIZE],
    rx_buffer: [u8; BUF_SIZE],
    request: [u8; BUF_SIZE],
    response: String<BUF_SIZE>,
    stack: Stack<'a>,
    control: Control<'a>,
}

impl<'a, const BUF_SIZE: usize> HttpServer<'a, BUF_SIZE> {
    pub fn new(stack: Stack<'a>, control: Control<'a>) -> Self {
        let rx_buffer = [0; BUF_SIZE];
        let tx_buffer = [0; BUF_SIZE];
        let request = [0; BUF_SIZE];
        let response = heapless::String::<BUF_SIZE>::new();

        Self {
            tx_buffer,
            rx_buffer,
            request,
            response,
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
                let n = match socket.read(&mut self.request).await {
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

                info!("Received: {}", from_utf8(&self.request[..n]).unwrap());

                self.response.clear();

                core::write!(
                    &mut self.response,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n",
                )
                .unwrap();

                let rtc = RTC.lock().await;

                if let Ok(dt) = rtc.as_ref().unwrap().now() {
                    core::write!(
                        &mut self.response,
                        "<p>{}-{}-{} {:02}:{:02}</p>",
                        dt.year, dt.month, dt.day, dt.hour, dt.minute
                    ).unwrap();
                }

                core::write!(
                    &mut self.response,
                    "<h3>T: {} Rh: {}</h3>",
                    TEMPERATURE.load(Ordering::Relaxed),
                    HUMIDITY.load(Ordering::Relaxed)
                )
                .unwrap();

                match socket.write_all(self.response.as_bytes()).await {
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
