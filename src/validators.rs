use crate::db::Database;
use crate::error::{Result, TickError};
use crate::models::{IssueStatus, Resolution};

/// Validate that an issue can be started:
/// - branch must be non-empty
/// - all "depends-on" issues must be closed (resolved)
pub fn validate_start(db: &Database, issue_id: i64, branch: &str) -> Result<()> {
    if branch.trim().is_empty() {
        return Err(TickError::InvalidArgument(
            "branch name must not be empty".to_string(),
        ));
    }

    let (depends_on, _) = db.list_links(issue_id)?;
    for dep in &depends_on {
        if dep.status != IssueStatus::Closed {
            return Err(TickError::InvalidArgument(format!(
                "cannot start: dependency issue {} ('{}') is not closed",
                dep.id, dep.title
            )));
        }
        // Closed with wontfix does not count as resolved
        let full = db.get_issue(dep.id)?;
        if full.resolution != Some(Resolution::Resolved) {
            return Err(TickError::InvalidArgument(format!(
                "cannot start: dependency issue {} ('{}') is closed but not resolved",
                dep.id, dep.title
            )));
        }
    }

    Ok(())
}

/// Validate that a resolution is valid for the given status transition to "closed":
/// - open/in-progress → closed: only wontfix allowed
/// - done → closed: resolved or wontfix allowed
pub fn validate_close_resolution(status: &IssueStatus, resolution: &Resolution) -> Result<()> {
    match status {
        IssueStatus::Open | IssueStatus::InProgress => {
            if *resolution != Resolution::Wontfix {
                return Err(TickError::InvalidArgument(format!(
                    "issues with status '{}' can only be closed with resolution 'wontfix'",
                    status
                )));
            }
        }
        IssueStatus::Done => {
            // Both resolved and wontfix are allowed
        }
        IssueStatus::Closed => {
            return Err(TickError::Conflict("issue is already closed".to_string()));
        }
    }
    Ok(())
}

/// Validate that setting `new_parent_id` as parent of `issue_id` does not create a cycle.
/// Walks the ancestor chain from `new_parent_id` upward; if `issue_id` is found, it's a cycle.
pub fn validate_parent_no_cycle(db: &Database, issue_id: i64, new_parent_id: i64) -> Result<()> {
    if issue_id == new_parent_id {
        return Err(TickError::InvalidArgument(format!(
            "issue {} cannot be its own parent",
            issue_id
        )));
    }

    let mut current_id = new_parent_id;
    // Walk up ancestor chain
    loop {
        let issue = db.get_issue(current_id)?;
        match issue.parent_id {
            None => break,
            Some(pid) => {
                if pid == issue_id {
                    return Err(TickError::InvalidArgument(format!(
                        "setting parent of issue {} to {} would create a cycle",
                        issue_id, new_parent_id
                    )));
                }
                current_id = pid;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::NamedTempFile;

    fn make_db() -> (Database, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let mut db = Database::open(file.path()).unwrap();
        db.migrate().unwrap();
        (db, file)
    }

    #[test]
    fn test_validate_start_empty_branch() {
        let (db, _file) = make_db();
        let id = db
            .create_issue(
                "test",
                "",
                &crate::models::IssueType::Feature,
                &crate::models::Priority::Medium,
                None,
            )
            .unwrap();
        let result = validate_start(&db, id, "");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TickError::InvalidArgument(_)));
    }

    #[test]
    fn test_validate_close_resolution_open_resolved_rejected() {
        let result = validate_close_resolution(&IssueStatus::Open, &Resolution::Resolved);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TickError::InvalidArgument(_)));
    }

    #[test]
    fn test_validate_close_resolution_open_wontfix_allowed() {
        let result = validate_close_resolution(&IssueStatus::Open, &Resolution::Wontfix);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_close_resolution_done_resolved_allowed() {
        let result = validate_close_resolution(&IssueStatus::Done, &Resolution::Resolved);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_parent_self_reference() {
        let (db, _file) = make_db();
        let id = db
            .create_issue(
                "self",
                "",
                &crate::models::IssueType::Feature,
                &crate::models::Priority::Medium,
                None,
            )
            .unwrap();
        let result = validate_parent_no_cycle(&db, id, id);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TickError::InvalidArgument(_)));
    }

    #[test]
    fn test_validate_parent_cycle() {
        // Create A and B, set B.parent = A, then try to set A.parent = B (should fail)
        let (db, _file) = make_db();
        let a = db
            .create_issue(
                "A",
                "",
                &crate::models::IssueType::Feature,
                &crate::models::Priority::Medium,
                None,
            )
            .unwrap();
        let b = db
            .create_issue(
                "B",
                "",
                &crate::models::IssueType::Feature,
                &crate::models::Priority::Medium,
                Some(a),
            )
            .unwrap();
        // Now try to set A's parent to B — should detect cycle
        let result = validate_parent_no_cycle(&db, a, b);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TickError::InvalidArgument(_)));
    }
}
