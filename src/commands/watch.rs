use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use console::style;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::build;

const DEBOUNCE: Duration = Duration::from_millis(250);

pub fn run(root: &Path) -> Result<()> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    // Build once on startup so the user sees the current state.
    if let Err(e) = build::run(&root) {
        eprintln!("{} {:#}", style("error:").red().bold(), e);
    }
    println!("{} {}", style("Watching").cyan().bold(), root.display());

    let (tx, rx) = channel::<notify::Result<notify::Event>>();
    let mut watcher =
        RecommendedWatcher::new(tx, Config::default()).context("creating filesystem watcher")?;

    for sub in ["findings", "templates", "reptr.toml", "client.toml"] {
        let path = root.join(sub);
        if path.exists() {
            watcher
                .watch(&path, RecursiveMode::Recursive)
                .with_context(|| format!("watching {}", path.display()))?;
        }
    }

    let mut pending: Vec<PathBuf> = Vec::new();
    loop {
        match rx.recv_timeout(if pending.is_empty() {
            Duration::from_secs(60 * 60)
        } else {
            DEBOUNCE
        }) {
            Ok(Ok(event)) => {
                if !is_relevant(&event.kind) {
                    continue;
                }
                pending.extend(event.paths);
            }
            Ok(Err(e)) => {
                eprintln!("{} {e}", style("watch error:").yellow().bold());
            }
            Err(RecvTimeoutError::Timeout) => {
                if !pending.is_empty() {
                    let started = Instant::now();
                    let trigger = pretty_trigger(&pending, &root);
                    pending.clear();

                    match build::run(&root) {
                        Ok(()) => {
                            println!(
                                "{} in {:.0?} (triggered by {trigger})",
                                style("✓ Rebuilt").green(),
                                started.elapsed()
                            );
                        }
                        Err(e) => {
                            eprintln!("{} {:#}", style("✗ Build failed:").red().bold(), e);
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

fn is_relevant(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn pretty_trigger(paths: &[PathBuf], root: &Path) -> String {
    let mut names: Vec<String> = paths
        .iter()
        .filter_map(|p| p.strip_prefix(root).ok().map(|p| p.display().to_string()))
        .collect();
    names.sort();
    names.dedup();
    if names.len() <= 2 {
        names.join(", ")
    } else {
        format!("{} files", names.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

    // --- is_relevant ---------------------------------------------------------

    #[test]
    fn is_relevant_accepts_create() {
        assert!(is_relevant(&EventKind::Create(CreateKind::File)));
    }

    #[test]
    fn is_relevant_accepts_modify() {
        assert!(is_relevant(&EventKind::Modify(ModifyKind::Any)));
    }

    #[test]
    fn is_relevant_accepts_remove() {
        assert!(is_relevant(&EventKind::Remove(RemoveKind::File)));
    }

    #[test]
    fn is_relevant_rejects_access_events() {
        assert!(!is_relevant(&EventKind::Access(AccessKind::Any)));
    }

    #[test]
    fn is_relevant_rejects_other() {
        assert!(!is_relevant(&EventKind::Other));
    }

    // --- pretty_trigger ------------------------------------------------------

    #[test]
    fn pretty_trigger_single_file_shows_relative_path() {
        let root = PathBuf::from("/eng");
        let paths = vec![PathBuf::from("/eng/findings/001.md")];
        assert_eq!(pretty_trigger(&paths, &root), "findings/001.md");
    }

    #[test]
    fn pretty_trigger_two_files_joined_with_comma() {
        let root = PathBuf::from("/eng");
        let paths = vec![
            PathBuf::from("/eng/findings/001.md"),
            PathBuf::from("/eng/findings/002.md"),
        ];
        let result = pretty_trigger(&paths, &root);
        assert!(result.contains("findings/001.md"), "got: {result}");
        assert!(result.contains("findings/002.md"), "got: {result}");
        assert!(
            result.contains(','),
            "two paths should be comma-separated. got: {result}"
        );
    }

    #[test]
    fn pretty_trigger_three_files_shows_count() {
        let root = PathBuf::from("/eng");
        let paths = vec![
            PathBuf::from("/eng/findings/001.md"),
            PathBuf::from("/eng/findings/002.md"),
            PathBuf::from("/eng/findings/003.md"),
        ];
        let result = pretty_trigger(&paths, &root);
        assert_eq!(result, "3 files", "got: {result}");
    }

    #[test]
    fn pretty_trigger_deduplicates_repeated_paths() {
        let root = PathBuf::from("/eng");
        // Same path sent twice (common when an editor does a save + chmod).
        let paths = vec![
            PathBuf::from("/eng/findings/001.md"),
            PathBuf::from("/eng/findings/001.md"),
        ];
        let result = pretty_trigger(&paths, &root);
        // After dedup it's a single path — should NOT say "2 files".
        assert!(
            !result.contains("files"),
            "duplicate paths should be deduped. got: {result}"
        );
        assert!(result.contains("findings/001.md"), "got: {result}");
    }

    #[test]
    fn pretty_trigger_strips_root_prefix() {
        let root = PathBuf::from("/long/path/to/engagement");
        let paths = vec![PathBuf::from("/long/path/to/engagement/reptr.toml")];
        assert_eq!(pretty_trigger(&paths, &root), "reptr.toml");
    }

    #[test]
    fn pretty_trigger_out_of_root_path_excluded() {
        // A path that can't have the root stripped is silently dropped.
        let root = PathBuf::from("/eng");
        let paths = vec![PathBuf::from("/other/dir/file.md")];
        // strip_prefix fails → filter_map drops it → result is empty string
        assert_eq!(pretty_trigger(&paths, &root), "");
    }
}
