use core::str::FromStr;

use defmt::Format;
use heapless::{LinearMap, String};

use super::response::StatusCode;

#[derive(Format, Clone, Copy)]
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

impl core::convert::TryFrom<&str> for Method {
    type Error = StatusCode;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "TRACE" => Ok(Method::TRACE),
            _ => Err(StatusCode::BadRequest),
        }
    }
}

type KeyValueMap = LinearMap<String<16>, String<16>, 8>;

// For now ignores header and payload
pub struct HttpRequest {
    method: Method,
    path: String<32>,
    parameters: Option<KeyValueMap>,
    payload: Option<KeyValueMap>,
}

impl HttpRequest {
    pub fn parse(request_str: &str) -> Result<Self, StatusCode> {
        let (header_str, payload_str) = request_str
            .split_once("\r\n\r\n")
            .ok_or(StatusCode::BadRequest)?;

        let (method, path, parameters) = Self::parse_header(header_str)?;
        let payload = if payload_str.is_empty() {
            None
        } else {
            Some(Self::parse_key_value(payload_str)?)
        };

        Ok(Self {
            method,
            path,
            parameters,
            payload,
        })
    }

    fn parse_header(
        header_str: &str,
    ) -> Result<(Method, String<32>, Option<KeyValueMap>), StatusCode> {
        let (start_line, _) = header_str
            .split_once("\r\n")
            .ok_or(StatusCode::BadRequest)?;
        let (method, remaining) = start_line.split_once(" ").ok_or(StatusCode::BadRequest)?;
        let (path, _) = remaining.split_once(" ").ok_or(StatusCode::BadRequest)?;

        let (path, parameters) = match path.split_once("?") {
            Some((tup_path, tup_parameters)) => (tup_path, Some(tup_parameters)),
            None => (path, None),
        };

        Ok((
            Method::try_from(method)?,
            String::from_str(path).map_err(|_| StatusCode::UriTooLong)?,
            match parameters {
                Some(parameters) => Some(Self::parse_key_value(parameters)?),
                None => None,
            },
        ))
    }

    fn parse_key_value(text: &str) -> Result<KeyValueMap, StatusCode> {
        let mut map: KeyValueMap = LinearMap::new();

        for pair in text.split("&") {
            let (key, value) = pair
                .split_once("=")
                .ok_or(StatusCode::UnprocessableContent)?;
            map.insert(
                String::from_str(key).map_err(|_| StatusCode::UriTooLong)?,
                String::from_str(value).map_err(|_| StatusCode::UriTooLong)?,
            )
            .map_err(|_| StatusCode::UriTooLong)?;
        }

        Ok(map)
    }

    pub fn get_method(&self) -> Method {
        self.method
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }
}

impl Format for HttpRequest {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Method: {}\nPath: {}\n", self.method, self.path);

        if let Some(parameter_map) = self.parameters.as_ref() {
            for (key, value) in parameter_map.iter() {
                defmt::write!(f, "{} = {}\n", key, value)
            }
        }

        defmt::write!(f, "Payload:\n");

        if let Some(payload_map) = self.payload.as_ref() {
            for (key, value) in payload_map.iter() {
                defmt::write!(f, "{} = {}\n", key, value)
            }
        }
    }
}
