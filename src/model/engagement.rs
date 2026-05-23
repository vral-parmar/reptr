use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::finding::{Finding, Severity};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Engagement {
    pub meta: EngagementMeta,
    pub client: Client,
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub appendices: Vec<Appendix>,
    pub output: OutputConfig,
    #[serde(default)]
    pub template: TemplateConfig,
    #[serde(default)]
    pub severity_thresholds: SeverityThresholds,
    #[serde(default)]
    pub library: LibraryConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EngagementMeta {
    pub name: String,
    pub slug: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default = "default_report_version")]
    pub report_version: String,
}

fn default_report_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Client {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub contact: String,
    #[serde(default)]
    pub email: String,
    #[serde(default, flatten)]
    pub extra: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutputConfig {
    #[serde(default = "default_formats")]
    pub formats: Vec<String>,
    #[serde(default = "default_output_dir")]
    pub directory: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            formats: default_formats(),
            directory: default_output_dir(),
        }
    }
}

fn default_formats() -> Vec<String> {
    vec!["html".to_string(), "json".to_string()]
}

fn default_output_dir() -> String {
    "output".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TemplateConfig {
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub docx: Option<String>,
    #[serde(default)]
    pub pdf: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LibraryConfig {
    /// Directory containing finding templates (relative to the engagement root,
    /// or absolute). Defaults to `./findings-library`.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeverityThresholds {
    #[serde(default = "default_critical")]
    pub critical: f32,
    #[serde(default = "default_high")]
    pub high: f32,
    #[serde(default = "default_medium")]
    pub medium: f32,
    #[serde(default = "default_low")]
    pub low: f32,
}

impl Default for SeverityThresholds {
    fn default() -> Self {
        Self {
            critical: default_critical(),
            high: default_high(),
            medium: default_medium(),
            low: default_low(),
        }
    }
}

fn default_critical() -> f32 {
    9.0
}
fn default_high() -> f32 {
    7.0
}
fn default_medium() -> f32 {
    4.0
}
fn default_low() -> f32 {
    0.1
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Appendix {
    pub title: String,
    pub body_markdown: String,
    pub body_html: String,
}

impl Engagement {
    /// Sort findings by severity descending, then by id ascending. Stable enough
    /// for executive summaries; the plan calls this out as a derived view.
    pub fn sort_findings(&mut self) {
        self.findings.sort_by(|a, b| {
            b.severity
                .rank()
                .cmp(&a.severity.rank())
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    /// Count findings by severity. Returns a fixed-order Vec so templates can
    /// render rows without knowing about hash ordering.
    pub fn severity_counts(&self) -> Vec<(Severity, usize)> {
        let mut counts = [
            (Severity::Critical, 0usize),
            (Severity::High, 0),
            (Severity::Medium, 0),
            (Severity::Low, 0),
            (Severity::Info, 0),
        ];
        for f in &self.findings {
            for slot in &mut counts {
                if slot.0 == f.severity {
                    slot.1 += 1;
                }
            }
        }
        counts.to_vec()
    }
}
