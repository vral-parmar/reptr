use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn rank(self) -> u8 {
        match self {
            Severity::Critical => 4,
            Severity::High => 3,
            Severity::Medium => 2,
            Severity::Low => 1,
            Severity::Info => 0,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
            Severity::Info => "info",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Open,
    Resolved,
    Accepted,
    #[serde(rename = "false_positive")]
    FalsePositive,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Resolved => "resolved",
            Status::Accepted => "accepted",
            Status::FalsePositive => "false_positive",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cvss: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cvss_vector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owasp: Option<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub affected_assets: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Raw markdown body (without the front matter block).
    pub body_markdown: String,
    /// Pre-rendered HTML body — populated by the parse stage.
    pub body_html: String,
    /// Disk path the finding came from.
    pub source_path: PathBuf,
    /// Image references discovered in the body (resolved at parse time).
    #[serde(default)]
    pub images: Vec<ImageRef>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageRef {
    /// Alt text from `![alt](src)`.
    pub alt: String,
    /// The raw `src` value as it appeared in the markdown.
    pub markdown_src: String,
    /// Resolved absolute path on disk. `None` for remote URLs (http/https/data:).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_path: Option<PathBuf>,
}
