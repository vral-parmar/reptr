# reptr build

Parse all findings, validate them, and render every output format configured in `reptr.toml`.

## Usage

```bash
reptr build [PATH]
```

`PATH` defaults to the current directory.

## What it does

1. **Parse** — reads `reptr.toml`, `client.toml`, and every `*.md` file under `findings/`
2. **Validate** — runs all validation rules (see below); fails fast if any error is found
3. **Render** — writes one file per format to `output/<slug>.<ext>`

## Example output

```
✓ Parsed 3 findings
✓ Rendered HTML  → output/acme-webapp-2026.html
✓ Rendered JSON  → output/acme-webapp-2026.json
Done in 8ms.
```

## Validation rules

| Rule | Error |
|---|---|
| Unique finding IDs | Duplicate `id` across two files |
| Non-empty title | `title` field is blank or missing |
| Valid CVSS score | `cvss` (if present) must parse as a number between 0.0 and 10.0 |
| Valid CVSS vector | `cvss_vector` (if present) must be a well-formed CVSS 3.x string |
| Score/vector agreement | If both are present, the stated score must match the vector's computed value (±0.05) |
| Non-empty slug | `slug` in `reptr.toml` must not be blank |
| Severity thresholds | Open finding counts must not exceed configured limits |

If validation fails, `reptr build` prints each error and exits non-zero:

```
✗ Validation failed:
  • finding `findings/002-sqli.md` CVSS score `2.0` does not match 9.8
    computed from vector `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H`
    — update the score or correct the vector
error: 1 validation error(s)
```

## Output formats

Configure which formats to render in `reptr.toml`:

```toml
[output]
formats   = ["html", "json", "docx", "pdf"]
directory = "output"
```

| Format | Notes |
|---|---|
| `html` | Self-contained HTML. Uses the embedded default template unless you override it. |
| `json` | Machine-readable engagement snapshot. Schema matches the `Engagement` struct. |
| `docx` | LibreOffice-compatible Word document. Referenced images are embedded. |
| `pdf` | Requires `typst` CLI on `$PATH`. |

## CVSS auto-derivation

If a finding has `cvss_vector` but no `cvss` score, `reptr build` derives the score automatically:

```yaml
# In the finding front matter:
cvss_vector: "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N"
# cvss is derived as "7.5" — no need to compute it yourself
```

The derived score is written into all output formats (JSON, HTML, DOCX, PDF).
