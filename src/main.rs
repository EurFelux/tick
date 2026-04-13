use clap::Parser;
use serde_json::json;

use tick::cli::comment::CommentCommands;
use tick::cli::issue::IssueCommands;
use tick::cli::{Cli, Commands};
use tick::commands::{comment as cmd_comment, config as cmd_config, init, issue as cmd_issue};
use tick::db::migrate;
use tick::error::Result;
use tick::models::Issue;
use tick::output::{json as out_json, pretty};

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            e.exit();
        }
    };
    let pretty_mode = cli.pretty;
    if let Err(e) = run(cli) {
        if pretty_mode {
            pretty::print_error(&e.to_string());
        } else {
            let json = serde_json::json!({
                "error": e.to_string(),
                "code": e.error_code(),
            });
            eprintln!("{}", json);
        }
        std::process::exit(e.exit_code());
    }
}

fn print_issue_output(
    issue: &Issue,
    pretty_mode: bool,
    quiet: bool,
    fields: &Option<Vec<String>>,
) {
    if quiet {
        println!("{}", issue.id);
    } else if let Some(ref field_list) = fields {
        let refs: Vec<&str> = field_list.iter().map(|s| s.as_str()).collect();
        out_json::print_filtered(issue, &refs);
    } else if pretty_mode {
        pretty::print_issue(issue);
    } else {
        out_json::print(issue);
    }
}

fn run(cli: Cli) -> Result<()> {
    let pretty_mode = cli.pretty;
    let db_path = cli.db.as_deref();
    let quiet = cli.quiet;
    let dry_run = cli.dry_run;
    // --quiet takes precedence over --fields
    let fields: Option<Vec<String>> = if quiet {
        None
    } else {
        cli.fields
            .as_deref()
            .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
    };

    match cli.command {
        Commands::Init => {
            let db = init::run(db_path)?;
            let version = db.schema_version()?;
            if pretty_mode {
                println!("tick initialized");
                println!("schema_version: {}", version);
            } else {
                out_json::print(&json!({
                    "status": "initialized",
                    "schema_version": version,
                }));
            }
        }

        Commands::Version => {
            let schema_version = migrate::expected_version();
            if pretty_mode {
                println!("tick {}", env!("CARGO_PKG_VERSION"));
                println!("schema_version: {}", schema_version);
            } else {
                out_json::print(&json!({
                    "version": env!("CARGO_PKG_VERSION"),
                    "schema_version": schema_version,
                }));
            }
        }

        Commands::Status => {
            let db = init::open_db(db_path)?;
            let counts = db.count_by_status()?;
            if pretty_mode {
                pretty::print_status_counts(&counts);
            } else {
                out_json::print(&counts);
            }
        }

        Commands::Comment(cmd) => {
            let db = init::open_db(db_path)?;
            match cmd {
                CommentCommands::Add {
                    issue_id,
                    body,
                    role,
                } => {
                    let comment = cmd_comment::add(&db, issue_id, &body, &role)?;
                    if quiet {
                        println!("{}", comment.id);
                    } else if let Some(ref field_list) = fields {
                        let refs: Vec<&str> = field_list.iter().map(|s| s.as_str()).collect();
                        out_json::print_filtered(&comment, &refs);
                    } else if pretty_mode {
                        pretty::print_comment(&comment);
                    } else {
                        out_json::print(&comment);
                    }
                }
                CommentCommands::List { issue_id, role } => {
                    let comments = cmd_comment::list(&db, issue_id, role.as_deref())?;
                    if quiet {
                        for c in &comments {
                            println!("{}", c.id);
                        }
                    } else if let Some(ref field_list) = fields {
                        let refs: Vec<&str> = field_list.iter().map(|s| s.as_str()).collect();
                        out_json::print_filtered(&comments, &refs);
                    } else if pretty_mode {
                        pretty::print_comment_list(&comments);
                    } else {
                        out_json::print(&comments);
                    }
                }
            }
        }

        Commands::Config(args) => {
            let db = init::open_db(db_path)?;
            let result = cmd_config::run(
                &db,
                args.set.as_deref(),
                args.get.as_deref(),
                args.list,
            )?;
            if pretty_mode {
                pretty::print_config(&result);
            } else {
                out_json::print(&result);
            }
        }

        Commands::Issue(cmd) => {
            let db = init::open_db(db_path)?;
            match cmd {
                IssueCommands::Create {
                    title,
                    description,
                    issue_type,
                    priority,
                    parent,
                } => {
                    let issue = cmd_issue::create(
                        &db,
                        &title,
                        description.as_deref(),
                        &issue_type,
                        &priority,
                        parent,
                    )?;
                    print_issue_output(&issue, pretty_mode, quiet, &fields);
                }

                IssueCommands::List {
                    status,
                    issue_type,
                    priority,
                    parent,
                    root,
                    limit,
                    offset,
                } => {
                    let issues = cmd_issue::list(
                        &db,
                        status.as_deref(),
                        issue_type.as_deref(),
                        priority.as_deref(),
                        parent,
                        root,
                        limit,
                        offset,
                    )?;
                    if quiet {
                        for issue in &issues {
                            println!("{}", issue.id);
                        }
                    } else if let Some(ref field_list) = fields {
                        let refs: Vec<&str> = field_list.iter().map(|s| s.as_str()).collect();
                        out_json::print_filtered(&issues, &refs);
                    } else if pretty_mode {
                        pretty::print_issue_list(&issues);
                    } else {
                        out_json::print(&issues);
                    }
                }

                IssueCommands::Show { id } => {
                    let detail = cmd_issue::show(&db, id)?;
                    if quiet {
                        println!("{}", detail.issue.id);
                    } else if let Some(ref field_list) = fields {
                        let refs: Vec<&str> = field_list.iter().map(|s| s.as_str()).collect();
                        out_json::print_filtered(&detail, &refs);
                    } else if pretty_mode {
                        pretty::print_issue_detail(&detail);
                    } else {
                        out_json::print(&detail);
                    }
                }

                IssueCommands::Update {
                    id,
                    title,
                    description,
                    issue_type,
                    priority,
                    parent,
                    expect_version,
                } => {
                    if dry_run {
                        // Check issue exists
                        db.get_issue(id)?;
                        out_json::print(&json!({"dry_run": true, "would_succeed": true}));
                    } else {
                        let issue = cmd_issue::update(
                            &db,
                            id,
                            title.as_deref(),
                            description.as_deref(),
                            issue_type.as_deref(),
                            priority.as_deref(),
                            parent,
                            expect_version,
                        )?;
                        print_issue_output(&issue, pretty_mode, quiet, &fields);
                    }
                }

                IssueCommands::Start { id, branch, expect_version } => {
                    if dry_run {
                        tick::validators::validate_start(&db, id, &branch)?;
                        out_json::print(&json!({"dry_run": true, "would_succeed": true}));
                    } else {
                        let issue = cmd_issue::start(&db, id, &branch, expect_version)?;
                        print_issue_output(&issue, pretty_mode, quiet, &fields);
                    }
                }

                IssueCommands::Done { id, expect_version } => {
                    if dry_run {
                        db.get_issue(id)?;
                        out_json::print(&json!({"dry_run": true, "would_succeed": true}));
                    } else {
                        let issue = cmd_issue::done(&db, id, expect_version)?;
                        print_issue_output(&issue, pretty_mode, quiet, &fields);
                    }
                }

                IssueCommands::Close {
                    id,
                    comment,
                    role,
                    resolution,
                    expect_version,
                } => {
                    if dry_run {
                        let issue = db.get_issue(id)?;
                        let res = resolution
                            .parse::<tick::models::Resolution>()
                            .map_err(tick::error::TickError::InvalidArgument)?;
                        tick::validators::validate_close_resolution(&issue.status, &res)?;
                        out_json::print(&json!({"dry_run": true, "would_succeed": true}));
                    } else {
                        let issue = cmd_issue::close(
                            &db,
                            id,
                            comment.as_deref(),
                            &role,
                            &resolution,
                            expect_version,
                        )?;
                        print_issue_output(&issue, pretty_mode, quiet, &fields);
                    }
                }

                IssueCommands::Reopen { id, expect_version } => {
                    if dry_run {
                        db.get_issue(id)?;
                        out_json::print(&json!({"dry_run": true, "would_succeed": true}));
                    } else {
                        let issue = cmd_issue::reopen(&db, id, expect_version)?;
                        print_issue_output(&issue, pretty_mode, quiet, &fields);
                    }
                }

                IssueCommands::Search { query, limit, offset } => {
                    let issues = cmd_issue::search(&db, &query, limit, offset)?;
                    if quiet {
                        for issue in &issues {
                            println!("{}", issue.id);
                        }
                    } else if let Some(ref field_list) = fields {
                        let refs: Vec<&str> = field_list.iter().map(|s| s.as_str()).collect();
                        out_json::print_filtered(&issues, &refs);
                    } else if pretty_mode {
                        pretty::print_issue_list(&issues);
                    } else {
                        out_json::print(&issues);
                    }
                }

                IssueCommands::Link {
                    from_id,
                    relation,
                    to_id,
                } => {
                    let result = cmd_issue::link(&db, from_id, &relation, to_id)?;
                    out_json::print(&result);
                }

                IssueCommands::Unlink { from_id, to_id } => {
                    let result = cmd_issue::unlink(&db, from_id, to_id)?;
                    out_json::print(&result);
                }

                IssueCommands::BatchCreate => {
                    let (results, has_error) = cmd_issue::batch_create(&db)?;
                    out_json::print(&results);
                    if has_error {
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    Ok(())
}
