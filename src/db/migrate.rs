use rusqlite::Connection;

use crate::error::{Result, TickError};

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_init", include_str!("../../migrations/001_init.sql")),
    ("002_fts", include_str!("../../migrations/002_fts.sql")),
];

pub fn expected_version() -> i64 {
    MIGRATIONS.len() as i64
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    Ok(version)
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Bootstrap: ensure schema_version table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now', 'utc'))
        );",
    )
    .map_err(TickError::Db)?;

    let current = schema_version(conn)?;
    let expected = expected_version();

    if current >= expected {
        return Ok(());
    }

    for (idx, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let migration_version = (idx + 1) as i64;
        if migration_version <= current {
            continue;
        }
        apply_migration(conn, sql, name)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            rusqlite::params![migration_version],
        )?;
    }

    Ok(())
}

fn apply_migration(conn: &Connection, sql: &str, _name: &str) -> Result<()> {
    // execute_batch handles multiple statements including triggers with semicolons in their body.
    // Migration 001 includes `CREATE TABLE schema_version` (without IF NOT EXISTS) but the
    // bootstrap already created it, so we strip that statement out.
    // We do a line-level strip: skip from `CREATE TABLE schema_version` until the closing `);`.
    let mut filtered_lines: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in sql.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("CREATE TABLE schema_version") {
            skipping = true;
        }
        if skipping {
            if trimmed.ends_with(");") {
                skipping = false;
            }
            continue;
        }
        filtered_lines.push(line);
    }
    let filtered = filtered_lines.join("\n");
    if !filtered.trim().is_empty() {
        conn.execute_batch(&filtered).map_err(TickError::Db)?;
    }
    Ok(())
}
