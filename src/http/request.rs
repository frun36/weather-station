use core::str::FromStr;

use defmt::Format;
use heapless::{LinearMap, String};

#[derive(Format)]
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
    type Error = ();

    fn try_from(s: &str) -> Result<Self, ()> {
        match s {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "TRACE" => Ok(Method::TRACE),
            _ => Err(()),
        }
    }
}

type ParameterMap = LinearMap<String<16>, String<16>, 8>;

// For now ignores header and payload
pub struct HttpRequest {
    method: Method,
    path: String<32>,
    parameter_map: Option<ParameterMap>,
}

impl HttpRequest {
    // For now no error handling
    pub fn parse(request_str: &str) -> Result<Self, ()> {
        let (start_line, _) = request_str.split_once("\r\n").ok_or(())?;
        let (method, remaining) = start_line.split_once(" ").ok_or(())?;
        let (path, _) = remaining.split_once(" ").ok_or(())?;

        let (path, parameters) = match path.split_once("?") {
            Some((tup_path, tup_parameters)) => (tup_path, Some(tup_parameters)),
            None => (path, None),
        };

        let method = Method::try_from(method)?;
        let path = String::from_str(path).map_err(|_| {})?;
        let parameter_map = match parameters {
            Some(parameters) => Some(Self::parse_parameters(parameters)?),
            None => None,
        };

        Ok(Self {
            method,
            path,
            parameter_map,
        })
    }

    fn parse_parameters(text: &str) -> Result<ParameterMap, ()> {
        let mut map: ParameterMap = LinearMap::new();

        for pair in text.split("&") {
            let (key, value) = pair.split_once("=").ok_or(())?;
            map.insert(
                String::from_str(key).map_err(|_| {})?,
                String::from_str(value).map_err(|_| {})?,
            )
            .map_err(|_| {})?;
        }

        Ok(map)
    }
}

impl Format for HttpRequest {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Method: {}\nPath: {}\n", self.method, self.path);

        if let Some(parameter_map) = self.parameter_map.as_ref() {
            for (key, value) in parameter_map.iter() {
                defmt::write!(f, "{} = {}\n", key, value)
            }
        }
    }
}
