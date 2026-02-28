use clap::Args;

#[derive(Args)]
pub struct ScanArgs {
    /// Root directory to scan
    #[arg(default_value = ".")]
    pub path: String,
    /// Write graph JSON to file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,
    /// Output format: json|dot|mermaid
    #[arg(long, default_value = "json")]
    pub format: String,
    /// Graph detail: file|function
    #[arg(long, default_value = "file")]
    pub level: String,
    /// Exclude patterns
    #[arg(long)]
    pub exclude: Vec<String>,
    /// Languages to include
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
    let graph = xray_core::graph::types::XrayGraph::new(&args.path);
    let json = if args.pretty {
        xray_core::graph::export::to_json_pretty(&graph)?
    } else {
        xray_core::graph::export::to_json(&graph)?
    };
    if let Some(output) = &args.output {
        std::fs::write(output, &json)?;
    } else {
        println!("{json}");
    }
    if args.stats {
        eprintln!("stats: file_count=0 edge_count=0 parse_duration_ms=0");
    }
    Ok(())
}
