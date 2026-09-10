//! `xray annotate` — read and write node annotations stored in
//! `.xray/annotations.yml`.
//!
//! Pro-gated. Requires a valid license key stored at `~/.xray/license.key`.

use clap::Args;
use xray_core::annotations::{self, NodeAnnotation};

use crate::license;

#[derive(Args)]
pub struct AnnotateArgs {
    /// Root directory of the repository to annotate
    #[arg(default_value = ".")]
    pub path: String,

    /// Relative node path to annotate (e.g. `src/main.ts`)
    #[arg(long)]
    pub set: Option<String>,

    /// Set the team label on the node
    #[arg(long)]
    pub team: Option<String>,

    /// Set the domain label on the node
    #[arg(long)]
    pub domain: Option<String>,

    /// Set the status label on the node (e.g. stable, wip, deprecated)
    #[arg(long)]
    pub status: Option<String>,

    /// Comma-separated tags to set on the node (e.g. "critical,needs-refactor")
    #[arg(long, use_value_delimiter = true)]
    pub tags: Option<Vec<String>>,

    /// List all annotations in the repository
    #[arg(long)]
    pub list: bool,

    /// Remove all annotations from the specified node
    #[arg(long)]
    pub remove: Option<String>,
}

pub fn run(args: AnnotateArgs) -> Result<(), Box<dyn std::error::Error>> {
    if !license::is_pro() {
        eprintln!("License required. Run: xray license activate <key>");
        std::process::exit(4);
    }

    let root = std::path::Path::new(&args.path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&args.path));

    // ── List ──────────────────────────────────────────────────────────────────
    if args.list {
        let ann = annotations::load(&root)?;
        if ann.is_empty() {
            println!("No annotations found. Use: xray annotate --set <file> --team <team>");
        } else {
            let mut entries: Vec<_> = ann.iter().collect();
            entries.sort_by_key(|(k, _)| k.as_str());
            for (path, node_ann) in entries {
                let mut parts: Vec<String> = Vec::new();
                if let Some(t) = &node_ann.team {
                    parts.push(format!("team={t}"));
                }
                if let Some(d) = &node_ann.domain {
                    parts.push(format!("domain={d}"));
                }
                if let Some(s) = &node_ann.status {
                    parts.push(format!("status={s}"));
                }
                if !node_ann.tags.is_empty() {
                    parts.push(format!("tags={}", node_ann.tags.join(",")));
                }
                println!("{path}: {}", parts.join("  "));
            }
        }
        return Ok(());
    }

    // ── Remove ────────────────────────────────────────────────────────────────
    if let Some(node_path) = &args.remove {
        let mut ann = annotations::load(&root)?;
        if ann.remove(node_path).is_some() {
            annotations::save(&root, &ann)?;
            println!("Removed annotations for {node_path}");
        } else {
            println!("No annotations found for {node_path}");
        }
        return Ok(());
    }

    // ── Set ───────────────────────────────────────────────────────────────────
    let node_path = match &args.set {
        Some(p) => p.clone(),
        None => {
            return Err(
                "Specify --set <file> to annotate, --list to view, or --remove <file> to delete"
                    .into(),
            );
        }
    };

    if args.team.is_none() && args.domain.is_none() && args.status.is_none() && args.tags.is_none()
    {
        return Err("Provide at least one of --team, --domain, --status, --tags".into());
    }

    let mut ann = annotations::load(&root)?;
    let entry: &mut NodeAnnotation = ann.entry(node_path.clone()).or_default();

    if let Some(t) = args.team {
        entry.team = Some(t);
    }
    if let Some(d) = args.domain {
        entry.domain = Some(d);
    }
    if let Some(s) = args.status {
        entry.status = Some(s);
    }
    if let Some(t) = args.tags {
        entry.tags = t;
    }

    annotations::save(&root, &ann)?;
    println!("Annotated {node_path}");

    Ok(())
}
