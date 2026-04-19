//! Integrated terminal panel for running code and viewing output.
//!
//! Provides a bottom panel that can spawn subprocesses, capture their
//! stdout/stderr, and display the output with proper coloring.

#![allow(
    clippy::cast_possible_truncation,
    clippy::struct_excessive_bools,
)]

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;

use crate::theme::Theme;

/// A single line of terminal output.
#[derive(Debug, Clone)]
pub enum OutputLine {
    /// Standard output.
    Stdout(String),
    /// Standard error.
    Stderr(String),
    /// System message (process started, exited, etc.).
    System(String),
}

/// State for the integrated terminal panel.
pub struct TerminalPanel {
    /// Whether the panel is visible.
    pub visible: bool,
    /// Output lines collected from the subprocess.
    pub output: Vec<OutputLine>,
    /// Scroll offset within the output.
    pub scroll_offset: usize,
    /// Whether a process is currently running.
    pub running: bool,
    /// Exit code of the last process (None if still running or never ran).
    pub exit_code: Option<i32>,
    /// Receiver for incoming output lines from the async subprocess.
    pub receiver: Option<mpsc::UnboundedReceiver<OutputLine>>,
    /// Panel height in lines (excluding border).
    pub height: u16,
    /// Whether the terminal panel has keyboard focus.
    pub focused: bool,
}

impl TerminalPanel {
    /// Create a new terminal panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            visible: false,
            output: Vec::new(),
            scroll_offset: 0,
            running: false,
            exit_code: None,
            receiver: None,
            height: 10,
            focused: false,
        }
    }

    /// Toggle panel visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Clear the output buffer.
    pub fn clear(&mut self) {
        self.output.clear();
        self.scroll_offset = 0;
    }

    /// Drain any pending output from the async receiver.
    ///
    /// Returns `true` if new output was received (triggers redraw).
    pub fn drain_output(&mut self) -> bool {
        let mut received = false;
        if let Some(ref mut rx) = self.receiver {
            while let Ok(line) = rx.try_recv() {
                // Check for process exit
                if let OutputLine::System(ref msg) = line
                    && msg.starts_with("Process exited")
                {
                    self.running = false;
                }
                self.output.push(line);
                received = true;
            }
        }
        // Auto-scroll to bottom when new output arrives
        if received {
            let max_scroll = self.output.len().saturating_sub(1);
            self.scroll_offset = max_scroll;
        }
        received
    }

    /// Spawn a subprocess and capture its output.
    ///
    /// The command is run asynchronously. Output is sent via an mpsc channel
    /// and drained in the event loop via `drain_output()`.
    pub fn run_command(&mut self, cmd: &str, args: &[&str], cwd: &Path) {
        // Clear previous output
        self.clear();
        self.running = true;
        self.exit_code = None;
        self.visible = true;

        let display_cmd = if args.is_empty() {
            cmd.to_string()
        } else {
            format!("{cmd} {}", args.join(" "))
        };
        self.output.push(OutputLine::System(format!(
            "$ {display_cmd}  (in {})", cwd.display()
        )));

        let (tx, rx) = mpsc::unbounded_channel();
        self.receiver = Some(rx);

        let cmd_str = cmd.to_string();
        let args_owned: Vec<String> = args.iter().map(ToString::to_string).collect();
        let cwd_owned = cwd.to_path_buf();

        tokio::spawn(async move {
            let result = TokioCommand::new(&cmd_str)
                .args(&args_owned)
                .current_dir(&cwd_owned)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match result {
                Ok(mut child) => {
                    // Capture stdout
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();
                    let tx_out = tx.clone();
                    let tx_err = tx.clone();

                    let stdout_handle = stdout.map(|out| tokio::spawn(async move {
                        let reader = BufReader::new(out);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = tx_out.send(OutputLine::Stdout(line));
                        }
                    }));

                    let stderr_handle = stderr.map(|err| tokio::spawn(async move {
                        let reader = BufReader::new(err);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            let _ = tx_err.send(OutputLine::Stderr(line));
                        }
                    }));

                    // Wait for streams to finish
                    if let Some(h) = stdout_handle { let _ = h.await; }
                    if let Some(h) = stderr_handle { let _ = h.await; }

                    // Wait for process to exit
                    match child.wait().await {
                        Ok(status) => {
                            let code = status.code().unwrap_or(-1);
                            let _ = tx.send(OutputLine::System(format!(
                                "Process exited with code {code}"
                            )));
                        }
                        Err(e) => {
                            let _ = tx.send(OutputLine::System(format!(
                                "Process error: {e}"
                            )));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(OutputLine::System(format!(
                        "Failed to start: {e}"
                    )));
                }
            }
        });
    }

    /// Scroll up in the output.
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down in the output.
    pub fn scroll_down(&mut self, amount: usize) {
        let max = self.output.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max);
    }

    /// Draw the terminal panel.
    pub fn draw(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let title = if self.running {
            " Terminal ● Running "
        } else if let Some(code) = self.exit_code {
            if code == 0 {
                " Terminal ✓ "
            } else {
                " Terminal ✗ "
            }
        } else {
            " Terminal "
        };

        let border_color = if self.focused {
            theme.accent
        } else {
            theme.sidebar_border
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.sidebar_bg).fg(theme.sidebar_fg));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.output.is_empty() {
            let hint = Paragraph::new("  Press F5 to run current file, or Ctrl+` to toggle")
                .style(Style::default().fg(theme.gutter_fg));
            frame.render_widget(hint, inner);
            return;
        }

        // Render visible output lines
        let visible_height = inner.height as usize;
        let end = (self.scroll_offset + visible_height).min(self.output.len());
        let start = end.saturating_sub(visible_height);

        let lines: Vec<Line<'_>> = self.output[start..end]
            .iter()
            .map(|line| match line {
                OutputLine::Stdout(text) => {
                    Line::from(Span::styled(format!("  {text}"), Style::default().fg(theme.fg)))
                }
                OutputLine::Stderr(text) => {
                    Line::from(Span::styled(format!("  {text}"), Style::default().fg(theme.error)))
                }
                OutputLine::System(text) => {
                    Line::from(Span::styled(
                        format!("  {text}"),
                        Style::default().fg(theme.accent).add_modifier(Modifier::DIM),
                    ))
                }
            })
            .collect();

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, inner);
    }
}

impl Default for TerminalPanel {
    fn default() -> Self { Self::new() }
}

/// Detect the run command for a file based on its extension.
///
/// Returns `(command, args, use_workspace_root)` or `None` if unsupported.
#[must_use]
pub fn detect_run_command(path: &Path) -> Option<RunConfig> {
    let ext = path.extension()?.to_str()?;
    let file = path.to_string_lossy().to_string();

    match ext {
        "py" => Some(RunConfig {
            command: "python".to_string(),
            args: vec![file],
            use_file_dir: true,
        }),
        "js" => Some(RunConfig {
            command: "node".to_string(),
            args: vec![file],
            use_file_dir: true,
        }),
        "ts" => Some(RunConfig {
            command: "npx".to_string(),
            args: vec!["tsx".to_string(), file],
            use_file_dir: true,
        }),
        "rs" => Some(RunConfig {
            command: "cargo".to_string(),
            args: vec!["run".to_string()],
            use_file_dir: false, // use workspace root
        }),
        "go" => Some(RunConfig {
            command: "go".to_string(),
            args: vec!["run".to_string(), file],
            use_file_dir: true,
        }),
        "sh" => Some(RunConfig {
            command: "bash".to_string(),
            args: vec![file],
            use_file_dir: true,
        }),
        "ps1" => Some(RunConfig {
            command: "powershell".to_string(),
            args: vec!["-File".to_string(), file],
            use_file_dir: true,
        }),
        "rb" => Some(RunConfig {
            command: "ruby".to_string(),
            args: vec![file],
            use_file_dir: true,
        }),
        "java" => Some(RunConfig {
            command: "java".to_string(),
            args: vec![file],
            use_file_dir: true,
        }),
        "cpp" | "cc" | "cxx" => Some(RunConfig {
            command: "g++".to_string(),
            args: vec!["-o".to_string(), "a.out".to_string(), file, "&&".to_string(), "./a.out".to_string()],
            use_file_dir: true,
        }),
        _ => None,
    }
}

/// Configuration for running a file.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// The command to execute.
    pub command: String,
    /// Arguments to the command.
    pub args: Vec<String>,
    /// Whether to use the file's directory as cwd (vs workspace root).
    pub use_file_dir: bool,
}
