use rusqlite::{params, Connection};

use crate::error::{Result, TickError};
use crate::models::{IssueStatus, IssueSummary};

pub fn create(conn: &Connection, from_id: i64, to_id: i64) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO issue_links (from_issue_id, to_issue_id, relation) VALUES (?1, ?2, 'depends-on')",
        params![from_id, to_id],
    )?;
    tx.execute(
        "INSERT INTO issue_links (from_issue_id, to_issue_id, relation) VALUES (?1, ?2, 'depended-by')",
        params![to_id, from_id],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn delete(conn: &Connection, from_id: i64, to_id: i64) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    let deleted = tx.execute(
        "DELETE FROM issue_links WHERE from_issue_id = ?1 AND to_issue_id = ?2 AND relation = 'depends-on'",
        params![from_id, to_id],
    )?;
    tx.execute(
        "DELETE FROM issue_links WHERE from_issue_id = ?1 AND to_issue_id = ?2 AND relation = 'depended-by'",
        params![to_id, from_id],
    )?;
    tx.commit()?;
    if deleted == 0 {
        return Err(TickError::NotFound(format!(
            "link from #{from_id} to #{to_id} not found"
        )));
    }
    Ok(())
}

pub fn get_depended_by_ids(conn: &Connection, issue_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT from_issue_id FROM issue_links WHERE to_issue_id = ?1 AND relation = 'depends-on'",
    )?;
    let ids = stmt
        .query_map([issue_id], |row| row.get(0))?
        .collect::<std::result::Result<Vec<i64>, _>>()?;
    Ok(ids)
}

pub fn list_by_issue(
    conn: &Connection,
    issue_id: i64,
) -> Result<(Vec<IssueSummary>, Vec<IssueSummary>)> {
    // depends_on: issues that this issue depends on (from_issue_id = issue_id, relation = 'depends-on')
    let depends_on = query_linked_summaries(conn, issue_id, "depends-on", true)?;
    // depended_by: issues that depend on this issue (to_issue_id = issue_id, relation = 'depends-on')
    let depended_by = query_linked_summaries(conn, issue_id, "depended-by", false)?;

    Ok((depends_on, depended_by))
}

fn query_linked_summaries(
    conn: &Connection,
    issue_id: i64,
    relation: &str,
    from_perspective: bool,
) -> Result<Vec<IssueSummary>> {
    let sql = if from_perspective {
        // from_issue_id = issue_id means this issue depends on to_issue_id
        "SELECT i.id, i.title, i.status FROM issue_links il \
         JOIN issues i ON i.id = il.to_issue_id \
         WHERE il.from_issue_id = ?1 AND il.relation = ?2 \
         ORDER BY i.id"
    } else {
        // to_issue_id = issue_id means from_issue_id depends on this issue
        "SELECT i.id, i.title, i.status FROM issue_links il \
         JOIN issues i ON i.id = il.from_issue_id \
         WHERE il.to_issue_id = ?1 AND il.relation = 'depends-on' \
         ORDER BY i.id"
    };

    let mut stmt = conn.prepare(sql)?;
    let params: &[&dyn rusqlite::types::ToSql] = if from_perspective {
        &[&issue_id, &relation]
    } else {
        &[&issue_id]
    };

    let rows = stmt.query_map(params, |row| {
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
