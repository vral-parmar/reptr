//! DOCX renderer.
//!
//! Per build plan §10 ("Hardest Parts"), this aims for "opens cleanly in Word,
//! LibreOffice, and Google Docs" rather than perfect layout. The body Markdown
//! is rendered with a small block-level converter (headings, paragraphs, code
//! blocks, bullet lists) — full Markdown fidelity is out of scope for v0.2.

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
use docx_rs::*;
use regex::Regex;

use crate::model::{Engagement, Finding, ImageRef, Severity};

/// One inch = 914,400 EMU (English Metric Units, used throughout OOXML).
const EMU_PER_INCH: u32 = 914_400;
/// Default max image width in EMU — 5.5" fits a US-letter page with 1" margins.
const MAX_IMAGE_WIDTH_EMU: u32 = 5_486_400;

pub fn render(engagement: &Engagement) -> Result<Vec<u8>> {
    let mut doc = Docx::new();

    doc = add_cover(doc, engagement);
    doc = add_severity_summary(doc, engagement);
    doc = add_findings_overview(doc, engagement);

    for finding in &engagement.findings {
        doc = add_finding(doc, finding);
    }

    let mut buffer = Vec::with_capacity(64 * 1024);
    doc.build().pack(&mut std::io::Cursor::new(&mut buffer))?;
    Ok(buffer)
}

// --- top-level sections --------------------------------------------------

fn add_cover(mut doc: Docx, eng: &Engagement) -> Docx {
    doc = doc.add_paragraph(heading(&eng.meta.name, "Heading1"));
    if !eng.meta.kind.is_empty() {
        doc = doc.add_paragraph(plain_paragraph(&eng.meta.kind));
    }

    let mut rows = Vec::new();
    if !eng.client.name.is_empty() {
        rows.push(("Client", eng.client.name.clone()));
    }
    if let Some(start) = eng.meta.start_date.as_deref().filter(|s| !s.is_empty()) {
        rows.push(("Start", start.to_string()));
    }
    if let Some(end) = eng.meta.end_date.as_deref().filter(|s| !s.is_empty()) {
        rows.push(("End", end.to_string()));
    }
    rows.push(("Version", eng.meta.report_version.clone()));

    if !rows.is_empty() {
        doc = doc.add_table(metadata_table(rows));
    }
    doc
}

fn add_severity_summary(mut doc: Docx, eng: &Engagement) -> Docx {
    doc = doc.add_paragraph(heading("Executive Summary", "Heading2"));

    let header = TableRow::new(vec![header_cell("Severity"), header_cell("Count")]);
    let mut rows = vec![header];
    for (sev, count) in eng.severity_counts() {
        rows.push(TableRow::new(vec![
            body_cell(sev.as_str()),
            body_cell(&count.to_string()),
        ]));
    }
    let table = Table::new(rows)
        .set_grid(vec![2200, 1500])
        .width(8000, WidthType::Dxa);
    doc.add_table(table)
}

fn add_findings_overview(mut doc: Docx, eng: &Engagement) -> Docx {
    doc = doc.add_paragraph(heading("Findings Overview", "Heading2"));
    if eng.findings.is_empty() {
        return doc.add_paragraph(plain_paragraph("(no findings)"));
    }

    let header = TableRow::new(vec![
        header_cell("ID"),
        header_cell("Severity"),
        header_cell("Title"),
        header_cell("Status"),
    ]);
    let mut rows = vec![header];
    for f in &eng.findings {
        rows.push(TableRow::new(vec![
            body_cell(&f.id),
            body_cell(f.severity.as_str()),
            body_cell(&f.title),
            body_cell(f.status.as_str()),
        ]));
    }
    let table = Table::new(rows)
        .set_grid(vec![1100, 1300, 5000, 1200])
        .width(8000, WidthType::Dxa);
    doc.add_table(table)
}

fn add_finding(mut doc: Docx, f: &Finding) -> Docx {
    doc = doc.add_paragraph(heading(&format!("{} — {}", f.id, f.title), "Heading2"));
    doc = doc.add_paragraph(badges_line(f));

    if !f.affected_assets.is_empty() {
        doc = doc.add_paragraph(plain_paragraph(&format!(
            "Affected: {}",
            f.affected_assets.join(", ")
        )));
    }
    if !f.tags.is_empty() {
        doc = doc.add_paragraph(plain_paragraph(&format!("Tags: {}", f.tags.join(", "))));
    }

    for block in markdown_to_blocks(&f.body_markdown) {
        doc = apply_block(doc, block, f);
    }
    doc
}

// --- markdown → docx blocks (lightweight) -------------------------------

enum DocBlock {
    Heading(usize, String),
    Para(String),
    BulletItem(String),
    Code(String),
    /// A line that consists solely of an image reference, e.g. `![alt](path)`.
    Image {
        alt: String,
        markdown_src: String,
    },
}

fn image_line_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"^\s*!\[(?P<alt>[^\]]*)\]\((?P<src>[^)\s]+)(?:\s+"[^"]*")?\)\s*$"#).unwrap()
    })
}

fn markdown_to_blocks(md: &str) -> Vec<DocBlock> {
    let mut out = Vec::new();
    let mut para_buf = String::new();
    let mut in_code = false;
    let mut code_buf = String::new();

    let flush_para = |buf: &mut String, out: &mut Vec<DocBlock>| {
        let trimmed = buf.trim();
        if !trimmed.is_empty() {
            out.push(DocBlock::Para(trimmed.to_string()));
        }
        buf.clear();
    };

    for line in md.lines() {
        if let Some(rest) = line.strip_prefix("```") {
            // toggle code block
            if in_code {
                out.push(DocBlock::Code(code_buf.clone()));
                code_buf.clear();
                in_code = false;
            } else {
                flush_para(&mut para_buf, &mut out);
                in_code = true;
                let _ = rest; // language hint ignored
            }
            continue;
        }
        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }

        if let Some(rest) = line.strip_prefix("###### ") {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Heading(6, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("##### ") {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Heading(5, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("#### ") {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Heading(4, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("### ") {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Heading(3, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("## ") {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Heading(2, rest.to_string()));
        } else if let Some(rest) = line.strip_prefix("# ") {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Heading(1, rest.to_string()));
        } else if let Some(cap) = image_line_regex().captures(line) {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::Image {
                alt: cap["alt"].to_string(),
                markdown_src: cap["src"].to_string(),
            });
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            flush_para(&mut para_buf, &mut out);
            out.push(DocBlock::BulletItem(rest.to_string()));
        } else if line.trim().is_empty() {
            flush_para(&mut para_buf, &mut out);
        } else {
            if !para_buf.is_empty() {
                para_buf.push(' ');
            }
            para_buf.push_str(line.trim());
        }
    }
    if in_code && !code_buf.is_empty() {
        out.push(DocBlock::Code(code_buf));
    }
    flush_para(&mut para_buf, &mut out);
    out
}

fn apply_block(doc: Docx, block: DocBlock, finding: &Finding) -> Docx {
    match block {
        DocBlock::Heading(level, text) => {
            // Inside a finding we already used Heading2 for the title; nest
            // deeper for any markdown headings inside the finding body.
            let style = match level {
                1 | 2 => "Heading3",
                3 => "Heading4",
                4 => "Heading5",
                _ => "Heading6",
            };
            doc.add_paragraph(heading(&text, style))
        }
        DocBlock::Para(text) => doc.add_paragraph(plain_paragraph(&text)),
        DocBlock::BulletItem(text) => doc.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(format!("• {text}")))
                .indent(Some(360), None, None, None),
        ),
        DocBlock::Code(text) => doc.add_paragraph(code_paragraph(&text)),
        DocBlock::Image { alt, markdown_src } => image_paragraph(doc, finding, &alt, &markdown_src),
    }
}

fn image_paragraph(doc: Docx, finding: &Finding, alt: &str, src: &str) -> Docx {
    let Some(image) = lookup_image(finding, src) else {
        // Unknown image — fall back to inline text so the user notices.
        return doc.add_paragraph(plain_paragraph(&format!("[image: {src}]")));
    };

    let Some(path) = image.resolved_path.as_deref() else {
        tracing::warn!(src = src, "remote images aren't embedded in DOCX yet");
        return doc.add_paragraph(plain_paragraph(&format!("[remote image: {src}]")));
    };

    match load_pic(path) {
        Ok(pic) => doc.add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_image(pic)),
        ),
        Err(e) => {
            tracing::warn!(src = src, error = %e, "could not embed image, falling back to text");
            doc.add_paragraph(plain_paragraph(&format!(
                "[image missing: {src}{}]",
                if alt.is_empty() {
                    String::new()
                } else {
                    format!(" — {alt}")
                }
            )))
        }
    }
}

fn lookup_image<'a>(finding: &'a Finding, src: &str) -> Option<&'a ImageRef> {
    finding.images.iter().find(|i| i.markdown_src == src)
}

fn load_pic(path: &Path) -> Result<Pic> {
    let bytes = std::fs::read(path)?;
    let (w_px, h_px) = image::image_dimensions(path)?;
    let (w_emu, h_emu) = scale_to_max_width_emu(w_px, h_px, MAX_IMAGE_WIDTH_EMU);
    Ok(Pic::new(&bytes).size(w_emu, h_emu))
}

fn scale_to_max_width_emu(w_px: u32, h_px: u32, max_w_emu: u32) -> (u32, u32) {
    // Treat the source as 96 DPI — Word/LibreOffice both honor explicit EMU
    // dimensions so this is just how we pick a default.
    const PX_TO_EMU: u32 = EMU_PER_INCH / 96;
    let mut w = w_px.saturating_mul(PX_TO_EMU);
    let mut h = h_px.saturating_mul(PX_TO_EMU);
    if w > max_w_emu && w > 0 {
        h = ((h as u64 * max_w_emu as u64) / w as u64) as u32;
        w = max_w_emu;
    }
    (w.max(1), h.max(1))
}

// --- low-level helpers ---------------------------------------------------

fn heading(text: &str, style: &str) -> Paragraph {
    Paragraph::new()
        .style(style)
        .add_run(Run::new().add_text(text.to_string()))
}

fn plain_paragraph(text: &str) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text.to_string()))
}

fn code_paragraph(text: &str) -> Paragraph {
    let body = text.trim_end_matches('\n');
    let fonts = RunFonts::new().ascii("Consolas").hi_ansi("Consolas");
    let mut para = Paragraph::new();
    let mut first = true;
    for line in body.split('\n') {
        let mut run = Run::new().fonts(fonts.clone());
        if !first {
            run = run.add_break(BreakType::TextWrapping);
        }
        first = false;
        run = run.add_text(line.to_string());
        para = para.add_run(run);
    }
    para
}

fn badges_line(f: &Finding) -> Paragraph {
    let mut parts = vec![format!("Severity: {}", f.severity.as_str().to_uppercase())];
    if let Some(cvss) = &f.cvss {
        parts.push(format!("CVSS: {cvss}"));
    }
    if let Some(cwe) = &f.cwe {
        parts.push(format!("CWE: {cwe}"));
    }
    if let Some(owasp) = &f.owasp {
        parts.push(format!("OWASP: {owasp}"));
    }
    parts.push(format!("Status: {}", f.status.as_str()));

    let color = severity_color(f.severity);
    Paragraph::new().add_run(Run::new().add_text(parts.join(" · ")).color(color))
}

fn severity_color(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "B00020",
        Severity::High => "C2410C",
        Severity::Medium => "B45309",
        Severity::Low => "2563EB",
        Severity::Info => "4B5563",
    }
}

fn header_cell(text: &str) -> TableCell {
    TableCell::new()
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text(text.to_string()).bold()))
}

fn body_cell(text: &str) -> TableCell {
    TableCell::new().add_paragraph(plain_paragraph(text))
}

fn metadata_table(rows: Vec<(&'static str, String)>) -> Table {
    let trs: Vec<TableRow> = rows
        .into_iter()
        .map(|(k, v)| TableRow::new(vec![header_cell(k), body_cell(&v)]))
        .collect();
    Table::new(trs)
        .set_grid(vec![1800, 6200])
        .width(8000, WidthType::Dxa)
}
