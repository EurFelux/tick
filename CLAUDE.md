# tick

Local Agent-First Issue Tracker. See PLAN.md for full design and docs/superpowers/specs/ for Phase 1 spec.

## Build & Test

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`

## Conventions

- Default output is JSON. `--pretty` for human-readable.
- All state transitions use semantic subcommands: `start`, `done`, `close`, `reopen`.
- `update` only modifies non-status fields.
- Validators are in `src/validators.rs` — never inline validation in commands.
- DB path: `<git-common-dir>/tick/tick.db`. Use `git rev-parse --git-common-dir` to find it.
- Every UPDATE must include `version = version + 1, updated_at = datetime('now', 'utc')`.
