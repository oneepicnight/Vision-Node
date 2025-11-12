// Simple static file server for production dist folder
// Serves files from ./dist on port 4173

use axum::Router;
use axum::routing::get_service;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // Create a service to serve static files from the dist directory
    let serve_dir = ServeDir::new("dist");
    
    let app = Router::new()
        .fallback_service(get_service(serve_dir));

    let addr = SocketAddr::from(([127, 0, 0, 1], 4173));
    println!("========================================");
    println!("  Vision Wallet - Frontend Server");
    println!("========================================");
    println!();
    println!("Frontend: http://{}", addr);
    println!("Serving:  ./dist folder");
    println!();
    println!("Press Ctrl+C to stop");
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
