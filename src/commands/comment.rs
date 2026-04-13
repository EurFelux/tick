use crate::db::Database;
use crate::error::{Result, TickError};
use crate::models::{Comment, CommentRole};

pub fn add(db: &Database, issue_id: i64, body: &str, role: &str) -> Result<Comment> {
    let r = role.parse::<CommentRole>().map_err(TickError::InvalidArgument)?;
    db.get_issue(issue_id)?; // verify exists
    let id = db.create_comment(issue_id, body, &r)?;
    let comments = db.list_comments(issue_id, None)?;
    comments.into_iter().find(|c| c.id == id).ok_or_else(|| {
        TickError::Internal(anyhow::anyhow!("comment not found after creation"))
    })
}

pub fn list(db: &Database, issue_id: i64, role: Option<&str>) -> Result<Vec<Comment>> {
    db.get_issue(issue_id)?;
    let r = role
        .map(|s| s.parse::<CommentRole>().map_err(TickError::InvalidArgument))
        .transpose()?;
    db.list_comments(issue_id, r.as_ref())
}
