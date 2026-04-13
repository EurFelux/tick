pub mod comment;
pub mod config;
pub mod issue;

use clap::Parser;
use comment::CommentCommands;
use config::ConfigArgs;
use issue::IssueCommands;

#[derive(Parser)]
#[command(name = "tick", version, about = "Local Agent-First Issue Tracker")]
pub struct Cli {
    #[arg(long, global = true)]
    pub pretty: bool,

    #[arg(long, global = true)]
    pub db: Option<String>,

    /// Comma-separated list of fields to include in output
    #[arg(long, global = true)]
    pub fields: Option<String>,

    /// Print only the id (for write commands) or one id per line (for list commands)
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Validate without writing to the database
    #[arg(long = "dry-run", global = true)]
    pub dry_run: bool,

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
    #[command(subcommand)]
    Comment(CommentCommands),
    Config(ConfigArgs),
}
