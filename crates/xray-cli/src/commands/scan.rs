use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;
use std::collections::HashSet;
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
pub struct ScanArgs {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub path: String,
    /// Write graph JSON to file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,
    /// Output format: json|dot
    #[arg(long, default_value = "json")]
    pub format: String,
    /// Graph detail: file|function
    #[arg(long, default_value = "file")]
    pub level: String,
    /// Exclude patterns
    #[arg(long)]
    pub exclude: Vec<String>,
    /// Languages to include (default: all detected)
    #[arg(long)]
    pub include: Vec<String>,
    /// Max directory depth
    #[arg(long)]
    pub depth: Option<usize>,
    /// Pretty-print JSON output
    #[arg(long)]
    pub pretty: bool,
    /// Print scan stats to stderr
    #[arg(long)]
    pub stats: bool,
}

pub fn run(args: ScanArgs) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(&args.path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&args.path));

    // Open caches under <root>/.xray/
    let cache_dir = root.join(".xray");
    let db = CacheDb::open(&cache_dir.join("cache.db"))?;

    // Progress spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    spinner.set_message("Scanning…");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let start = Instant::now();

    // Walk directory using `ignore` (respects .gitignore)
    let mut walk_builder = ignore::WalkBuilder::new(&root);
    if let Some(max_depth) = args.depth {
        walk_builder.max_depth(Some(max_depth));
    }
    // Add exclude overrides
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

        // Detect language
        let language = match detect_language(path) {
            Some(l) => l,
            None => continue,
        };

        // Apply language filter if provided
        if !args.include.is_empty() && !args.include.iter().any(|l| l == language) {
            continue;
        }

        // Relative path from repo root
        let rel_path = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Read file contents and hash
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(err) => {
                tracing::warn!("Failed to read '{}': {err}", rel_path);
                continue;
            }
        };
        let hash = blake3::hash(&content);
        let hash_str = hash.to_hex().to_string();

        // Check SQLite cache
        let imports = match db.get_deps(&hash_str) {
            Ok(Some(cached)) => {
                cache_hits += 1;
                cached
            }
            Ok(None) => {
                // Parse file with language-specific parser
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
            path: rel_path.clone(),
            language: language.to_string(),
            imports,
            exports: vec![],
            functions: vec![],
            classes: vec![],
        });

        spinner.set_message(format!("Scanned {} files…", file_asts.len()));
    }

    let parse_duration_ms = start.elapsed().as_millis() as u64;
    spinner.finish_with_message(format!("Found {} files", file_asts.len()));

    // Resolve import specifiers to actual file paths
    let known_paths: HashSet<String> = file_asts.iter().map(|a| a.path.clone()).collect();
    resolve_imports(&mut file_asts, &known_paths);

    // Build graph using petgraph StableGraph
    let builder = GraphBuilder::new(root.to_string_lossy().to_string());
    let graph = builder.build_from_asts(file_asts, parse_duration_ms, cache_hits, &args.level);

    // Save bincode graph cache for instant re-serve
    let graph_bin_path = cache_dir.join("graph.bin");
    if let Err(err) = graph_bin::save(&graph, &graph_bin_path) {
        tracing::warn!("Failed to save graph cache: {err}");
    }

    // Serialize output
    let output_str = match args.format.as_str() {
        "dot" => xray_core::graph::export::to_dot(&graph),
        _ => {
            if args.pretty {
                export::to_json_pretty(&graph)?
            } else {
                export::to_json(&graph)?
            }
        }
    };

    if let Some(output_file) = &args.output {
        std::fs::write(output_file, &output_str)?;
    } else {
        println!("{output_str}");
    }

    if args.stats {
        let s = &graph.stats;
        eprintln!(
            "stats: file_count={} edge_count={} parse_duration_ms={} cache_hits={}",
            s.file_count, s.edge_count, s.parse_duration_ms, s.cache_hits
        );
    }

    Ok(())
}

/// Dispatch to language-specific parser stubs.
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
