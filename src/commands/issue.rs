use crate::db::{Database, ListFilter};
use crate::error::{Result, TickError};
use crate::models::{
    CommentRole, Issue, IssueDetail, IssueStatus, IssueType, Priority, Resolution,
};
use crate::validators;

pub fn create(
    db: &Database,
    title: &str,
    description: Option<&str>,
    issue_type: &str,
    priority: &str,
    parent_id: Option<i64>,
) -> Result<Issue> {
    let itype = issue_type
        .parse::<IssueType>()
        .map_err(TickError::InvalidArgument)?;
    let prio = priority
        .parse::<Priority>()
        .map_err(TickError::InvalidArgument)?;

    // Validate parent exists if specified
    if let Some(pid) = parent_id {
        db.get_issue(pid)?;
    }

    let id = db.create_issue(title, description.unwrap_or(""), &itype, &prio, parent_id)?;
    db.get_issue(id)
}

#[allow(clippy::too_many_arguments)]
pub fn list(
    db: &Database,
    status: Option<&str>,
    issue_type: Option<&str>,
    priority: Option<&str>,
    parent_id: Option<i64>,
    root: bool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Issue>> {
    let status = status
        .map(|s| s.parse::<IssueStatus>().map_err(TickError::InvalidArgument))
        .transpose()?;
    let itype = issue_type
        .map(|s| s.parse::<IssueType>().map_err(TickError::InvalidArgument))
        .transpose()?;
    let prio = priority
        .map(|s| s.parse::<Priority>().map_err(TickError::InvalidArgument))
        .transpose()?;

    let filter = ListFilter {
        status,
        issue_type: itype,
        priority: prio,
        parent_id,
        root_only: root,
        limit: Some(limit),
        offset: Some(offset),
    };

    db.list_issues(&filter)
}

pub fn show(db: &Database, id: i64) -> Result<IssueDetail> {
    let issue = db.get_issue(id)?;

    let parent = match issue.parent_id {
        Some(pid) => Some(db.get_issue_summary(pid)?),
        None => None,
    };

    let children = db.get_children(id)?;
    let (depends_on, depended_by) = db.list_links(id)?;
    let comments = db.list_comments(id, None)?;

    Ok(IssueDetail {
        issue,
        parent,
        children,
        depends_on,
        depended_by,
        comments,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    db: &Database,
    id: i64,
    title: Option<&str>,
    description: Option<&str>,
    issue_type: Option<&str>,
    priority: Option<&str>,
    parent_id: Option<i64>,
    expect_version: Option<i64>,
) -> Result<Issue> {
    let itype = issue_type
        .map(|s| s.parse::<IssueType>().map_err(TickError::InvalidArgument))
        .transpose()?;
    let prio = priority
        .map(|s| s.parse::<Priority>().map_err(TickError::InvalidArgument))
        .transpose()?;

    // Validate parent cycle if parent is being changed
    if let Some(new_parent) = parent_id {
        validators::validate_parent_no_cycle(db, id, new_parent)?;
    }

    // parent_id is Option<i64> from CLI, but update_issue_fields takes Option<Option<i64>>
    // We need to wrap it: Some(None) = "clear parent", Some(Some(x)) = "set parent to x"
    // But the CLI doesn't support clearing parent, so if parent_id is provided we set it,
    // otherwise we pass None (no change).
    let parent_opt: Option<Option<i64>> = parent_id.map(Some);

    db.update_issue_fields(
        id,
        title,
        description,
        itype.as_ref(),
        prio.as_ref(),
        parent_opt,
        expect_version,
    )
}

pub fn start(db: &Database, id: i64, branch: &str, expect_version: Option<i64>) -> Result<Issue> {
    validators::validate_start(db, id, branch)?;
    db.update_issue_status_atomic(
        id,
        &IssueStatus::Open,
        &IssueStatus::InProgress,
        None,
        Some(Some(branch)),
        false,
        false,
        expect_version,
    )
}

pub fn done(db: &Database, id: i64, expect_version: Option<i64>) -> Result<Issue> {
    db.update_issue_status_atomic(
        id,
        &IssueStatus::InProgress,
        &IssueStatus::Done,
        None,
        None,
        false,
        false,
        expect_version,
    )
}

pub fn close(
    db: &Database,
    id: i64,
    comment: Option<&str>,
    role: &str,
    resolution: &str,
    expect_version: Option<i64>,
) -> Result<Issue> {
    let res = resolution
        .parse::<Resolution>()
        .map_err(TickError::InvalidArgument)?;
    let crole = role
        .parse::<CommentRole>()
        .map_err(TickError::InvalidArgument)?;

    let issue = db.get_issue(id)?;
    validators::validate_close_resolution(&issue.status, &res)?;

    let updated = db.update_issue_status_atomic(
        id,
        &issue.status,
        &IssueStatus::Closed,
        Some(Some(&res)),
        None,
        false,
        false,
        expect_version,
    )?;

    if let Some(body) = comment {
        db.create_comment(id, body, &crole)?;
    }

    if res == Resolution::Wontfix {
        validators::cascade_wontfix(db, id)?;
    }

    Ok(updated)
}

pub fn link(db: &Database, from_id: i64, relation: &str, to_id: i64) -> Result<serde_json::Value> {
    if relation != "depends-on" {
        return Err(TickError::InvalidArgument(format!(
            "unknown relation '{}', only 'depends-on' is supported",
            relation
        )));
    }
    db.get_issue(from_id)?;
    db.get_issue(to_id)?;
    validators::validate_link(db, from_id, to_id)?;
    db.create_link(from_id, to_id)?;
    Ok(serde_json::json!({"linked": true, "from": from_id, "to": to_id, "relation": "depends-on"}))
}

pub fn unlink(db: &Database, from_id: i64, to_id: i64) -> Result<serde_json::Value> {
    db.delete_link(from_id, to_id)?;
    Ok(serde_json::json!({"unlinked": true, "from": from_id, "to": to_id}))
}

pub fn reopen(db: &Database, id: i64, expect_version: Option<i64>) -> Result<Issue> {
    db.update_issue_status_atomic(
        id,
        &IssueStatus::Closed,
        &IssueStatus::Open,
        None,
        None,
        true,
        true,
        expect_version,
    )
}
