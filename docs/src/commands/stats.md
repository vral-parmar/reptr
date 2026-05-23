# reptr stats

Summarise findings across all engagements under a directory.

## Usage

```bash
reptr stats [PATH] [--format text|json]
```

`PATH` defaults to the current directory.

## How it works

`reptr stats` walks the immediate subdirectories of `PATH`. Any directory containing a `reptr.toml` is treated as an engagement and its findings are parsed and counted.

It also works when called from inside a single engagement directory — it produces a one-row table for that engagement.

## Example: text output (default)

```bash
reptr stats ~/engagements
```

```
Engagements under /Users/you/engagements (3 engagements)

  engagement             crit  high   med   low  info  total   open
  acme-webapp-2026          2     4     3     1     0     10      8
  contoso-mobile-2026       1     2     1     0     0      4      4
  globex-api-2026           0     3     5     2     0     10      6
  ───────────────────────────────────────────────────────────────────────
  TOTAL                     3     9     9     3     0     24     18
```

## Example: JSON output

```bash
reptr stats ~/engagements --format json
```

```json
{
  "engagements": [
    {
      "slug": "acme-webapp-2026",
      "name": "Acme Web Application Assessment",
      "counts": {
        "critical": 2,
        "high": 4,
        "medium": 3,
        "low": 1,
        "info": 0
      },
      "total": 10,
      "open": 8,
      "resolved": 2
    }
  ],
  "totals": {
    "engagements": 3,
    "total": 24,
    "open": 18,
    "resolved": 6,
    "counts": {
      "critical": 3,
      "high": 9,
      "medium": 9,
      "low": 3,
      "info": 0
    }
  }
}
```

The JSON schema is stable and suitable for piping into other tools, dashboards, or scripts.
