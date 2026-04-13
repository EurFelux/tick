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
