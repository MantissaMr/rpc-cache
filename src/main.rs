use std::net::SocketAddr;
use tokio::net::TcpListener;
use axum::{
    routing::get, 
    Router,
    http::StatusCode,  
};


#[tokio::main]
async fn main() {

    // Build router 
    let app = Router::new().route("/health", get(health));

    // Bind to localhost:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on {}", addr);

    // Create a TCP listener 
    let listener = TcpListener::bind(addr)
        .await
        .unwrap();

    // Start the server
    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn health() -> StatusCode {
    StatusCode::OK
}