use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use console::style;
use minijinja::{context, Environment};
use serde::Serialize;

use crate::model::{Engagement, Severity, Status};
use crate::parse::load_engagement_config;

// ── Delta types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FindingDelta {
    pub id: String,
    pub title: String,
    /// Current severity (before-severity when the finding was removed).
    pub severity: Severity,
    /// "new" | "removed" | "resolved" | "regressed" | "changed" | "unchanged"
    pub change_type: String,
    /// Human-readable description of the change (e.g. "open → resolved").
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_severity: Option<Severity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_severity: Option<Severity>,
}

#[derive(Debug, Serialize)]
pub struct RetestDiff {
    pub engagement_name: String,
    pub deltas: Vec<FindingDelta>,
    pub new_count: usize,
    pub removed_count: usize,
    pub resolved_count: usize,
    pub regressed_count: usize,
    pub changed_count: usize,
    pub unchanged_count: usize,
    pub generated_at: String,
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(root: &Path) -> Result<()> {
    // Resolve slug before build so we know where to read the snapshot from.
    let (cfg, _) = load_engagement_config(root)?;
    let slug = cfg.engagement.slug.clone();
    let engagement_name = cfg.engagement.name.clone();
    let out_dir = root.join(&cfg.output.directory);
    let snapshot_path = out_dir.join(format!("{slug}.json"));

    // Read the old snapshot BEFORE build::run() overwrites it.
    let old: Option<Engagement> = if snapshot_path.exists() {
        let data = std::fs::read_to_string(&snapshot_path)
            .with_context(|| format!("reading snapshot {}", snapshot_path.display()))?;
        match serde_json::from_str::<Engagement>(&data) {
            Ok(e) => Some(e),
            Err(e) => {
                tracing::warn!(
                    "snapshot at {} could not be parsed ({}); treating as first run",
                    snapshot_path.display(),
                    e
                );
                None
            }
        }
    } else {
        None
    };

    if old.is_none() {
        println!(
            "{} No previous build found — running baseline build. \
             Run `reptr retest` again after your next round of remediation.",
            style("→").cyan()
        );
    }

    // Run the full build (prints its own progress; rewrites output/<slug>.json).
    super::build::run(root)?;

    // If there was no prior snapshot this was just a baseline — nothing to diff.
    let old = match old {
        Some(e) => e,
        None => return Ok(()),
    };

    // Load the freshly written JSON as the "after" state.
    let new_data = std::fs::read_to_string(&snapshot_path)
        .with_context(|| format!("reading updated snapshot {}", snapshot_path.display()))?;
    let new: Engagement = serde_json::from_str(&new_data)
        .context("parsing freshly built engagement JSON")?;

    let diff = compute_diff(&engagement_name, &old, &new);
    print_diff(&diff);
    write_delta_outputs(&out_dir, &slug, &diff)?;
    Ok(())
}

// ── Diff computation ──────────────────────────────────────────────────────────

fn compute_diff(engagement_name: &str, old: &Engagement, new: &Engagement) -> RetestDiff {
    let old_map: HashMap<&str, _> = old.findings.iter().map(|f| (f.id.as_str(), f)).collect();
    let new_map: HashMap<&str, _> = new.findings.iter().map(|f| (f.id.as_str(), f)).collect();

    let mut deltas: Vec<FindingDelta> = Vec::new();
    let (mut new_count, mut removed_count, mut resolved_count) = (0usize, 0, 0);
    let (mut regressed_count, mut changed_count, mut unchanged_count) = (0usize, 0, 0);

    // Findings present in the old build: may have changed or been removed.
    for (&id, old_f) in &old_map {
        if let Some(new_f) = new_map.get(id) {
            let status_same = old_f.status == new_f.status;
            let severity_same = old_f.severity == new_f.severity;

            if status_same && severity_same {
                unchanged_count += 1;
                deltas.push(FindingDelta {
                    id: id.to_string(),
                    title: new_f.title.clone(),
                    severity: new_f.severity,
                    change_type: "unchanged".to_string(),
                    label: "Unchanged".to_string(),
                    before_status: None,
                    after_status: None,
                    before_severity: None,
                    after_severity: None,
                });
            } else {
                let change_type =
                    classify_change(old_f.status, new_f.status, old_f.severity, new_f.severity);
                match change_type {
                    "resolved" => resolved_count += 1,
                    "regressed" => regressed_count += 1,
                    _ => changed_count += 1,
                }
                let label =
                    build_label(old_f.status, new_f.status, old_f.severity, new_f.severity);
                deltas.push(FindingDelta {
                    id: id.to_string(),
                    title: new_f.title.clone(),
                    severity: new_f.severity,
                    change_type: change_type.to_string(),
                    label,
                    before_status: (!status_same).then_some(old_f.status),
                    after_status: (!status_same).then_some(new_f.status),
                    before_severity: (!severity_same).then_some(old_f.severity),
                    after_severity: (!severity_same).then_some(new_f.severity),
                });
            }
        } else {
            removed_count += 1;
            deltas.push(FindingDelta {
                id: id.to_string(),
                title: old_f.title.clone(),
                severity: old_f.severity,
                change_type: "removed".to_string(),
                label: "Removed".to_string(),
                before_status: None,
                after_status: None,
                before_severity: None,
                after_severity: None,
            });
        }
    }

    // Findings present only in the new build: newly added.
    for (&id, new_f) in &new_map {
        if !old_map.contains_key(id) {
            new_count += 1;
            deltas.push(FindingDelta {
                id: id.to_string(),
                title: new_f.title.clone(),
                severity: new_f.severity,
                change_type: "new".to_string(),
                label: "New".to_string(),
                before_status: None,
                after_status: None,
                before_severity: None,
                after_severity: None,
            });
        }
    }

    // Changed findings first, then by severity desc, then by ID.
    deltas.sort_by(|a, b| {
        let a_notable = a.change_type != "unchanged";
        let b_notable = b.change_type != "unchanged";
        b_notable
            .cmp(&a_notable)
            .then_with(|| b.severity.rank().cmp(&a.severity.rank()))
            .then_with(|| a.id.cmp(&b.id))
    });

    RetestDiff {
        engagement_name: engagement_name.to_string(),
        deltas,
        new_count,
        removed_count,
        resolved_count,
        regressed_count,
        changed_count,
        unchanged_count,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn classify_change(
    before_s: Status,
    after_s: Status,
    before_sev: Severity,
    after_sev: Severity,
) -> &'static str {
    let status_changed = before_s != after_s;
    if status_changed {
        if before_s != Status::Resolved && after_s == Status::Resolved {
            return "resolved";
        }
        if before_s == Status::Resolved && after_s == Status::Open {
            return "regressed";
        }
    }
    if status_changed || before_sev != after_sev {
        return "changed";
    }
    "unchanged"
}

fn build_label(
    before_s: Status,
    after_s: Status,
    before_sev: Severity,
    after_sev: Severity,
) -> String {
    let mut parts = Vec::new();
    if before_s != after_s {
        parts.push(format!("{} → {}", before_s.as_str(), after_s.as_str()));
    }
    if before_sev != after_sev {
        parts.push(format!("sev: {} → {}", before_sev.as_str(), after_sev.as_str()));
    }
    parts.join(" / ")
}

// ── Terminal output ───────────────────────────────────────────────────────────

fn print_diff(diff: &RetestDiff) {
    println!();
    println!("{}", style("── Retest Delta ─────────────────────────────────────────").dim());
    println!(
        "  {}  ·  {}  ·  {}  ·  {}  ·  {}  ·  {}",
        format!("{} new", style(diff.new_count).cyan()),
        format!("{} resolved", style(diff.resolved_count).green()),
        if diff.regressed_count > 0 {
            format!("{} regressed", style(diff.regressed_count).red().bold())
        } else {
            format!("{} regressed", style(diff.regressed_count).dim())
        },
        format!("{} changed", style(diff.changed_count).yellow()),
        format!("{} removed", style(diff.removed_count).dim()),
        format!("{} unchanged", style(diff.unchanged_count).dim()),
    );

    let notable: Vec<&FindingDelta> =
        diff.deltas.iter().filter(|d| d.change_type != "unchanged").collect();

    if notable.is_empty() {
        println!("  No changes since last build.");
        return;
    }

    println!();
    for d in notable {
        let change = match d.change_type.as_str() {
            "new" => style(format!("+ NEW")).cyan().to_string(),
            "removed" => style("− REMOVED").dim().to_string(),
            "resolved" => style(format!("✓ {}", d.label)).green().to_string(),
            "regressed" => style(format!("↓ {}", d.label)).red().bold().to_string(),
            _ => style(format!("~ {}", d.label)).yellow().to_string(),
        };
        let sev_char = d.severity.as_str().chars().next().unwrap_or('?').to_uppercase().to_string();
        println!(
            "  [{}] {}  {}  {}",
            style(sev_char).bold(),
            style(&d.id).dim(),
            d.title,
            change,
        );
    }
}

// ── File output ───────────────────────────────────────────────────────────────

fn write_delta_outputs(out_dir: &Path, slug: &str, diff: &RetestDiff) -> Result<()> {
    std::fs::create_dir_all(out_dir)
        .with_context(|| format!("creating output dir {}", out_dir.display()))?;

    let json_path = out_dir.join(format!("{slug}-retest.json"));
    let json_body = serde_json::to_string_pretty(diff)?;
    std::fs::write(&json_path, json_body)
        .with_context(|| format!("writing {}", json_path.display()))?;
    println!(
        "{} {:<10} → {}",
        style("✓ Rendered").green(),
        "RETEST JSON",
        json_path.display()
    );

    let html_path = out_dir.join(format!("{slug}-retest.html"));
    let html_body = render_html(diff)?;
    std::fs::write(&html_path, html_body)
        .with_context(|| format!("writing {}", html_path.display()))?;
    println!(
        "{} {:<10} → {}",
        style("✓ Rendered").green(),
        "RETEST HTML",
        html_path.display()
    );

    Ok(())
}

// ── HTML template ─────────────────────────────────────────────────────────────

const RETEST_TEMPLATE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Retest Report — {{ engagement_name }}</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: system-ui, -apple-system, sans-serif; color: #111; background: #f8f9fa; }
  .page { max-width: 900px; margin: 2rem auto; padding: 0 1.5rem; }
  h1 { font-size: 1.6rem; margin-bottom: .2rem; }
  .subtitle { color: #6b7280; font-size: .85rem; margin-bottom: 2rem; }
  .summary { display: flex; gap: 1rem; flex-wrap: wrap; margin-bottom: 2rem; }
  .card { background: #fff; border: 1px solid #e5e7eb; border-radius: 8px; padding: .75rem 1.25rem; text-align: center; min-width: 90px; }
  .card .num { font-size: 1.8rem; font-weight: 700; line-height: 1.1; }
  .card .lbl { font-size: .7rem; text-transform: uppercase; letter-spacing: .06em; color: #9ca3af; margin-top: .2rem; }
  .c-new .num    { color: #2563eb; }
  .c-resolved .num  { color: #16a34a; }
  .c-regressed .num { color: #dc2626; }
  .c-changed .num   { color: #d97706; }
  .c-removed .num   { color: #6b7280; }
  .c-unchanged .num { color: #d1d5db; }
  table { width: 100%; border-collapse: collapse; background: #fff;
          border-radius: 8px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,.08); }
  th { text-align: left; padding: .5rem .8rem; font-size: .72rem; text-transform: uppercase;
       letter-spacing: .06em; background: #f8fafc; border-bottom: 1px solid #e2e8f0; color: #64748b; }
  td { padding: .55rem .8rem; border-bottom: 1px solid #f1f5f9; font-size: .875rem; }
  tr:last-child td { border-bottom: none; }
  .badge { display: inline-block; padding: .15em .5em; border-radius: 4px;
           font-size: .72rem; font-weight: 600; text-transform: uppercase; letter-spacing: .03em; }
  .sev-critical { background: #fee2e2; color: #b91c1c; }
  .sev-high     { background: #ffedd5; color: #c2410c; }
  .sev-medium   { background: #fef3c7; color: #b45309; }
  .sev-low      { background: #dbeafe; color: #1d4ed8; }
  .sev-info     { background: #f1f5f9; color: #475569; }
  .tag-new       { background: #dbeafe; color: #1d4ed8; }
  .tag-removed   { background: #f1f5f9; color: #9ca3af; }
  .tag-resolved  { background: #dcfce7; color: #15803d; }
  .tag-regressed { background: #fee2e2; color: #b91c1c; }
  .tag-changed   { background: #fef3c7; color: #b45309; }
  .tag-unchanged { background: #f9fafb; color: #d1d5db; }
  footer { margin-top: 2rem; font-size: .75rem; color: #d1d5db; text-align: center; padding-bottom: 2rem; }
</style>
</head>
<body>
<div class="page">
  <h1>Retest Report</h1>
  <p class="subtitle">{{ engagement_name }} &middot; {{ generated_at }}</p>

  <div class="summary">
    <div class="card c-new"><div class="num">{{ new_count }}</div><div class="lbl">New</div></div>
    <div class="card c-resolved"><div class="num">{{ resolved_count }}</div><div class="lbl">Resolved</div></div>
    <div class="card c-regressed"><div class="num">{{ regressed_count }}</div><div class="lbl">Regressed</div></div>
    <div class="card c-changed"><div class="num">{{ changed_count }}</div><div class="lbl">Changed</div></div>
    <div class="card c-removed"><div class="num">{{ removed_count }}</div><div class="lbl">Removed</div></div>
    <div class="card c-unchanged"><div class="num">{{ unchanged_count }}</div><div class="lbl">Unchanged</div></div>
  </div>

  <table>
    <thead>
      <tr><th>ID</th><th>Severity</th><th>Title</th><th>Change</th></tr>
    </thead>
    <tbody>
      {% for d in deltas %}
      <tr>
        <td>{{ d.id }}</td>
        <td><span class="badge sev-{{ d.severity }}">{{ d.severity }}</span></td>
        <td>{{ d.title }}</td>
        <td><span class="badge tag-{{ d.change_type }}">{{ d.label }}</span></td>
      </tr>
      {% endfor %}
    </tbody>
  </table>

  <footer>Generated by reptr &middot; {{ generated_at }}</footer>
</div>
</body>
</html>"#;

fn render_html(diff: &RetestDiff) -> Result<String> {
    let mut env = Environment::new();
    env.add_template_owned("retest".to_string(), RETEST_TEMPLATE.to_string())
        .context("registering retest HTML template")?;
    let tmpl = env.get_template("retest")?;
    Ok(tmpl.render(context! {
        engagement_name => &diff.engagement_name,
        generated_at => &diff.generated_at,
        new_count => diff.new_count,
        resolved_count => diff.resolved_count,
        regressed_count => diff.regressed_count,
        changed_count => diff.changed_count,
        removed_count => diff.removed_count,
        unchanged_count => diff.unchanged_count,
        deltas => &diff.deltas,
    })?)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{
        Client, Engagement, EngagementMeta, Finding, LibraryConfig, OutputConfig,
        Severity, SeverityThresholds, Status, TemplateConfig,
    };

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn make_finding(id: &str, severity: Severity, status: Status) -> Finding {
        Finding {
            id: id.to_string(),
            title: format!("Finding {id}"),
            severity,
            cvss: None,
            cvss_vector: None,
            cwe: None,
            owasp: None,
            status,
            affected_assets: vec![],
            tags: vec![],
            body_markdown: String::new(),
            body_html: String::new(),
            source_path: PathBuf::from(format!("findings/{}.md", id.to_lowercase())),
            images: vec![],
        }
    }

    fn make_engagement(findings: Vec<Finding>) -> Engagement {
        Engagement {
            meta: EngagementMeta {
                name: "Test Engagement".to_string(),
                slug: "test-2026".to_string(),
                kind: String::new(),
                start_date: None,
                end_date: None,
                report_version: "1.0".to_string(),
            },
            client: Client::default(),
            findings,
            appendices: vec![],
            output: OutputConfig::default(),
            template: TemplateConfig::default(),
            severity_thresholds: SeverityThresholds::default(),
            library: LibraryConfig::default(),
        }
    }

    // ── classify_change ───────────────────────────────────────────────────────

    #[test]
    fn classify_open_to_resolved_is_resolved() {
        assert_eq!(
            classify_change(Status::Open, Status::Resolved, Severity::High, Severity::High),
            "resolved"
        );
    }

    #[test]
    fn classify_accepted_to_resolved_is_resolved() {
        assert_eq!(
            classify_change(Status::Accepted, Status::Resolved, Severity::Medium, Severity::Medium),
            "resolved"
        );
    }

    #[test]
    fn classify_false_positive_to_resolved_is_resolved() {
        assert_eq!(
            classify_change(
                Status::FalsePositive,
                Status::Resolved,
                Severity::Low,
                Severity::Low
            ),
            "resolved"
        );
    }

    #[test]
    fn classify_resolved_to_open_is_regressed() {
        assert_eq!(
            classify_change(Status::Resolved, Status::Open, Severity::Critical, Severity::Critical),
            "regressed"
        );
    }

    #[test]
    fn classify_open_to_accepted_is_changed() {
        assert_eq!(
            classify_change(Status::Open, Status::Accepted, Severity::High, Severity::High),
            "changed"
        );
    }

    #[test]
    fn classify_open_to_false_positive_is_changed() {
        assert_eq!(
            classify_change(
                Status::Open,
                Status::FalsePositive,
                Severity::Low,
                Severity::Low
            ),
            "changed"
        );
    }

    #[test]
    fn classify_resolved_to_accepted_is_changed() {
        // Resolved → Accepted is not a regression (accepted is not Open), just a change.
        assert_eq!(
            classify_change(
                Status::Resolved,
                Status::Accepted,
                Severity::Medium,
                Severity::Medium
            ),
            "changed"
        );
    }

    #[test]
    fn classify_severity_only_change_is_changed() {
        // Status identical — only severity differs.
        assert_eq!(
            classify_change(Status::Open, Status::Open, Severity::Critical, Severity::High),
            "changed"
        );
    }

    #[test]
    fn classify_status_to_resolved_plus_severity_change_is_resolved() {
        // When status moves to resolved, that label wins even if severity also changed.
        assert_eq!(
            classify_change(Status::Open, Status::Resolved, Severity::Critical, Severity::High),
            "resolved"
        );
    }

    #[test]
    fn classify_resolved_to_resolved_with_severity_change_is_changed() {
        // Status unchanged (both resolved), severity changed → "changed".
        assert_eq!(
            classify_change(
                Status::Resolved,
                Status::Resolved,
                Severity::Critical,
                Severity::High
            ),
            "changed"
        );
    }

    // ── build_label ───────────────────────────────────────────────────────────

    #[test]
    fn label_status_only_change() {
        let lbl = build_label(Status::Open, Status::Resolved, Severity::High, Severity::High);
        assert_eq!(lbl, "open → resolved");
    }

    #[test]
    fn label_severity_only_change() {
        let lbl =
            build_label(Status::Open, Status::Open, Severity::Critical, Severity::High);
        assert_eq!(lbl, "sev: critical → high");
    }

    #[test]
    fn label_both_status_and_severity_changed() {
        let lbl = build_label(Status::Open, Status::Resolved, Severity::Critical, Severity::High);
        assert_eq!(lbl, "open → resolved / sev: critical → high");
    }

    #[test]
    fn label_false_positive_transition() {
        let lbl =
            build_label(Status::Open, Status::FalsePositive, Severity::Low, Severity::Low);
        assert_eq!(lbl, "open → false_positive");
    }

    #[test]
    fn label_regression() {
        let lbl = build_label(Status::Resolved, Status::Open, Severity::High, Severity::High);
        assert_eq!(lbl, "resolved → open");
    }

    // ── compute_diff ──────────────────────────────────────────────────────────

    #[test]
    fn diff_no_changes_all_unchanged() {
        let old = make_engagement(vec![
            make_finding("F-001", Severity::Critical, Status::Open),
            make_finding("F-002", Severity::High, Status::Open),
        ]);
        let new = old.clone();
        let diff = compute_diff("Test", &old, &new);

        assert_eq!(diff.unchanged_count, 2);
        assert_eq!(diff.new_count, 0);
        assert_eq!(diff.removed_count, 0);
        assert_eq!(diff.resolved_count, 0);
        assert_eq!(diff.regressed_count, 0);
        assert_eq!(diff.changed_count, 0);
        assert_eq!(diff.deltas.len(), 2);
        assert!(diff.deltas.iter().all(|d| d.change_type == "unchanged"));
    }

    #[test]
    fn diff_open_to_resolved_increments_resolved_count() {
        let old = make_engagement(vec![make_finding("F-001", Severity::Critical, Status::Open)]);
        let mut new_f = old.findings.clone();
        new_f[0].status = Status::Resolved;
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);

        assert_eq!(diff.resolved_count, 1);
        assert_eq!(diff.unchanged_count, 0);

        let d = diff.deltas.iter().find(|d| d.id == "F-001").unwrap();
        assert_eq!(d.change_type, "resolved");
        assert_eq!(d.before_status, Some(Status::Open));
        assert_eq!(d.after_status, Some(Status::Resolved));
        assert_eq!(d.before_severity, None, "severity unchanged — should be None");
        assert_eq!(d.label, "open → resolved");
    }

    #[test]
    fn diff_resolved_to_open_increments_regressed_count() {
        let old =
            make_engagement(vec![make_finding("F-001", Severity::High, Status::Resolved)]);
        let mut new_f = old.findings.clone();
        new_f[0].status = Status::Open;
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);

        assert_eq!(diff.regressed_count, 1);
        let d = diff.deltas.iter().find(|d| d.id == "F-001").unwrap();
        assert_eq!(d.change_type, "regressed");
        assert_eq!(d.label, "resolved → open");
    }

    #[test]
    fn diff_accepted_to_resolved_counts_as_resolved() {
        let old =
            make_engagement(vec![make_finding("F-001", Severity::Medium, Status::Accepted)]);
        let mut new_f = old.findings.clone();
        new_f[0].status = Status::Resolved;
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);
        assert_eq!(diff.resolved_count, 1);
        assert_eq!(diff.changed_count, 0);
    }

    #[test]
    fn diff_open_to_accepted_counts_as_changed_not_resolved() {
        let old = make_engagement(vec![make_finding("F-001", Severity::Low, Status::Open)]);
        let mut new_f = old.findings.clone();
        new_f[0].status = Status::Accepted;
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);
        assert_eq!(diff.changed_count, 1);
        assert_eq!(diff.resolved_count, 0);
    }

    #[test]
    fn diff_new_finding_only_in_new_build() {
        let old = make_engagement(vec![make_finding("F-001", Severity::High, Status::Open)]);
        let mut new_f = old.findings.clone();
        new_f.push(make_finding("F-002", Severity::Critical, Status::Open));
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);

        assert_eq!(diff.new_count, 1);
        assert_eq!(diff.unchanged_count, 1);

        let d = diff.deltas.iter().find(|d| d.id == "F-002").unwrap();
        assert_eq!(d.change_type, "new");
        assert_eq!(d.severity, Severity::Critical);
        assert_eq!(d.label, "New");
        assert_eq!(d.before_status, None);
        assert_eq!(d.after_status, None);
    }

    #[test]
    fn diff_removed_finding_only_in_old_build() {
        let old = make_engagement(vec![
            make_finding("F-001", Severity::High, Status::Open),
            make_finding("F-002", Severity::Low, Status::Open),
        ]);
        let new = make_engagement(vec![make_finding("F-001", Severity::High, Status::Open)]);

        let diff = compute_diff("Test", &old, &new);

        assert_eq!(diff.removed_count, 1);
        assert_eq!(diff.unchanged_count, 1);

        let d = diff.deltas.iter().find(|d| d.id == "F-002").unwrap();
        assert_eq!(d.change_type, "removed");
        assert_eq!(d.severity, Severity::Low);
        assert_eq!(d.label, "Removed");
    }

    #[test]
    fn diff_severity_change_only() {
        let old =
            make_engagement(vec![make_finding("F-001", Severity::Critical, Status::Open)]);
        let mut new_f = old.findings.clone();
        new_f[0].severity = Severity::High;
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);

        assert_eq!(diff.changed_count, 1);
        assert_eq!(diff.resolved_count, 0);

        let d = diff.deltas.iter().find(|d| d.id == "F-001").unwrap();
        assert_eq!(d.change_type, "changed");
        assert_eq!(d.before_severity, Some(Severity::Critical));
        assert_eq!(d.after_severity, Some(Severity::High));
        assert_eq!(d.before_status, None, "status unchanged — should be None");
        assert_eq!(d.label, "sev: critical → high");
    }

    #[test]
    fn diff_status_and_severity_both_changed() {
        let old =
            make_engagement(vec![make_finding("F-001", Severity::Critical, Status::Open)]);
        let mut new_f = old.findings.clone();
        new_f[0].status = Status::Resolved;
        new_f[0].severity = Severity::High;
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);
        assert_eq!(diff.resolved_count, 1); // status→resolved wins
        let d = diff.deltas.iter().find(|d| d.id == "F-001").unwrap();
        assert_eq!(d.change_type, "resolved");
        assert_eq!(d.before_severity, Some(Severity::Critical));
        assert_eq!(d.after_severity, Some(Severity::High));
        assert!(d.label.contains("open → resolved"));
        assert!(d.label.contains("sev: critical → high"));
    }

    #[test]
    fn diff_notable_findings_sort_before_unchanged() {
        let old = make_engagement(vec![
            make_finding("F-001", Severity::High, Status::Open),
            make_finding("F-002", Severity::Medium, Status::Open),
            make_finding("F-003", Severity::Low, Status::Open),
        ]);
        let mut new_f = old.findings.clone();
        new_f[1].status = Status::Resolved; // F-002 resolved
        let new = make_engagement(new_f);

        let diff = compute_diff("Test", &old, &new);

        let notable_pos = diff.deltas.iter().position(|d| d.id == "F-002").unwrap();
        let unchanged_pos_001 = diff.deltas.iter().position(|d| d.id == "F-001").unwrap();
        let unchanged_pos_003 = diff.deltas.iter().position(|d| d.id == "F-003").unwrap();
        assert!(
            notable_pos < unchanged_pos_001,
            "resolved finding should appear before unchanged F-001"
        );
        assert!(
            notable_pos < unchanged_pos_003,
            "resolved finding should appear before unchanged F-003"
        );
    }

    #[test]
    fn diff_multiple_concurrent_changes_counted_correctly() {
        let old = make_engagement(vec![
            make_finding("F-001", Severity::Critical, Status::Open),   // → severity downgraded
            make_finding("F-002", Severity::High, Status::Resolved),   // → regressed
            make_finding("F-003", Severity::Medium, Status::Open),     // → resolved
            make_finding("F-004", Severity::Low, Status::Open),        // → unchanged
        ]);
        let new = make_engagement(vec![
            { let mut f = old.findings[0].clone(); f.severity = Severity::High; f },
            { let mut f = old.findings[1].clone(); f.status = Status::Open; f },
            { let mut f = old.findings[2].clone(); f.status = Status::Resolved; f },
            old.findings[3].clone(),
            make_finding("F-005", Severity::Info, Status::Open), // new
        ]);

        let diff = compute_diff("Test", &old, &new);
        assert_eq!(diff.new_count, 1);
        assert_eq!(diff.removed_count, 0);
        assert_eq!(diff.resolved_count, 1);
        assert_eq!(diff.regressed_count, 1);
        assert_eq!(diff.changed_count, 1);
        assert_eq!(diff.unchanged_count, 1);
        assert_eq!(diff.deltas.len(), 5);
    }

    #[test]
    fn diff_empty_old_all_findings_are_new() {
        let old = make_engagement(vec![]);
        let new = make_engagement(vec![
            make_finding("F-001", Severity::Critical, Status::Open),
            make_finding("F-002", Severity::High, Status::Open),
        ]);

        let diff = compute_diff("Test", &old, &new);
        assert_eq!(diff.new_count, 2);
        assert_eq!(diff.unchanged_count, 0);
        assert_eq!(diff.deltas.len(), 2);
        assert!(diff.deltas.iter().all(|d| d.change_type == "new"));
    }

    #[test]
    fn diff_empty_new_all_findings_removed() {
        let old = make_engagement(vec![
            make_finding("F-001", Severity::High, Status::Open),
            make_finding("F-002", Severity::Low, Status::Open),
        ]);
        let new = make_engagement(vec![]);

        let diff = compute_diff("Test", &old, &new);
        assert_eq!(diff.removed_count, 2);
        assert_eq!(diff.new_count, 0);
        assert!(diff.deltas.iter().all(|d| d.change_type == "removed"));
    }

    #[test]
    fn diff_engagement_name_is_propagated() {
        let old = make_engagement(vec![]);
        let new = make_engagement(vec![]);
        let diff = compute_diff("My Test Engagement", &old, &new);
        assert_eq!(diff.engagement_name, "My Test Engagement");
    }

    #[test]
    fn diff_generated_at_is_set() {
        let old = make_engagement(vec![]);
        let new = make_engagement(vec![]);
        let diff = compute_diff("Test", &old, &new);
        assert!(!diff.generated_at.is_empty());
        assert!(diff.generated_at.contains('T'), "should be an ISO 8601 timestamp");
    }

    // ── render_html ───────────────────────────────────────────────────────────

    fn make_diff_with_one_resolved() -> RetestDiff {
        RetestDiff {
            engagement_name: "Acme Corp".to_string(),
            deltas: vec![FindingDelta {
                id: "F-001".to_string(),
                title: "SQL Injection".to_string(),
                severity: Severity::Critical,
                change_type: "resolved".to_string(),
                label: "open → resolved".to_string(),
                before_status: Some(Status::Open),
                after_status: Some(Status::Resolved),
                before_severity: None,
                after_severity: None,
            }],
            new_count: 0,
            removed_count: 0,
            resolved_count: 1,
            regressed_count: 0,
            changed_count: 0,
            unchanged_count: 0,
            generated_at: "2026-05-23T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn html_contains_engagement_name() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(html.contains("Acme Corp"), "engagement name missing from HTML");
    }

    #[test]
    fn html_contains_finding_id_and_title() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(html.contains("F-001"));
        assert!(html.contains("SQL Injection"));
    }

    #[test]
    fn html_applies_severity_css_class() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(
            html.contains("sev-critical"),
            "severity badge class missing. html snippet: {}",
            &html[..500.min(html.len())]
        );
    }

    #[test]
    fn html_applies_change_type_css_class() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(html.contains("tag-resolved"), "change-type badge class missing");
    }

    #[test]
    fn html_shows_change_label() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(html.contains("open → resolved"));
    }

    #[test]
    fn html_shows_summary_card_counts() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        // The resolved card should contain "1"; the others should contain "0".
        assert!(html.contains("c-resolved"));
        assert!(html.contains("c-new"));
        assert!(html.contains("c-regressed"));
    }

    #[test]
    fn html_contains_table_headers() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(html.contains("Severity"));
        assert!(html.contains("Title"));
        assert!(html.contains("Change"));
    }

    #[test]
    fn html_is_well_formed_doctype() {
        let diff = make_diff_with_one_resolved();
        let html = render_html(&diff).unwrap();
        assert!(html.trim().starts_with("<!doctype html>"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn html_regressed_finding_gets_tag_regressed_class() {
        let diff = RetestDiff {
            engagement_name: "Test".to_string(),
            deltas: vec![FindingDelta {
                id: "F-001".to_string(),
                title: "XSS".to_string(),
                severity: Severity::High,
                change_type: "regressed".to_string(),
                label: "resolved → open".to_string(),
                before_status: Some(Status::Resolved),
                after_status: Some(Status::Open),
                before_severity: None,
                after_severity: None,
            }],
            new_count: 0,
            removed_count: 0,
            resolved_count: 0,
            regressed_count: 1,
            changed_count: 0,
            unchanged_count: 0,
            generated_at: "2026-05-23T00:00:00Z".to_string(),
        };
        let html = render_html(&diff).unwrap();
        assert!(html.contains("tag-regressed"));
        assert!(html.contains("resolved → open"));
    }

    #[test]
    fn html_empty_deltas_renders_empty_table() {
        let diff = RetestDiff {
            engagement_name: "Empty".to_string(),
            deltas: vec![],
            new_count: 0,
            removed_count: 0,
            resolved_count: 0,
            regressed_count: 0,
            changed_count: 0,
            unchanged_count: 0,
            generated_at: "2026-05-23T00:00:00Z".to_string(),
        };
        let html = render_html(&diff).unwrap();
        assert!(html.contains("</table>"));
        assert!(!html.contains("<td>"), "no rows expected in empty diff");
    }
}
