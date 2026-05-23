use std::path::Path;

use anyhow::Result;
use console::style;

use crate::library;
use crate::parse::load_engagement_config;

pub fn list(root: &Path) -> Result<()> {
    let (cfg, _client) = load_engagement_config(root)?;
    let dir = library::resolve_library_dir(root, &cfg.library);
    let templates = library::list_templates(&dir)?;

    if templates.is_empty() {
        println!(
            "{} no templates found in {}",
            style("note:").yellow().bold(),
            dir.display()
        );
        println!();
        println!("Create one with:");
        println!(
            "  mkdir -p {} && {} {}/xss-stored.md",
            dir.display(),
            style("$EDITOR").dim(),
            dir.display()
        );
        return Ok(());
    }

    let name_w = templates
        .iter()
        .map(|t| t.name.chars().count())
        .max()
        .unwrap_or(8)
        .max(4);
    let sev_w = templates
        .iter()
        .map(|t| t.severity.chars().count())
        .max()
        .unwrap_or(8)
        .max(8);

    println!(
        "{} {} ({} template{})",
        style("Library:").dim(),
        style(dir.display()).bold(),
        templates.len(),
        if templates.len() == 1 { "" } else { "s" }
    );
    println!();
    println!(
        "  {name:<nw$}  {sev:<sw$}  {title}",
        name = style("name").bold(),
        nw = name_w,
        sev = style("severity").bold(),
        sw = sev_w,
        title = style("title").bold(),
    );
    for t in &templates {
        let sev = match t.severity.as_str() {
            "critical" => style(&t.severity).red(),
            "high" => style(&t.severity).color256(208),
            "medium" => style(&t.severity).yellow(),
            "low" => style(&t.severity).blue(),
            "info" => style(&t.severity).dim(),
            _ => style(&t.severity),
        };
        println!(
            "  {name:<nw$}  {sev:<sw$}  {title}",
            name = t.name,
            nw = name_w,
            sev = sev,
            sw = sev_w,
            title = t.title,
        );
    }
    println!();
    println!(
        "Import with: {} reptr add finding {} --from <name>",
        style("$").dim(),
        style("\"my title\"").dim()
    );
    Ok(())
}
