<!--
Sync Impact Report
Version: 1.0.0 (initial)
Added sections: All (initial creation)
Templates requiring updates: N/A (new project)
Follow-up TODOs: None
-->

# Project Constitution: Kryon

**Version**: 1.0.0
**Ratification Date**: 2026-04-19
**Last Amended Date**: 2026-04-19
**Status**: Active

---

## 1. Project Mission

Kryon is a terminal-first, production-grade code editor written in Rust. It MUST deliver a VS Code-caliber editing experience in the terminal, with a modular architecture that later powers a web-based companion editor. The product targets professional developers who demand speed, safety, extensibility, and polish.

---

## 2. Core Principles

### Principle 1: Performance First

All design decisions MUST prioritize responsiveness and low resource consumption. The editor MUST start instantly, render only changed regions, never block the UI thread, and gracefully degrade on constrained terminals. Hot paths MUST avoid unnecessary allocations.

**Rationale**: A code editor that feels sluggish is abandoned regardless of features.

### Principle 2: Security by Default

All user input, file paths, plugin metadata, subprocess arguments, and external data MUST be treated as untrusted. The system MUST use structured subprocess APIs (never string concatenation for commands), capability-based permissions, path normalization with boundary checks, and explicit confirmation for destructive operations. Secrets MUST never be logged.

**Rationale**: Developer tools operate on sensitive source code and credentials. A compromised editor compromises everything.

### Principle 3: Modular Architecture

The system MUST be organized as layered, independently testable modules: Core Engine, UI Layer, Language Services, Debugging, Plugin System, Security Boundary, and Web Companion. The editor "brain" (core engine) MUST be reusable across terminal, web, and future desktop frontends without duplicating business logic.

**Rationale**: Shared core prevents divergence, reduces maintenance burden, and enables multi-platform delivery.

### Principle 4: Safety and Data Integrity

The editor MUST never silently discard user changes. Unsaved buffers MUST be tracked, autosave MUST be configurable, crash recovery MUST restore session state, and destructive operations (file delete, overwrite, close unsaved) MUST require explicit confirmation. File writes MUST be atomic when the platform permits.

**Rationale**: Developers trust their editor with their most important asset — their code.

### Principle 5: Extensibility Through Constraints

Plugins MUST operate within a sandboxed, capability-based permission model (preferably WASM via Wasmtime). Extensions MUST declare capabilities in a manifest, MUST NOT bypass security boundaries, and MUST be crash-isolated from the host process. The plugin API MUST be versioned with a stable contract.

**Rationale**: Powerful extensibility without guardrails becomes a security and stability liability.

### Principle 6: Discoverability and Inclusivity

The editor MUST be immediately usable by newcomers without requiring deep modal-editing knowledge. A command palette, keybinding hints, tooltips, onboarding flow, and searchable settings MUST be first-class features. Mouse interaction MUST be a first-class input method alongside keyboard.

**Rationale**: Adoption depends on a low barrier to entry and gentle learning curve.

### Principle 7: Rust Idioms and Quality Standards

All code MUST compile with zero warnings under `clippy` (pedantic lint level). All public APIs MUST have documentation. All modules MUST have unit tests. Error handling MUST use `Result`/`Option` types — panics are forbidden in library code. The project MUST use the Rust 2024 Edition and follow standard `rustfmt` formatting.

**Rationale**: Rust's type system and tooling enforce correctness when used idiomatically.

---

## 3. Technology Decisions

| Domain | Decision | Rationale |
|--------|----------|-----------|
| Language | Rust (2024 Edition) | Safety, performance, ecosystem fit for terminal tooling |
| TUI Framework | Ratatui + Crossterm | Mature, actively maintained, cross-platform terminal rendering |
| Syntax Parsing | Tree-sitter | Incremental parsing, error tolerance, language injection support |
| Language Features | LSP (client) | Industry standard for language intelligence |
| Debugging | DAP (client) | Industry standard for debug adapters |
| Plugin Runtime | Wasmtime (WASM Component Model) | Sandboxed, language-agnostic, capability-based |
| CLI Argument Parsing | clap (derive) | Industry standard, auto-generated help, shell completions |
| Async Runtime | tokio | Standard for Rust async I/O, required by LSP/DAP |
| Text Buffer | Ropey (rope data structure) | O(log n) editing, efficient for large files |
| Config Format | TOML (user config), JSON (state/session) | TOML for human editing, JSON for programmatic state |
| Search | ripgrep engine (grep-searcher crate) | Fast, Unicode-aware, gitignore-respecting |
| Error Types | thiserror (library), anyhow (application) | Idiomatic Rust error handling |
| Logging | tracing crate | Structured, async-compatible, span-based |
| Web Future | Local HTTP server + WebSocket bridge | Same backend, browser-based presentation layer |

---

## 4. Quality Standards

### Code Quality
- MUST pass `cargo clippy --all-targets -- -D warnings`
- MUST pass `cargo fmt -- --check`
- MUST maintain ≥80% code coverage on core engine modules
- MUST have zero `unsafe` blocks unless reviewed and documented

### Security
- MUST follow OWASP guidelines adapted for desktop/CLI applications
- MUST validate all file paths against workspace boundaries
- MUST use `std::process::Command` with explicit argument arrays (never shell strings)
- MUST implement path normalization and traversal prevention
- MUST redact sensitive content in logs and diagnostics

### Architecture
- MUST maintain clear module boundaries with explicit public APIs
- MUST use dependency injection for testability
- MUST keep UI rendering free of business logic
- MUST separate state management from view rendering (Elm/Flux architecture)

### Documentation
- Every public function, struct, and enum MUST have rustdoc comments
- All non-obvious design decisions MUST be documented with rationale
- Architecture Decision Records (ADRs) MUST be created for significant choices

---

## 5. Governance

### Amendment Procedure
1. Propose amendment with rationale
2. Review impact on existing codebase and artifacts
3. Update constitution with version bump
4. Propagate changes to dependent documents

### Versioning
- MAJOR: Principle removal or incompatible redefinition
- MINOR: New principle or material expansion
- PATCH: Clarification, wording, or formatting fix

### Compliance Review
- Constitution compliance MUST be reviewed at each phase checkpoint
- New code MUST align with all active principles before merge
