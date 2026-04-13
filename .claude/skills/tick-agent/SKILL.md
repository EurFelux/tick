---
name: tick-agent
description: |
  Use tick to manage issues in the current git repository. tick is a local Agent-First issue tracker — a serverless CLI that stores data in SQLite inside the git directory. Use this skill whenever you need to: create/query/update issues, track work progress, manage dependencies between tasks, start or complete work on an issue, or coordinate with other agents via structured issue data. Also use when the user mentions "tick", "issues", "tasks to do", "what needs to be done", "create a bug", "mark as done", or anything related to issue/task tracking in the current project. tick must be initialized (`tick init`) before use — if you get a NOT_INITIALIZED error, run `tick init` first.
---

# tick — Local Agent-First Issue Tracker

tick is a CLI issue tracker that lives inside the git repo. No server, no daemon — just a binary that reads/writes SQLite. All output is JSON by default, making it ideal for agent consumption.

## Before You Start

tick requires a git repository. If not initialized yet:

```bash
tick init
```

This creates the database at `<git-common-dir>/tick/tick.db`, shared across all worktrees.

## Core Workflow

The typical lifecycle of an issue:

```bash
# 1. Create
tick issue create "Fix login bug" -t bug -p high

# 2. Start working (sets branch, transitions to in-progress)
tick issue start 1 --branch fix/login

# 3. Add notes about your solution
tick comment add 1 "Fixed SQL injection in auth.rs:42" --role worker

# 4. Mark as done (ready for review)
tick issue done 1

# 5. Review and close
tick comment add 1 "LGTM" --role reviewer
tick issue close 1
```

## State Machine

Issues follow a strict lifecycle. Use semantic commands — not `update --status`:

```
open → in-progress → done → closed(resolved|wontfix)
```

| Command | Transition | When to use |
|---------|-----------|-------------|
| `tick issue start <id> --branch B` | open → in-progress | You're beginning work. `--branch` is required. |
| `tick issue done <id>` | in-progress → done | Work is complete, ready for review. |
| `tick issue close <id>` | done → closed(resolved) | Review passed. |
| `tick issue close <id> --resolution wontfix` | any → closed(wontfix) | Abandoning this issue. Cascades to dependents. |
| `tick issue reopen <id>` | closed → open | Reopen a closed issue. Clears branch and resolution. |

`tick issue update` only changes non-status fields (title, description, type, priority, parent).

## Command Reference

### Issue Commands

**Create:**
```bash
tick issue create <title> [-d <description>] [-t <type>] [-p <priority>] [--parent <id>]
# type: bug, feature, refactor, docs, test, chore (default: feature)
# priority: low, medium, high, critical (default: medium)
```

**List:**
```bash
tick issue list [--status <S>] [--type <T>] [--priority <P>] [--parent <id>] [--root] [--limit N] [--offset O]
# Default limit: 50. Returns JSON array.
```

**Show:**
```bash
tick issue show <id>
# Returns full detail: parent, children, depends_on, depended_by, comments
```

**Update (non-status fields only):**
```bash
tick issue update <id> [--title <T>] [--desc <D>] [--type <T>] [--priority <P>] [--parent <id>]
```

**Search:**
```bash
tick issue search <query> [--limit N] [--offset O]
# Full-text search on title + description
```

**Dependencies:**
```bash
tick issue link <from-id> depends-on <to-id>    # Create dependency
tick issue unlink <from-id> <to-id>              # Remove dependency
# from-id cannot start until to-id is closed(resolved)
# If to-id is wontfix'd, from-id is automatically wontfix'd too (cascade)
```

**Batch Create:**
```bash
tick issue batch-create < issues.jsonl
# Each line: {"title": "...", "type": "bug", "priority": "high", "description": "...", "parent": 1}
# Only title is required. Returns JSON array of results.
```

### Comment Commands

```bash
tick comment add <issue-id> <body> [--role <R>]
tick comment list <issue-id> [--role <R>]
# Roles: worker, reviewer, pm, qa, user (default: user)
```

Use roles to indicate your perspective:
- `worker`: describing your implementation/solution
- `reviewer`: code review feedback
- `pm`: requirements/scope assessment
- `qa`: testing/edge case concerns
- `user`: general notes (default)

### Status & Config

```bash
tick status                    # Issue counts by status
tick version                   # Version + schema version
tick config --set key=value    # Set project config
tick config --get key          # Get config value
tick config --list             # List all config
```

### Global Flags

| Flag | Effect |
|------|--------|
| `--pretty` | Human-readable output instead of JSON |
| `--fields id,title,status` | Only include these keys in JSON output |
| `--quiet` | Output only the resource ID |
| `--dry-run` | Run validators without writing to DB |
| `--db <path>` | Override database path |
| `--expect-version <V>` | Optimistic locking (on write commands) |

`--quiet` conflicts with `--fields` and `--pretty`.

### Exit Codes

| Code | Meaning | JSON code |
|------|---------|-----------|
| 0 | Success | — |
| 1 | Internal error | `INTERNAL_ERROR` |
| 2 | Not found | `NOT_FOUND` |
| 3 | Invalid argument | `INVALID_ARGUMENT` |
| 4 | Database error | `DB_ERROR` |
| 5 | Not initialized | `NOT_INITIALIZED` |
| 6 | Conflict (concurrent edit) | `CONFLICT` |

Errors are JSON too: `{"error": "...", "code": "NOT_FOUND"}`

## Agent Workflow Patterns

### Starting a session

Before working on issues, check what's available:

```bash
tick issue list --status open          # What needs doing?
tick issue list --status in-progress   # What's already being worked on?
tick status                            # Overview
```

### Picking up work

```bash
tick issue show <id>                   # Read the details + comments
tick issue start <id> --branch <name>  # Claim it (atomic — if another agent already started it, you'll get CONFLICT)
```

If you get exit code 6 (CONFLICT), the issue is already being worked on. Pick a different one.

### Completing work

```bash
tick comment add <id> "Describe what you did and why" --role worker
tick issue done <id>
```

### Reviewing work

```bash
tick issue list --status done          # Find issues awaiting review
tick issue show <id>                   # Read worker's comments + check branch
tick comment add <id> "Review feedback" --role reviewer
tick issue close <id>                  # Or: tick issue close <id> --resolution wontfix
```

### Handling dependencies

```bash
# Before starting, check if dependencies are resolved
tick issue show <id>                   # Look at depends_on array
# If depends_on issues aren't closed(resolved), tick issue start will fail
```

### Concurrent safety

Multiple agents may be working in the same repo. The `start` command is atomic — only one agent can start a given issue. If you need stronger guarantees on other operations, use `--expect-version`:

```bash
tick issue show 1                      # Note the "version" field (e.g., 3)
tick issue update 1 --title "New" --expect-version 3  # Fails if version changed
```
