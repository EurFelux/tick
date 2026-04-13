use crate::db::Database;
use crate::error::{Result, TickError};

pub fn run(
    db: &Database,
    set: Option<&str>,
    get: Option<&str>,
    list: bool,
) -> Result<serde_json::Value> {
    if let Some(kv) = set {
        let (key, value) = kv.split_once('=').ok_or_else(|| {
            TickError::InvalidArgument("--set requires key=value format".to_string())
        })?;
        db.config_set(key, value)?;
        Ok(serde_json::json!({"key": key, "value": value}))
    } else if let Some(key) = get {
        let entry = db.config_get(key)?;
        Ok(serde_json::to_value(&entry).unwrap())
    } else if list {
        let entries = db.config_list()?;
        Ok(serde_json::to_value(&entries).unwrap())
    } else {
        Err(TickError::InvalidArgument(
            "provide --set, --get, or --list".to_string(),
        ))
    }
}
