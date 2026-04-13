use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum IssueStatus {
    Open,
    InProgress,
    Done,
    Closed,
}

impl fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueStatus::Open => write!(f, "open"),
            IssueStatus::InProgress => write!(f, "in-progress"),
            IssueStatus::Done => write!(f, "done"),
            IssueStatus::Closed => write!(f, "closed"),
        }
    }
}

impl std::str::FromStr for IssueStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "open" => Ok(IssueStatus::Open),
            "in-progress" => Ok(IssueStatus::InProgress),
            "done" => Ok(IssueStatus::Done),
            "closed" => Ok(IssueStatus::Closed),
            _ => Err(format!("invalid status: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum IssueType {
    Bug,
    Feature,
    Refactor,
    Docs,
    Test,
    Chore,
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueType::Bug => write!(f, "bug"),
            IssueType::Feature => write!(f, "feature"),
            IssueType::Refactor => write!(f, "refactor"),
            IssueType::Docs => write!(f, "docs"),
            IssueType::Test => write!(f, "test"),
            IssueType::Chore => write!(f, "chore"),
        }
    }
}

impl std::str::FromStr for IssueType {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "bug" => Ok(IssueType::Bug),
            "feature" => Ok(IssueType::Feature),
            "refactor" => Ok(IssueType::Refactor),
            "docs" => Ok(IssueType::Docs),
            "test" => Ok(IssueType::Test),
            "chore" => Ok(IssueType::Chore),
            _ => Err(format!("invalid type: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Medium => write!(f, "medium"),
            Priority::High => write!(f, "high"),
            Priority::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "low" => Ok(Priority::Low),
            "medium" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            "critical" => Ok(Priority::Critical),
            _ => Err(format!("invalid priority: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Resolution {
    Resolved,
    Wontfix,
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resolution::Resolved => write!(f, "resolved"),
            Resolution::Wontfix => write!(f, "wontfix"),
        }
    }
}

impl std::str::FromStr for Resolution {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "resolved" => Ok(Resolution::Resolved),
            "wontfix" => Ok(Resolution::Wontfix),
            _ => Err(format!("invalid resolution: {s}")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub description: String,
    #[serde(rename = "type")]
    pub issue_type: IssueType,
    pub status: IssueStatus,
    pub priority: Priority,
    pub resolution: Option<Resolution>,
    pub branch: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueDetail {
    #[serde(flatten)]
    pub issue: Issue,
    pub parent: Option<IssueSummary>,
    pub children: Vec<IssueSummary>,
    pub depends_on: Vec<IssueSummary>,
    pub depended_by: Vec<IssueSummary>,
    pub comments: Vec<super::comment::Comment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueSummary {
    pub id: i64,
    pub title: String,
    pub status: IssueStatus,
}
