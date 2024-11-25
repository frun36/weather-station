use core::fmt::Display;

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum StatusCode {
    Ok = 200,
    BadRequest = 400,
    NotFound = 404,
    MethodNotAllowed = 405,
    UriTooLong = 414,
    UnprocessableContent = 422,
    InternalServerError = 500,
    NotImplemented = 501,
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            StatusCode::Ok => "OK",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::UriTooLong => "URI Too Long",
            StatusCode::UnprocessableContent => "Unprocessable Content",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
        };

        core::write!(f, "{}", message)?;
        Ok(())
    }
}

pub struct HttpResponse<'a> {
    status_code: StatusCode,
    payload: &'a str,
}

impl<'a> HttpResponse<'a> {
    pub fn new(status_code: StatusCode, payload: &'a str) -> Self {
        Self {
            status_code,
            payload,
        }
    }
}

impl<'a> Display for HttpResponse<'a> {    
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::write!(
            f,
            "HTTP/1.1 {} {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            self.status_code as u16,
            self.status_code,
            self.payload.len(),
            self.payload
        )?;

        Ok(())
    }
}
