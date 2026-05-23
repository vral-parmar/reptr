# reptr

> Write your pentest findings as Markdown. Run `reptr build`. Get a polished HTML, JSON, DOCX, and PDF report.  
> No Docker. No database. No SaaS.

`reptr` is a single binary that turns a folder of Markdown files into a professional pentest report. Every finding is plain text you can commit to git, review in a pull request, and diff between assessments.

## Why reptr?

Most pentest report tools are web applications — Docker, Postgres, Nginx, cloud sync. `reptr` is different:

| Feature | reptr | SysReptor | Ghostwriter | PwnDoc-ng | AttackForge |
|---|---|---|---|---|---|
| Install | `cargo install` | Docker compose | Docker + Postgres | Docker + Node + Mongo | SaaS |
| Storage | Files (git) | PostgreSQL | PostgreSQL | MongoDB | Cloud DB |
| Editor | Yours | Web UI | Web UI | Web UI | Web UI |
| Single binary | ✓ | ✗ | ✗ | ✗ | ✗ |
| Works offline | ✓ | ✓ | ✓ | ✓ | ✗ |
| Cost | Free | Free (Pro paid) | Free | Free | Paid |

`reptr` trades team collaboration features for git-native, single-binary simplicity. If you need real-time multi-user editing, use one of the others.

## What you get

- **HTML** — self-contained report, default template embedded in the binary
- **JSON** — machine-readable snapshot for automation and diffing
- **DOCX** — LibreOffice-compatible Word document with embedded images
- **PDF** — via the `typst` CLI (optional)

## How it works

```
findings/
├── 001-sql-injection.md     ← YAML front matter + Markdown body
├── 002-missing-headers.md
└── 003-idor.md
```

```bash
reptr build
# ✓ Parsed 3 findings
# ✓ Rendered HTML  → output/acme-webapp-2026.html
# ✓ Rendered JSON  → output/acme-webapp-2026.json
# Done in 8ms.
```

That's it. No server to start, no database to migrate, no config files beyond `reptr.toml`.
