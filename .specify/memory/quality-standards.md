# Kryon Code Quality & Maintainability Standards

**Version**: 1.0.0
**Date**: 2026-04-19

## Rust Coding Standards

### Formatting & Linting
- ALL code MUST pass `cargo fmt -- --check`
- ALL code MUST pass `cargo clippy --all-targets -- -D warnings`
- Use `#![deny(clippy::all, clippy::pedantic)]` at crate level
- Allow specific lints only with documented justification

### Naming Conventions
- Modules: `snake_case` (e.g., `text_buffer`, `file_explorer`)
- Types: `PascalCase` (e.g., `TextBuffer`, `CursorPosition`)
- Functions: `snake_case` verb phrases (e.g., `insert_text`, `move_cursor`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BUFFER_SIZE`)
- Boolean functions: `is_`, `has_`, `can_` prefix (e.g., `is_modified`)
- Builder methods: return `&mut Self` or `Self`

### Error Handling
- Library crates MUST use `thiserror` for typed errors
- Binary crates MAY use `anyhow` for top-level error aggregation
- NEVER use `unwrap()` or `expect()` in library code
- Use `?` operator for error propagation
- All error types MUST implement `std::error::Error`
- Error messages MUST be lowercase without trailing punctuation (Rust convention)

### Documentation
- ALL public items MUST have rustdoc comments (`///`)
- Module-level documentation MUST use `//!`
- Examples in doc comments MUST compile (`cargo test --doc`)
- Non-obvious design decisions MUST include `# Design` section

### Architecture Patterns
- Elm/Flux pattern for UI state: Action → Update → View
- Command pattern for all editor operations (enables undo/redo)
- Trait-based abstractions for testability (dependency injection)
- Message passing between components (channels, not shared mutable state)
- Single-writer principle for shared state

### Testing Requirements
- Unit tests in same file (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory
- Property-based testing with `proptest` for buffer operations
- Snapshot testing for UI rendering (`insta` crate)
- Minimum 80% coverage on core engine
- All public API functions MUST have at least one test

### Performance Guidelines
- Render only changed terminal regions (diff-based rendering)
- Use `ropey` for O(log n) text operations
- Limit Tree-sitter queries to visible viewport
- Debounce expensive operations (search, diagnostics)
- Use `tokio::task::spawn_blocking` for heavy computation
- Profile with `criterion` benchmarks for hot paths
- Virtual scroll for large file lists and long documents

### Module Boundaries
- Each crate MUST have a clear, documented public API
- Internal types MUST be `pub(crate)` or private
- Cross-crate communication through well-defined trait interfaces
- No circular dependencies between crates
