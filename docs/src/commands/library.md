# reptr library

Manage your personal finding library — a collection of reusable finding templates.

## Usage

```bash
reptr library list [--format text|json]
reptr library add <PATH>
reptr library remove <NAME>
reptr library show <NAME>
```

## What is the library?

The library is a directory of Markdown finding files stored at `~/.config/reptr/library/`. Each file is a fully-formed finding template you can import into any engagement with `reptr add finding --from <name>`.

## Listing templates

```bash
reptr library list
```

```
Library templates (5)

  web/sql-injection            SQL Injection                 critical
  web/xss-stored               Stored Cross-Site Scripting   high
  web/xss-reflected            Reflected Cross-Site Scripting high
  web/idor                     Broken Object-Level Auth       high
  infra/default-credentials    Default Credentials            medium
```

JSON output:

```bash
reptr library list --format json
```

```json
[
  {
    "name": "web/sql-injection",
    "title": "SQL Injection",
    "severity": "critical",
    "path": "~/.config/reptr/library/web/sql-injection.md"
  }
]
```

## Adding a template

```bash
reptr library add findings/001-sql-injection.md --name web/sql-injection
```

The file is copied into the library under the given name. Subdirectory separators (the `/` in `web/sql-injection`) create folders in the library — useful for organising templates by category.

If `--name` is omitted the template is stored using the source file's stem.

## Removing a template

```bash
reptr library remove web/sql-injection
# Removed web/sql-injection
```

## Showing a template

```bash
reptr library show web/sql-injection
```

Prints the raw Markdown source of the template, including front matter.

## Using a template in an engagement

```bash
reptr add finding --from web/sql-injection
# Created findings/002-sql-injection.md
```

See [reptr add](add.md) for full options.

## Sharing libraries

The library directory is plain Markdown files under `~/.config/reptr/library/`. You can version-control it in a separate git repository and symlink or copy it across machines:

```bash
# Clone your team's shared library
git clone https://github.com/your-org/reptr-library ~/.config/reptr/library

# Pull updates before a new engagement
cd ~/.config/reptr/library && git pull
```
