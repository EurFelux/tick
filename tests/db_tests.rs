use tempfile::TempDir;
use tick::db::{Database, ListFilter};
use tick::models::{CommentRole, IssueStatus, IssueType, Priority};

fn setup_db() -> (TempDir, Database) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let db_path = dir.path().join("test.db");
    let mut db = Database::open(&db_path).expect("failed to open database");
    db.migrate().expect("failed to migrate");
    (dir, db)
}

#[test]
fn test_open_and_migrate() {
    let (_dir, db) = setup_db();
    let version = db.schema_version().expect("failed to get schema version");
    assert_eq!(version, 2);
}

#[test]
fn test_migrate_is_idempotent() {
    let dir = TempDir::new().expect("failed to create temp dir");
    let db_path = dir.path().join("test.db");
    let mut db = Database::open(&db_path).expect("failed to open database");
    db.migrate().expect("failed to migrate first time");
    db.migrate().expect("failed to migrate second time");
    let version = db.schema_version().expect("failed to get schema version");
    assert_eq!(version, 2);
}

#[test]
fn test_create_and_get_issue() {
    let (_dir, db) = setup_db();
    let id = db
        .create_issue(
            "Test Issue",
            "A description",
            &IssueType::Bug,
            &Priority::High,
            None,
        )
        .expect("failed to create issue");

    let issue = db.get_issue(id).expect("failed to get issue");
    assert_eq!(issue.id, id);
    assert_eq!(issue.title, "Test Issue");
    assert_eq!(issue.description, "A description");
    assert_eq!(issue.issue_type, IssueType::Bug);
    assert_eq!(issue.priority, Priority::High);
    assert_eq!(issue.status, IssueStatus::Open);
    assert_eq!(issue.parent_id, None);
    assert_eq!(issue.resolution, None);
    assert_eq!(issue.branch, None);
    assert_eq!(issue.version, 1);
    assert!(!issue.created_at.is_empty());
    assert!(!issue.updated_at.is_empty());
}

#[test]
fn test_get_nonexistent_issue() {
    let (_dir, db) = setup_db();
    let result = db.get_issue(9999);
    assert!(result.is_err());
    match result.unwrap_err() {
        tick::error::TickError::NotFound(_) => {}
        e => panic!("expected NotFound, got {:?}", e),
    }
}

#[test]
fn test_list_issues_with_filters() {
    let (_dir, db) = setup_db();
    db.create_issue("Bug 1", "", &IssueType::Bug, &Priority::High, None)
        .expect("create bug 1");
    db.create_issue(
        "Feature 1",
        "",
        &IssueType::Feature,
        &Priority::Medium,
        None,
    )
    .expect("create feature 1");
    db.create_issue("Bug 2", "", &IssueType::Bug, &Priority::Low, None)
        .expect("create bug 2");

    let filter = ListFilter {
        issue_type: Some(IssueType::Bug),
        ..Default::default()
    };
    let issues = db.list_issues(&filter).expect("failed to list issues");
    assert_eq!(issues.len(), 2);
    for issue in &issues {
        assert_eq!(issue.issue_type, IssueType::Bug);
    }
}

#[test]
fn test_list_issues_limit_offset() {
    let (_dir, db) = setup_db();
    for i in 0..5 {
        db.create_issue(
            &format!("Issue {}", i),
            "",
            &IssueType::Feature,
            &Priority::Medium,
            None,
        )
        .expect("create issue");
    }

    let filter_page1 = ListFilter {
        limit: Some(2),
        offset: Some(0),
        ..Default::default()
    };
    let page1 = db.list_issues(&filter_page1).expect("list page 1");
    assert_eq!(page1.len(), 2);

    let filter_page2 = ListFilter {
        limit: Some(2),
        offset: Some(2),
        ..Default::default()
    };
    let page2 = db.list_issues(&filter_page2).expect("list page 2");
    assert_eq!(page2.len(), 2);

    // Ensure different issues
    assert_ne!(page1[0].id, page2[0].id);
}

#[test]
fn test_create_sub_issue() {
    let (_dir, db) = setup_db();
    let parent_id = db
        .create_issue(
            "Parent Issue",
            "",
            &IssueType::Feature,
            &Priority::Medium,
            None,
        )
        .expect("create parent");
    let child_id = db
        .create_issue(
            "Child Issue",
            "",
            &IssueType::Bug,
            &Priority::Low,
            Some(parent_id),
        )
        .expect("create child");

    let child = db.get_issue(child_id).expect("get child");
    assert_eq!(child.parent_id, Some(parent_id));

    let children = db.get_children(parent_id).expect("get children");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, child_id);
}

#[test]
fn test_count_by_status() {
    let (_dir, db) = setup_db();
    db.create_issue("Issue 1", "", &IssueType::Feature, &Priority::Medium, None)
        .expect("create issue 1");
    db.create_issue("Issue 2", "", &IssueType::Bug, &Priority::High, None)
        .expect("create issue 2");

    let counts = db.count_by_status().expect("count by status");
    assert_eq!(*counts.get("open").unwrap_or(&0), 2);
}

#[test]
fn test_create_and_list_comments() {
    let (_dir, db) = setup_db();
    let issue_id = db
        .create_issue(
            "Issue with comments",
            "",
            &IssueType::Feature,
            &Priority::Medium,
            None,
        )
        .expect("create issue");

    let comment1_id = db
        .create_comment(issue_id, "First comment", &CommentRole::User)
        .expect("create comment 1");
    let comment2_id = db
        .create_comment(issue_id, "Second comment by worker", &CommentRole::Worker)
        .expect("create comment 2");

    let comments = db.list_comments(issue_id, None).expect("list comments");
    assert_eq!(comments.len(), 2);

    assert_eq!(comments[0].id, comment1_id);
    assert_eq!(comments[0].issue_id, issue_id);
    assert_eq!(comments[0].body, "First comment");
    assert_eq!(comments[0].role, CommentRole::User);

    assert_eq!(comments[1].id, comment2_id);
    assert_eq!(comments[1].body, "Second comment by worker");
    assert_eq!(comments[1].role, CommentRole::Worker);
}

#[test]
fn test_list_links_empty() {
    let (_dir, db) = setup_db();
    let issue_id = db
        .create_issue(
            "Isolated Issue",
            "",
            &IssueType::Feature,
            &Priority::Medium,
            None,
        )
        .expect("create issue");

    let (depends_on, depended_by) = db.list_links(issue_id).expect("list links");
    assert!(depends_on.is_empty());
    assert!(depended_by.is_empty());
}
