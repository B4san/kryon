//! Kryon — Terminal-first code editor.
//!
//! Entry point for the CLI. Parses arguments and launches the TUI.

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

/// Kryon — Terminal-first code editor
#[derive(Parser, Debug)]
#[command(name = "kryon", version, about = "Terminal-first code editor")]
struct Cli {
    /// File or directory to open
    path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("kryon=info".parse()?),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    tracing::info!(path = ?cli.path, "starting kryon");

    // Load content and determine workspace root
    let mut app = if let Some(ref path) = cli.path {
        if path.is_file() {
            let content = std::fs::read_to_string(path)?;
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            let workspace = canonical.parent().map(|p| p.to_path_buf());
            kryon_tui::app::App::with_file(&content, canonical, workspace)
        } else if path.is_dir() {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            kryon_tui::app::App::with_workspace(canonical)
        } else {
            // Path doesn't exist yet — use current directory as workspace
            let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            kryon_tui::app::App::with_workspace(workspace)
        }
    } else {
        // No path given — use current directory as workspace
        let workspace = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        kryon_tui::app::App::with_workspace(workspace)
    };

    // Initialize terminal
    let mut terminal = kryon_tui::terminal::init()?;

    // Run the event loop
    let result = kryon_tui::app::run(&mut terminal, &mut app).await;

    // Restore terminal
    kryon_tui::terminal::restore()?;

    // Report any errors
    result?;

    Ok(())
}
