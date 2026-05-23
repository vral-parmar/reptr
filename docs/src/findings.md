# Writing findings

Each finding is a Markdown file inside the `findings/` directory of your engagement. The file has a YAML front matter block followed by free-form Markdown.

## File naming

Files are named with a numeric prefix followed by a slug:

```
findings/001-sql-injection.md
findings/002-stored-xss.md
findings/003-idor.md
```

The prefix determines display order. `reptr add finding` assigns the next available number automatically.

## Front matter reference

```yaml
---
id: F-001                            # Required. Unique identifier within the engagement.
title: SQL Injection in Login Form   # Required. Human-readable title.
severity: critical                   # Required. One of: critical high medium low info
status: open                         # Required. One of: open resolved accepted false_positive

# --- Optional fields ---
affected_assets:                     # List of affected systems or URLs.
  - https://example.com/login
  - https://example.com/api/auth

tags:                                # Free-form tags for filtering.
  - injection
  - authentication

cvss: "9.8"                          # CVSS 3.x score as a string (0.0–10.0).
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"  # CVSS 3.x vector.

cwe: "CWE-89"                        # CWE identifier.
owasp: "A03:2021"                    # OWASP Top 10 category.
---
```

## Severity values

| Value | When to use |
|---|---|
| `critical` | Direct, high-impact exploitation with no user interaction (RCE, SQLi with data exfiltration) |
| `high` | Significant impact but requires some conditions (auth bypass, stored XSS) |
| `medium` | Limited impact or requires specific conditions (CSRF, open redirect) |
| `low` | Minimal direct impact (verbose errors, weak headers) |
| `info` | Observations and hardening recommendations |

## Status values

| Value | Meaning |
|---|---|
| `open` | Finding is confirmed and unresolved |
| `resolved` | Client has fixed the issue; verified by tester |
| `accepted` | Client acknowledges and accepts the risk |
| `false_positive` | Initially flagged but confirmed not exploitable |

`reptr retest` tracks transitions between these states.

## CVSS auto-derivation

If you supply `cvss_vector` but omit `cvss`, reptr computes the score automatically:

```yaml
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N"
# cvss is derived as "7.5" — no manual calculation needed
```

If you supply both, reptr validates they agree (within ±0.05). Mismatches are a build error.

## Body sections

The Markdown body is rendered directly into the report. Use any standard Markdown. Common sections:

```markdown
## Description

Explain what the vulnerability is, where it was found, and why it matters.

## Proof of Concept

Step-by-step reproduction. Include request/response snippets, screenshots, or payload examples.

## Impact

Business impact if exploited — what data, systems, or users are at risk.

## Remediation

Specific, actionable guidance for the developer fixing this issue.

## References

- [OWASP SQL Injection](https://owasp.org/www-community/attacks/SQL_Injection)
- [CWE-89](https://cwe.mitre.org/data/definitions/89.html)
```

These sections are conventional — you can add, remove, or rename them freely. Custom templates can render whatever fields they need.

## Images

Embed images with standard Markdown:

```markdown
![Request showing injection payload](../screenshots/sqli-request.png)
```

Paths are relative to the finding file. Images are embedded in DOCX output.

## Example finding

```markdown
---
id: F-001
title: SQL Injection in Login Form
severity: critical
status: open
affected_assets:
  - https://example.com/login
tags:
  - injection
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
cwe: "CWE-89"
owasp: "A03:2021"
---

## Description

The login form at `/login` is vulnerable to SQL injection via the `username` parameter.
Input is concatenated directly into a SQL query without sanitisation or parameterisation.

## Proof of Concept

1. Navigate to `https://example.com/login`
2. Enter `' OR 1=1--` as the username and any value as the password
3. Observe successful authentication without valid credentials

```http
POST /login HTTP/1.1
Host: example.com

username=%27+OR+1%3D1--&password=anything
```

## Impact

An attacker can bypass authentication, extract the full user database, and potentially
achieve remote code execution via `xp_cmdshell` (if running MSSQL).

## Remediation

Use parameterised queries or a prepared statement library. Never concatenate user input
into SQL strings.

## References

- [OWASP SQL Injection Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html)
- [CWE-89: Improper Neutralization of Special Elements](https://cwe.mitre.org/data/definitions/89.html)
```
