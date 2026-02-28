use clap::{Args, Subcommand};

use crate::license;

#[derive(Args)]
pub struct LicenseArgs {
    #[command(subcommand)]
    pub subcommand: LicenseSubcommand,
}

#[derive(Subcommand)]
pub enum LicenseSubcommand {
    /// Activate a Pro license key
    Activate { key: String },
    /// Show current license status
    Status,
    /// Remove stored license
    Deactivate,
}

pub fn run(args: LicenseArgs) -> Result<(), Box<dyn std::error::Error>> {
    match args.subcommand {
        LicenseSubcommand::Activate { key } => {
            if !license::is_valid_key(&key) {
                eprintln!("error: invalid license key format. Expected: XRAY-XXXX-XXXX");
                std::process::exit(1);
            }
            license::save_license(&key)?;
            println!("Pro license activated.");
        }
        LicenseSubcommand::Status => match license::get_license() {
            Some(key) if license::is_valid_key(&key) => {
                println!("Pro license: active ({})", redact_key(&key));
            }
            Some(_) => {
                eprintln!("License: invalid key stored. Run: xray license activate <key>");
            }
            None => {
                println!("License: Community edition (no Pro license stored)");
            }
        },
        LicenseSubcommand::Deactivate => {
            license::remove_license()?;
            println!("Pro license removed.");
        }
    }
    Ok(())
}

/// Returns the key with the last segment redacted: `XRAY-XXXX-****`.
fn redact_key(key: &str) -> String {
    let parts: Vec<&str> = key.split('-').collect();
    if parts.len() >= 3 {
        format!("{}-{}-****", parts[0], parts[1])
    } else {
        "****".to_string()
    }
}
