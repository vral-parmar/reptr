use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;
use walkdir::WalkDir;

use crate::library::{self, Template};
use crate::model::Severity;
use crate::parse::load_engagement_config;

const FINDING_TEMPLATE: &str = r#"---
id: {id}
title: {title}
severity: {severity}
status: open
affected_assets: []
tags: []
# Optional — uncomment and fill in as needed:
# cvss: "0.0"
# cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N"
# cwe: "CWE-000"
# owasp: "A00:2021"
---

## Description

## Proof of Concept

## Impact

## Remediation

## References
"#;

pub fn run(root: &Path, title: Option<&str>, severity: Severity, from: Option<&str>) -> Result<()> {
    let findings_dir = root.join("findings");
    fs::create_dir_all(&findings_dir)
        .with_context(|| format!("creating {}", findings_dir.display()))?;

    let next_num = next_finding_number(&findings_dir)?;
    let id = format!("F-{:03}", next_num);

    let (final_title, body) = match from {
        Some(name) => build_from_template(root, name, title, &id)?,
        None => {
            let t = title
                .ok_or_else(|| anyhow::anyhow!("title is required when --from is not given"))?;
            if t.trim().is_empty() {
                bail!("title cannot be empty");
            }
            let body = FINDING_TEMPLATE
                .replace("{id}", &id)
                .replace("{title}", t)
                .replace("{severity}", severity.as_str());
            (t.to_string(), body)
        }
    };

    let slug = slugify(&final_title);
    let filename = format!("{:03}-{}.md", next_num, slug);
    let path = findings_dir.join(&filename);
    if path.exists() {
        bail!("{} already exists", path.display());
    }

    fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;

    println!("{} {}", style("Created").green().bold(), path.display());
    Ok(())
}

/// Load a library template, rewrite its `id` (and optionally `title`), and
/// return (effective_title, file_body).
fn build_from_template(
    root: &Path,
    name: &str,
    title_override: Option<&str>,
    new_id: &str,
) -> Result<(String, String)> {
    // Find the library directory the same way `reptr build` would.
    let (cfg, _client) = load_engagement_config(root)
        .with_context(|| "reading engagement config for library lookup")?;
    let lib_dir = library::resolve_library_dir(root, &cfg.library);

    let template: Template = library::load_template(&lib_dir, name)?;

    let mut kvs: Vec<(&str, &str)> = vec![("id", new_id)];
    if let Some(t) = title_override {
        kvs.push(("title", t));
    }
    let body = library::rewrite_front_matter(&template.raw_text, &kvs);

    let effective_title = title_override
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            if template.title.trim().is_empty() {
                None
            } else {
                Some(template.title.clone())
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "template `{name}` has no title — pass one explicitly: \
                 `reptr add finding \"My title\" --from {name}`"
            )
        })?;
    Ok((effective_title, body))
}

fn next_finding_number(findings_dir: &Path) -> Result<u32> {
    let mut max = 0u32;
    for entry in WalkDir::new(findings_dir)
        .max_depth(1)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str() else {
            continue;
        };
        let Some(prefix) = name.split('-').next() else {
            continue;
        };
        if let Ok(n) = prefix.parse::<u32>() {
            if n > max {
                max = n;
            }
        }
    }
    Ok(max + 1)
}

fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut prev_dash = true;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}
