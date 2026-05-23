# Configuration

reptr uses two TOML files in the engagement root.

## reptr.toml

The engagement configuration file. Created by `reptr new`.

```toml
[engagement]
slug        = "acme-webapp-2026"
name        = "Acme Web Application Assessment"
date        = "2026-05-01"
report_date = "2026-05-23"
version     = "1.0"
client      = "acme"           # matches the [client] section in client.toml

[output]
formats   = ["html", "json"]   # html | json | docx | pdf
directory = "output"

# Uncomment to enforce open-finding limits during build (useful in CI):
# [severity_thresholds]
# critical = 0   # fail if any critical finding is open
# high     = 5   # fail if more than 5 high findings are open
# medium   = 10
# low      = 20
```

### [engagement] fields

| Field | Type | Description |
|---|---|---|
| `slug` | string | URL/filename-safe identifier. Must be non-empty. |
| `name` | string | Full engagement name, appears in report headers. |
| `date` | string | Engagement start date (ISO 8601, e.g. `2026-05-01`). |
| `report_date` | string | Report issue date. |
| `version` | string | Report version string (e.g. `1.0`, `1.1-draft`). |
| `client` | string | Must match the client slug in `client.toml`. |

### [output] fields

| Field | Type | Description |
|---|---|---|
| `formats` | array | Output formats to generate. See [reptr build](commands/build.md). |
| `directory` | string | Directory to write output files. Default: `output`. |

### [severity_thresholds] fields

All fields are optional. When set, the value is a count limit: build fails if the number of **open** findings of that severity exceeds the limit. `0` means fail if any open finding of that severity exists.

| Field | Type | Description |
|---|---|---|
| `critical` | integer | Max allowed open critical findings. |
| `high` | integer | Max allowed open high findings. |
| `medium` | integer | Max allowed open medium findings. |
| `low` | integer | Max allowed open low findings. |

See [Severity thresholds](thresholds.md) for full details and CI usage.

## client.toml

Client contact and branding information. Created by `reptr new`.

```toml
[client]
slug         = "acme"
name         = "Acme Corporation"
contact_name = "Jane Smith"
contact_email = "jsmith@acme.example"
logo         = "assets/acme-logo.png"   # optional, relative to engagement root
```

### [client] fields

| Field | Type | Description |
|---|---|---|
| `slug` | string | Short identifier. Must match the `client` field in `reptr.toml`. |
| `name` | string | Full legal name of the client organisation. |
| `contact_name` | string | Primary contact name, used in report cover pages. |
| `contact_email` | string | Primary contact email. |
| `logo` | string | Optional path to a logo image (PNG/SVG). Embedded in HTML and DOCX output. |

## Environment variables

| Variable | Description |
|---|---|
| `REPTR_LIBRARY` | Override the library directory (default: `~/.config/reptr/library`). |
| `REPTR_TEMPLATE_DIR` | Override the directory searched for custom templates. |

## Custom templates

Place custom templates in the `templates/` subdirectory of your engagement. See [Custom templates](templates.md).
