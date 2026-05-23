use anyhow::Result;
use clap::Parser;
use console::style;
use tracing_subscriber::EnvFilter;

use reptr::cli::{AddTarget, Cli, Command, LibraryAction, StatsFormat};
use reptr::commands;

fn main() {
    init_tracing();
    if let Err(err) = run() {
        eprintln!("{} {:#}", style("error:").red().bold(), err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::New { name } => commands::new::run(&name),
        Command::Add { what } => match what {
            AddTarget::Finding {
                title,
                severity,
                from,
                path,
            } => commands::add::run(&path, title.as_deref(), severity.into(), from.as_deref()),
        },
        Command::Build { path } => commands::build::run(&path),
        Command::Watch { path } => commands::watch::run(&path),
        Command::Stats { path, format } => {
            let fmt = match format {
                StatsFormat::Text => commands::stats::Format::Text,
                StatsFormat::Json => commands::stats::Format::Json,
            };
            commands::stats::run(&path, fmt)
        }
        Command::Library { action } => match action {
            LibraryAction::List { path } => commands::library::list(&path),
        },
        Command::Retest { path } => commands::retest::run(&path),
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_env("REPTR_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
}
