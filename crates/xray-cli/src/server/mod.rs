pub mod routes;

use routes::{FunctionsState, GraphState};
use std::{path::PathBuf, sync::Arc};

/// A TCP listener bound to a local port, ready to serve.
pub struct BoundServer {
    pub port: u16,
    listener: tokio::net::TcpListener,
}

impl BoundServer {
    /// Start serving the graph JSON, functions data, and static web assets.
    /// Runs until the connection is dropped or the process exits.
    pub async fn serve(
        self,
        graph_json: GraphState,
        functions: FunctionsState,
        web_dist: Option<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let router = routes::build_router(graph_json, functions, web_dist);
        tracing::info!("Server listening on http://127.0.0.1:{}", self.port);
        axum::serve(self.listener, router).await?;
        Ok(())
    }
}

/// Bind to `127.0.0.1:<port>`, retrying up to +10 if the port is already in use.
///
/// Returns the successfully bound server so the caller can discover the actual port
/// before opening the browser.
pub async fn bind(
    port: u16,
) -> Result<BoundServer, Box<dyn std::error::Error + Send + Sync>> {
    for p in port..=port.saturating_add(10) {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{p}")).await {
            Ok(listener) => {
                return Ok(BoundServer { port: p, listener });
            }
            Err(_) if p < port.saturating_add(10) => {
                eprintln!("xray: port {p} in use, trying {}…", p + 1);
            }
            Err(e) => {
                return Err(Box::new(e) as _);
            }
        }
    }
    Err("Could not bind to any port in range".into())
}

/// Convenience: look for a pre-built web bundle in well-known locations
/// relative to the current working directory.
///
/// Returns `Some(path)` if found, `None` if the bundle hasn't been built yet.
pub fn find_web_dist() -> Option<PathBuf> {
    // Check web/dist relative to CWD (the workspace root when running via `cargo run`).
    let p = PathBuf::from("web/dist");
    if p.join("index.html").exists() {
        return Some(p);
    }
    // One level up (e.g., running from inside a crate directory).
    let p = PathBuf::from("../web/dist");
    if p.join("index.html").exists() {
        return Some(p);
    }
    None
}

/// Convenience wrapper: bind + serve in one call.
/// Exposed for tests and simple callers that don't need to split the two steps.
#[allow(dead_code)]
pub async fn start_server(
    port: u16,
    graph_json: Arc<String>,
    functions: FunctionsState,
    web_dist: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = bind(port).await?;
    server.serve(graph_json, functions, web_dist).await
}
