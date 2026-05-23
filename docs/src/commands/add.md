# reptr add

Add a new finding to an engagement — either a blank stub or one imported from your finding library.

## Usage

```bash
reptr add finding [OPTIONS] [TITLE]
```

| Option | Default | Description |
|---|---|---|
| `--severity` | `medium` | One of `critical`, `high`, `medium`, `low`, `info` |
| `--from` | — | Library template name (e.g. `web/xss-stored`) |
| `--path` | `.` | Engagement root directory |

## Create a blank stub

```bash
reptr add finding "SQL Injection in Login Form" --severity critical
# Created findings/002-sql-injection-in-login-form.md
```

The filename is auto-derived from the title (lower-cased, spaces → hyphens), prefixed with the next available sequence number. The finding ID (`F-NNN`) matches the sequence number.

## Import from a library

```bash
reptr add finding "Stored XSS in Comments" --from web/xss-stored
# Created findings/003-stored-xss-in-comments.md
```

The imported file keeps the template's severity, CVSS, CWE, body markdown, and all other fields. Only the `id` is freshly assigned and the `title` is overridden if you passed one.

If you omit `TITLE`, the template's own title is used:

```bash
reptr add finding --from web/sql-injection
# Created findings/004-sql-injection.md  (title from template)
```

See [reptr library](library.md) for how to manage templates.

## Stub format

```markdown
---
id: F-002
title: SQL Injection in Login Form
severity: critical
status: open
affected_assets: []
tags: []
# Optional — uncomment and fill in as needed:
# cvss: "0.0"
# cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N"
# cwe: "CWE-000"
# owasp: "A00:2021"
---

## Description

## Proof of Concept

## Impact

## Remediation

## References
```
