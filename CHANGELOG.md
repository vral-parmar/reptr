# Changelog

All notable changes to this project will be documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning: [SemVer](https://semver.org/).

## [Unreleased]

### Added
- Severity threshold enforcement — `[severity_thresholds]` in `reptr.toml` now
  gates `reptr build`. Each field (`critical`, `high`, `medium`, `low`) sets the
  maximum number of **open** findings allowed at that severity level before the build
  fails. `0` means "fail if any are open". Resolved, accepted, and false-positive
  findings are not counted. Omitting a field (the default) means no limit.
- `reptr retest` — diff the current findings against the previous build and emit a
  delta report. On the first run it establishes a baseline (same as `reptr build`).
  On every subsequent run it prints a summary (`N new · N resolved · N regressed …`)
  and writes `output/<slug>-retest.html` and `output/<slug>-retest.json`. Change
  types: **new**, **removed**, **resolved** (open/accepted/false-positive → resolved),
  **regressed** (resolved → open), and **changed** (any other status or severity shift).
- CVSS vector validation — if `cvss_vector` is present in a finding's front matter
  it is now parsed as a valid CVSS 3.x string; an invalid vector fails `reptr build`
  with a clear message.
- CVSS score/vector cross-check — when both `cvss` and `cvss_vector` are provided,
  `reptr build` verifies that the stated score agrees (within ±0.05) with the value
  computed from the vector. A mismatch is reported as a validation error with the
  corrected computed value shown.
- Automatic CVSS score derivation — if `cvss_vector` is present but `cvss` (the
  numeric score) is absent, the score is derived automatically at parse time and
  written into the JSON/HTML/DOCX output (formatted to one decimal place).

## [0.8.0] - 2026-05-22

### Added
- Finding-library import. Drop pre-written templates under
  `findings-library/` (or wherever `[library].path` points), then run
  `reptr add finding "My title" --from web/xss-stored` to pull one in.
  The new finding gets a fresh `F-NNN` id; everything else in the template
  (severity, CVSS, body) carries over unchanged.
- `reptr library list` shows every template with its title and a
  severity-coloured chip, plus the path it resolved from.
- `[library].path` in `reptr.toml` (default: `./findings-library`,
  absolute paths also accepted).
- Front-matter rewriting helper that preserves comments, ordering, and
  quoting style — only the keys we explicitly change get touched.

## [0.7.0] - 2026-05-22

### Added
- `reptr stats [path]` — multi-engagement summary. Walks the immediate
  subdirectories of `path`, treats every dir with a `reptr.toml` as an
  engagement, and prints a column-aligned table with severity counts,
  totals, and open/resolved status per engagement.
- `reptr stats --format json` for piping into other tools. Stable schema:
  `{ engagements: [...], totals: {...} }`.
- Works from inside a single engagement dir too — gives you a one-row view.

## [0.6.0] - 2026-05-22

### Added
- HTML template overrides. Setting `[template].html = "path/to/report.html"`
  in `reptr.toml` uses a user-supplied MiniJinja template instead of the
  embedded default. The template receives `engagement`, `severity_counts`,
  and `generated_at` in context.
- README now documents the template variables.

## [0.5.0] - 2026-05-22

### Added
- Image embedding across DOCX and PDF. `![alt](path)` references in finding
  bodies are resolved at parse time and:
  - **DOCX:** embedded under `word/media/`, scaled to fit a 5.5" max width
    while preserving aspect ratio.
  - **PDF:** emitted as `#image("relative/path", width: 90%)`; the typst CLI
    is invoked with `--root <engagement-dir>` so paths above `output/` resolve.
  - **HTML:** `<img>` tags from comrak keep working (relative paths line up
    between `findings/` and `output/`).
- `ImageRef { alt, markdown_src, resolved_path }` now appears in the JSON dump.
- Remote URLs (`http://`, `https://`, `data:`) are skipped in DOCX/PDF with a
  `tracing::warn!`; HTML keeps the original URL.

## [0.4.0] - 2026-05-22

### Added
- `reptr watch` — initial build, then auto-rebuild on every save with a 250ms debounce.
  Watches `findings/`, `templates/`, `reptr.toml`, and `client.toml`.
- Cross-platform release workflow (`.github/workflows/release.yml`) producing
  Linux gnu+musl (x86_64, aarch64), macOS (Intel + Apple Silicon), and Windows binaries.
- `cargo binstall` metadata so `cargo binstall reptr` pulls a prebuilt binary.
- Dual `LICENSE-MIT` + `LICENSE-APACHE` files.

## [0.3.0] - 2026-05-22

### Added
- PDF output via Typst shell-out. Generates a `.typ` source next to the PDF so
  users can hand-tweak layout.
- Clear error if `typst` is not installed, with install instructions.

## [0.2.0] - 2026-05-22

### Added
- DOCX output via `docx-rs`. Cover page, executive-summary table,
  findings-overview table, one section per finding with severity-coloured badges
  and code blocks with real line breaks.

## [0.1.0] - 2026-05-22

Initial release.

### Added
- `reptr new <slug>` — scaffolds an engagement directory.
- `reptr add finding "<title>" [--severity ...]` — appends a numbered Markdown stub.
- `reptr build` — parses front-matter Markdown findings, validates (unique IDs,
  CVSS 0–10, non-empty title/slug), sorts by severity desc, renders HTML and JSON.
- HTML template with executive summary, findings overview table, and per-finding
  detail with severity badges. Print-friendly and dark/light agnostic.
- JSON output: full `Engagement` dump with both raw Markdown and pre-rendered HTML bodies.
