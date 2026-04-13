use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn tick() -> Command {
    Command::cargo_bin("tick").unwrap()
}

fn setup() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
    tick().args(["--db", &db_path, "init"]).assert().success();
    (dir, db_path)
}

#[test]
fn test_version() {
    tick()
        .args(["version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("version"));
}

#[test]
fn test_init_creates_db() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_path_str = db_path.to_str().unwrap();

    tick()
        .args(["--db", db_path_str, "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized"));

    assert!(db_path.exists(), "db file should be created after init");
}

#[test]
fn test_create_issue() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Fix the thing",
            "--type",
            "bug",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\":1"))
        .stdout(predicate::str::contains("\"status\":\"open\""))
        .stdout(predicate::str::contains("\"type\":\"bug\""));
}

#[test]
fn test_list_issues() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "Bug one", "--type", "bug",
        ])
        .assert()
        .success();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Feature two",
            "--type",
            "feature",
        ])
        .assert()
        .success();

    // List all — should have both
    tick()
        .args(["--db", &db_path, "issue", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bug one"))
        .stdout(predicate::str::contains("Feature two"));

    // Filter by type bug — should only have Bug one
    let output = tick()
        .args(["--db", &db_path, "issue", "list", "--type", "bug"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("Bug one"),
        "list --type bug should contain Bug one"
    );
    assert!(
        !stdout.contains("Feature two"),
        "list --type bug should NOT contain Feature two"
    );
}

#[test]
fn test_show_issue() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "Show me", "--type", "bug",
        ])
        .assert()
        .success();

    tick()
        .args(["--db", &db_path, "issue", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"children\""))
        .stdout(predicate::str::contains("\"comments\""))
        .stdout(predicate::str::contains("Show me"));
}

#[test]
fn test_show_nonexistent_issue() {
    let (_dir, db_path) = setup();

    tick()
        .args(["--db", &db_path, "issue", "show", "999"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("NOT_FOUND"));
}

#[test]
fn test_full_lifecycle() {
    let (_dir, db_path) = setup();

    // Create
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Lifecycle test",
            "--type",
            "bug",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"open\""));

    // Start
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "start",
            "1",
            "--branch",
            "fix/lifecycle",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"in-progress\""));

    // Done
    tick()
        .args(["--db", &db_path, "issue", "done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"done\""));

    // Close
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "close",
            "1",
            "--resolution",
            "resolved",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"closed\""))
        .stdout(predicate::str::contains("\"resolution\":\"resolved\""));

    // Status command should show 1 closed
    tick()
        .args(["--db", &db_path, "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"closed\":1"));
}

#[test]
fn test_start_already_in_progress() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "In progress test",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    // First start — ok
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "start",
            "1",
            "--branch",
            "fix/conflict",
        ])
        .assert()
        .success();

    // Second start — should fail with exit code 6 (CONFLICT)
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "start",
            "1",
            "--branch",
            "fix/conflict",
        ])
        .assert()
        .failure()
        .code(6)
        .stderr(predicate::str::contains("CONFLICT"));
}

#[test]
fn test_close_wontfix_from_open() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Wontfix test",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "close",
            "1",
            "--resolution",
            "wontfix",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"closed\""))
        .stdout(predicate::str::contains("\"resolution\":\"wontfix\""));
}

#[test]
fn test_close_resolved_from_open_rejected() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Rejected close test",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    // Closing an open issue with "resolved" should be rejected (exit code 3)
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "close",
            "1",
            "--resolution",
            "resolved",
        ])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_reopen() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Reopen test",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    // Close with wontfix from open
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "close",
            "1",
            "--resolution",
            "wontfix",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"closed\""));

    // Reopen — status should be back to open
    tick()
        .args(["--db", &db_path, "issue", "reopen", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"open\""));
}

#[test]
fn test_sub_issue() {
    let (_dir, db_path) = setup();

    // Create parent
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Parent issue",
            "--type",
            "feature",
        ])
        .assert()
        .success();

    // Create child with --parent 1
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Child issue",
            "--type",
            "bug",
            "--parent",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"parent_id\":1"));

    // Show parent — children should contain child
    tick()
        .args(["--db", &db_path, "issue", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"children\""))
        .stdout(predicate::str::contains("Child issue"));
}

#[test]
fn test_update_fields() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Old title",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "update",
            "1",
            "--title",
            "New title",
            "--priority",
            "high",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"New title\""))
        .stdout(predicate::str::contains("\"priority\":\"high\""));
}

#[test]
fn test_pretty_output() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Pretty test issue",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    tick()
        .args(["--db", &db_path, "--pretty", "issue", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id: 1"))
        .stdout(predicate::str::contains("title:"));
}

#[test]
fn test_close_with_comment() {
    let (_dir, db_path) = setup();

    // Create issue
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Comment close test",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    // Start with branch
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "start",
            "1",
            "--branch",
            "fix/comment-close",
        ])
        .assert()
        .success();

    // Done
    tick()
        .args(["--db", &db_path, "issue", "done", "1"])
        .assert()
        .success();

    // Close with comment and role reviewer
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "close",
            "1",
            "-c",
            "LGTM",
            "--role",
            "reviewer",
            "--resolution",
            "resolved",
        ])
        .assert()
        .success();

    // Show issue and verify the comment appears
    tick()
        .args(["--db", &db_path, "issue", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("LGTM"));
}

#[test]
fn test_reopen_clears_branch_and_resolution() {
    let (_dir, db_path) = setup();

    // Create issue
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "create",
            "Reopen clears test",
            "--type",
            "bug",
        ])
        .assert()
        .success();

    // Start with branch
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "start",
            "1",
            "--branch",
            "fix/reopen-clears",
        ])
        .assert()
        .success();

    // Done
    tick()
        .args(["--db", &db_path, "issue", "done", "1"])
        .assert()
        .success();

    // Close with resolved
    tick()
        .args([
            "--db",
            &db_path,
            "issue",
            "close",
            "1",
            "--resolution",
            "resolved",
        ])
        .assert()
        .success();

    // Reopen
    tick()
        .args(["--db", &db_path, "issue", "reopen", "1"])
        .assert()
        .success();

    // Show the issue and verify status is open, branch is null, resolution is null
    let output = tick()
        .args(["--db", &db_path, "issue", "show", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("\"status\":\"open\""),
        "status should be open after reopen"
    );
    assert!(
        stdout.contains("\"branch\":null"),
        "branch should be null after reopen"
    );
    assert!(
        stdout.contains("\"resolution\":null"),
        "resolution should be null after reopen"
    );
}

#[test]
fn test_empty_database_status() {
    let (_dir, db_path) = setup();

    // Run status with no issues created
    tick()
        .args(["--db", &db_path, "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"open\":0"))
        .stdout(predicate::str::contains("\"closed\":0"));
}

#[test]
fn test_comment_add_and_list() {
    let (_dir, db_path) = setup();

    // Create an issue to comment on
    tick()
        .args([
            "--db", &db_path, "issue", "create", "Comment test issue", "--type", "bug",
        ])
        .assert()
        .success();

    // Add a comment with role worker
    tick()
        .args([
            "--db", &db_path, "comment", "add", "1", "First comment", "--role", "worker",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("First comment"))
        .stdout(predicate::str::contains("\"role\":\"worker\""));

    // Add a comment with role reviewer
    tick()
        .args([
            "--db", &db_path, "comment", "add", "1", "LGTM", "--role", "reviewer",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("LGTM"))
        .stdout(predicate::str::contains("\"role\":\"reviewer\""));

    // List all comments — should have both
    let output = tick()
        .args(["--db", &db_path, "comment", "list", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("First comment"), "should contain first comment");
    assert!(stdout.contains("LGTM"), "should contain second comment");

    // Filter by role worker — should only have first comment
    let output = tick()
        .args(["--db", &db_path, "comment", "list", "1", "--role", "worker"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let filtered = String::from_utf8_lossy(&output);
    assert!(filtered.contains("First comment"), "should contain worker comment");
    assert!(!filtered.contains("LGTM"), "should NOT contain reviewer comment");

    // Filter by role reviewer — should only have LGTM
    let output = tick()
        .args(["--db", &db_path, "comment", "list", "1", "--role", "reviewer"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let reviewer_filtered = String::from_utf8_lossy(&output);
    assert!(reviewer_filtered.contains("LGTM"), "should contain reviewer comment");
    assert!(!reviewer_filtered.contains("First comment"), "should NOT contain worker comment");
}

#[test]
fn test_comment_add_nonexistent_issue() {
    let (_dir, db_path) = setup();

    // Attempt to add a comment to a non-existent issue
    tick()
        .args(["--db", &db_path, "comment", "add", "999", "This should fail"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("NOT_FOUND"));
}

#[test]
fn test_link_and_show() {
    let (_dir, db_path) = setup();

    // Create two issues
    tick()
        .args(["--db", &db_path, "issue", "create", "Base feature", "--type", "feature"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "create", "Dependent task", "--type", "bug"])
        .assert()
        .success();

    // Link: issue 2 depends-on issue 1
    tick()
        .args(["--db", &db_path, "issue", "link", "2", "depends-on", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"linked\":true"))
        .stdout(predicate::str::contains("\"from\":2"))
        .stdout(predicate::str::contains("\"to\":1"));

    // Show issue 2 — depends_on should contain issue 1
    tick()
        .args(["--db", &db_path, "issue", "show", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"depends_on\""))
        .stdout(predicate::str::contains("Base feature"));
}

#[test]
fn test_link_self_reference_rejected() {
    let (_dir, db_path) = setup();

    tick()
        .args(["--db", &db_path, "issue", "create", "Self ref test", "--type", "bug"])
        .assert()
        .success();

    // Linking an issue to itself should fail with exit code 3
    tick()
        .args(["--db", &db_path, "issue", "link", "1", "depends-on", "1"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_link_cycle_rejected() {
    let (_dir, db_path) = setup();

    tick()
        .args(["--db", &db_path, "issue", "create", "Issue A", "--type", "feature"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "create", "Issue B", "--type", "feature"])
        .assert()
        .success();

    // A depends-on B
    tick()
        .args(["--db", &db_path, "issue", "link", "1", "depends-on", "2"])
        .assert()
        .success();

    // B depends-on A should fail — cycle
    tick()
        .args(["--db", &db_path, "issue", "link", "2", "depends-on", "1"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_unlink() {
    let (_dir, db_path) = setup();

    tick()
        .args(["--db", &db_path, "issue", "create", "Issue X", "--type", "feature"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "create", "Issue Y", "--type", "feature"])
        .assert()
        .success();

    // Link
    tick()
        .args(["--db", &db_path, "issue", "link", "2", "depends-on", "1"])
        .assert()
        .success();

    // Unlink
    tick()
        .args(["--db", &db_path, "issue", "unlink", "2", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"unlinked\":true"));

    // Show issue 2 — depends_on should now be empty (no "Issue X" in depends_on list)
    let output = tick()
        .args(["--db", &db_path, "issue", "show", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // depends_on array should be empty
    assert!(
        stdout.contains("\"depends_on\":[]"),
        "depends_on should be empty after unlink, got: {stdout}"
    );
}

#[test]
fn test_start_blocked_by_unresolved_dependency() {
    let (_dir, db_path) = setup();

    // Create dependency (open) and dependent
    tick()
        .args(["--db", &db_path, "issue", "create", "Blocker issue", "--type", "feature"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "create", "Blocked issue", "--type", "bug"])
        .assert()
        .success();

    // Issue 2 depends-on issue 1 (still open)
    tick()
        .args(["--db", &db_path, "issue", "link", "2", "depends-on", "1"])
        .assert()
        .success();

    // Try to start issue 2 — should fail because issue 1 is not closed
    tick()
        .args(["--db", &db_path, "issue", "start", "2", "--branch", "fix/blocked"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("INVALID_ARGUMENT"));
}

#[test]
fn test_cascade_wontfix() {
    let (_dir, db_path) = setup();

    // Create base issue and two dependents
    tick()
        .args(["--db", &db_path, "issue", "create", "Base issue", "--type", "feature"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "create", "Dependent A", "--type", "bug"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "create", "Dependent B", "--type", "bug"])
        .assert()
        .success();

    // Both dependents depend on base issue
    tick()
        .args(["--db", &db_path, "issue", "link", "2", "depends-on", "1"])
        .assert()
        .success();
    tick()
        .args(["--db", &db_path, "issue", "link", "3", "depends-on", "1"])
        .assert()
        .success();

    // Wontfix the base issue
    tick()
        .args(["--db", &db_path, "issue", "close", "1", "--resolution", "wontfix"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"closed\""))
        .stdout(predicate::str::contains("\"resolution\":\"wontfix\""));

    // Both dependents should be closed
    tick()
        .args(["--db", &db_path, "issue", "show", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"closed\""))
        .stdout(predicate::str::contains("\"resolution\":\"wontfix\""));

    tick()
        .args(["--db", &db_path, "issue", "show", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"closed\""))
        .stdout(predicate::str::contains("\"resolution\":\"wontfix\""));

    // Dependents should have system comment from cascade
    tick()
        .args(["--db", &db_path, "comment", "list", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed by cascade: dependency #1 was abandoned"));

    tick()
        .args(["--db", &db_path, "comment", "list", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Closed by cascade: dependency #1 was abandoned"));
}

#[test]
fn test_expect_version_correct() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "Version test", "--type", "bug",
        ])
        .assert()
        .success();

    // version after create is 1
    tick()
        .args([
            "--db", &db_path, "issue", "update", "1", "--title", "Updated title",
            "--expect-version", "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"Updated title\""));
}

#[test]
fn test_expect_version_wrong() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "Version conflict test", "--type", "bug",
        ])
        .assert()
        .success();

    // version is 1, but we pass 999 — should fail with CONFLICT (exit 6)
    tick()
        .args([
            "--db", &db_path, "issue", "update", "1", "--title", "Should fail",
            "--expect-version", "999",
        ])
        .assert()
        .failure()
        .code(6)
        .stderr(predicate::str::contains("CONFLICT"));
}

#[test]
fn test_expect_version_not_provided() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "No version flag test", "--type", "bug",
        ])
        .assert()
        .success();

    // No --expect-version flag — should succeed normally
    tick()
        .args([
            "--db", &db_path, "issue", "update", "1", "--title", "New title",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"New title\""));
}

#[test]
fn test_fields_filter() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "Fields test", "--type", "bug",
        ])
        .assert()
        .success();

    let output = tick()
        .args([
            "--db", &db_path, "--fields", "id,title", "issue", "show", "1",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("\"id\""), "output should contain id");
    assert!(stdout.contains("\"title\""), "output should contain title");
    assert!(!stdout.contains("\"status\""), "output should NOT contain status");
}

#[test]
fn test_quiet_mode() {
    let (_dir, db_path) = setup();

    let output = tick()
        .args([
            "--db", &db_path, "--quiet", "issue", "create", "Quiet test", "--type", "bug",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert_eq!(stdout.trim(), "1", "quiet mode should output just the id");
}

#[test]
fn test_dry_run_start() {
    let (_dir, db_path) = setup();

    tick()
        .args([
            "--db", &db_path, "issue", "create", "Dry run test", "--type", "bug",
        ])
        .assert()
        .success();

    // dry-run start — should succeed and output dry_run info, NOT change status
    tick()
        .args([
            "--db", &db_path, "--dry-run", "issue", "start", "1", "--branch", "fix/dry",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"dry_run\":true"))
        .stdout(predicate::str::contains("\"would_succeed\":true"));

    // Issue should still be open
    tick()
        .args(["--db", &db_path, "issue", "show", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\":\"open\""));
}

#[test]
fn test_search() {
    let (_dir, db_path) = setup();

    tick()
        .args(["--db", &db_path, "issue", "create", "Alpha feature request", "--type", "feature"])
        .assert()
        .success();

    tick()
        .args(["--db", &db_path, "issue", "create", "Beta bug report", "--type", "bug"])
        .assert()
        .success();

    // Search for "Alpha" — should only match first issue
    let output = tick()
        .args(["--db", &db_path, "issue", "search", "Alpha"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Alpha feature request"), "search should match 'Alpha feature request'");
    assert!(!stdout.contains("Beta bug report"), "search should NOT match 'Beta bug report'");
}

#[test]
fn test_search_no_results() {
    let (_dir, db_path) = setup();

    tick()
        .args(["--db", &db_path, "issue", "create", "Some issue", "--type", "bug"])
        .assert()
        .success();

    // Search for nonexistent term
    tick()
        .args(["--db", &db_path, "issue", "search", "xyznonexistent123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}
