use core::fmt;

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

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

        f.write_str(message)
    }
}

pub enum ContentType {
    TextHtml,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let content_type_str = match self {
            ContentType::TextHtml => "text/html",
        };
        f.write_str(content_type_str)
    }
}

pub struct HttpResponseHeader {
    status_code: StatusCode,
    content_type: ContentType,
    content_length: usize,
}

impl HttpResponseHeader {
    pub fn new(status_code: StatusCode, content_length: usize) -> Self {
        Self {
            status_code,
            content_type: ContentType::TextHtml,
            content_length,
        }
    }
}

pub struct HttpResponse<'a> {
    pub header: HttpResponseHeader,
    pub content: &'a str,
}

impl<'a> HttpResponse<'a> {
    pub fn new(status_code: StatusCode, content: &'a str) -> Self {
        Self {
            header: HttpResponseHeader::new(status_code, content.len()),
            content,
        }
    }
}

impl fmt::Display for HttpResponseHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        core::write!(
            f,
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
            self.status_code as u16,
            self.status_code,
            self.content_type,
            self.content_length
        )?;

        Ok(())
    }
}
