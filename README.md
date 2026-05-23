# `reptr`

> Local-first pentest report generator.  
> Write findings as Markdown. Run `reptr build`. Get HTML, JSON, DOCX, and PDF.  
> No Docker, no database, no SaaS.

[![CI](https://github.com/yourhandle/reptr/actions/workflows/ci.yml/badge.svg)](https://github.com/yourhandle/reptr/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/reptr.svg)](https://crates.io/crates/reptr)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

---

## Why reptr?

Most pentest report tools are web apps — Docker, Postgres, Nginx, cloud sync. `reptr` is a single binary. Every finding is a Markdown file you commit to git. Output formats render from a local build step. No account, no server, no internet required.

| Feature | reptr | SysReptor | Ghostwriter | PwnDoc-ng | AttackForge |
|---|---|---|---|---|---|
| Install | `cargo install` | Docker compose | Docker + Postgres | Docker + Node + Mongo | SaaS |
| Storage | Files (git) | PostgreSQL | PostgreSQL | MongoDB | Cloud DB |
| Editor | Yours | Web UI | Web UI | Web UI | Web UI |
| Single binary | yes | no | no | no | no |
| Works offline | yes | yes | yes | yes | no |
| Cost | Free | Free (Pro paid) | Free | Free | Paid |

---

## Installation

### Prebuilt binary (recommended)

```bash
cargo binstall reptr
```

### From crates.io

```bash
cargo install reptr
```

### From source

```bash
git clone https://github.com/yourhandle/reptr
cd reptr
cargo install --path .
```

### PDF output (optional)

PDF generation requires the [`typst`](https://typst.app) CLI:

```bash
brew install typst                    # macOS
cargo install --locked typst-cli      # any platform
```

`reptr` runs on Linux (glibc and musl), macOS (Intel and Apple Silicon), and Windows.

---

## Quick start

```bash
# 1. Scaffold a new engagement
reptr new acme-webapp-2026
cd acme-webapp-2026

# 2. Add findings
reptr add finding "SQL Injection in Login Form" --severity critical
reptr add finding "Missing Security Headers" --severity low

# 3. Edit findings in your editor
$EDITOR findings/001-sql-injection-in-login-form.md

# 4. Build the report
reptr build
open output/acme-webapp-2026.html
```

Live-reload while writing:

```bash
reptr watch
# ✓ Rebuilt in 137ms (triggered by findings/001-sql-injection-in-login-form.md)
```

---

## Commands

| Command | What it does |
|---|---|
| `reptr new <slug>` | Scaffold a new engagement directory with sample files. |
| `reptr add finding "<title>" [--severity ...]` | Append a numbered finding stub. |
| `reptr add finding "<title>" --from <template>` | Import a finding from your library (e.g. `--from web/xss-stored`). |
| `reptr build [path]` | Parse, validate, and render all formats defined in `reptr.toml`. |
| `reptr watch [path]` | Build once, then auto-rebuild on every file save (debounced 250 ms). |
| `reptr retest [path]` | Diff the current findings against the previous build and write a delta report. |
| `reptr stats [path] [--format text\|json]` | Multi-engagement summary table with severity counts and status. |
| `reptr library list [path]` | List all templates available in your finding library. |

Run `reptr <subcommand> --help` for all flags.

---

## Engagement layout

```
acme-webapp-2026/
├── reptr.toml              # engagement metadata, output formats, thresholds
├── client.toml             # client name, contacts
├── findings/
│   ├── 001-sql-injection.md
│   └── 002-missing-headers.md
├── assets/
│   └── screenshots/        # images embedded in finding bodies
├── templates/              # optional HTML template overrides
└── output/                 # written by `reptr build` — do not commit
```

---

## Writing findings

Each finding is a Markdown file with a YAML front matter block:

```markdown
---
id: F-001
title: SQL Injection in Login Form
severity: critical
cvss: "9.8"
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
cwe: "CWE-89"
owasp: "A03:2021"
status: open
affected_assets:
  - https://app.example.com/login
tags: [injection, authentication, p1]
---

## Description

The `/login` endpoint is vulnerable to SQL injection via the `username` parameter.

## Proof of Concept

```http
POST /login HTTP/1.1
Host: app.example.com
Content-Type: application/x-www-form-urlencoded

username=admin'-- &password=anything
```

## Impact

An attacker can authenticate as any user, including administrators.

## Remediation

Use parameterized queries or prepared statements.

## References

- https://owasp.org/Top10/A03_2021-Injection/
```

### CVSS auto-derivation

If you provide a `cvss_vector` but omit the numeric `cvss` score, `reptr build` derives it automatically:

```yaml
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N"
# cvss: derived as "7.5" automatically — no need to compute it yourself
```

If both are present, `reptr build` validates that the stated score matches the vector (within ±0.05). A mismatch is caught as a validation error.

### Status values

| Value | Meaning |
|---|---|
| `open` | Active, unresolved (default) |
| `resolved` | Fixed and verified |
| `accepted` | Risk accepted by the client |
| `false_positive` | Confirmed not a real issue |

---

## Retest workflow

After remediation, run `reptr retest` to diff the current findings against the previous build:

```bash
# First run — establishes baseline (same as reptr build)
reptr retest

# After remediation — shows what changed
reptr retest
# 2 resolved · 0 regressed · 0 new · 1 unchanged
```

This writes `output/<slug>-retest.html` and `output/<slug>-retest.json` with a per-finding delta:

| Change type | Meaning |
|---|---|
| `resolved` | Was open/accepted/false-positive, now resolved |
| `regressed` | Was resolved, now open again |
| `new` | Appeared for the first time |
| `removed` | No longer present |
| `changed` | Status or severity shifted in another way |
| `unchanged` | No change detected |

---

## Finding library

Reuse findings across engagements with a template library:

```bash
$ reptr library list
Library: ./findings-library (3 templates)

  name               severity   title
  api/idor           high       Insecure Direct Object Reference
  web/sql-injection  critical   SQL Injection
  web/xss-stored     high       Stored Cross-Site Scripting

$ reptr add finding "Stored XSS in /comments" --from web/xss-stored
Created findings/003-stored-xss-in-comments.md
```

The imported file keeps the template's severity, CVSS, CWE, and body — only `id` is freshly assigned and `title` is overridden if you pass one.

Configure the library path in `reptr.toml` (defaults to `./findings-library`):

```toml
[library]
path = "../shared-findings-library"   # absolute paths also accepted
```

---

## Configuration reference

`reptr.toml` (auto-generated by `reptr new`):

```toml
[engagement]
name    = "Acme Web Application Assessment"
slug    = "acme-webapp-2026"
kind    = "web application"
start_date = "2026-01-15"
end_date   = "2026-01-25"
report_version = "1.0"

[output]
formats = ["html", "json"]        # also: "docx", "pdf"

[template]
# html = "templates/report.html"  # override the embedded default

[library]
# path = "findings-library"

[severity_thresholds]
# critical = 1    # fail build if any critical finding is open (CI use)
```

---

## Custom HTML templates

The default template is embedded in the binary. To brand a report:

```toml
[template]
html = "templates/report.html"
```

Templates use [MiniJinja](https://docs.rs/minijinja) syntax. The render context:

| Variable | Type | Notes |
|---|---|---|
| `engagement` | `Engagement` | `meta`, `client`, `findings`, `output` |
| `engagement.findings` | `[Finding]` | Sorted Critical → Info |
| `engagement.findings[i].body_html` | `string` | Pre-rendered HTML — use `\| safe` |
| `severity_counts` | `[{name, count}]` | One entry per severity in fixed order |
| `generated_at` | `string` | ISO-8601 timestamp |

Copy `templates/report.html.tera` from this repo and modify it in place as a starting point.

---

## Output formats

| Format | Notes |
|---|---|
| `html` | Self-contained HTML. Default template embedded in binary. |
| `json` | Machine-readable engagement snapshot. Schema matches the internal `Engagement` struct. |
| `docx` | LibreOffice-compatible Word document. Images are embedded. |
| `pdf` | Requires `typst` CLI on `$PATH`. |

---

## CI integration

Fail the pipeline if any critical finding is unresolved:

```toml
[severity_thresholds]
critical = 1
```

```yaml
# .github/workflows/report.yml
- run: cargo install reptr
- run: reptr build
```

---

## Contributing

Open an issue first for anything that changes the data model, adds a new output format, or touches the CLI surface. Smaller fixes — typos, edge-case tests, doc clarifications — are welcome as a direct PR.

```bash
git clone https://github.com/yourhandle/reptr
cd reptr
cargo test
```

---

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE), at your option.
