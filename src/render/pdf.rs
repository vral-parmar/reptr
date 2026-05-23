//! PDF renderer via Typst.
//!
//! Build plan §8 (Weekend 8) calls for shelling out to the `typst` CLI rather
//! than embedding the compiler. We write a `.typ` source file next to the
//! output PDF (so users can hand-tweak it if they want) and invoke
//! `typst compile <in> <out>`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;

use crate::model::{Engagement, Finding, Severity};

pub fn render(root: &Path, out_dir: &Path, engagement: &Engagement) -> Result<PathBuf> {
    let slug = &engagement.meta.slug;
    let typ_path = out_dir.join(format!("{slug}.typ"));
    let pdf_path = out_dir.join(format!("{slug}.pdf"));

    let typ_source = build_typst_source(engagement, out_dir);
    std::fs::write(&typ_path, typ_source)
        .with_context(|| format!("writing {}", typ_path.display()))?;

    ensure_typst_available()?;

    let status = Command::new("typst")
        .arg("compile")
        // Allow image references that live anywhere under the engagement root
        // (e.g. ../assets/screenshots/foo.png from output/<slug>.typ).
        .arg("--root")
        .arg(root)
        .arg(&typ_path)
        .arg(&pdf_path)
        .current_dir(root)
        .status()
        .with_context(|| "running `typst compile`")?;

    if !status.success() {
        bail!(
            "typst exited with {:?} while compiling {} — see message above",
            status.code(),
            typ_path.display()
        );
    }
    Ok(pdf_path)
}

fn ensure_typst_available() -> Result<()> {
    let probe = Command::new("typst").arg("--version").output();
    match probe {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(anyhow!(
            "`typst --version` failed: {}",
            String::from_utf8_lossy(&o.stderr)
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(anyhow!(
            "typst is not installed. Install it with `brew install typst`, \
             `cargo install --locked typst-cli`, or from https://typst.app/, \
             then re-run `reptr build`."
        )),
        Err(e) => Err(anyhow!("could not invoke typst: {e}")),
    }
}

// --- typst source generation --------------------------------------------

fn build_typst_source(eng: &Engagement, out_dir: &Path) -> String {
    let mut out = String::new();
    let title = escape(&eng.meta.name);
    let subtitle = escape(&eng.meta.kind);
    let version = escape(&eng.meta.report_version);

    out.push_str(&format!(
        r#"#set document(title: "{title}")
#set page(paper: "us-letter", margin: (x: 1in, y: 1in))
#set text(font: ("Helvetica Neue", "Helvetica", "Arial"), size: 11pt)
#set heading(numbering: none)

#show heading.where(level: 1): set text(size: 22pt, weight: "bold")
#show heading.where(level: 2): set text(size: 16pt, weight: "bold")
#show heading.where(level: 3): set text(size: 13pt, weight: "bold")

= {title}

_{subtitle}_

#v(0.5em)
"#,
        title = title,
        subtitle = subtitle,
    ));

    // metadata
    out.push_str("#table(\n  columns: (auto, 1fr),\n  stroke: none,\n  inset: 4pt,\n");
    if !eng.client.name.is_empty() {
        out.push_str(&format!("  [*Client*], [{}],\n", escape(&eng.client.name)));
    }
    if let Some(s) = eng.meta.start_date.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("  [*Start*], [{}],\n", escape(s)));
    }
    if let Some(e) = eng.meta.end_date.as_deref().filter(|s| !s.is_empty()) {
        out.push_str(&format!("  [*End*], [{}],\n", escape(e)));
    }
    out.push_str(&format!("  [*Version*], [{version}],\n"));
    out.push_str(")\n\n#pagebreak()\n\n");

    // exec summary
    out.push_str("== Executive Summary\n\n");
    out.push_str(
        "#table(\n  columns: (auto, auto),\n  inset: 6pt,\n  stroke: 0.6pt + luma(80%),\n  [*Severity*], [*Count*],\n",
    );
    for (sev, count) in eng.severity_counts() {
        out.push_str(&format!(
            "  [#text(fill: {color})[{sev}]], [{count}],\n",
            color = typst_color_for(sev),
            sev = sev.as_str(),
            count = count,
        ));
    }
    out.push_str(")\n\n");

    // findings overview
    out.push_str("== Findings Overview\n\n");
    if eng.findings.is_empty() {
        out.push_str("_No findings._\n\n");
    } else {
        out.push_str(
            "#table(\n  columns: (auto, auto, 1fr, auto),\n  inset: 5pt,\n  stroke: 0.6pt + luma(80%),\n  [*ID*], [*Severity*], [*Title*], [*Status*],\n",
        );
        for f in &eng.findings {
            out.push_str(&format!(
                "  [{id}], [#text(fill: {color})[{sev}]], [{title}], [{status}],\n",
                id = escape(&f.id),
                color = typst_color_for(f.severity),
                sev = f.severity.as_str(),
                title = escape(&f.title),
                status = f.status.as_str(),
            ));
        }
        out.push_str(")\n\n");
    }

    // findings detail
    out.push_str("#pagebreak()\n\n== Findings Detail\n\n");
    for f in &eng.findings {
        out.push_str(&render_finding(f, out_dir));
        out.push('\n');
    }
    out
}

fn render_finding(f: &Finding, out_dir: &Path) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "=== {id} — {title}\n\n",
        id = escape(&f.id),
        title = escape(&f.title),
    ));

    let mut badges = vec![format!(
        "*Severity:* #text(fill: {})[{}]",
        typst_color_for(f.severity),
        f.severity.as_str().to_uppercase()
    )];
    if let Some(cvss) = &f.cvss {
        badges.push(format!("*CVSS:* {}", escape(cvss)));
    }
    if let Some(cwe) = &f.cwe {
        badges.push(format!("*CWE:* {}", escape(cwe)));
    }
    if let Some(owasp) = &f.owasp {
        badges.push(format!("*OWASP:* {}", escape(owasp)));
    }
    badges.push(format!("*Status:* {}", f.status.as_str()));
    out.push_str(&badges.join(" · "));
    out.push_str("\n\n");

    if !f.affected_assets.is_empty() {
        let assets: Vec<String> = f.affected_assets.iter().map(|s| escape(s)).collect();
        out.push_str(&format!("*Affected:* {}\n\n", assets.join(", ")));
    }
    if !f.tags.is_empty() {
        out.push_str(&format!("*Tags:* {}\n\n", f.tags.join(", ")));
    }

    out.push_str(&markdown_to_typst(&f.body_markdown, f, out_dir));
    out.push('\n');
    out
}

fn image_line_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*!\[(?P<alt>[^\]]*)\]\((?P<src>[^)\s]+)(?:\s+"[^"]*")?\)\s*$"#).unwrap()
    })
}

fn markdown_to_typst(md: &str, finding: &Finding, out_dir: &Path) -> String {
    let mut out = String::new();
    let mut in_code = false;

    for line in md.lines() {
        if line.starts_with("```") {
            if in_code {
                out.push_str("```\n\n");
                in_code = false;
            } else {
                let lang = line.trim_start_matches('`').trim();
                if lang.is_empty() {
                    out.push_str("```\n");
                } else {
                    out.push_str(&format!("```{lang}\n"));
                }
                in_code = true;
            }
            continue;
        }
        if in_code {
            out.push_str(line);
            out.push('\n');
            continue;
        }

        if let Some(cap) = image_line_regex().captures(line) {
            let src = cap["src"].to_string();
            out.push_str(&typst_image_block(finding, &src, out_dir));
            continue;
        }

        if let Some(rest) = line.strip_prefix("### ") {
            out.push_str(&format!("==== {}\n\n", escape(rest)));
        } else if let Some(rest) = line.strip_prefix("## ") {
            out.push_str(&format!("==== {}\n\n", escape(rest)));
        } else if let Some(rest) = line.strip_prefix("# ") {
            out.push_str(&format!("=== {}\n\n", escape(rest)));
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            out.push_str(&format!("- {}\n", escape(rest)));
        } else if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str(&escape(line));
            out.push('\n');
        }
    }
    if in_code {
        out.push_str("```\n");
    }
    out
}

fn typst_image_block(finding: &Finding, src: &str, out_dir: &Path) -> String {
    let Some(image) = finding.images.iter().find(|i| i.markdown_src == src) else {
        return format!("_(image not found: {})_\n\n", escape(src));
    };
    let Some(abs) = image.resolved_path.as_deref() else {
        tracing::warn!(src = src, "remote images aren't embedded in PDF yet");
        return format!("_(remote image: {})_\n\n", escape(src));
    };
    if !abs.exists() {
        tracing::warn!(path = %abs.display(), "image file missing, skipping");
        return format!(
            "_(image missing: {})_\n\n",
            escape(&abs.display().to_string())
        );
    }
    let rel = relativize(abs, out_dir).unwrap_or_else(|| abs.to_path_buf());
    // Forward-slashes are portable inside Typst string literals on every platform.
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    let escaped = rel_str.replace('"', "\\\"");
    format!("#image(\"{escaped}\", width: 90%)\n\n")
}

/// Compute `target` expressed relative to `base`. Falls back to None when the
/// paths don't share a prefix (e.g. different drives on Windows).
fn relativize(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let t: Vec<_> = target.components().collect();
    let b: Vec<_> = base.components().collect();
    let common = t.iter().zip(b.iter()).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None;
    }
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in &t[common..] {
        out.push(c.as_os_str());
    }
    Some(out)
}

fn typst_color_for(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "rgb(\"#b00020\")",
        Severity::High => "rgb(\"#c2410c\")",
        Severity::Medium => "rgb(\"#b45309\")",
        Severity::Low => "rgb(\"#2563eb\")",
        Severity::Info => "rgb(\"#4b5563\")",
    }
}

/// Escape characters that have special meaning in Typst markup. Only the
/// minimum set: we want bold/italic from our own templating to keep working,
/// so we don't escape `*` / `_`.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '#' => out.push_str("\\#"),
            '<' => out.push_str("\\<"),
            '>' => out.push_str("\\>"),
            '@' => out.push_str("\\@"),
            '[' => out.push_str("\\["),
            ']' => out.push_str("\\]"),
            '$' => out.push_str("\\$"),
            other => out.push(other),
        }
    }
    out
}
