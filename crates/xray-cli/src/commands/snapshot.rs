//! `xray snapshot` — generate a self-contained shareable graph HTML.
//!
//! Scans the codebase (using cached `.xray/graph.bin` if available), then
//! writes a single `.html` file that embeds the full graph JSON and loads
//! Sigma.js v3 + graphology from CDN for offline-capable rendering.
//!
//! Pro-gated. Requires a valid license key stored at `~/.xray/license.key`.

use clap::Args;
use std::collections::HashSet;
use std::time::Instant;
use xray_core::{
    cache::{db::CacheDb, graph_bin},
    detect_language,
    graph::{builder::GraphBuilder, export},
    scanner::{
        languages::{go, java, python, rust, typescript},
        resolve::resolve_imports,
        ExportDecl, FileAst,
    },
};

use crate::license;

#[derive(Args)]
pub struct SnapshotArgs {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub path: String,

    /// Output file path (default: `xray-snapshot.html`)
    #[arg(short, long, default_value = "xray-snapshot.html")]
    pub output: String,

    /// Title shown in the snapshot HTML
    #[arg(long)]
    pub title: Option<String>,
}

pub fn run(args: SnapshotArgs) -> Result<(), Box<dyn std::error::Error>> {
    if !license::is_pro() {
        eprintln!("License required. Run: xray license activate <key>");
        std::process::exit(4);
    }

    let root = std::path::Path::new(&args.path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&args.path));

    eprintln!("xray: scanning '{}'…", root.display());

    let graph = {
        let cache_dir = root.join(".xray");
        std::fs::create_dir_all(&cache_dir)?;
        let graph_bin_path = cache_dir.join("graph.bin");
        match graph_bin::load(&graph_bin_path) {
            Ok(Some(g)) => g,
            _ => build_graph(&root, &cache_dir)?,
        }
    };

    let graph_json = export::to_json(&graph)?;
    let title = args
        .title
        .unwrap_or_else(|| format!("Xray — {}", root.display()));

    let html = render_snapshot_html(&title, &graph_json);
    std::fs::write(&args.output, &html)?;

    eprintln!("xray: snapshot written to {}", args.output);
    eprintln!(
        "xray: {} nodes, {} edges",
        graph.nodes.len(),
        graph.edges.len()
    );

    Ok(())
}

// ── Graph builder ─────────────────────────────────────────────────────────────

fn build_graph(
    root: &std::path::Path,
    cache_dir: &std::path::Path,
) -> Result<xray_core::XrayGraph, Box<dyn std::error::Error>> {
    let db = CacheDb::open(&cache_dir.join("cache.db"))?;
    let start = Instant::now();
    let walk_builder = ignore::WalkBuilder::new(root);
    let mut file_asts: Vec<FileAst> = Vec::new();
    let mut cache_hits: u64 = 0;

    for entry in walk_builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!("Walk error: {err}");
                continue;
            }
        };
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let language = match detect_language(path) {
            Some(l) => l,
            None => continue,
        };
        let rel_path = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(err) => {
                tracing::warn!("Failed to read '{}': {err}", rel_path);
                continue;
            }
        };
        let hash_str = blake3::hash(&content).to_hex().to_string();
        let imports = match db.get_deps(&hash_str) {
            Ok(Some(cached)) => {
                cache_hits += 1;
                cached
            }
            Ok(None) => {
                let source = String::from_utf8_lossy(&content).into_owned();
                let ast = parse_file(language, &source, &rel_path);
                let imports = ast.imports.clone();
                if let Err(err) = db.put_deps(&hash_str, &rel_path, language, &imports) {
                    tracing::warn!("Cache write failed for '{}': {err}", rel_path);
                }
                imports
            }
            Err(err) => {
                tracing::warn!("Cache read error: {err}");
                vec![]
            }
        };
        file_asts.push(FileAst {
            path: rel_path,
            language: language.to_string(),
            imports,
            exports: vec![],
            functions: vec![],
            classes: vec![],
        });
    }

    let parse_duration_ms = start.elapsed().as_millis() as u64;
    let known_paths: HashSet<String> = file_asts.iter().map(|a| a.path.clone()).collect();
    resolve_imports(&mut file_asts, &known_paths);

    let builder = GraphBuilder::new(root.to_string_lossy().to_string());
    let graph = builder.build_from_asts(file_asts, parse_duration_ms, cache_hits, "file");

    let graph_bin_path = cache_dir.join("graph.bin");
    if let Err(err) = graph_bin::save(&graph, &graph_bin_path) {
        tracing::warn!("Failed to save graph cache: {err}");
    }

    Ok(graph)
}

fn parse_file(language: &str, source: &str, path: &str) -> FileAst {
    match language {
        "typescript" => typescript::parse(source, path),
        "python" => python::parse(source, path),
        "rust" => rust::parse(source, path),
        "go" => go::parse(source, path),
        "java" => java::parse(source, path),
        _ => FileAst {
            path: path.to_string(),
            language: language.to_string(),
            imports: vec![],
            exports: Vec::<ExportDecl>::new(),
            functions: vec![],
            classes: vec![],
        },
    }
}

// ── HTML template ─────────────────────────────────────────────────────────────

/// Render the self-contained snapshot HTML.
///
/// Embeds `graph_json` as an inline JS variable; loads graphology + sigma
/// from the jsDelivr CDN. The resulting file works in any modern browser
/// without requiring a local server.
///
/// All user-derived strings are embedded via `textContent` (not innerHTML)
/// to prevent XSS.
fn render_snapshot_html(title: &str, graph_json: &str) -> String {
    let safe_title = html_escape(title);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>{safe_title}</title>
  <style>
    :root {{
      --bg:#0f1117; --surface:#1a1d27; --border:#2a2d3e;
      --text:#e2e8f0; --muted:#64748b; --accent:#6366f1;
    }}
    *, *::before, *::after {{ box-sizing:border-box; margin:0; padding:0; }}
    body {{ font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
            background:var(--bg); color:var(--text); height:100vh;
            display:flex; flex-direction:column; overflow:hidden; }}
    header {{ background:var(--surface); border-bottom:1px solid var(--border);
              padding:10px 20px; display:flex; align-items:center; gap:16px;
              flex-shrink:0; }}
    header h1 {{ color:var(--accent); font-size:1rem; font-weight:700; }}
    .stats {{ display:flex; gap:20px; margin-left:auto; }}
    .stat {{ font-size:0.8rem; color:var(--muted); }}
    .stat strong {{ color:var(--text); }}
    #sigma-container {{ flex:1; }}
    #tooltip {{ position:fixed; display:none; background:var(--surface);
                border:1px solid var(--border); border-radius:6px;
                padding:8px 12px; font-size:0.8rem; pointer-events:none;
                max-width:320px; z-index:100; }}
    #tt-path {{ font-family:monospace; color:var(--accent); margin-bottom:4px; word-break:break-all; }}
    #tt-meta {{ color:var(--muted); font-size:0.75rem; }}
    footer {{ background:var(--surface); border-top:1px solid var(--border);
              padding:6px 20px; font-size:0.72rem; color:var(--muted);
              display:flex; align-items:center; gap:8px; flex-shrink:0; }}
  </style>
</head>
<body>
  <header>
    <h1 id="page-title"></h1>
    <div class="stats">
      <div class="stat"><strong id="s-nodes">–</strong> files</div>
      <div class="stat"><strong id="s-edges">–</strong> edges</div>
      <div class="stat"><strong id="s-langs">–</strong> languages</div>
    </div>
  </header>
  <div id="sigma-container"></div>
  <div id="tooltip">
    <div id="tt-path"></div>
    <div id="tt-meta"></div>
  </div>
  <footer>
    <span>Generated by <strong>xray</strong></span>
    <span>·</span>
    <span id="footer-date"></span>
    <span>·</span>
    <span>Click a node to inspect &nbsp;|&nbsp; Scroll to zoom &nbsp;|&nbsp; Drag to pan</span>
  </footer>

  <script src="https://cdn.jsdelivr.net/npm/graphology@0.25/dist/graphology.umd.min.js"></script>
  <script src="https://cdn.jsdelivr.net/npm/sigma@3/build/sigma.min.js"></script>
  <script>
    // All graph data is parsed from trusted JSON generated by xray, not user input.
    const GRAPH_DATA = {graph_json};

    // Set page title safely via textContent
    document.getElementById('page-title').textContent = {title_json};
    document.title = {title_json};

    // ── Language color map ────────────────────────────────────────────────────
    const LANG_COLOR = {{
      typescript: '#3b82f6',
      python:     '#f97316',
      rust:       '#22c55e',
      go:         '#06b6d4',
      java:       '#ef4444',
    }};

    function nodeColor(lang) {{
      return LANG_COLOR[lang] ?? '#6366f1';
    }}

    // ── Layout: Fibonacci lattice (even scatter, no overlap) ─────────────────
    function layoutNodes(nodes) {{
      const count = nodes.length;
      const positions = {{}};
      const phi = Math.PI * (3 - Math.sqrt(5));
      const radius = Math.max(10, Math.sqrt(count) * 2.5);
      nodes.forEach((n, i) => {{
        const r = radius * Math.sqrt((i + 0.5) / count);
        const theta = phi * i;
        positions[n.id] = {{ x: r * Math.cos(theta), y: r * Math.sin(theta) }};
      }});
      return positions;
    }}

    // ── Build graphology graph ────────────────────────────────────────────────
    const graph = new graphology.Graph({{ type: 'directed', multi: false }});
    const positions = layoutNodes(GRAPH_DATA.nodes);

    GRAPH_DATA.nodes.forEach(n => {{
      const pos = positions[n.id] ?? {{ x: 0, y: 0 }};
      graph.addNode(n.id, {{
        x:        pos.x,
        y:        pos.y,
        size:     4 + Math.min(8, (n.metadata?.exports?.length ?? 0) * 0.5),
        color:    nodeColor(n.language),
        label:    n.label || n.path,
        _path:    n.path,
        _lang:    n.language,
        _kind:    n.kind,
      }});
    }});

    GRAPH_DATA.edges.forEach(e => {{
      if (graph.hasNode(e.source) && graph.hasNode(e.target) &&
          !graph.hasEdge(e.source, e.target)) {{
        graph.addDirectedEdge(e.source, e.target, {{
          size:  1,
          color: '#2a2d3e',
        }});
      }}
    }});

    // ── Stats ─────────────────────────────────────────────────────────────────
    document.getElementById('s-nodes').textContent = GRAPH_DATA.nodes.length;
    document.getElementById('s-edges').textContent = GRAPH_DATA.edges.length;
    const langs = [...new Set(GRAPH_DATA.nodes.map(n => n.language))];
    document.getElementById('s-langs').textContent = langs.length;
    const footerDate = document.getElementById('footer-date');
    footerDate.textContent = GRAPH_DATA.scanned_at ?? '';

    // ── Sigma renderer ────────────────────────────────────────────────────────
    const container = document.getElementById('sigma-container');
    const renderer = new Sigma(graph, container, {{
      renderEdgeLabels:  false,
      defaultEdgeType:   'line',
      labelColor:        {{ color: '#e2e8f0' }},
      labelSize:         11,
      labelFont:         'monospace',
      minCameraRatio:    0.05,
      maxCameraRatio:    10,
    }});

    // ── Tooltip on hover (uses textContent, no innerHTML) ─────────────────────
    const tooltip = document.getElementById('tooltip');
    const ttPath  = document.getElementById('tt-path');
    const ttMeta  = document.getElementById('tt-meta');

    renderer.on('enterNode', ({{ node }}) => {{
      const attr = graph.getNodeAttributes(node);
      ttPath.textContent = attr._path;
      ttMeta.textContent = attr._lang + (attr._kind ? ' · ' + attr._kind : '');
      tooltip.style.display = 'block';
    }});

    renderer.on('leaveNode', () => {{
      tooltip.style.display = 'none';
    }});

    document.addEventListener('mousemove', e => {{
      if (tooltip.style.display === 'block') {{
        tooltip.style.left = (e.clientX + 14) + 'px';
        tooltip.style.top  = (e.clientY + 14) + 'px';
      }}
    }});

    // ── Click: highlight neighbors ────────────────────────────────────────────
    let selectedNode = null;

    function resetHighlight() {{
      graph.forEachNode(n => {{
        graph.setNodeAttribute(n, 'color',
          nodeColor(graph.getNodeAttribute(n, '_lang')));
      }});
      graph.forEachEdge(e => graph.setEdgeAttribute(e, 'color', '#2a2d3e'));
      selectedNode = null;
    }}

    renderer.on('clickNode', ({{ node }}) => {{
      if (selectedNode === node) {{ resetHighlight(); return; }}
      selectedNode = node;
      const neighbors = new Set(graph.neighbors(node));
      graph.forEachNode(n => {{
        graph.setNodeAttribute(n, 'color',
          (n === node || neighbors.has(n))
            ? nodeColor(graph.getNodeAttribute(n, '_lang'))
            : '#1e293b');
      }});
      graph.forEachEdge(e => {{
        const connected = graph.source(e) === node || graph.target(e) === node;
        graph.setEdgeAttribute(e, 'color', connected ? '#6366f1' : '#1a1d27');
      }});
    }});

    renderer.on('clickStage', () => {{
      if (selectedNode !== null) resetHighlight();
    }});
  </script>
</body>
</html>
"#,
        safe_title = safe_title,
        graph_json = graph_json,
        title_json = serde_json::to_string(title).unwrap_or_else(|_| "\"Xray Snapshot\"".to_string()),
    )
}

/// Escape `<`, `>`, `&`, `"`, `'` for safe HTML attribute/text embedding.
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_handles_special_chars() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("it's"), "it&#39;s");
        assert_eq!(html_escape("normal"), "normal");
    }

    #[test]
    fn render_snapshot_html_contains_graph_json() {
        let graph_json = r#"{"nodes":[],"edges":[]}"#;
        let html = render_snapshot_html("Test Snapshot", graph_json);
        assert!(html.contains(graph_json));
        assert!(html.contains("Test Snapshot"));
        assert!(html.contains("sigma"));
        assert!(html.contains("graphology"));
        assert!(html.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn render_snapshot_html_escapes_title_in_head() {
        let html = render_snapshot_html("<Xray> & 'test'", "{}");
        // <title> tag must have the escaped title
        assert!(html.contains("<title>&lt;Xray&gt; &amp; &#39;test&#39;</title>"));
    }

    #[test]
    fn render_snapshot_html_uses_cdn_urls() {
        let html = render_snapshot_html("T", "{}");
        assert!(html.contains("cdn.jsdelivr.net"));
        assert!(html.contains("sigma@3"));
        assert!(html.contains("graphology@0.25"));
    }
}
