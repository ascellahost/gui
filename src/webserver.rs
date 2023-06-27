use std::{convert::Infallible, net::SocketAddr};

use crate::{utils::ascella_notif, RequestResponse};
use anyhow::Result;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};

use tokio::sync::mpsc::UnboundedSender;

fn create_res(body: Body) -> Response<Body> {
    let mut res = Response::new(body);
    *res.status_mut() = hyper::StatusCode::OK;

    res.headers_mut().insert(
        "Access-Control-Allow-Origin",
        hyper::header::HeaderValue::from_static("https://ascella.host"),
    );
    res.headers_mut().insert(
        "Access-Control-Allow-Methods",
        hyper::header::HeaderValue::from_static("GET, POST, OPTIONS"),
    );

    res
}

async fn handle_req(req: Request<Body>, sender: UnboundedSender<RequestResponse>) -> crate::Result<Response<Body>> {
    let uri = req.uri().clone();
    let res = match (uri.path(), req.method()) {
        ("/", &hyper::Method::POST) => {
            let body = hyper::body::to_bytes(req.into_body()).await?;
            sender
                .send(RequestResponse::UpdateConfigFromStringSxcu(body.to_vec()))
                .ok();
            ascella_notif().body("Config Imported successfully").show()?;
            return Ok(create_res(Body::empty()));
        }

        _ => Ok(create_res(Body::empty())),
    };
    res
}

pub async fn start_server(sender: UnboundedSender<RequestResponse>) -> Result<()> {
    // Make a server from the previous examples...
    let addr = SocketAddr::from(([127, 0, 0, 1], 3234));
    // Prepare some signal for when the server should start shutting down...
    let make_service = make_service_fn(move |_client: &AddrStream| {
        // let ip = client.remote_addr();
        let sender = sender.clone();
        async move {
            // This is the request handler.
            Ok::<_, Infallible>(service_fn(move |req| {
                let sender = sender.clone();
                async {
                    let res = handle_req(req, sender).await;
                    if res.is_err() {
                        let mut response = create_res(Body::empty());
                        *response.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
                        Ok(response)
                    } else {
                        res
                    }
                }
            }))
        }
    });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    let _ = server.await;
    Ok(())
}
