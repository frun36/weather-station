use heapless::LinearMap;

use super::{
    request::{HttpRequest, KeyValueMap, Method, RequestIndentification},
    response::HttpResponse,
    StatusCode,
};

pub type RequestHandler<const RESPONSE_CAPACITY: usize> = fn(
    parameters: Option<&KeyValueMap>,
    content: Option<&KeyValueMap>,
) -> HttpResponse<RESPONSE_CAPACITY>;

pub struct Router<'a, const RESPONSE_CAPACITY: usize> {
    routes: LinearMap<RequestIndentification<'a>, RequestHandler<RESPONSE_CAPACITY>, 32>,
}

impl<'a, const RESPONSE_CAPACITY: usize> Router<'a, RESPONSE_CAPACITY> {
    pub fn empty() -> Self {
        Self {
            routes: LinearMap::new(),
        }
    }

    pub fn route(
        self,
        path: &'a str,
        method: Method,
        handler: RequestHandler<RESPONSE_CAPACITY>,
    ) -> Result<Self, ()> {
        let mut routes = self.routes;
        routes.insert((path, method), handler).map_err(|_| ())?;
        Ok(Self { routes })
    }

    pub fn handle(&self, http_request: HttpRequest) -> HttpResponse<RESPONSE_CAPACITY> {
        let key = http_request.get_identification();
        let handler = self.routes.get(&key);
        match handler {
            Some(handler) => handler(
                http_request.parameters.as_ref(),
                http_request.payload.as_ref(),
            ),
            None => HttpResponse::empty(StatusCode::NotFound),
        }
    }
}
