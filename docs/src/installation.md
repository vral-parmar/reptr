# Installation

## Prebuilt binary (recommended)

If you have `cargo-binstall` installed, this grabs the prebuilt binary directly from GitHub Releases — no compilation needed:

```bash
cargo binstall reptr
```

## From crates.io

Compiles from source using your local Rust toolchain:

```bash
cargo install reptr
```

## From source

```bash
git clone https://github.com/vral-parmar/reptr
cd reptr
cargo install --path .
```

## Platform support

`reptr` is tested on:

- **macOS** — Intel and Apple Silicon
- **Linux** — glibc (x86-64, aarch64) and musl (x86-64, aarch64)
- **Windows** — x86-64 MSVC

## PDF output (optional)

PDF generation requires the [`typst`](https://typst.app) CLI on your `$PATH`. Everything else (HTML, JSON, DOCX) works without it.

```bash
# macOS
brew install typst

# Any platform with cargo
cargo install --locked typst-cli

# Verify
typst --version
```

If `typst` is not found and you request PDF output, `reptr build` will error with a clear message rather than silently producing nothing.

## Verify the install

```bash
reptr --version
# reptr 0.8.0
```
