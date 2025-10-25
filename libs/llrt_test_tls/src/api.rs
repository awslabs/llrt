use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};

pub(crate) async fn echo(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut response = Response::new(Full::default());
    match (req.method(), req.uri().path()) {
        // Help route.
        (&Method::GET, "/") => {
            *response.body_mut() = Full::from("Try POST /echo\n");
        },
        // Echo service route.
        (&Method::POST, "/echo") => {
            *response.body_mut() = Full::from(req.into_body().collect().await?.to_bytes());
        },
        // Catch-all 404.
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        },
    };
    Ok(response)
}
