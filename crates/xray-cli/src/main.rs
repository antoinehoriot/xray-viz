use clap::{Parser, Subcommand};

mod commands;
mod server;

#[derive(Parser)]
#[command(name = "xray", version, about = "Interactive codebase architecture visualizer")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a codebase and open the interactive browser UI (default)
    View(commands::view::ViewArgs),
    /// Parse a codebase and output a graph (no browser)
    Scan(commands::scan::ScanArgs),
    /// Export a graph slice for AI/LLM context
    Export(commands::export::ExportArgs),
    /// Manage Pro license
    License(commands::license::LicenseArgs),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_env("XRAY_LOG")
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::View(args)) => commands::view::run(args).await,
        Some(Commands::Scan(args)) => commands::scan::run(args),
        Some(Commands::Export(args)) => commands::export::run(args),
        Some(Commands::License(args)) => commands::license::run(args),
        // Default: xray . is equivalent to xray view .
        None => {
            commands::view::run(commands::view::ViewArgs {
                path: ".".to_string(),
                port: 7000,
                no_open: false,
                level: "file".to_string(),
                exclude: vec![],
                include: vec![],
                depth: None,
                watch: false,
                output: None,
                format: "json".to_string(),
            })
            .await
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
