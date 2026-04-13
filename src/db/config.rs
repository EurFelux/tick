use crate::error::{Result, TickError};
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, key: &str) -> Result<ConfigEntry> {
    conn.query_row(
        "SELECT key, value FROM config WHERE key = ?1",
        [key],
        |row| {
            Ok(ConfigEntry {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            TickError::NotFound(format!("config key '{key}' not found"))
        }
        other => TickError::Db(other),
    })
}

pub fn list(conn: &Connection) -> Result<Vec<ConfigEntry>> {
    let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
    let entries = stmt
        .query_map([], |row| {
            Ok(ConfigEntry {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(entries)
}
