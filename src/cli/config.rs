use clap::Args;

#[derive(Args)]
pub struct ConfigArgs {
    /// Set a config value (key=value)
    #[arg(long)]
    pub set: Option<String>,

    /// Get a config value
    #[arg(long)]
    pub get: Option<String>,

    /// List all config values
    #[arg(long)]
    pub list: bool,
}
