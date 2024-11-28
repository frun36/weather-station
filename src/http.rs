use core::{fmt::Write as _, str};

use cyw43::Control;
use defmt::*;
use embassy_net::{
    tcp::{Error, TcpSocket},
    Stack,
};
use embassy_time::Duration;
use embedded_io_async::Write as _;
use heapless::Vec;
use request::HttpRequest;
use response::{HttpResponse, StatusCode};

use crate::devices;

mod request;
mod response;

const PORT: u16 = 80;
const INDEX: &str = include_str!("../static/index.html");

pub struct HttpServer<'a, const BUF_SIZE: usize> {
    buffer: Vec<u8, BUF_SIZE>,
    stack: Stack<'a>,
    control: Control<'a>,
}

impl<'a, 'b, const BUF_SIZE: usize> HttpServer<'a, BUF_SIZE> {
    pub fn new(stack: Stack<'a>, control: Control<'a>) -> Self {
        let buffer = Vec::<u8, BUF_SIZE>::new();

        Self {
            buffer,
            stack,
            control,
        }
    }

    async fn init_connection(
        stack: Stack<'b>,
        control: &mut Control<'_>,
        rx_buffer: &'b mut [u8],
        tx_buffer: &'b mut [u8],
    ) -> Option<TcpSocket<'b>> {
        let mut socket = TcpSocket::new(stack, rx_buffer, tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(5)));

        info!("Listening on TCP port {}", PORT);
        control.gpio_set(0, true).await;

        if let Err(e) = socket.accept(PORT).await {
            warn!("Accept error: {:?}", e);
            return None;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());

        Some(socket)
    }

    async fn get_request(
        socket: &mut TcpSocket<'_>,
        buffer: &mut Vec<u8, BUF_SIZE>,
    ) -> Result<HttpRequest, StatusCode> {
        unsafe {
            buffer.set_len(BUF_SIZE);
        }
        let n = match socket.read(buffer.as_mut_slice()).await {
            Ok(0) => {
                warn!("Read EOF");
                return Err(StatusCode::BadRequest);
            }
            Ok(n) => n,
            Err(e) => {
                warn!("Read error: {:?}", e);
                return Err(StatusCode::InternalServerError);
            }
        };
        buffer.truncate(n);

        let request_str = str::from_utf8(buffer.as_slice()).unwrap();
        debug!("Received:\n{}", request_str);
        let http_request = HttpRequest::parse(request_str).unwrap();

        buffer.clear();

        Ok(http_request)
    }

    async fn send_response(
        socket: &mut TcpSocket<'_>,
        response: HttpResponse<'_>,
    ) -> Result<(), Error> {
        let mut header_buffer: Vec<u8, 128> = Vec::new();
        core::write!(header_buffer, "{}", response.header).unwrap();

        socket.write_all(header_buffer.as_slice()).await?;
        socket.write_all(response.content.as_bytes()).await
    }

    pub async fn run(mut self) {
        let mut rx_buffer = [0; BUF_SIZE];
        let mut tx_buffer = [0; BUF_SIZE];
        loop {
            let mut socket = Self::init_connection(
                self.stack,
                &mut self.control,
                &mut rx_buffer,
                &mut tx_buffer,
            )
            .await
            .unwrap();

            loop {
                let response = match Self::get_request(&mut socket, &mut self.buffer).await {
                    Ok(http_request) => {
                        info!("HttpRequest: {}", http_request);

                        match http_request.path.as_str() {
                            "/" => HttpResponse::new(StatusCode::Ok, INDEX),
                            "/rtc" => HttpResponse::new(StatusCode::Ok, "time"),
                            "/data" => HttpResponse::new(StatusCode::Ok, "t, rh"),
                            _ => HttpResponse::new(StatusCode::NotFound, ""),
                        }
                    }
                    Err(e) => HttpResponse::new(e, ""),
                };

                // let now = devices::rtc::now().await;
                // if let Some(dt) = now {
                //     core::write!(
                //         &mut self.buffer,
                //         "<p>{}-{}-{} {:02}:{:02}:{:02}</p>",
                //         dt.year,
                //         dt.month,
                //         dt.day,
                //         dt.hour,
                //         dt.minute,
                //         dt.second
                //     )
                //     .unwrap();
                // }

                // let reading = devices::dht::read().await;
                // if let Some(reading) = reading {
                //     core::write!(
                //         &mut self.buffer,
                //         "<h3>T: {} Rh: {}</h3>",
                //         reading.get_temp(),
                //         reading.get_hum()
                //     )
                //     .unwrap();
                // }

                if Self::send_response(&mut socket, response)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}
