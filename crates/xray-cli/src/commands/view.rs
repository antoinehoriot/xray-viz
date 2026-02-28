use clap::Args;

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
    /// Re-scan on file changes
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
    tracing::info!("Scanning '{}'...", args.path);
    // M1: scan path, build graph
    // M2: start axum server and open browser
    let url = format!("http://127.0.0.1:{}", args.port);
    eprintln!("xray: server not yet implemented. Would serve at {url}");
    Ok(())
}
