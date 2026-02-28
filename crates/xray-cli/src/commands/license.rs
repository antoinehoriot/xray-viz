use clap::{Args, Subcommand};

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
        LicenseSubcommand::Activate { key: _ } => eprintln!("xray license activate: not yet implemented (M4)"),
        LicenseSubcommand::Status => eprintln!("xray license status: Community edition"),
        LicenseSubcommand::Deactivate => eprintln!("xray license deactivate: no license to remove"),
    }
    Ok(())
}
