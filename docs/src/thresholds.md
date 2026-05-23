# Severity thresholds

Severity thresholds let you enforce finding limits as part of `reptr build`. When a threshold is exceeded the build exits non-zero — making it suitable as a CI gate.

## Configuration

Add a `[severity_thresholds]` section to `reptr.toml`:

```toml
[severity_thresholds]
critical = 0   # fail if any open critical finding exists
high     = 5   # fail if more than 5 open high findings exist
medium   = 10
low      = 20
```

Any combination of fields is valid. Omitting a field means no limit is enforced for that severity.

## Semantics

The threshold value is a **maximum allowed count** of open findings at that severity.

| Value | Meaning |
|---|---|
| `0` | Fail if even one open finding of this severity exists |
| `N` | Fail if more than N open findings of this severity exist |
| *(absent)* | No limit — any count is allowed |

Only findings with `status: open` count against thresholds. `resolved`, `accepted`, and `false_positive` findings are ignored.

## Build output when thresholds are exceeded

```
✗ Validation failed:
  • 2 open critical finding(s) exceed the allowed limit of 0
    — resolve them or raise [severity_thresholds].critical in reptr.toml
  • 7 open high finding(s) exceed the allowed limit of 5
    — resolve them or raise [severity_thresholds].high in reptr.toml
error: 2 validation error(s)
```

## Common patterns

### Zero-tolerance for critical findings

Fail the build if any critical finding remains open:

```toml
[severity_thresholds]
critical = 0
```

### Graduated enforcement

Gradually tighten thresholds as an engagement progresses:

```toml
# Initial assessment — no limits yet
# [severity_thresholds]

# Mid-remediation — critical must be resolved
# [severity_thresholds]
# critical = 0

# Pre-delivery — critical and high must be resolved
[severity_thresholds]
critical = 0
high     = 0
```

### Informational gate only

Use thresholds in CI to block merging until findings are resolved, without blocking the report build during the engagement itself:

```toml
# reptr.toml (engagement config — no thresholds during engagement)
[output]
formats = ["html", "json"]
```

```yaml
# .github/workflows/ci.yml (CI applies thresholds via a separate command)
- run: |
    # Patch thresholds just for the CI check
    cat >> reptr.toml <<'EOF'
    [severity_thresholds]
    critical = 0
    EOF
    reptr build
```

## Using thresholds in CI

See [CI integration](ci.md) for complete GitHub Actions examples.
