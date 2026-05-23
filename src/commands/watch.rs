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
