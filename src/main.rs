use clap::Parser;
use serde_json::json;

use tick::cli::issue::IssueCommands;
use tick::cli::{Cli, Commands};
use tick::commands::{init, issue as cmd_issue};
use tick::db::migrate;
use tick::error::Result;
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

fn run(cli: Cli) -> Result<()> {
    let pretty_mode = cli.pretty;
    let db_path = cli.db.as_deref();

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
                    if pretty_mode {
                        pretty::print_issue(&issue);
                    } else {
                        out_json::print(&issue);
                    }
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
                    if pretty_mode {
                        pretty::print_issue_list(&issues);
                    } else {
                        out_json::print(&issues);
                    }
                }

                IssueCommands::Show { id } => {
                    let detail = cmd_issue::show(&db, id)?;
                    if pretty_mode {
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
                } => {
                    let issue = cmd_issue::update(
                        &db,
                        id,
                        title.as_deref(),
                        description.as_deref(),
                        issue_type.as_deref(),
                        priority.as_deref(),
                        parent,
                    )?;
                    if pretty_mode {
                        pretty::print_issue(&issue);
                    } else {
                        out_json::print(&issue);
                    }
                }

                IssueCommands::Start { id, branch } => {
                    let issue = cmd_issue::start(&db, id, &branch)?;
                    if pretty_mode {
                        pretty::print_issue(&issue);
                    } else {
                        out_json::print(&issue);
                    }
                }

                IssueCommands::Done { id } => {
                    let issue = cmd_issue::done(&db, id)?;
                    if pretty_mode {
                        pretty::print_issue(&issue);
                    } else {
                        out_json::print(&issue);
                    }
                }

                IssueCommands::Close {
                    id,
                    comment,
                    role,
                    resolution,
                } => {
                    let issue = cmd_issue::close(&db, id, comment.as_deref(), &role, &resolution)?;
                    if pretty_mode {
                        pretty::print_issue(&issue);
                    } else {
                        out_json::print(&issue);
                    }
                }

                IssueCommands::Reopen { id } => {
                    let issue = cmd_issue::reopen(&db, id)?;
                    if pretty_mode {
                        pretty::print_issue(&issue);
                    } else {
                        out_json::print(&issue);
                    }
                }
            }
        }
    }

    Ok(())
}
