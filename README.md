# tick

Local Agent-First Issue Tracker. Serverless CLI for single-developer multi-agent workflows.

## What is tick?

tick is a local issue tracker that lives inside your git repo. No server, no port, no runtime — a single binary reads and writes a SQLite database at `<git-common-dir>/tick/tick.db`, shared across all worktrees.

Designed for developers who use multiple AI agents (Claude, Copilot, Codex, etc.) in parallel. Default output is JSON so agents can parse it directly. Humans get `--pretty`.

## Install

```bash
# From source
cargo install --path .

# Or download the binary from the latest release
# https://github.com/EurFelux/tick/releases
```

## Quick Start

```bash
cd your-git-repo
tick init

# Create issues
tick issue create "Fix login bug" -t bug -p high
tick issue create "Add dark mode" -t feature

# Work on an issue
tick issue start 1 --branch fix/login
tick comment add 1 "Fixed SQL injection in auth.rs:42" --role worker
tick issue done 1

# Review and close
tick comment add 1 "LGTM" --role reviewer
tick issue close 1

# Check status
tick status
```

## State Machine

Issues follow a strict lifecycle with semantic commands:

```
              [open]
                │
          tick issue start --branch <B>
                │
            [in-progress]
                │
          tick issue done
                │
              [done]
                │
          tick issue close
                │
           [closed(R)]
         resolved | wontfix
```

| Command | Transition | Atomic check |
|---------|-----------|--------------|
| `tick issue start <id> --branch B` | open &rarr; in-progress | `WHERE status = 'open'` |
| `tick issue done <id>` | in-progress &rarr; done | `WHERE status = 'in-progress'` |
| `tick issue close <id>` | done &rarr; closed(resolved) | `WHERE status = 'done'` |
| `tick issue close <id> --resolution wontfix` | any &rarr; closed(wontfix) | validates allowed transitions |
| `tick issue reopen <id>` | closed &rarr; open | `WHERE status = 'closed'` |

`tick issue update` only modifies non-status fields (title, description, type, priority, parent).

## Commands

### Issues

```bash
tick issue create <title> [-d desc] [-t type] [-p priority] [--parent <id>]
tick issue list [--status S] [--type T] [--priority P] [--parent <id>] [--root] [--limit N] [--offset O]
tick issue show <id>
tick issue update <id> [--title T] [--desc D] [--type T] [--priority P] [--parent <id>]
tick issue start <id> --branch <B>
tick issue done <id>
tick issue close <id> [-c comment] [--role R] [--resolution resolved|wontfix]
tick issue reopen <id>
tick issue search <query>
tick issue link <from> depends-on <to>
tick issue unlink <from> <to>
tick issue batch-create < issues.jsonl
```

### Comments

```bash
tick comment add <issue-id> <body> [--role worker|reviewer|pm|qa|user]
tick comment list <issue-id> [--role R]
```

### Other

```bash
tick init                          # Initialize in current git repo
tick version                       # Version + schema version
tick status                        # Issue count by status
tick config --set key=value        # Set config
tick config --get key              # Get config
tick config --list                 # List all config
```

### Global Flags

```bash
tick [--pretty]              # Human-readable output
tick [--fields id,title]     # Filter output fields
tick [--quiet]               # Output only IDs
tick [--dry-run]             # Validate without writing
tick [--db <path>]           # Override DB path
tick [--expect-version V]    # Optimistic locking (on write commands)
```

## Agent-First Design

- **JSON by default**: every command outputs machine-readable JSON. Errors too: `{"error": "...", "code": "NOT_FOUND"}`
- **Deterministic IDs**: auto-increment integers, no ambiguity
- **Atomic transitions**: each state change uses `WHERE status = ?` CAS — two agents can't start the same issue
- **No delete**: issues can only be closed, never deleted. Agent mistakes are recoverable
- **Dependency tracking**: `tick issue link 2 depends-on 1` — blocked issues can't start until dependencies resolve. Wontfix cascades automatically
- **Exit codes**: 0=ok, 1=internal, 2=not found, 3=invalid arg, 4=db error, 5=not initialized, 6=conflict

## Dependencies & Cascade

```bash
tick issue link 5 depends-on 3     # Issue 5 depends on issue 3
tick issue start 5 --branch fix    # Fails: dependency #3 is not resolved
tick issue close 3 --resolution wontfix  # Cascades: issue 5 auto-closed(wontfix)
```

## Comment Roles

Comments carry a `role` indicating the perspective, not the identity:

| Role | Purpose |
|------|---------|
| `worker` | Implementation notes, solution description |
| `reviewer` | Code review feedback |
| `pm` | Requirements, scope, UX |
| `qa` | Testing, edge cases, regression |
| `user` | Human input (default) |
| `system` | Auto-generated (e.g., cascade close) |

## Agent Skill

tick ships with a Claude Code skill at `.claude/skills/tick-agent/`. When the skill is loaded, agents automatically know how to use all tick commands, follow the state machine, and coordinate with other agents — no `--help` exploration needed.

The skill is picked up automatically when working in a repo that has tick installed. For other repos, copy the skill directory:

```bash
cp -r .claude/skills/tick-agent /path/to/your-repo/.claude/skills/
```

## Storage

- Database: `<git-common-dir>/tick/tick.db`
- Discovered via `git rev-parse --git-common-dir`
- Shared across all git worktrees
- WAL mode for concurrent agent access
- Not tracked by git, not affected by checkout/stash/merge

## License

MIT
