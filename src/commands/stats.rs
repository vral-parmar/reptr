//! `reptr stats` — multi-engagement summary across a parent directory.
//!
//! Walks the immediate children of `path`. Any subdirectory containing a
//! `reptr.toml` is treated as an engagement; we parse it, count findings by
//! severity and open/resolved status, and produce a column-aligned table or
//! a stable JSON document.
//!
//! If `path` itself contains a `reptr.toml`, that engagement is included too
//! — so `reptr stats` works from inside a single engagement dir as well.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use console::style;
use serde::Serialize;

use crate::model::{Engagement, Severity, Status};
use crate::parse::{load_engagement_config, load_findings};

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Text,
    Json,
}

pub fn run(path: &Path, format: Format) -> Result<()> {
    let (rows, totals) = collect_stats(path)?;
    match format {
        Format::Text => print_text(path, &rows, &totals),
        Format::Json => print_json(&rows, &totals)?,
    }
    Ok(())
}

/// Discover and aggregate. Exposed so tests can verify the numbers without
/// having to parse the rendered text output.
pub fn collect_stats(path: &Path) -> Result<(Vec<StatsRow>, StatsTotals)> {
    let engagement_dirs = discover_engagements(path)?;
    if engagement_dirs.is_empty() {
        bail!(
            "no engagements found under {} (looking for reptr.toml)",
            path.display()
        );
    }

    let mut rows = Vec::with_capacity(engagement_dirs.len());
    for dir in &engagement_dirs {
        match collect_one(dir) {
            Ok(row) => rows.push(row),
            Err(e) => {
                eprintln!(
                    "{} {}: {:#}",
                    style("skipped").yellow().bold(),
                    dir.display(),
                    e
                );
            }
        }
    }
    if rows.is_empty() {
        bail!("every candidate engagement failed to parse");
    }

    // Sort: most-critical-first, then by slug.
    rows.sort_by(|a, b| {
        Reverse(a.counts.critical)
            .cmp(&Reverse(b.counts.critical))
            .then_with(|| Reverse(a.counts.high).cmp(&Reverse(b.counts.high)))
            .then_with(|| a.slug.cmp(&b.slug))
    });

    let totals = totals(&rows);
    Ok((rows, totals))
}

// --- model -----------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct StatsRow {
    pub slug: String,
    pub name: String,
    pub path: PathBuf,
    pub counts: SeverityCounts,
    pub total: usize,
    pub open: usize,
    pub resolved: usize,
}

#[derive(Debug, Serialize, Default, Clone, Copy)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

#[derive(Debug, Serialize)]
pub struct StatsTotals {
    pub engagements: usize,
    pub counts: SeverityCounts,
    pub total: usize,
    pub open: usize,
    pub resolved: usize,
}

#[derive(Debug, Serialize)]
struct StatsReport<'a> {
    engagements: &'a [StatsRow],
    totals: &'a StatsTotals,
}

// --- discovery -------------------------------------------------------------

fn discover_engagements(path: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if path.join("reptr.toml").is_file() {
        out.push(path.to_path_buf());
    }
    let entries =
        std::fs::read_dir(path).with_context(|| format!("reading directory {}", path.display()))?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() && p.join("reptr.toml").is_file() {
            out.push(p);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn collect_one(dir: &Path) -> Result<StatsRow> {
    let (cfg, client) = load_engagement_config(dir)?;
    let findings = load_findings(&dir.join("findings"))?;
    let engagement = Engagement {
        meta: cfg.engagement,
        client,
        findings,
        appendices: vec![],
        output: cfg.output,
        template: cfg.template,
        severity_thresholds: cfg.severity_thresholds,
        library: cfg.library,
    };

    let mut counts = SeverityCounts::default();
    let mut open = 0;
    let mut resolved = 0;
    for f in &engagement.findings {
        match f.severity {
            Severity::Critical => counts.critical += 1,
            Severity::High => counts.high += 1,
            Severity::Medium => counts.medium += 1,
            Severity::Low => counts.low += 1,
            Severity::Info => counts.info += 1,
        }
        match f.status {
            Status::Open => open += 1,
            Status::Resolved | Status::Accepted | Status::FalsePositive => resolved += 1,
        }
    }

    Ok(StatsRow {
        slug: engagement.meta.slug,
        name: engagement.meta.name,
        path: dir.to_path_buf(),
        counts,
        total: engagement.findings.len(),
        open,
        resolved,
    })
}

fn totals(rows: &[StatsRow]) -> StatsTotals {
    let mut t = StatsTotals {
        engagements: rows.len(),
        counts: SeverityCounts::default(),
        total: 0,
        open: 0,
        resolved: 0,
    };
    for r in rows {
        t.counts.critical += r.counts.critical;
        t.counts.high += r.counts.high;
        t.counts.medium += r.counts.medium;
        t.counts.low += r.counts.low;
        t.counts.info += r.counts.info;
        t.total += r.total;
        t.open += r.open;
        t.resolved += r.resolved;
    }
    t
}

// --- text output -----------------------------------------------------------

fn print_text(path: &Path, rows: &[StatsRow], totals: &StatsTotals) {
    let slug_w = rows
        .iter()
        .map(|r| r.slug.chars().count())
        .max()
        .unwrap_or(8)
        .max(8);

    println!(
        "{} {} ({} engagement{})",
        style("Engagements under").dim(),
        style(path.display()).bold(),
        rows.len(),
        if rows.len() == 1 { "" } else { "s" }
    );
    println!();

    // Header
    let header = format!(
        "  {slug:<sw$}  {crit:>5} {high:>5} {med:>5} {low:>5} {info:>5}  {total:>5}  {open:>5}",
        slug = "engagement",
        sw = slug_w,
        crit = style("crit").red().bold(),
        high = style("high").color256(208).bold(), // orange
        med = style("med").yellow().bold(),
        low = style("low").blue().bold(),
        info = style("info").dim().bold(),
        total = "total",
        open = "open",
    );
    println!("{header}");

    for r in rows {
        println!(
            "  {slug:<sw$}  {crit:>5} {high:>5} {med:>5} {low:>5} {info:>5}  {total:>5}  {open:>5}",
            slug = r.slug,
            sw = slug_w,
            crit = r.counts.critical,
            high = r.counts.high,
            med = r.counts.medium,
            low = r.counts.low,
            info = r.counts.info,
            total = r.total,
            open = r.open,
        );
    }

    println!("  {}", "─".repeat(slug_w + 50));
    println!(
        "  {slug:<sw$}  {crit:>5} {high:>5} {med:>5} {low:>5} {info:>5}  {total:>5}  {open:>5}",
        slug = style("TOTAL").bold(),
        sw = slug_w,
        crit = style(totals.counts.critical).bold(),
        high = style(totals.counts.high).bold(),
        med = style(totals.counts.medium).bold(),
        low = style(totals.counts.low).bold(),
        info = style(totals.counts.info).bold(),
        total = style(totals.total).bold(),
        open = style(totals.open).bold(),
    );
}

fn print_json(rows: &[StatsRow], totals: &StatsTotals) -> Result<()> {
    let report = StatsReport {
        engagements: rows,
        totals,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
