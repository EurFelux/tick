pub mod comments;
pub mod issues;
pub mod links;
pub mod migrate;

use std::collections::HashMap;
use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;
use crate::models::{
    Comment, CommentRole, Issue, IssueStatus, IssueSummary, IssueType, Priority, Resolution,
};

pub use issues::ListFilter;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Set pragmas
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA foreign_keys = ON;
        ",
        )?;

        Ok(Database { conn })
    }

    pub fn migrate(&mut self) -> Result<()> {
        migrate::run_migrations(&self.conn)
    }

    pub fn schema_version(&self) -> Result<i64> {
        migrate::schema_version(&self.conn)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    // Issue methods

    pub fn create_issue(
        &self,
        title: &str,
        description: &str,
        issue_type: &IssueType,
        priority: &Priority,
        parent_id: Option<i64>,
    ) -> Result<i64> {
        issues::create(
            &self.conn,
            title,
            description,
            issue_type,
            priority,
            parent_id,
        )
    }

    pub fn get_issue(&self, id: i64) -> Result<Issue> {
        issues::get(&self.conn, id)
    }

    pub fn list_issues(&self, filter: &ListFilter) -> Result<Vec<Issue>> {
        issues::list(&self.conn, filter)
    }

    pub fn get_children(&self, parent_id: i64) -> Result<Vec<IssueSummary>> {
        issues::get_children(&self.conn, parent_id)
    }

    pub fn get_issue_summary(&self, id: i64) -> Result<IssueSummary> {
        issues::get_summary(&self.conn, id)
    }

    pub fn update_issue_fields(
        &self,
        id: i64,
        title: Option<&str>,
        description: Option<&str>,
        issue_type: Option<&IssueType>,
        priority: Option<&Priority>,
        parent_id: Option<Option<i64>>,
    ) -> Result<Issue> {
        issues::update_fields(
            &self.conn,
            id,
            title,
            description,
            issue_type,
            priority,
            parent_id,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_issue_status_atomic(
        &self,
        id: i64,
        expected_status: &IssueStatus,
        new_status: &IssueStatus,
        resolution: Option<Option<&Resolution>>,
        branch: Option<Option<&str>>,
        clear_branch: bool,
        clear_resolution: bool,
    ) -> Result<Issue> {
        issues::update_status_atomic(
            &self.conn,
            id,
            expected_status,
            new_status,
            resolution,
            branch,
            clear_branch,
            clear_resolution,
        )
    }

    pub fn count_by_status(&self) -> Result<HashMap<String, i64>> {
        issues::count_by_status(&self.conn)
    }

    // Comment methods

    pub fn create_comment(&self, issue_id: i64, body: &str, role: &CommentRole) -> Result<i64> {
        comments::create(&self.conn, issue_id, body, role)
    }

    pub fn list_comments(&self, issue_id: i64, role: Option<&CommentRole>) -> Result<Vec<Comment>> {
        comments::list_by_issue(&self.conn, issue_id, role)
    }

    // Link methods

    pub fn list_links(&self, issue_id: i64) -> Result<(Vec<IssueSummary>, Vec<IssueSummary>)> {
        links::list_by_issue(&self.conn, issue_id)
    }

    pub fn create_link(&self, from_id: i64, to_id: i64) -> Result<()> {
        links::create(&self.conn, from_id, to_id)
    }

    pub fn delete_link(&self, from_id: i64, to_id: i64) -> Result<()> {
        links::delete(&self.conn, from_id, to_id)
    }

    pub fn get_depended_by_ids(&self, issue_id: i64) -> Result<Vec<i64>> {
        links::get_depended_by_ids(&self.conn, issue_id)
    }
}
