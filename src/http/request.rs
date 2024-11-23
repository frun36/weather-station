#[derive(Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
}

pub struct HttpRequest {

}