use clap::Args;
use std::collections::HashSet;
use std::time::Instant;
use xray_core::{
    cache::{db::CacheDb, graph_bin},
    detect_language,
    graph::{
        builder::GraphBuilder,
        export::{to_ai_json, to_ai_markdown},
        subgraph::extract_subgraph,
    },
    scanner::{
        languages::{go, java, python, rust, typescript},
        resolve::resolve_imports,
        ExportDecl, FileAst,
    },
};

use crate::license;

#[derive(Args)]
pub struct ExportArgs {
    /// Root directory
    #[arg(default_value = ".")]
    pub path: String,
    /// Starting node (file path or function name)
    #[arg(long)]
    pub from: Option<String>,
    /// Traversal depth from starting node
    #[arg(long, default_value_t = 2)]
    pub depth: usize,
    /// Direction: upstream|downstream|both
    #[arg(long, default_value = "both")]
    pub direction: String,
    /// Output format: json|markdown
    #[arg(long, default_value = "markdown")]
    pub format: String,
    /// Cap exported nodes
    #[arg(long, default_value_t = 200)]
    pub max_nodes: usize,
    /// Write to file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,
}

pub fn run(args: ExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    if !license::is_pro() {
        eprintln!("License required. Run: xray license activate <key>");
        std::process::exit(4);
    }
    let root = std::path::Path::new(&args.path).canonicalize()?;
    let cache_dir = root.join(".xray");
    std::fs::create_dir_all(&cache_dir)?;
    let graph_bin_path = cache_dir.join("graph.bin");
    let graph = match graph_bin::load(&graph_bin_path) {
        Ok(Some(g)) => g,
        _ => build_graph(&root, &cache_dir)?,
    };
    let from = match &args.from {
        Some(f) => f.clone(),
        None => return Err("--from is required".into()),
    };
    let export = extract_subgraph(&graph, &from, args.depth, &args.direction, args.max_nodes);
    let exported_at = utc_now_iso8601();
    let output_str = match args.format.as_str() {
        "json" => to_ai_json(&export, &exported_at)?,
        _ => to_ai_markdown(&export, &exported_at),
    };
    if let Some(output_file) = &args.output {
        std::fs::write(output_file, &output_str)?;
    } else {
        println!("{}", output_str);
    }
    Ok(())
}

/// Scan a directory and build a file-level dependency graph.
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
        let hash = blake3::hash(&content);
        let hash_str = hash.to_hex().to_string();

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

    // Save binary cache for next run.
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

/// UTC timestamp as ISO 8601 string.
fn utc_now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}
