//! Safe subprocess execution.
//!
//! All subprocess invocations MUST use structured argument arrays via
//! `std::process::Command`. Shell string interpolation is forbidden.
//! This module provides a safe wrapper that enforces this pattern.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Errors during subprocess execution.
#[derive(Debug, thiserror::Error)]
pub enum SubprocessError {
    /// The command failed to start.
    #[error("failed to spawn process '{command}': {source}")]
    SpawnFailed {
        /// The command that failed to start.
        command: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The command exited with a non-zero status.
    #[error("process '{command}' exited with status {status}: {stderr}")]
    NonZeroExit {
        /// The command that failed.
        command: String,
        /// The exit status code.
        status: i32,
        /// Captured stderr output.
        stderr: String,
    },

    /// The command was not in the allowlist.
    #[error("command '{0}' is not allowed")]
    NotAllowed(String),
}

/// Execute a subprocess safely with structured arguments.
///
/// # Arguments
///
/// * `program` - The program to execute (must not contain shell metacharacters)
/// * `args` - Structured argument list (never concatenated into a shell string)
/// * `working_dir` - The working directory for the subprocess
///
/// # Security
///
/// This function NEVER passes arguments through a shell. Each argument is
/// passed directly to the OS as a separate entry, preventing injection attacks.
///
/// # Errors
///
/// Returns `SubprocessError::SpawnFailed` if the process cannot be started.
/// Returns `SubprocessError::NonZeroExit` if the process exits with a non-zero code.
pub fn safe_exec<S, I, A>(program: S, args: I, working_dir: &Path) -> Result<Output, SubprocessError>
where
    S: AsRef<OsStr>,
    I: IntoIterator<Item = A>,
    A: AsRef<OsStr>,
{
    let program_ref = program.as_ref();
    let cmd_name = program_ref.to_string_lossy().to_string();

    tracing::debug!(command = %cmd_name, dir = %working_dir.display(), "executing subprocess");

    let output = Command::new(program_ref)
        .args(args)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| SubprocessError::SpawnFailed {
            command: cmd_name.clone(),
            source,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status.code().unwrap_or(-1);
        return Err(SubprocessError::NonZeroExit {
            command: cmd_name,
            status,
            stderr,
        });
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_exec_echo() {
        // Use a command that exists on all platforms
        #[cfg(windows)]
        let result = safe_exec("cmd", ["/C", "echo", "hello"], Path::new("."));
        #[cfg(not(windows))]
        let result = safe_exec("echo", ["hello"], Path::new("."));

        assert!(result.is_ok());
        let output = result.unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hello"));
    }

    #[test]
    fn test_safe_exec_nonexistent_command() {
        let result = safe_exec(
            "this_command_does_not_exist_12345",
            Vec::<&str>::new(),
            Path::new("."),
        );
        assert!(matches!(result, Err(SubprocessError::SpawnFailed { .. })));
    }
}
