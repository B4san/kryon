# Kryon Security Guidelines

**Version**: 1.0.0
**Date**: 2026-04-19

## Threat Model

Kryon is a code editor that:
- Operates on user source code (high-value asset)
- Executes subprocesses (LSP servers, build tools, formatters)
- Loads third-party plugins (WASM extensions)
- Reads/writes arbitrary files within a workspace
- Will later expose a localhost web server (web companion)

### Threat Categories

| Threat | Vector | Mitigation |
|--------|--------|------------|
| Path traversal | Malicious file paths from user input or plugins | Canonical path resolution + boundary check |
| Command injection | Subprocess execution with user-supplied arguments | `Command::new()` with explicit arg arrays, never shell strings |
| Plugin escape | WASM plugin attempting host access | Wasmtime sandboxing, capability-based permissions, fuel limits |
| Symlink attacks | Symlinks pointing outside workspace | Resolve symlinks before boundary checks |
| Data loss | Accidental file overwrite or deletion | Confirmation prompts, undo-aware operations, atomic writes |
| Secret leakage | Credentials in logs, diagnostics, or error messages | Structured logging with redaction filters |
| Supply chain | Malicious or typosquatted dependencies | Cargo.lock pinning, `cargo audit`, minimal dependencies |
| Web companion CSRF | Cross-origin requests to localhost server | Token-based auth, Origin validation, localhost binding |

## Rust-Specific Security Rules

### Path Handling
```rust
// MUST: Always normalize and validate paths
fn safe_resolve(workspace_root: &Path, user_path: &str) -> Result<PathBuf> {
    let canonical_root = workspace_root.canonicalize()?;
    let resolved = canonical_root.join(user_path).canonicalize()?;
    if !resolved.starts_with(&canonical_root) {
        return Err(SecurityError::PathTraversal);
    }
    Ok(resolved)
}
```

### Subprocess Execution
```rust
// MUST: Use structured Command API
let output = Command::new("rustfmt")
    .arg("--edition")
    .arg("2024")
    .arg(&file_path)  // Never interpolated into a shell string
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

// NEVER: Shell string concatenation
// let output = Command::new("sh").arg("-c").arg(format!("rustfmt {}", path));
```

### Error Messages
```rust
// MUST: Generic user-facing errors, detailed internal logs
match operation_result {
    Err(e) => {
        tracing::error!(error = %e, path = %path.display(), "File operation failed");
        return Err(UserError::new("Unable to complete file operation"));
    }
}
```

## Plugin Security Model

1. Plugins run in WASM sandbox (Wasmtime)
2. Plugins declare required capabilities in manifest
3. Host grants capabilities explicitly (file read, file write, network, subprocess)
4. Fuel limits prevent infinite loops
5. Memory limits prevent DoS
6. Plugins MUST NOT access host filesystem directly
7. Plugins communicate through structured message passing

## Dependency Policy

- Pin all versions via Cargo.lock (committed to VCS)
- Run `cargo audit` in CI pipeline
- Prefer `std` over third-party when equivalent
- Review new dependencies for maintenance status and security history
- Minimize transitive dependency count
