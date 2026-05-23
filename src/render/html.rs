use std::path::Path;

use anyhow::{Context, Result};
use minijinja::{context, Environment};
use serde::Serialize;

use crate::model::Engagement;

const DEFAULT_TEMPLATE: &str = include_str!("../../templates/report.html.tera");

#[derive(Serialize)]
struct SeverityRow {
    name: &'static str,
    count: usize,
}

pub fn render(root: &Path, engagement: &Engagement) -> Result<String> {
    let (template_name, template_source) = resolve_template(root, engagement)?;

    let mut env = Environment::new();
    env.add_template_owned(template_name.clone(), template_source)
        .with_context(|| format!("registering HTML template `{template_name}`"))?;
    let tmpl = env.get_template(&template_name)?;

    let counts: Vec<SeverityRow> = engagement
        .severity_counts()
        .into_iter()
        .map(|(sev, count)| SeverityRow {
            name: sev.as_str(),
            count,
        })
        .collect();

    let rendered = tmpl.render(context! {
        engagement => engagement,
        severity_counts => counts,
        generated_at => chrono::Utc::now().to_rfc3339(),
    })?;
    Ok(rendered)
}

/// Returns `(template_name, source)`. If `[template].html` is set in
/// `reptr.toml` and the file exists, that file's contents are used; otherwise
/// the embedded default template is returned.
fn resolve_template(root: &Path, engagement: &Engagement) -> Result<(String, String)> {
    if let Some(rel) = engagement.template.html.as_deref() {
        let path = root.join(rel);
        let source = std::fs::read_to_string(&path)
            .with_context(|| format!("reading HTML template {}", path.display()))?;
        return Ok((rel.to_string(), source));
    }
    Ok(("report".to_string(), DEFAULT_TEMPLATE.to_string()))
}
