use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::model::Severity;

#[derive(Debug, Parser)]
#[command(
    name = "reptr",
    version,
    about = "Local-first pentest report generator",
    long_about = "Write your pentest findings as Markdown files. Run `reptr build`. \
                  Get a polished HTML, JSON, DOCX, and PDF report. \
                  No Docker, no database, no SaaS."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scaffold a new engagement directory.
    New {
        /// Engagement slug (also used as directory name).
        name: String,
    },
    /// Add a new finding, stub or otherwise.
    Add {
        #[command(subcommand)]
        what: AddTarget,
    },
    /// Parse, validate, and render the engagement in `path` (default: cwd).
    Build {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Build once, then auto-rebuild whenever findings, templates, or config change.
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Summarise findings across every engagement directory under `path`.
    Stats {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = StatsFormat::Text)]
        format: StatsFormat,
    },
    /// Manage and inspect the finding-template library.
    Library {
        #[command(subcommand)]
        action: LibraryAction,
    },
    /// Build the engagement, diff findings against the previous build, and emit a delta report.
    ///
    /// On the first run `reptr retest` establishes a baseline (same as `reptr build`).
    /// On every subsequent run it shows which findings are new, resolved, regressed, or changed
    /// and writes `output/<slug>-retest.{html,json}`.
    Retest {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum LibraryAction {
    /// List every template in the library directory.
    List {
        /// Engagement root (default: cwd) — used to resolve [library].path.
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum StatsFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
pub enum AddTarget {
    /// Add a finding stub at `findings/NNN-<slug>.md`.
    Finding {
        /// Finding title (optional when --from is supplied; falls back to the template's title).
        title: Option<String>,
        /// Severity — one of critical|high|medium|low|info. Ignored when --from is set
        /// and the library template defines its own severity.
        #[arg(long, value_enum, default_value_t = SeverityArg::Medium)]
        severity: SeverityArg,
        /// Import from a finding-library template by name (e.g. `xss-stored` or `web/xss-stored`).
        #[arg(long)]
        from: Option<String>,
        /// Engagement root (default: cwd).
        #[arg(long, default_value = ".")]
        path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SeverityArg {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl From<SeverityArg> for Severity {
    fn from(value: SeverityArg) -> Self {
        match value {
            SeverityArg::Critical => Severity::Critical,
            SeverityArg::High => Severity::High,
            SeverityArg::Medium => Severity::Medium,
            SeverityArg::Low => Severity::Low,
            SeverityArg::Info => Severity::Info,
        }
    }
}
