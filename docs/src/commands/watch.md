# reptr watch

Build once, then auto-rebuild whenever findings, templates, or config files change.

## Usage

```bash
reptr watch [PATH]
```

`PATH` defaults to the current directory.

## How it works

On startup, `reptr watch` runs a full build — the same as `reptr build`. If the build fails the error is printed but watching continues.

It then watches these paths for changes:

| Path | What triggers a rebuild |
|---|---|
| `findings/` | Any finding file created, modified, or deleted |
| `templates/` | Custom template files modified |
| `reptr.toml` | Engagement config changed |
| `client.toml` | Client info changed |

Changes are debounced for 250 ms. If you save rapidly, `reptr watch` waits until saves settle before rebuilding — so you won't get a partial-file build mid-type.

## Example session

```
$ reptr watch
✓ Parsed 2 findings
✓ Rendered HTML  → output/acme-webapp-2026.html
✓ Rendered JSON  → output/acme-webapp-2026.json
Done in 8ms.
Watching /path/to/acme-webapp-2026

# ... you save a finding file ...

✓ Rebuilt in 11ms (triggered by findings/002-sqli.md)

# ... you save a template ...

✓ Rebuilt in 9ms (triggered by templates/report.html)

# ... a build fails (e.g. bad CVSS vector) ...

✗ Build failed: 1 validation error(s)
```

## Workflow tip

Keep `reptr watch` running in a terminal while you write findings. Open the HTML output in a browser and refresh after each save. No server needed — the output file is written directly to disk.

```bash
# Terminal 1
reptr watch

# Terminal 2 (or your editor's integrated terminal)
$EDITOR findings/003-idor.md
```
