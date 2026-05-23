use std::path::Path;

use anyhow::{bail, Result};
use console::style;

use crate::model::{validate_engagement, Engagement};
use crate::parse::{load_engagement_config, load_findings};
use crate::render::render_all;

pub fn run(root: &Path) -> Result<()> {
    let started = std::time::Instant::now();

    let (cfg, client) = load_engagement_config(root)?;
    let findings = load_findings(&root.join("findings"))?;
    println!("{} {} findings", style("✓ Parsed").green(), findings.len());

    let mut engagement = Engagement {
        meta: cfg.engagement,
        client,
        findings,
        appendices: vec![],
        output: cfg.output,
        template: cfg.template,
        severity_thresholds: cfg.severity_thresholds,
        library: cfg.library,
    };
    engagement.sort_findings();

    let errors = validate_engagement(&engagement);
    if !errors.is_empty() {
        eprintln!("{}", style("✗ Validation failed:").red().bold());
        for err in &errors {
            eprintln!("  • {err}");
        }
        bail!("{} validation error(s)", errors.len());
    }

    let produced = render_all(root, &engagement)?;
    for path in &produced {
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_uppercase();
        println!(
            "{} {:<5} → {}",
            style("✓ Rendered").green(),
            ext,
            path.display()
        );
    }

    let elapsed = started.elapsed();
    println!("Done in {:.1?}.", elapsed);
    Ok(())
}
