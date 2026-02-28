use clap::Args;

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

pub fn run(_args: ExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("xray export: not yet implemented (M4)");
    Ok(())
}
