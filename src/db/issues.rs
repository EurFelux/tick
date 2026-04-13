use rusqlite::{Connection, Row};

use crate::error::{Result, TickError};
use crate::models::{Issue, IssueStatus, IssueSummary, IssueType, Priority, Resolution};

#[derive(Debug, Default)]
pub struct ListFilter {
    pub status: Option<IssueStatus>,
    pub issue_type: Option<IssueType>,
    pub priority: Option<Priority>,
    pub parent_id: Option<i64>,
    pub root_only: bool,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn row_to_issue(row: &Row) -> rusqlite::Result<Issue> {
    let status_str: String = row.get(5)?;
    let type_str: String = row.get(4)?;
    let priority_str: String = row.get(6)?;
    let resolution_str: Option<String> = row.get(7)?;

    let status = status_str
        .parse::<IssueStatus>()
        .map_err(rusqlite::Error::InvalidColumnName)?;
    let issue_type = type_str
        .parse::<IssueType>()
        .map_err(rusqlite::Error::InvalidColumnName)?;
    let priority = priority_str
        .parse::<Priority>()
        .map_err(rusqlite::Error::InvalidColumnName)?;
    let resolution = resolution_str
        .map(|s| {
            s.parse::<Resolution>()
                .map_err(rusqlite::Error::InvalidColumnName)
        })
        .transpose()?;

    Ok(Issue {
        id: row.get(0)?,
        parent_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        issue_type,
        status,
        priority,
        resolution,
        branch: row.get(8)?,
        version: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

pub fn create(
    conn: &Connection,
    title: &str,
    description: &str,
    issue_type: &IssueType,
    priority: &Priority,
    parent_id: Option<i64>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO issues (title, description, type, priority, parent_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            title,
            description,
            issue_type.to_string(),
            priority.to_string(),
            parent_id,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Issue> {
    let result = conn.query_row(
        "SELECT id, parent_id, title, description, type, status, priority, resolution, branch, version, created_at, updated_at FROM issues WHERE id = ?1",
        rusqlite::params![id],
        row_to_issue,
    );
    match result {
        Ok(issue) => Ok(issue),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(TickError::NotFound(format!("issue {} not found", id)))
        }
        Err(e) => Err(TickError::Db(e)),
    }
}

pub fn get_summary(conn: &Connection, id: i64) -> Result<IssueSummary> {
    let result = conn.query_row(
        "SELECT id, title, status FROM issues WHERE id = ?1",
        rusqlite::params![id],
        |row| {
            let status_str: String = row.get(2)?;
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, status_str))
        },
    );
    match result {
        Ok((id, title, status_str)) => {
            let status = status_str
                .parse::<IssueStatus>()
                .map_err(TickError::InvalidArgument)?;
            Ok(IssueSummary { id, title, status })
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Err(TickError::NotFound(format!("issue {} not found", id)))
        }
        Err(e) => Err(TickError::Db(e)),
    }
}

pub fn get_children(conn: &Connection, parent_id: i64) -> Result<Vec<IssueSummary>> {
    let mut stmt =
        conn.prepare("SELECT id, title, status FROM issues WHERE parent_id = ?1 ORDER BY id")?;
    let rows = stmt.query_map(rusqlite::params![parent_id], |row| {
        let status_str: String = row.get(2)?;
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, status_str))
    })?;

    let mut summaries = Vec::new();
    for row in rows {
        let (id, title, status_str) = row?;
        let status = status_str
            .parse::<IssueStatus>()
            .map_err(TickError::InvalidArgument)?;
        summaries.push(IssueSummary { id, title, status });
    }
    Ok(summaries)
}

pub fn list(conn: &Connection, filter: &ListFilter) -> Result<Vec<Issue>> {
    let mut conditions: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1usize;

    if let Some(ref status) = filter.status {
        conditions.push(format!("status = ?{}", param_idx));
        param_values.push(Box::new(status.to_string()));
        param_idx += 1;
    }

    if let Some(ref issue_type) = filter.issue_type {
        conditions.push(format!("type = ?{}", param_idx));
        param_values.push(Box::new(issue_type.to_string()));
        param_idx += 1;
    }

    if let Some(ref priority) = filter.priority {
        conditions.push(format!("priority = ?{}", param_idx));
        param_values.push(Box::new(priority.to_string()));
        param_idx += 1;
    }

    if filter.root_only {
        conditions.push("parent_id IS NULL".to_string());
    } else if let Some(pid) = filter.parent_id {
        conditions.push(format!("parent_id = ?{}", param_idx));
        param_values.push(Box::new(pid));
        param_idx += 1;
    }

    let limit = filter.limit.unwrap_or(50);
    let offset = filter.offset.unwrap_or(0);

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, parent_id, title, description, type, status, priority, resolution, branch, version, created_at, updated_at FROM issues {} ORDER BY id LIMIT ?{} OFFSET ?{}",
        where_clause, param_idx, param_idx + 1
    );

    param_values.push(Box::new(limit));
    param_values.push(Box::new(offset));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|v| v.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), row_to_issue)?;

    let mut issues = Vec::new();
    for row in rows {
        issues.push(row?);
    }
    Ok(issues)
}

pub fn update_fields(
    conn: &Connection,
    id: i64,
    title: Option<&str>,
    description: Option<&str>,
    issue_type: Option<&IssueType>,
    priority: Option<&Priority>,
    parent_id: Option<Option<i64>>,
) -> Result<Issue> {
    let mut sets: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1usize;

    if let Some(t) = title {
        sets.push(format!("title = ?{}", param_idx));
        param_values.push(Box::new(t.to_string()));
        param_idx += 1;
    }

    if let Some(d) = description {
        sets.push(format!("description = ?{}", param_idx));
        param_values.push(Box::new(d.to_string()));
        param_idx += 1;
    }

    if let Some(it) = issue_type {
        sets.push(format!("type = ?{}", param_idx));
        param_values.push(Box::new(it.to_string()));
        param_idx += 1;
    }

    if let Some(p) = priority {
        sets.push(format!("priority = ?{}", param_idx));
        param_values.push(Box::new(p.to_string()));
        param_idx += 1;
    }

    if let Some(pid) = parent_id {
        sets.push(format!("parent_id = ?{}", param_idx));
        match pid {
            Some(v) => param_values.push(Box::new(v)),
            None => param_values.push(Box::new(rusqlite::types::Null)),
        }
        param_idx += 1;
    }

    if sets.is_empty() {
        return get(conn, id);
    }

    sets.push("version = version + 1".to_string());
    sets.push("updated_at = datetime('now', 'utc')".to_string());

    let sql = format!(
        "UPDATE issues SET {} WHERE id = ?{}",
        sets.join(", "),
        param_idx
    );

    param_values.push(Box::new(id));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|v| v.as_ref()).collect();
    let affected = conn.execute(&sql, params_refs.as_slice())?;

    if affected == 0 {
        return Err(TickError::NotFound(format!("issue {} not found", id)));
    }

    get(conn, id)
}

#[allow(clippy::too_many_arguments)]
pub fn update_status_atomic(
    conn: &Connection,
    id: i64,
    expected_status: &IssueStatus,
    new_status: &IssueStatus,
    resolution: Option<Option<&Resolution>>,
    branch: Option<Option<&str>>,
    clear_branch: bool,
    clear_resolution: bool,
) -> Result<Issue> {
    let mut sets: Vec<String> = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1usize;

    sets.push(format!("status = ?{}", param_idx));
    param_values.push(Box::new(new_status.to_string()));
    param_idx += 1;

    if clear_resolution {
        sets.push("resolution = NULL".to_string());
    } else if let Some(res_opt) = resolution {
        sets.push(format!("resolution = ?{}", param_idx));
        match res_opt {
            Some(r) => param_values.push(Box::new(r.to_string())),
            None => param_values.push(Box::new(rusqlite::types::Null)),
        }
        param_idx += 1;
    }

    if clear_branch {
        sets.push("branch = NULL".to_string());
    } else if let Some(branch_opt) = branch {
        sets.push(format!("branch = ?{}", param_idx));
        match branch_opt {
            Some(b) => param_values.push(Box::new(b.to_string())),
            None => param_values.push(Box::new(rusqlite::types::Null)),
        }
        param_idx += 1;
    }

    sets.push("version = version + 1".to_string());
    sets.push("updated_at = datetime('now', 'utc')".to_string());

    let sql = format!(
        "UPDATE issues SET {} WHERE id = ?{} AND status = ?{}",
        sets.join(", "),
        param_idx,
        param_idx + 1
    );

    param_values.push(Box::new(id));
    param_values.push(Box::new(expected_status.to_string()));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|v| v.as_ref()).collect();
    let affected = conn.execute(&sql, params_refs.as_slice())?;

    if affected == 0 {
        // Check if issue exists
        match get(conn, id) {
            Ok(_) => Err(TickError::Conflict(format!(
                "issue {} status conflict: expected {}",
                id, expected_status
            ))),
            Err(TickError::NotFound(_)) => {
                Err(TickError::NotFound(format!("issue {} not found", id)))
            }
            Err(e) => Err(e),
        }
    } else {
        get(conn, id)
    }
}

pub fn count_by_status(conn: &Connection) -> Result<std::collections::HashMap<String, i64>> {
    let mut stmt = conn.prepare("SELECT status, COUNT(*) FROM issues GROUP BY status")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    // Initialize all four statuses at 0 so zero-count statuses are always present
    let mut map = std::collections::HashMap::new();
    map.insert("open".to_string(), 0i64);
    map.insert("in-progress".to_string(), 0i64);
    map.insert("done".to_string(), 0i64);
    map.insert("closed".to_string(), 0i64);

    for row in rows {
        let (status, count) = row?;
        map.insert(status, count);
    }
    Ok(map)
}
