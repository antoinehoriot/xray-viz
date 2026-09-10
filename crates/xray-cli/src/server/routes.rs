//! Axum HTTP routes for the embedded server.
//!
//! Routes:
//!   GET /health                        — health check
//!   GET /api/graph                     — full XrayGraph JSON
//!   GET /api/functions?file=<path>     — lazy function-level subgraph for a file
//!   GET /api/events                    — SSE stream for hot-reload (--watch mode)
//!   GET /api/annotations               — read node annotations (Pro)
//!   PUT /api/annotations               — write node annotations (Pro)
//!   GET /*                             — static file serving from web/dist (if present),
//!                                        otherwise a minimal inline fallback UI
//!
//! Listens on 127.0.0.1 only (not 0.0.0.0) for security.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use std::{
    collections::HashMap,
    convert::Infallible,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;
use tower_http::services::ServeDir;
use xray_core::annotations::{self, Annotations};

use crate::license;

/// Combined application state shared across all routes.
#[derive(Clone)]
pub struct AppState {
    pub graph_json: Arc<RwLock<String>>,
    pub functions: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub update_tx: broadcast::Sender<()>,
    /// Canonical root directory of the scanned repository.
    /// Used by annotation endpoints to locate `.xray/annotations.yml`.
    pub root: Arc<PathBuf>,
}

/// Build the axum router.
///
/// - `state`: combined application state (graph JSON, functions, SSE sender, root)
/// - `web_dist`: optional path to web/dist for full UI assets;
///   if None, serves a minimal fallback HTML
pub fn build_router(state: AppState, web_dist: Option<PathBuf>) -> Router {
    // State-dependent routes
    let base = Router::new()
        .route("/health", get(health))
        .route("/api/graph", get(get_graph))
        .route("/api/functions", get(get_functions))
        .route("/api/events", get(events))
        .route(
            "/api/annotations",
            get(get_annotations).put(put_annotations),
        )
        .with_state(state);

    // Static file serving (no state needed).
    match web_dist {
        Some(dir) => {
            base.fallback_service(ServeDir::new(dir).append_index_html_on_directories(true))
        }
        None => base.fallback(serve_fallback),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
}

async fn get_graph(State(state): State<AppState>) -> Response {
    let json = state.graph_json.read().unwrap().clone();
    ([("content-type", "application/json")], json).into_response()
}

/// GET /api/functions?file=<relative_path>
///
/// Pro-gated endpoint. Returns the function-level subgraph for a single file:
/// `{ "file": "...", "functions": [...], "classes": [...] }`
///
/// Returns 402 Payment Required when no valid Pro license is stored.
async fn get_functions(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if !license::is_pro() {
        return (
            StatusCode::PAYMENT_REQUIRED,
            "Pro license required. Run: xray license activate <key>",
        )
            .into_response();
    }
    let file = match params.get("file") {
        Some(f) => f.clone(),
        None => {
            return (StatusCode::BAD_REQUEST, "missing 'file' query parameter").into_response();
        }
    };
    let functions = state.functions.read().unwrap();
    match functions.get(&file) {
        Some(data) => ([("content-type", "application/json")], data.to_string()).into_response(),
        None => (StatusCode::NOT_FOUND, format!("file not found: {file}")).into_response(),
    }
}

/// GET /api/events
///
/// Server-Sent Events stream. In watch mode, emits a `graph_updated` event
/// whenever the graph has been rescanned. In non-watch mode the stream stays
/// open but never emits (the broadcast channel simply has no senders).
async fn events(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.update_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|r| r.ok())
        .map(|_| Ok(Event::default().event("graph_updated").data("reload")));
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// GET /api/annotations
///
/// Pro-gated. Returns all node annotations as JSON:
/// `{ "src/main.ts": { "team": "platform", "domain": "core" }, ... }`
async fn get_annotations(State(state): State<AppState>) -> Response {
    if !license::is_pro() {
        return (
            StatusCode::PAYMENT_REQUIRED,
            "Pro license required. Run: xray license activate <key>",
        )
            .into_response();
    }

    match annotations::load(&state.root) {
        Ok(ann) => Json(ann).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to load annotations: {e}"),
        )
            .into_response(),
    }
}

/// PUT /api/annotations
///
/// Pro-gated. Replaces the full annotation map with the JSON body.
/// Body: `{ "<node_path>": { "team": "...", "domain": "...", "status": "..." } }`
async fn put_annotations(State(state): State<AppState>, Json(body): Json<Annotations>) -> Response {
    if !license::is_pro() {
        return (
            StatusCode::PAYMENT_REQUIRED,
            "Pro license required. Run: xray license activate <key>",
        )
            .into_response();
    }

    match annotations::save(&state.root, &body) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to save annotations: {e}"),
        )
            .into_response(),
    }
}

async fn serve_fallback() -> Html<&'static str> {
    Html(FALLBACK_HTML)
}

// ── Fallback HTML ─────────────────────────────────────────────────────────────

/// Minimal inline UI served when web/dist hasn't been built yet.
/// Fetches /api/graph and displays graph stats + node list.
/// Uses safe DOM text-node insertion (no innerHTML with user data).
const FALLBACK_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Xray — Codebase Visualizer</title>
  <style>
    :root { --bg:#0f1117; --surface:#1a1d27; --border:#2a2d3e;
            --text:#e2e8f0; --muted:#64748b; --accent:#6366f1; }
    *, *::before, *::after { box-sizing:border-box; margin:0; padding:0; }
    body { font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
           background:var(--bg); color:var(--text); min-height:100vh;
           display:flex; flex-direction:column; }
    header { background:var(--surface); border-bottom:1px solid var(--border);
             padding:12px 20px; display:flex; align-items:center; gap:12px; }
    header h1 { color:var(--accent); font-size:1.1rem; font-weight:700; }
    header span { color:var(--muted); font-size:0.8rem; margin-left:auto; }
    main { flex:1; padding:24px 20px; max-width:900px; width:100%; margin:0 auto; }
    .card { background:var(--surface); border:1px solid var(--border);
            border-radius:8px; padding:16px 20px; margin-bottom:16px; }
    .card h2 { font-size:0.85rem; color:var(--muted); text-transform:uppercase;
               letter-spacing:0.05em; margin-bottom:12px; }
    .stats { display:flex; gap:32px; }
    .stat { display:flex; flex-direction:column; }
    .stat-value { font-size:2rem; font-weight:700; color:var(--accent); }
    .stat-label { font-size:0.75rem; color:var(--muted); }
    table { width:100%; border-collapse:collapse; font-size:0.82rem; }
    th { text-align:left; color:var(--muted); font-weight:500; padding:6px 8px;
         border-bottom:1px solid var(--border); }
    td { padding:6px 8px; border-bottom:1px solid var(--border); }
    td:first-child { font-family:monospace; }
    .lang-ts { color:#3b82f6; }
    .lang-py { color:#f97316; }
    .lang-rs { color:#22c55e; }
    .lang-go { color:#06b6d4; }
    .lang-java { color:#ef4444; }
    .notice { background:var(--surface); border:1px solid var(--border);
              border-radius:8px; padding:16px 20px; color:var(--muted);
              font-size:0.85rem; }
    .notice code { background:rgba(255,255,255,0.07); padding:2px 6px;
                   border-radius:4px; font-family:monospace; }
    #loading { color:var(--muted); padding:40px 0; text-align:center; }
    #error { color:#ef4444; }
  </style>
</head>
<body>
  <header>
    <h1>Xray</h1>
    <span id="root-label"></span>
  </header>
  <main>
    <div id="loading">Loading graph...</div>
    <div id="content" style="display:none">
      <div class="card">
        <h2>Graph Stats</h2>
        <div class="stats">
          <div class="stat"><span class="stat-value" id="s-files">0</span><span class="stat-label">files</span></div>
          <div class="stat"><span class="stat-value" id="s-edges">0</span><span class="stat-label">edges</span></div>
          <div class="stat"><span class="stat-value" id="s-langs">0</span><span class="stat-label">languages</span></div>
          <div class="stat"><span class="stat-value" id="s-ms">0</span><span class="stat-label">ms parse</span></div>
        </div>
      </div>
      <div class="card">
        <h2>Nodes</h2>
        <table>
          <thead><tr><th>Path</th><th>Language</th><th>Kind</th></tr></thead>
          <tbody id="node-table"></tbody>
        </table>
      </div>
      <div class="notice">
        <strong>Full UI not available.</strong> To enable the interactive graph,
        run: <code>cd web &amp;&amp; bun install &amp;&amp; bun run build</code>,
        then restart <code>xray view</code>.
      </div>
    </div>
    <div id="error" style="display:none"></div>
  </main>
  <script type="module">
    const LANG_CLASS = { typescript:'lang-ts', python:'lang-py', rust:'lang-rs',
                         go:'lang-go', java:'lang-java' };

    function makeRow(path, language, kind) {
      const tr = document.createElement('tr');
      const tdPath = document.createElement('td');
      tdPath.style.fontFamily = 'monospace';
      tdPath.textContent = path;
      const tdLang = document.createElement('td');
      tdLang.className = LANG_CLASS[language] ?? '';
      tdLang.textContent = language;
      const tdKind = document.createElement('td');
      tdKind.textContent = kind;
      tr.appendChild(tdPath);
      tr.appendChild(tdLang);
      tr.appendChild(tdKind);
      return tr;
    }

    async function loadGraph() {
      try {
        const resp = await fetch('/api/graph');
        if (!resp.ok) throw new Error('HTTP ' + resp.status);
        const g = await resp.json();

        document.getElementById('root-label').textContent = g.root;
        document.getElementById('s-files').textContent = g.stats.file_count;
        document.getElementById('s-edges').textContent = g.stats.edge_count;
        document.getElementById('s-langs').textContent = g.languages.length;
        document.getElementById('s-ms').textContent = g.stats.parse_duration_ms;

        const tbody = document.getElementById('node-table');
        tbody.textContent = '';
        const slice = g.nodes.slice(0, 200);
        for (const n of slice) {
          tbody.appendChild(makeRow(n.path, n.language, n.kind));
        }
        if (g.nodes.length > 200) {
          const tr = document.createElement('tr');
          const td = document.createElement('td');
          td.setAttribute('colspan', '3');
          td.style.color = 'var(--muted)';
          td.textContent = '... and ' + (g.nodes.length - 200) + ' more';
          tr.appendChild(td);
          tbody.appendChild(tr);
        }

        document.getElementById('loading').style.display = 'none';
        document.getElementById('content').style.display = '';
      } catch (err) {
        document.getElementById('loading').style.display = 'none';
        const el = document.getElementById('error');
        el.style.display = '';
        el.textContent = 'Error: ' + err;
      }
    }

    // Hot reload via SSE
    const evtSource = new EventSource('/api/events');
    evtSource.addEventListener('graph_updated', function() { void loadGraph(); });

    void loadGraph();
  </script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_router_no_panic() {
        // Verifies build_router constructs successfully with no web dir.
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        let state = AppState {
            graph_json: Arc::new(RwLock::new("{}".to_string())),
            functions: Arc::new(RwLock::new(HashMap::new())),
            update_tx: tx,
            root: Arc::new(PathBuf::from(".")),
        };
        let _router = build_router(state, None);
    }

    #[test]
    fn test_fallback_html_contains_xray_branding() {
        assert!(FALLBACK_HTML.contains("Xray"));
        assert!(FALLBACK_HTML.contains("/api/graph"));
    }

    #[test]
    fn test_fallback_html_is_valid_html() {
        assert!(FALLBACK_HTML.starts_with("<!DOCTYPE html>"));
        assert!(FALLBACK_HTML.contains("</html>"));
    }
}
