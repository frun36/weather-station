use core::{fmt::Write as _, str};

use super::request::{HttpRequest, Method};
use super::response::{HttpResponse, StatusCode};
use super::router::{RequestHandler, Router};
use cyw43::Control;
use defmt::*;
use embassy_net::{
    tcp::{Error, TcpSocket},
    Stack,
};
use embassy_time::Duration;
use embedded_io_async::Write as _;
use heapless::Vec;

const PORT: u16 = 80;

pub struct HttpServer<'a, const BUF_SIZE: usize, const RESPONSE_CAPACITY: usize> {
    rx_buffer: [u8; BUF_SIZE],
    tx_buffer: [u8; BUF_SIZE],
    buffer: Vec<u8, BUF_SIZE>,
    stack: Stack<'a>,
    control: Control<'a>,
    router: Router<'a, RESPONSE_CAPACITY>,
}

impl<'a, 'b, const BUF_SIZE: usize, const RESPONSE_CAPACITY: usize>
    HttpServer<'a, BUF_SIZE, RESPONSE_CAPACITY>
{
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
            router: Router::empty(),
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
        response: HttpResponse<RESPONSE_CAPACITY>,
    ) -> Result<(), Error> {
        let mut header_buffer: Vec<u8, 128> = Vec::new();
        core::write!(header_buffer, "{}", response.header).unwrap();

        socket.write_all(header_buffer.as_slice()).await?;
        socket.write_all(&response.content).await
    }

    pub fn route(
        mut self,
        path: &'a str,
        method: Method,
        handler: RequestHandler<RESPONSE_CAPACITY>,
    ) -> Self {
        self.router = self
            .router
            .route(path, method, handler)
            .expect("Couldn't insert route handler - router full");
        self
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
                let request = Self::get_request(&mut socket, &mut self.buffer).await;

                let response = match request {
                    Ok(http_request) => self.router.handle(http_request),
                    Err(e) => HttpResponse::empty(e),
                };

                if Self::send_response(&mut socket, response).await.is_err() {
                    break;
                }
            }
        }
    }
}
