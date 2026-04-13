use std::path::Path;

use crate::db::migrate;
use crate::db::Database;
use crate::error::{Result, TickError};

/// Resolve the path to the tick database by asking git for the common git dir.
pub fn resolve_db_path() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .map_err(|e| TickError::Internal(anyhow::anyhow!("failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(TickError::NotInitialized(
            "not inside a git repository; cannot locate tick database".to_string(),
        ));
    }

    let git_common_dir = String::from_utf8(output.stdout)
        .map_err(|e| TickError::Internal(anyhow::anyhow!("git output not utf-8: {}", e)))?;
    let git_common_dir = git_common_dir.trim();

    Ok(format!("{}/tick/tick.db", git_common_dir))
}

/// Initialize tick: create the database directory, open and migrate the DB.
pub fn run(db_path: Option<&str>) -> Result<Database> {
    let path = match db_path {
        Some(p) => p.to_string(),
        None => resolve_db_path()?,
    };

    // Create parent directory if needed
    if let Some(parent) = Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            TickError::Internal(anyhow::anyhow!("failed to create directory: {}", e))
        })?;
    }

    let mut db = Database::open(&path)?;
    db.migrate()?;
    Ok(db)
}

/// Open an existing tick database, verifying it exists and has the right schema version.
pub fn open_db(db_path: Option<&str>) -> Result<Database> {
    let path = match db_path {
        Some(p) => p.to_string(),
        None => resolve_db_path()?,
    };

    if !Path::new(&path).exists() {
        return Err(TickError::NotInitialized(format!(
            "tick database not found at '{}'; run 'tick init' first",
            path
        )));
    }

    let db = Database::open(&path)?;
    let version = db.schema_version()?;
    let expected = migrate::expected_version();

    if version != expected {
        return Err(TickError::NotInitialized(format!(
            "tick database schema version {} does not match expected {}; please reinitialize",
            version, expected
        )));
    }

    Ok(db)
}
