#[repr(u16)]
#[derive(Debug)]
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