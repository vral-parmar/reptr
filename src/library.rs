//! Finding-library support.
//!
//! A "library" is just a directory of Markdown files with the same front-matter
//! shape as a finding. Each `.md` file is one importable template; the
//! template's name is its path relative to the library root, with `.md`
//! stripped (so `web/xss-stored.md` is referenced as `web/xss-stored`).
//!
//! Templates may omit the `id:` field — `reptr add finding --from <name>`
//! assigns a fresh `F-NNN` at import time.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gray_matter::engine::YAML;
use gray_matter::Matter;
use serde::Deserialize;
use walkdir::WalkDir;

use crate::model::LibraryConfig;

/// Resolve the library directory for an engagement.
///
/// Order:
/// 1. If the user set `[library].path` in `reptr.toml`, use that (relative to
///    the engagement root, or absolute).
/// 2. Otherwise default to `<engagement_root>/findings-library`.
pub fn resolve_library_dir(engagement_root: &Path, cfg: &LibraryConfig) -> PathBuf {
    match cfg.path.as_deref() {
        Some(p) => {
            let candidate = Path::new(p);
            if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                engagement_root.join(candidate)
            }
        }
        None => engagement_root.join("findings-library"),
    }
}

#[derive(Debug, Clone)]
pub struct Template {
    /// Name as the user references it — `web/xss-stored`, no `.md`.
    pub name: String,
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Title from the template's front matter (best-effort; may be empty).
    pub title: String,
    /// Severity from the template's front matter (best-effort; may be empty).
    pub severity: String,
    /// Full contents of the template file, front matter included.
    pub raw_text: String,
}

#[derive(Debug, Default, Deserialize)]
struct MinimalMeta {
    #[serde(default)]
    title: String,
    #[serde(default)]
    severity: String,
}

/// List every template under `dir`, sorted by name. Returns an empty Vec if
/// the directory doesn't exist — callers can decide how to phrase that.
pub fn list_templates(dir: &Path) -> Result<Vec<Template>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in WalkDir::new(dir).into_iter().flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let rel = path.strip_prefix(dir).unwrap_or(path);
        let name = rel.with_extension("").to_string_lossy().replace('\\', "/");
        let raw = fs::read_to_string(path)
            .with_context(|| format!("reading template {}", path.display()))?;
        let meta = parse_minimal_meta(&raw);
        out.push(Template {
            name,
            path: path.to_path_buf(),
            title: meta.title,
            severity: meta.severity,
            raw_text: raw,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Load one template by name (e.g. `xss-stored` or `web/xss-stored`). Returns
/// a clear error when the file is missing.
pub fn load_template(dir: &Path, name: &str) -> Result<Template> {
    let path = dir.join(format!("{name}.md"));
    if !path.is_file() {
        anyhow::bail!(
            "template `{name}` not found in {} (looking for {})",
            dir.display(),
            path.display()
        );
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading template {}", path.display()))?;
    let meta = parse_minimal_meta(&raw);
    Ok(Template {
        name: name.to_string(),
        path,
        title: meta.title,
        severity: meta.severity,
        raw_text: raw,
    })
}

fn parse_minimal_meta(raw: &str) -> MinimalMeta {
    let m = Matter::<YAML>::new();
    let parsed = m.parse(raw);
    parsed
        .data
        .and_then(|d| d.deserialize().ok())
        .unwrap_or_default()
}

/// Rewrite or append YAML key-value pairs inside the front-matter region of
/// `raw`. Lines outside the `--- ... ---` fences are untouched.
///
/// - If a key already appears in the front matter, its line is replaced.
/// - If a key is missing, it's appended just before the closing fence.
///
/// This is intentionally line-based rather than a YAML round-trip — it
/// preserves the user's authored ordering, comments, and quoting style.
pub fn rewrite_front_matter(raw: &str, kvs: &[(&str, &str)]) -> String {
    let lines: Vec<String> = raw.lines().map(|s| s.to_string()).collect();
    let fences: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim() == "---")
        .map(|(i, _)| i)
        .take(2)
        .collect();
    if fences.len() < 2 {
        // No front matter — synthesise one at the top.
        let mut fm = vec!["---".to_string()];
        for (k, v) in kvs {
            fm.push(format!("{k}: {v}"));
        }
        fm.push("---".to_string());
        fm.push(String::new());
        let preserve_trailing = raw.ends_with('\n');
        let mut out = fm.join("\n");
        out.push('\n');
        out.push_str(raw);
        if preserve_trailing && !out.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }
    let (open, mut close) = (fences[0], fences[1]);

    let mut new_lines = lines;
    let mut updated = vec![false; kvs.len()];

    for line in new_lines.iter_mut().take(close).skip(open + 1) {
        for (i, (k, v)) in kvs.iter().enumerate() {
            if line_starts_with_key(line, k) {
                *line = format!("{k}: {v}");
                updated[i] = true;
                break;
            }
        }
    }

    for (i, (k, v)) in kvs.iter().enumerate() {
        if !updated[i] {
            new_lines.insert(close, format!("{k}: {v}"));
            close += 1;
        }
    }

    let preserve_trailing = raw.ends_with('\n');
    let mut out = new_lines.join("\n");
    if preserve_trailing && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn line_starts_with_key(line: &str, key: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with(key) {
        return false;
    }
    let rest = &trimmed[key.len()..];
    rest.starts_with(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_replaces_existing_keys() {
        let raw = "---\nid: F-old\ntitle: Old\nseverity: low\n---\n\nbody\n";
        let out = rewrite_front_matter(raw, &[("id", "F-042"), ("title", "New title")]);
        assert!(out.contains("id: F-042"));
        assert!(out.contains("title: New title"));
        assert!(out.contains("severity: low"));
        assert!(out.contains("body"));
        assert!(!out.contains("F-old"));
    }

    #[test]
    fn rewrite_appends_missing_keys() {
        let raw = "---\nseverity: high\n---\n\nbody\n";
        let out = rewrite_front_matter(raw, &[("id", "F-001"), ("title", "Hello")]);
        // Both keys should sit between the fences, before the body.
        let body_pos = out.find("body").unwrap();
        let id_pos = out.find("id: F-001").unwrap();
        let title_pos = out.find("title: Hello").unwrap();
        assert!(id_pos < body_pos);
        assert!(title_pos < body_pos);
    }

    #[test]
    fn rewrite_does_not_touch_body() {
        // A line in the body that looks like a key shouldn't be modified.
        let raw = "---\nid: F-1\n---\n\nseverity: low (this is body text)\n";
        let out = rewrite_front_matter(raw, &[("severity", "critical")]);
        // Body untouched
        assert!(out.contains("severity: low (this is body text)"));
        // FM gained the key
        let fm_end = out.find("---\n\nseverity: low").unwrap();
        let fm = &out[..fm_end];
        assert!(fm.contains("severity: critical"));
    }
}
