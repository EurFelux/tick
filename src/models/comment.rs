use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum CommentRole {
    Worker,
    Reviewer,
    Pm,
    Qa,
    User,
    System,
}

impl fmt::Display for CommentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommentRole::Worker => write!(f, "worker"),
            CommentRole::Reviewer => write!(f, "reviewer"),
            CommentRole::Pm => write!(f, "pm"),
            CommentRole::Qa => write!(f, "qa"),
            CommentRole::User => write!(f, "user"),
            CommentRole::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for CommentRole {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "worker" => Ok(CommentRole::Worker),
            "reviewer" => Ok(CommentRole::Reviewer),
            "pm" => Ok(CommentRole::Pm),
            "qa" => Ok(CommentRole::Qa),
            "user" => Ok(CommentRole::User),
            "system" => Ok(CommentRole::System),
            _ => Err(format!("invalid role: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Comment {
    pub id: i64,
    pub issue_id: i64,
    pub body: String,
    pub role: CommentRole,
    pub created_at: String,
}
