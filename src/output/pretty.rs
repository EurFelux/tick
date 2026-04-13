use std::collections::HashMap;

use crate::models::{Issue, IssueDetail, IssueSummary};

pub fn print_issue(issue: &Issue) {
    println!("id: {}", issue.id);
    println!("title: {}", issue.title);
    println!("type: {}", issue.issue_type);
    println!("status: {}", issue.status);
    println!("priority: {}", issue.priority);
    if let Some(ref r) = issue.resolution {
        println!("resolution: {}", r);
    }
    if let Some(ref b) = issue.branch {
        println!("branch: {}", b);
    }
    if let Some(pid) = issue.parent_id {
        println!("parent_id: {}", pid);
    }
    if !issue.description.is_empty() {
        println!("description: {}", issue.description);
    }
    println!("version: {}", issue.version);
    println!("created_at: {}", issue.created_at);
    println!("updated_at: {}", issue.updated_at);
}

pub fn print_issue_detail(detail: &IssueDetail) {
    print_issue(&detail.issue);

    if let Some(ref parent) = detail.parent {
        println!("parent: #{} [{}] {}", parent.id, parent.status, parent.title);
    }

    if !detail.children.is_empty() {
        println!("children:");
        for child in &detail.children {
            println!("  #{} [{}] {}", child.id, child.status, child.title);
        }
    }

    if !detail.depends_on.is_empty() {
        println!("depends_on:");
        for dep in &detail.depends_on {
            println!("  #{} [{}] {}", dep.id, dep.status, dep.title);
        }
    }

    if !detail.depended_by.is_empty() {
        println!("depended_by:");
        for dep in &detail.depended_by {
            println!("  #{} [{}] {}", dep.id, dep.status, dep.title);
        }
    }

    if !detail.comments.is_empty() {
        println!("comments:");
        for comment in &detail.comments {
            println!("  [{}] {}: {}", comment.created_at, comment.role, comment.body);
        }
    }
}

pub fn print_issue_list(issues: &[Issue]) {
    for issue in issues {
        println!(
            "#{} [{}] [{}] [{}] {}",
            issue.id, issue.status, issue.issue_type, issue.priority, issue.title
        );
    }
}

pub fn print_summary_list(issues: &[IssueSummary]) {
    for issue in issues {
        println!("#{} [{}] {}", issue.id, issue.status, issue.title);
    }
}

pub fn print_status_counts(counts: &HashMap<String, i64>) {
    let mut entries: Vec<(&String, &i64)> = counts.iter().collect();
    entries.sort_by_key(|(k, _)| k.as_str());
    for (status, count) in entries {
        println!("{}: {}", status, count);
    }
}

pub fn print_error(message: &str) {
    eprintln!("error: {}", message);
}
