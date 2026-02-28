//! Axum HTTP routes for the embedded server.
//! M2 implementation: serve WASM bundle + graph JSON API.
//!
//! Listens on 127.0.0.1 only (not 0.0.0.0) for security.

use axum::{routing::get, Router};

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health))
        // M2: add /api/graph, /api/nodes/:id, static file serving
}

async fn health() -> &'static str {
    "ok"
}
