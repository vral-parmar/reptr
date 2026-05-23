use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use comrak::{markdown_to_html, ComrakOptions};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;
use walkdir::WalkDir;

use crate::model::{Finding, ImageRef, Severity, Status};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} has no YAML front matter (expected `---` fences at the top)")]
    MissingFrontMatter { path: PathBuf },
    #[error("{path} front matter is invalid: {message}")]
    InvalidFrontMatter { path: PathBuf, message: String },
    #[error("findings/ directory not found at {path}")]
    NoFindingsDir { path: PathBuf },
}

#[derive(Debug, Deserialize)]
struct FrontMatter {
    id: String,
    title: String,
    severity: Severity,
    #[serde(default)]
    cvss: Option<String>,
    #[serde(default)]
    cvss_vector: Option<String>,
    #[serde(default)]
    cwe: Option<String>,
    #[serde(default)]
    owasp: Option<String>,
    #[serde(default)]
    status: Option<Status>,
    #[serde(default)]
    affected_assets: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn comrak_options() -> ComrakOptions<'static> {
    let mut opts = ComrakOptions::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.footnotes = true;
    opts.extension.autolink = true;
    opts.render.unsafe_ = true; // allow inline HTML in finding bodies (screenshots, etc.)
    opts
}

pub fn parse_finding_file(path: &Path) -> Result<Finding, ParseError> {
    let raw = fs::read_to_string(path).map_err(|e| ParseError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&raw);
    let data = parsed.data.ok_or_else(|| ParseError::MissingFrontMatter {
        path: path.to_path_buf(),
    })?;
    let fm: FrontMatter = data
        .deserialize()
        .map_err(|e| ParseError::InvalidFrontMatter {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;

    let body_markdown = parsed.content.trim_start_matches('\n').to_string();
    let body_html = markdown_to_html(&body_markdown, &comrak_options());
    let finding_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let images = scan_images(&body_markdown, finding_dir);

    // When cvss_vector is provided but cvss (the numeric score) is absent,
    // derive the score automatically so templates and JSON always have it.
    let cvss_score = fm.cvss.clone().or_else(|| {
        fm.cvss_vector
            .as_deref()
            .and_then(|v| v.parse::<cvss::v3::Base>().ok())
            .map(|base| format!("{:.1}", base.score().value()))
    });

    Ok(Finding {
        id: fm.id,
        title: fm.title,
        severity: fm.severity,
        cvss: cvss_score,
        cvss_vector: fm.cvss_vector,
        cwe: fm.cwe,
        owasp: fm.owasp,
        status: fm.status.unwrap_or_default(),
        affected_assets: fm.affected_assets,
        tags: fm.tags,
        body_markdown,
        body_html,
        source_path: path.to_path_buf(),
        images,
    })
}

/// Find every `![alt](src)` in the body and resolve `src` to an absolute path
/// (when local). Remote URLs (`http://`, `https://`, `data:`) keep
/// `resolved_path: None` and downstream renderers can decide what to do.
fn scan_images(md: &str, finding_dir: &Path) -> Vec<ImageRef> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"!\[(?P<alt>[^\]]*)\]\((?P<src>[^)\s]+)(?:\s+"[^"]*")?\)"#).unwrap()
    });

    let mut out = Vec::new();
    for cap in re.captures_iter(md) {
        let alt = cap
            .name("alt")
            .map(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let src = cap["src"].to_string();
        let resolved = if is_remote(&src) {
            None
        } else {
            let p = finding_dir.join(&src);
            match p.canonicalize() {
                Ok(abs) => Some(abs),
                Err(_) => Some(p), // keep the joined path; renderer will warn if missing
            }
        };
        out.push(ImageRef {
            alt,
            markdown_src: src,
            resolved_path: resolved,
        });
    }
    out
}

fn is_remote(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:")
}

/// Walk `findings_dir` and parse every `*.md` file (sorted by filename so
/// downstream output is deterministic).
pub fn load_findings(findings_dir: &Path) -> Result<Vec<Finding>, ParseError> {
    if !findings_dir.exists() {
        return Err(ParseError::NoFindingsDir {
            path: findings_dir.to_path_buf(),
        });
    }

    let mut files: Vec<PathBuf> = WalkDir::new(findings_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    files.sort();

    let mut findings = Vec::with_capacity(files.len());
    for path in files {
        findings.push(parse_finding_file(&path)?);
    }
    Ok(findings)
}
