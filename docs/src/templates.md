# Custom templates

reptr ships with a built-in HTML template. You can override it — or add new formats — by placing template files in the `templates/` directory of your engagement.

## How template resolution works

When rendering output, reptr looks for a template in this order:

1. `templates/<format>.html` (or `.jinja`) in the engagement root
2. The embedded default template

If a custom template is found it is used in place of the default. `reptr watch` automatically rebuilds when files in `templates/` change.

## Template engine

Templates use [MiniJinja](https://github.com/mitsuhiko/minijinja) — a Rust port of Jinja2. Most Jinja2 syntax works:

- `{{ variable }}` — output a value
- `{% for ... %}` / `{% endfor %}` — loops
- `{% if ... %}` / `{% endif %}` — conditionals
- `{{ value | filter }}` — filters

## Context variables

The following variables are available in every template.

### `engagement`

The top-level engagement object.

| Variable | Type | Description |
|---|---|---|
| `engagement.slug` | string | Engagement slug from `reptr.toml` |
| `engagement.name` | string | Full engagement name |
| `engagement.date` | string | Engagement start date |
| `engagement.report_date` | string | Report issue date |
| `engagement.version` | string | Report version |

### `client`

| Variable | Type | Description |
|---|---|---|
| `client.name` | string | Client organisation name |
| `client.contact_name` | string | Primary contact name |
| `client.contact_email` | string | Primary contact email |
| `client.logo` | string or null | Path to logo image (base64-encoded in HTML output) |

### `findings`

A list of finding objects, sorted by severity then ID.

| Variable | Type | Description |
|---|---|---|
| `finding.id` | string | Finding ID (e.g. `F-001`) |
| `finding.title` | string | Finding title |
| `finding.severity` | string | `critical`, `high`, `medium`, `low`, or `info` |
| `finding.status` | string | `open`, `resolved`, `accepted`, or `false_positive` |
| `finding.affected_assets` | list of strings | Affected hosts or URLs |
| `finding.tags` | list of strings | Finding tags |
| `finding.cvss` | string or null | CVSS score (e.g. `"9.8"`) |
| `finding.cvss_vector` | string or null | CVSS 3.x vector string |
| `finding.cwe` | string or null | CWE identifier |
| `finding.owasp` | string or null | OWASP Top 10 category |
| `finding.body_html` | string | Rendered HTML from the Markdown body |

### `summary`

Pre-computed counts for convenience.

| Variable | Type | Description |
|---|---|---|
| `summary.total` | integer | Total number of findings |
| `summary.open` | integer | Open findings |
| `summary.resolved` | integer | Resolved findings |
| `summary.critical` | integer | Critical findings (all statuses) |
| `summary.high` | integer | High findings |
| `summary.medium` | integer | Medium findings |
| `summary.low` | integer | Low findings |
| `summary.info` | integer | Info findings |

## Example: minimal HTML template

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{{ engagement.name }} — Security Assessment</title>
</head>
<body>
  <h1>{{ engagement.name }}</h1>
  <p>Client: {{ client.name }} · Date: {{ engagement.report_date }}</p>
  <p>{{ summary.total }} findings ({{ summary.open }} open)</p>

  {% for finding in findings %}
  <section id="{{ finding.id }}">
    <h2>[{{ finding.severity | upper }}] {{ finding.id }}: {{ finding.title }}</h2>
    <p>Status: {{ finding.status }}</p>
    {{ finding.body_html }}
  </section>
  {% endfor %}
</body>
</html>
```

Save this as `templates/report.html` in your engagement root, then run `reptr build`.

## Example: severity badge filter

MiniJinja lets you define macros for repeated patterns:

```html
{% macro severity_badge(s) %}
  <span class="badge badge-{{ s }}">{{ s | upper }}</span>
{% endmacro %}

{% for finding in findings %}
  {{ severity_badge(finding.severity) }} {{ finding.title }}
{% endfor %}
```

## Tips

- Access the built-in template source by running `reptr build --dump-template` (outputs the embedded HTML to stdout — useful as a starting point).
- Keep images in `assets/` and reference them with relative paths. reptr inlines them as base64 in HTML output so the file is self-contained.
- The `body_html` field already contains rendered, sanitised HTML. Do not double-escape it — use `{{ finding.body_html | safe }}` if your Jinja environment escapes by default.
