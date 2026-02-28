//! Axum HTTP routes for the embedded server.
//!
//! Routes:
//!   GET /health        — health check
//!   GET /api/graph     — full XrayGraph JSON
//!   GET /*             — static file serving from web/dist (if present),
//!                        otherwise a minimal inline fallback UI
//!
//! Listens on 127.0.0.1 only (not 0.0.0.0) for security.

use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeDir;

/// Shared graph JSON state.
pub type GraphState = Arc<String>;

/// Build the axum router.
///
/// - `graph_json`: serialized XrayGraph (served at /api/graph)
/// - `web_dist`: optional path to web/dist for full UI assets;
///   if None, serves a minimal fallback HTML
pub fn build_router(graph_json: GraphState, web_dist: Option<PathBuf>) -> Router {
    // State-dependent routes — bake state in first.
    let base = Router::new()
        .route("/health", get(health))
        .route("/api/graph", get(get_graph))
        .with_state(graph_json);

    // Static file serving (no state needed).
    match web_dist {
        Some(dir) => base.fallback_service(
            ServeDir::new(dir).append_index_html_on_directories(true),
        ),
        None => base.fallback(serve_fallback),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> &'static str {
    "ok"
}

async fn get_graph(State(json): State<GraphState>) -> Response {
    ([("content-type", "application/json")], json.as_str().to_string()).into_response()
}

async fn serve_fallback() -> Html<&'static str> {
    Html(FALLBACK_HTML)
}

// ── Fallback HTML ─────────────────────────────────────────────────────────────

/// Minimal inline UI served when web/dist hasn't been built yet.
/// Fetches /api/graph and displays graph stats + node list.
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
    <div id="loading">Loading graph…</div>
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
    try {
      const resp = await fetch('/api/graph');
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const g = await resp.json();

      document.getElementById('root-label').textContent = g.root;
      document.getElementById('s-files').textContent = g.stats.file_count;
      document.getElementById('s-edges').textContent = g.stats.edge_count;
      document.getElementById('s-langs').textContent = g.languages.length;
      document.getElementById('s-ms').textContent = g.stats.parse_duration_ms;

      const tbody = document.getElementById('node-table');
      for (const n of g.nodes.slice(0, 200)) {
        const cls = LANG_CLASS[n.language] ?? '';
        tbody.insertAdjacentHTML('beforeend',
          `<tr><td>${n.path}</td><td class="${cls}">${n.language}</td><td>${n.kind}</td></tr>`);
      }
      if (g.nodes.length > 200) {
        tbody.insertAdjacentHTML('beforeend',
          `<tr><td colspan="3" style="color:var(--muted)">… and ${g.nodes.length - 200} more</td></tr>`);
      }

      document.getElementById('loading').style.display = 'none';
      document.getElementById('content').style.display = '';
    } catch (err) {
      document.getElementById('loading').style.display = 'none';
      const el = document.getElementById('error');
      el.style.display = '';
      el.textContent = `Error: ${err}`;
    }
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
        let _router = build_router(Arc::new("{}".to_string()), None);
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
