use std::net::SocketAddr;
use tokio::net::TcpListener;
use axum::{
    body::Bytes,
    http::StatusCode,
    response::Response,
    routing::get, 
    Router  
};
use reqwest;
//use serde_json;

// -- ENTRYPOINT --
/// 
#[tokio::main]
async fn main() {

    // Build router 
    let app = Router::new()
        .route("/health", get(health))
        .route("/", axum::routing::post(proxy));

    // Bind to localhost:3000
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server listening on {}", addr);

    // Create a TCP listener 
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    // Start the server
    axum::serve(listener, app)
        .await
        .expect("Server exited unexpectedly");
}

// -- HANDLERS --

// `/health` (GET): Returns a simple health check response
async fn health() -> StatusCode {
    StatusCode::OK
}

// `/` (POST): Proxies the incoming request to an external API and returns the response
async fn proxy(body: Bytes) -> Result<Response, StatusCode> {
    
    let client = reqwest::Client::new();
    let upstream_url = "https://ethereum.publicnode.com"; // TODO: Make configurable via a CLI flag later

    // TODO(refactor): Create one Client at startup, wrap it in Arc, inject it into the Axum router state
    // Every request handler clones the Arc and shares the same connection pool

    
    let upstream_response = client
        .post(upstream_url)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Extract Metadata (read-only)
    let status = upstream_response.status();
    let content_type = upstream_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    
    // Extract Body (consume the response)
    let response_bytes = upstream_response
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Construct response and return to client
    let response = axum::response::Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(axum::body::Body::from(response_bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)

}