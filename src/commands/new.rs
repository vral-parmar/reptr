use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use console::style;

const REPTR_TOML_TEMPLATE: &str = r#"[engagement]
name = "{name}"
slug = "{slug}"
type = "External Web Application Penetration Test"
start_date = ""
end_date = ""
report_version = "1.0"

[client]
file = "client.toml"

[output]
formats = ["html", "json"]
directory = "output"

# Uncomment to enforce open-finding limits during build (useful in CI):
# [severity_thresholds]
# critical = 0   # fail if any critical finding is open
# high     = 5   # fail if more than 5 high findings are open
"#;

const CLIENT_TOML_TEMPLATE: &str = r#"name = "Acme Corp"
contact = "Jane Doe"
email = "security@acme.example"
"#;

const EXAMPLE_FINDING: &str = r#"---
id: F-001
title: Example Finding — Replace Me
severity: medium
cvss: "5.3"
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:L/I:N/A:N"
cwe: "CWE-200"
owasp: "A01:2021"
status: open
affected_assets:
  - https://app.example.com/
tags: [example]
---

## Description

This is an example finding shipped with `reptr new`. Replace it with your own.

## Proof of Concept

```
example request / response goes here
```

## Impact

What an attacker can do because of this issue.

## Remediation

How to fix it.

## References

- https://owasp.org/Top10/
"#;

pub fn run(slug: &str) -> Result<()> {
    if slug.trim().is_empty() {
        bail!("engagement slug cannot be empty");
    }

    let root = Path::new(slug);
    if root.exists() {
        bail!(
            "`{}` already exists — refusing to overwrite",
            root.display()
        );
    }

    fs::create_dir_all(root.join("findings"))?;
    fs::create_dir_all(root.join("templates"))?;
    fs::create_dir_all(root.join("assets/screenshots"))?;
    fs::create_dir_all(root.join("output"))?;

    let name = humanize(slug);
    let reptr_toml = REPTR_TOML_TEMPLATE
        .replace("{name}", &name)
        .replace("{slug}", slug);
    fs::write(root.join("reptr.toml"), reptr_toml).with_context(|| "writing reptr.toml")?;
    fs::write(root.join("client.toml"), CLIENT_TOML_TEMPLATE)
        .with_context(|| "writing client.toml")?;
    fs::write(
        root.join("findings/001-example-finding.md"),
        EXAMPLE_FINDING,
    )
    .with_context(|| "writing example finding")?;

    println!("{} {}/", style("Created").green().bold(), slug);
    println!("  ├── reptr.toml");
    println!("  ├── client.toml");
    println!("  ├── findings/");
    println!("  │   └── 001-example-finding.md");
    println!("  ├── templates/");
    println!("  ├── assets/screenshots/");
    println!("  └── output/");
    println!();
    println!("Next: cd {slug} && reptr build");
    Ok(())
}

fn humanize(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
