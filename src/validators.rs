use std::collections::VecDeque;

use crate::db::Database;
use crate::error::{Result, TickError};
use crate::models::{CommentRole, IssueStatus, Resolution};

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

/// Validate that a dependency link can be created from from_id to to_id:
/// - no self-reference
/// - no cycle (BFS from to_id following depends-on, reject if from_id is reached)
/// - to_id must not already be closed (wontfix)
pub fn validate_link(db: &Database, from_id: i64, to_id: i64) -> Result<()> {
    if from_id == to_id {
        return Err(TickError::InvalidArgument(format!(
            "issue #{from_id} cannot depend on itself"
        )));
    }

    check_dependency_cycle(db, from_id, to_id)?;

    // If from issue is non-open, the to issue must be closed(resolved)
    let from_issue = db.get_issue(from_id)?;
    if from_issue.status != IssueStatus::Open {
        let to_issue = db.get_issue(to_id)?;
        if to_issue.status != IssueStatus::Closed
            || to_issue.resolution != Some(Resolution::Resolved)
        {
            return Err(TickError::InvalidArgument(format!(
                "issue #{from_id} is '{}': dependency #{to_id} must be closed(resolved)",
                from_issue.status
            )));
        }
    }

    Ok(())
}

/// BFS from to_id following depends-on links; if from_id is reached, a cycle would be created.
pub fn check_dependency_cycle(db: &Database, from_id: i64, to_id: i64) -> Result<()> {
    let mut visited = std::collections::HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(to_id);

    while let Some(current_id) = queue.pop_front() {
        if current_id == from_id {
            return Err(TickError::InvalidArgument(format!(
                "linking #{from_id} → #{to_id} would create a dependency cycle"
            )));
        }
        if visited.contains(&current_id) {
            continue;
        }
        visited.insert(current_id);

        let (depends_on, _) = db.list_links(current_id)?;
        for dep in depends_on {
            queue.push_back(dep.id);
        }
    }

    Ok(())
}

/// Recursively close all issues that depend on `wontfixed_issue_id` with wontfix,
/// adding a system comment explaining the cascade.
pub fn cascade_wontfix(db: &Database, wontfixed_issue_id: i64) -> Result<()> {
    let depended_by_ids = db.get_depended_by_ids(wontfixed_issue_id)?;

    for dep_id in depended_by_ids {
        let dep = db.get_issue(dep_id)?;
        if dep.status == IssueStatus::Closed {
            // Already closed, skip
            continue;
        }

        // Force close with wontfix (works from any non-closed status)
        db.update_issue_status_atomic(
            dep_id,
            &dep.status,
            &IssueStatus::Closed,
            Some(Some(&Resolution::Wontfix)),
            None,
            false,
            false,
            None,
        )?;

        db.create_comment(
            dep_id,
            &format!("Closed by cascade: dependency #{wontfixed_issue_id} was abandoned"),
            &CommentRole::System,
        )?;

        // Recurse
        cascade_wontfix(db, dep_id)?;
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
