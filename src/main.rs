use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::Response, 
    routing::get  
};
use reqwest;


//use serde_json;

struct AppState {
    // Shared HTTP client with connection pooling 
    client: reqwest::Client,

    // Key: Bytes (The raw request body)
    // Value: CacheEntry (The response saved)
    cache: Arc<tokio::sync::RwLock<std::collections::HashMap<Bytes, CacheEntry>>>,
}

#[derive(Clone)]
struct CacheEntry {
    body: Bytes,
    content_type: String,
}

// -- ENTRYPOINT --
/// 
#[tokio::main]
async fn main() {
    // Create HTTP client once at startup
    let client = reqwest::Client::new();
    
    // Wrap AppState in Arc for Axum 
    let state = Arc::new(AppState {
        client: client,
        cache: Arc::new(RwLock::new(HashMap::new())),
    });

    // Build router 
    let app = Router::new()
        .route("/health", get(health))
        .route("/", axum::routing::post(proxy))
        .with_state(state);

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
async fn proxy(
    State(state): State<Arc<AppState>>, 
    body: Bytes
    ) -> Result<Response, StatusCode> {
    
    // READ PATH - Check if the request body exists in cache
    {
        let read_guard = state.cache.read().await;
        if let Some(entry) = read_guard.get(&body) {
            // [HIT] Return cached response
            return Ok (Response::builder()
                .status(StatusCode::OK)
                .header("content-type", &entry.content_type)
                .body(axum::body::Body::from(entry.body.clone()))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
        }
    }

    // NETWORK PATH: Cache Miss -> Forward to Upstream -> Return Response
    let client = state.client.clone();
    let upstream_url = "https://ethereum.publicnode.com"; // TODO: Make configurable via a CLI flag later
    
    let upstream_response = client
        .post(upstream_url)
        .header("content-type", "application/json")
        .body(body.clone())
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

    // WRITE PATH: Save the response in cache for future requests
    {
        let mut write_guard = state.cache.write().await;
        write_guard.insert(
            body, // Key
            CacheEntry { 
                body: response_bytes.clone(), // Value (cloned so we can also return it below)
                content_type: content_type.clone() 
            }
        );
    }

    // Construct response and return to client
    let response = axum::response::Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(axum::body::Body::from(response_bytes))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(response)

}