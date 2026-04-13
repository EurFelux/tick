use clap::Subcommand;

#[derive(Subcommand)]
pub enum CommentCommands {
    /// Add a comment to an issue
    Add {
        /// Issue ID
        issue_id: i64,
        /// Comment body
        body: String,
        /// Role: worker, reviewer, pm, qa, user
        #[arg(long, default_value = "user")]
        role: String,
    },
    /// List comments for an issue
    List {
        /// Issue ID
        issue_id: i64,
        /// Filter by role
        #[arg(long)]
        role: Option<String>,
    },
}
