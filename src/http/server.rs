use core::{fmt::Write as _, str};

use super::request::{HttpRequest, Method};
use super::response::{HttpResponse, StatusCode};
use cyw43::Control;
use defmt::*;
use embassy_net::{
    tcp::{Error, TcpSocket},
    Stack,
};
use embassy_time::Duration;
use embedded_io_async::Write as _;
use heapless::Vec;

use crate::devices;

const PORT: u16 = 80;
const INDEX: &str = include_str!("../../static/index.html");

pub struct HttpServer<'a, const BUF_SIZE: usize> {
    rx_buffer: [u8; BUF_SIZE],
    tx_buffer: [u8; BUF_SIZE],
    buffer: Vec<u8, BUF_SIZE>,
    stack: Stack<'a>,
    control: Control<'a>,
}

impl<'a, 'b, const BUF_SIZE: usize> HttpServer<'a, BUF_SIZE> {
    pub fn new(stack: Stack<'a>, control: Control<'a>) -> Self {
        let rx_buffer = [0; BUF_SIZE];
        let tx_buffer = [0; BUF_SIZE];
        let buffer = Vec::<u8, BUF_SIZE>::new();

        Self {
            rx_buffer,
            tx_buffer,
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
        socket.write_all(response.content).await
    }

    async fn write_time(buffer: &mut Vec<u8, BUF_SIZE>) {
        let now = devices::rtc::now().await;
        if let Some(dt) = now {
            core::write!(
                buffer,
                "{}-{}-{} {:02}:{:02}:{:02}",
                dt.year,
                dt.month,
                dt.day,
                dt.hour,
                dt.minute,
                dt.second
            )
            .unwrap();
        }
    }

    async fn write_temperature(buffer: &mut Vec<u8, BUF_SIZE>) {
        let reading = devices::dht::read().await;
        if let Some(reading) = reading {
            core::write!(
                buffer,
                "T: {} Rh: {}",
                reading.get_temp(),
                reading.get_hum()
            )
            .unwrap();
        }
    }

    pub async fn run(mut self) {
        loop {
            let mut socket = Self::init_connection(
                self.stack,
                &mut self.control,
                &mut self.rx_buffer,
                &mut self.tx_buffer,
            )
            .await
            .unwrap();

            loop {
                let response = match Self::get_request(&mut socket, &mut self.buffer).await {
                    Ok(http_request) => {
                        info!("HttpRequest: {}", http_request);

                        match http_request.path.as_str() {
                            "/" => HttpResponse::new(StatusCode::Ok, INDEX.as_bytes()),
                            "/rtc" => match http_request.method {
                                Method::GET => {
                                    Self::write_time(&mut self.buffer).await;
                                    HttpResponse::new(StatusCode::Ok, self.buffer.as_slice())
                                }
                                Method::POST => HttpResponse::empty(StatusCode::NotImplemented),
                                _ => HttpResponse::empty(StatusCode::MethodNotAllowed),
                            },
                            "/data" => match http_request.method {
                                Method::GET => {
                                    Self::write_temperature(&mut self.buffer).await;
                                    HttpResponse::new(StatusCode::Ok, self.buffer.as_slice())
                                }
                                _ => HttpResponse::empty(StatusCode::MethodNotAllowed),
                            },
                            _ => HttpResponse::empty(StatusCode::NotFound),
                        }
                    }
                    Err(e) => HttpResponse::empty(e),
                };

                if Self::send_response(&mut socket, response).await.is_err() {
                    break;
                }
            }
        }
    }
}
