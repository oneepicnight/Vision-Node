use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(|_req: Request<Body>| async move {
            Ok::<_, Infallible>(Response::new(Body::from("OK")))
        }))
    });

    let server = Server::bind(&addr).serve(make_svc);

    println!("simple server listening on http://{}", addr);

    if let Err(err) = server.await {
        eprintln!("server error: {}", err);
    }
}
