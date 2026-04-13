pub mod issue;

use clap::Parser;
use issue::IssueCommands;

#[derive(Parser)]
#[command(name = "tick", version, about = "Local Agent-First Issue Tracker")]
pub struct Cli {
    #[arg(long, global = true)]
    pub pretty: bool,

    #[arg(long, global = true)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Init,
    Version,
    Status,
    #[command(subcommand)]
    Issue(IssueCommands),
}
