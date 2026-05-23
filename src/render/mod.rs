pub mod docx;
pub mod html;
pub mod json;
pub mod pdf;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::Engagement;

/// Render every format listed in `engagement.output.formats` into
/// `<root>/<output_dir>/<slug>.<ext>`. Unknown formats are reported via
/// `tracing::warn!` rather than failing the build.
pub fn render_all(root: &Path, engagement: &Engagement) -> Result<Vec<PathBuf>> {
    let out_dir = root.join(&engagement.output.directory);
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating output dir {}", out_dir.display()))?;

    let slug = &engagement.meta.slug;
    let mut produced = Vec::new();

    for format in &engagement.output.formats {
        match format.as_str() {
            "html" => {
                let path = out_dir.join(format!("{slug}.html"));
                let body = html::render(root, engagement)?;
                std::fs::write(&path, body)
                    .with_context(|| format!("writing {}", path.display()))?;
                produced.push(path);
            }
            "json" => {
                let path = out_dir.join(format!("{slug}.json"));
                let body = json::render(engagement)?;
                std::fs::write(&path, body)
                    .with_context(|| format!("writing {}", path.display()))?;
                produced.push(path);
            }
            "docx" => {
                let path = out_dir.join(format!("{slug}.docx"));
                let bytes = docx::render(engagement)?;
                std::fs::write(&path, bytes)
                    .with_context(|| format!("writing {}", path.display()))?;
                produced.push(path);
            }
            "pdf" => {
                let path = pdf::render(root, &out_dir, engagement)?;
                produced.push(path);
            }
            other => {
                tracing::warn!(format = other, "unknown output format — skipping");
            }
        }
    }

    Ok(produced)
}
