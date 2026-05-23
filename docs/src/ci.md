# CI integration

reptr works well as a CI step. The build exits non-zero on any validation error, making it easy to enforce quality gates in GitHub Actions or any other CI system.

## Basic setup

### Install reptr in CI

```yaml
- name: Install reptr
  run: cargo install reptr --locked
```

For faster installs, use `cargo-binstall` to download a pre-built binary:

```yaml
- name: Install cargo-binstall
  uses: cargo-bins/cargo-binstall@main

- name: Install reptr
  run: cargo binstall reptr --no-confirm
```

### Run a build

```yaml
- name: Build report
  run: reptr build path/to/engagement
```

If the build passes, all outputs are written to `output/`. If it fails (validation errors, threshold violations), the step exits non-zero and the job fails.

## Full GitHub Actions workflow

```yaml
# .github/workflows/reptr.yml
name: reptr build

on:
  push:
    paths:
      - 'engagements/**'
  pull_request:
    paths:
      - 'engagements/**'

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
          key: ${{ runner.os }}-cargo-reptr

      - name: Install reptr
        run: cargo install reptr --locked

      - name: Build all engagements
        run: |
          for dir in engagements/*/; do
            if [ -f "$dir/reptr.toml" ]; then
              echo "Building $dir"
              reptr build "$dir"
            fi
          done

      - name: Upload reports
        uses: actions/upload-artifact@v4
        with:
          name: reports
          path: engagements/*/output/
```

## Threshold enforcement in CI

Add `[severity_thresholds]` to `reptr.toml` to fail the build when too many findings remain open:

```toml
[severity_thresholds]
critical = 0   # block merge if any critical finding is unresolved
high     = 0
```

The CI output will clearly state which thresholds were exceeded:

```
✗ Validation failed:
  • 1 open critical finding(s) exceed the allowed limit of 0
    — resolve them or raise [severity_thresholds].critical in reptr.toml
error: 1 validation error(s)
```

## Storing reports as release assets

Tag-based releases can attach HTML/JSON reports:

```yaml
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      # ... install reptr ...
      - run: reptr build

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: output/*.html
```

## Retest in CI

Run `reptr retest` instead of `reptr build` to automatically generate a delta report comparing the current findings against the previous build:

```yaml
- name: Retest
  run: reptr retest

- name: Upload delta report
  uses: actions/upload-artifact@v4
  with:
    name: retest-delta
    path: output/*-retest.*
```

The delta JSON and HTML files show which findings were resolved, regressed, or added since the last run. See [reptr retest](commands/retest.md) for full details.

## Typst / PDF output

PDF generation requires `typst` on `$PATH`. Install it in CI with:

```yaml
- name: Install typst
  uses: typst-community/setup-typst@v4
```

Then add `pdf` to your formats list in `reptr.toml`:

```toml
[output]
formats = ["html", "json", "pdf"]
```
