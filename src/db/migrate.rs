use rusqlite::Connection;

use crate::error::{Result, TickError};

const MIGRATION_001: &str = include_str!("../../migrations/001_init.sql");

pub fn expected_version() -> i64 {
    1
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
    .map_err(|e| TickError::Db(e))?;

    let current = schema_version(conn)?;
    let expected = expected_version();

    if current >= expected {
        return Ok(());
    }

    // Apply migration 001
    if current < 1 {
        // We need to run 001, but schema_version might already have been created
        // So we run only the parts of 001 that don't include schema_version creation
        // Actually, run the full migration but skip schema_version creation since it exists
        apply_migration_001(conn)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            rusqlite::params![1i64],
        )?;
    }

    Ok(())
}

fn apply_migration_001(conn: &Connection) -> Result<()> {
    // Parse out and skip the schema_version CREATE TABLE since we already created it
    // Run each statement individually, skipping the schema_version one
    let statements: Vec<&str> = MIGRATION_001.split(';').collect();
    for stmt in statements {
        let trimmed = stmt.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Skip schema_version table creation since we already created it
        if trimmed.contains("CREATE TABLE schema_version") {
            continue;
        }
        conn.execute_batch(&format!("{};", trimmed))
            .map_err(|e| TickError::Db(e))?;
    }
    Ok(())
}
