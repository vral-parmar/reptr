# reptr retest

Build the engagement and diff findings against the previous build to track remediation progress.

## Usage

```bash
reptr retest [PATH]
```

`PATH` defaults to the current directory.

## How it works

`reptr retest` reads the previous build's JSON snapshot from `output/<slug>.json`, runs a fresh build, then compares the two sets of findings by ID.

**First run:** if no previous JSON exists, `reptr retest` behaves identically to `reptr build` — it establishes a baseline. No delta files are written.

**Subsequent runs:** computes the diff and writes:
- `output/<slug>-retest.json` — machine-readable delta
- `output/<slug>-retest.html` — human-readable HTML report

## Change types

| Type | Meaning |
|---|---|
| `new` | Finding appears for the first time (not in previous build) |
| `removed` | Finding was in previous build but is gone now |
| `resolved` | Status changed from `open`, `accepted`, or `false_positive` → `resolved` |
| `regressed` | Status changed from `resolved` → `open` |
| `changed` | Any other status or severity shift |
| `unchanged` | No change detected |

## Example

```bash
# Establish a baseline
reptr retest

# After the client remediates findings, update the finding files and run again
reptr retest
# ── Retest Delta ─────────────────────────────────────────
#   2 new  ·  3 resolved  ·  1 regressed  ·  0 changed  ·  0 removed  ·  4 unchanged
#
#   [C] F-001  SQL Injection in Login Form      ✓ open → resolved
#   [H] F-003  Stored XSS in Comments           ✓ open → resolved
#   [L] F-005  Missing HSTS Header              ✓ open → resolved
#   [C] F-007  SSRF in File Upload              ↩ resolved → open
#   [H] F-008  Broken Object-Level Auth         + New
#   [M] F-009  Verbose Error Messages           + New
```

## Delta JSON schema

```json
{
  "engagement_name": "Acme Web Application Assessment",
  "generated_at": "2026-05-23T10:00:00Z",
  "new_count": 2,
  "resolved_count": 3,
  "regressed_count": 1,
  "changed_count": 0,
  "removed_count": 0,
  "unchanged_count": 4,
  "deltas": [
    {
      "id": "F-001",
      "title": "SQL Injection in Login Form",
      "severity": "critical",
      "change_type": "resolved",
      "label": "open → resolved",
      "before_status": "open",
      "after_status": "resolved"
    }
  ]
}
```

## Typical remediation workflow

```
Initial assessment
    └── reptr retest          ← establishes baseline

Client remediates findings
    └── Update status: resolved in each fixed finding
    └── reptr retest          ← shows resolved/regressed/new

Verification assessment
    └── Update findings again
    └── reptr retest          ← final delta for the report
```
