use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
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

#[derive(Args)]
pub struct ViewArgs {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub path: String,
    /// Port for embedded HTTP server
    #[arg(long, default_value_t = 7000)]
    pub port: u16,
    /// Don't auto-open browser
    #[arg(long)]
    pub no_open: bool,
    /// Graph detail level: file|function
    #[arg(long, default_value = "file")]
    pub level: String,
    /// Exclude patterns
    #[arg(long)]
    pub exclude: Vec<String>,
    /// Languages to include
    #[arg(long)]
    pub include: Vec<String>,
    /// Maximum directory depth
    #[arg(long)]
    pub depth: Option<usize>,
    /// Re-scan on file changes (hot reload)
    #[arg(long)]
    pub watch: bool,
    /// Also write graph JSON to file
    #[arg(short, long)]
    pub output: Option<String>,
    /// Output format: json|dot
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub async fn run(args: ViewArgs) -> Result<(), Box<dyn std::error::Error>> {
    let port_hint = args.port;
    let no_open = args.no_open;
    let output_file = args.output.clone();

    // ── 1. Scan (blocking; runs on a thread-pool thread) ──────────────────────
    eprintln!("xray: scanning '{}'…", args.path);
    let (graph_json, functions_state) =
        tokio::task::spawn_blocking(move || scan_to_json(args))
            .await
            .map_err(|e| format!("Task join error: {e}"))?
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

    // ── 2. Optionally write graph JSON to file ────────────────────────────────
    if let Some(ref file) = output_file {
        std::fs::write(file, graph_json.as_str())?;
        eprintln!("xray: graph written to {file}");
    }

    // ── 3. Bind server (discovers actual port in case of conflicts) ───────────
    let bound = crate::server::bind(port_hint)
        .await
        .map_err(|e| e.to_string())?;
    let port = bound.port;
    let url = format!("http://127.0.0.1:{port}");
    eprintln!("xray: serving at {url}");

    // ── 4. Find web bundle (if built) ─────────────────────────────────────────
    let web_dist = crate::server::find_web_dist();
    if web_dist.is_some() {
        tracing::info!("Serving full Sigma.js UI from web/dist");
    } else {
        eprintln!("xray: web bundle not found — serving minimal fallback UI");
        eprintln!("      Build with: cd web && bun install && bun run build");
    }

    // ── 5. Start server in background ────────────────────────────────────────
    let server_handle =
        tokio::spawn(async move { bound.serve(graph_json, functions_state, web_dist).await });

    // Brief pause so the port is ready before opening the browser.
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;

    // ── 6. Open browser ───────────────────────────────────────────────────────
    if !no_open {
        match webbrowser::open(&url) {
            Ok(_) => tracing::info!("Browser opened at {url}"),
            Err(e) => eprintln!("xray: could not open browser ({e}). Visit {url}"),
        }
    }

    // ── 7. Run until Ctrl-C or server exits ───────────────────────────────────
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\nxray: shutting down");
        }
        result = server_handle => {
            match result {
                Ok(Err(e)) => eprintln!("xray: server error: {e}"),
                Ok(Ok(())) => {}
                Err(e) => eprintln!("xray: server task panicked: {e}"),
            }
        }
    }

    Ok(())
}

// ── Internal scan helper ──────────────────────────────────────────────────────

/// Synchronously scan `args.path` and return the serialized XrayGraph JSON
/// plus a per-file functions map for the lazy loading API.
///
/// Returns `Err(String)` so the result is `Send` and can be used in
/// `tokio::task::spawn_blocking`.
fn scan_to_json(
    args: ViewArgs,
) -> Result<(Arc<String>, crate::server::routes::FunctionsState), String> {
    let root = std::path::Path::new(&args.path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&args.path));

    let cache_dir = root.join(".xray");
    let db = CacheDb::open(&cache_dir.join("cache.db")).map_err(|e| e.to_string())?;

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Scanning…");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let start = Instant::now();

    let mut walk_builder = ignore::WalkBuilder::new(&root);
    if let Some(max_depth) = args.depth {
        walk_builder.max_depth(Some(max_depth));
    }
    for pattern in &args.exclude {
        walk_builder.add_ignore(pattern);
    }

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

        if !args.include.is_empty() && !args.include.iter().any(|l| l == language) {
            continue;
        }

        let rel_path = path
            .strip_prefix(&root)
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

        // Try to get both imports and functions from cache.
        // If either is missing, parse fresh and cache both.
        let (imports, functions, classes) = match (
            db.get_deps(&hash_str),
            db.get_functions(&hash_str),
        ) {
            (Ok(Some(cached_imports)), Ok(Some((funcs, cls)))) => {
                cache_hits += 1;
                (cached_imports, funcs, cls)
            }
            _ => {
                let source = String::from_utf8_lossy(&content).into_owned();
                let ast = parse_file(language, &source, &rel_path);
                if let Err(err) = db.put_deps(&hash_str, &rel_path, language, &ast.imports) {
                    tracing::warn!("Cache write failed for '{}': {err}", rel_path);
                }
                if let Err(err) = db.put_functions(&hash_str, &ast.functions, &ast.classes) {
                    tracing::warn!("Functions cache write failed for '{}': {err}", rel_path);
                }
                (ast.imports, ast.functions, ast.classes)
            }
        };

        file_asts.push(FileAst {
            path: rel_path.clone(),
            language: language.to_string(),
            imports,
            exports: vec![],
            functions,
            classes,
        });

        spinner.set_message(format!("Scanned {} files…", file_asts.len()));
    }

    let parse_duration_ms = start.elapsed().as_millis() as u64;
    spinner.finish_with_message(format!("Found {} files", file_asts.len()));

    // Resolve import specifiers to actual file paths
    let known_paths: HashSet<String> = file_asts.iter().map(|a| a.path.clone()).collect();
    resolve_imports(&mut file_asts, &known_paths);

    // Build functions state for the lazy loading API (before moving file_asts into the builder).
    let mut functions_map: HashMap<String, serde_json::Value> = HashMap::new();
    for ast in &file_asts {
        let data = serde_json::json!({
            "file": ast.path,
            "functions": ast.functions,
            "classes": ast.classes,
        });
        functions_map.insert(ast.path.clone(), data);
    }
    let functions_state = Arc::new(functions_map);

    let builder = GraphBuilder::new(root.to_string_lossy().to_string());
    let graph = builder.build_from_asts(file_asts, parse_duration_ms, cache_hits, &args.level);

    // Persist bincode graph cache for instant re-serve.
    let graph_bin_path = cache_dir.join("graph.bin");
    if let Err(err) = graph_bin::save(&graph, &graph_bin_path) {
        tracing::warn!("Failed to save graph cache: {err}");
    }

    let json = export::to_json(&graph).map_err(|e| e.to_string())?;
    Ok((Arc::new(json), functions_state))
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
