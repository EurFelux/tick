use rusqlite::Connection;

use crate::error::{Result, TickError};
use crate::models::{Comment, CommentRole};

pub fn create(conn: &Connection, issue_id: i64, body: &str, role: &CommentRole) -> Result<i64> {
    conn.execute(
        "INSERT INTO comments (issue_id, body, role) VALUES (?1, ?2, ?3)",
        rusqlite::params![issue_id, body, role.to_string()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_by_issue(
    conn: &Connection,
    issue_id: i64,
    role: Option<&CommentRole>,
) -> Result<Vec<Comment>> {
    let sql = if role.is_some() {
        "SELECT id, issue_id, body, role, created_at FROM comments WHERE issue_id = ?1 AND role = ?2 ORDER BY id"
    } else {
        "SELECT id, issue_id, body, role, created_at FROM comments WHERE issue_id = ?1 ORDER BY id"
    };

    let mut stmt = conn.prepare(sql)?;

    let map_row = |row: &rusqlite::Row| {
        let role_str: String = row.get(3)?;
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            role_str,
            row.get::<_, String>(4)?,
        ))
    };

    let rows: Vec<_> = if let Some(r) = role {
        stmt.query_map(rusqlite::params![issue_id, r.to_string()], map_row)?
            .collect()
    } else {
        stmt.query_map(rusqlite::params![issue_id], map_row)?
            .collect()
    };

    let mut comments = Vec::new();
    for row in rows {
        let (id, issue_id, body, role_str, created_at) = row?;
        let role = role_str
            .parse::<CommentRole>()
            .map_err(TickError::InvalidArgument)?;
        comments.push(Comment {
            id,
            issue_id,
            body,
            role,
            created_at,
        });
    }
    Ok(comments)
}
