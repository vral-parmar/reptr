# Quick start

This walks you from zero to a rendered report in under five minutes.

## 1. Scaffold a new engagement

```bash
reptr new acme-webapp-2026
cd acme-webapp-2026
```

This creates:

```
acme-webapp-2026/
├── reptr.toml              # engagement metadata and output config
├── client.toml             # client name and contacts
├── findings/
│   └── 001-example-finding.md
├── templates/              # drop custom HTML templates here
├── assets/screenshots/     # reference images from finding bodies
└── output/                 # written by reptr build — do not commit
```

## 2. Add findings

```bash
reptr add finding "SQL Injection in Login Form" --severity critical
reptr add finding "Missing Security Headers"    --severity low
```

Each command creates a numbered Markdown stub:

```
findings/
├── 001-example-finding.md
├── 002-sql-injection-in-login-form.md
└── 003-missing-security-headers.md
```

## 3. Edit a finding

Open any finding in your editor:

```bash
$EDITOR findings/002-sql-injection-in-login-form.md
```

Fill in the front matter and body:

```markdown
---
id: F-002
title: SQL Injection in Login Form
severity: critical
cvss: "9.8"
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
cwe: "CWE-89"
owasp: "A03:2021"
status: open
affected_assets:
  - https://app.example.com/login
tags: [injection, p1]
---

## Description

The `/login` endpoint is vulnerable to SQL injection via the `username` parameter.

## Proof of Concept

...

## Impact / Remediation / References
```

## 4. Build the report

```bash
reptr build
# ✓ Parsed 3 findings
# ✓ Rendered HTML  → output/acme-webapp-2026.html
# ✓ Rendered JSON  → output/acme-webapp-2026.json
# Done in 8ms.
```

Open the report:

```bash
open output/acme-webapp-2026.html    # macOS
xdg-open output/acme-webapp-2026.html  # Linux
```

## 5. Live reload while writing

```bash
reptr watch
# Watching /path/to/acme-webapp-2026
# ... edit a finding file ...
# ✓ Rebuilt in 12ms (triggered by findings/002-sql-injection-in-login-form.md)
```

`reptr watch` rebuilds automatically whenever you save a finding, template, or config file. Keep the HTML open in a browser — just refresh after each save.

## What's next?

- Add more output formats: see [Configuration](configuration.md)
- Import findings from your library: see [reptr library](commands/library.md)
- Track remediation between assessments: see [reptr retest](commands/retest.md)
- Enforce finding limits in CI: see [Severity thresholds](thresholds.md)
