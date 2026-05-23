# reptr new

Scaffold a new engagement directory with all required files.

## Usage

```bash
reptr new <NAME>
```

`NAME` becomes both the directory name and the engagement slug in `reptr.toml`.

## Example

```bash
reptr new acme-webapp-2026
```

Output:

```
Created acme-webapp-2026/
  ├── reptr.toml
  ├── client.toml
  ├── findings/
  │   └── 001-example-finding.md
  ├── templates/
  ├── assets/screenshots/
  └── output/

Next: cd acme-webapp-2026 && reptr build
```

## Generated files

### reptr.toml

```toml
[engagement]
name    = "Acme Webapp 2026"
slug    = "acme-webapp-2026"
type    = "External Web Application Penetration Test"
start_date = ""
end_date   = ""
report_version = "1.0"

[client]
file = "client.toml"

[output]
formats   = ["html", "json"]
directory = "output"

# Uncomment to enforce open-finding limits during build (useful in CI):
# [severity_thresholds]
# critical = 0   # fail if any critical finding is open
# high     = 5
```

### client.toml

```toml
name    = "Acme Corp"
contact = "Jane Doe"
email   = "security@acme.example"
```

### findings/001-example-finding.md

A pre-filled stub showing every supported front matter field, with optional fields commented out so the file is valid on first build.
