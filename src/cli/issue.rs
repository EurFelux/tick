use clap::Subcommand;

#[derive(Subcommand)]
pub enum IssueCommands {
    /// Create a new issue
    Create {
        /// Issue title
        title: String,

        /// Description
        #[arg(short = 'd', long)]
        description: Option<String>,

        /// Issue type (bug, feature, refactor, docs, test, chore)
        #[arg(short = 't', long = "type", default_value = "feature")]
        issue_type: String,

        /// Priority (low, medium, high, critical)
        #[arg(short = 'p', long, default_value = "medium")]
        priority: String,

        /// Parent issue ID
        #[arg(long)]
        parent: Option<i64>,
    },

    /// List issues
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Filter by type
        #[arg(long = "type")]
        issue_type: Option<String>,

        /// Filter by priority
        #[arg(long)]
        priority: Option<String>,

        /// Filter by parent ID
        #[arg(long)]
        parent: Option<i64>,

        /// Show only root issues (no parent)
        #[arg(long)]
        root: bool,

        /// Max results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Results offset
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Show issue details
    Show {
        /// Issue ID
        id: i64,
    },

    /// Update an issue
    Update {
        /// Issue ID
        id: i64,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description
        #[arg(long = "desc")]
        description: Option<String>,

        /// New type
        #[arg(long = "type")]
        issue_type: Option<String>,

        /// New priority
        #[arg(long)]
        priority: Option<String>,

        /// New parent ID
        #[arg(long)]
        parent: Option<i64>,

        /// Expected version for optimistic locking
        #[arg(long)]
        expect_version: Option<i64>,
    },

    /// Start working on an issue
    Start {
        /// Issue ID
        id: i64,

        /// Branch name
        #[arg(long, required = true)]
        branch: String,

        /// Expected version for optimistic locking
        #[arg(long)]
        expect_version: Option<i64>,
    },

    /// Mark an issue as done
    Done {
        /// Issue ID
        id: i64,

        /// Expected version for optimistic locking
        #[arg(long)]
        expect_version: Option<i64>,
    },

    /// Close an issue
    Close {
        /// Issue ID
        id: i64,

        /// Closing comment
        #[arg(short = 'c', long)]
        comment: Option<String>,

        /// Comment role
        #[arg(long, default_value = "user")]
        role: String,

        /// Resolution (resolved, wontfix)
        #[arg(long, default_value = "resolved")]
        resolution: String,

        /// Expected version for optimistic locking
        #[arg(long)]
        expect_version: Option<i64>,
    },

    /// Reopen a closed issue
    Reopen {
        /// Issue ID
        id: i64,

        /// Expected version for optimistic locking
        #[arg(long)]
        expect_version: Option<i64>,
    },

    /// Full-text search issues
    Search {
        /// Search query
        query: String,

        /// Max results
        #[arg(long, default_value_t = 50)]
        limit: i64,

        /// Results offset
        #[arg(long, default_value_t = 0)]
        offset: i64,
    },

    /// Create a dependency link
    Link {
        from_id: i64,
        /// Must be "depends-on"
        relation: String,
        to_id: i64,
    },

    /// Remove a dependency link
    Unlink {
        from_id: i64,
        to_id: i64,
    },

    /// Batch create issues from stdin JSONL
    BatchCreate,
}
