# Project Index: Kryon

**Last Updated**: 2026-04-19
**Status**: New Project (Pre-implementation)

## Overview

Kryon is a terminal-first, production-grade code editor written in Rust. It targets professional developers seeking a VS Code-caliber experience within the terminal, with a modular architecture enabling a future web-based companion.

## Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Language | Rust 2024 Edition | Core engine, all modules |
| TUI | Ratatui + Crossterm | Terminal rendering, input |
| Parsing | Tree-sitter | Syntax highlighting, incremental parsing |
| Text Buffer | Ropey | Rope data structure for efficient editing |
| Language Services | LSP Client | Completions, diagnostics, hover, etc. |
| Debugging | DAP Client | Debug adapter protocol |
| Plugins | Wasmtime (WASM) | Sandboxed extension runtime |
| CLI | clap (derive) | Argument parsing |
| Async | tokio | Async runtime |
| Search | grep-searcher | Workspace search |
| Config | TOML | User configuration |
| Logging | tracing | Structured logging |
| Errors | thiserror / anyhow | Error handling |

## Planned Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Kryon Architecture                  │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │              CLI Entry Point                  │  │
│  │    (clap, arg parsing, mode selection)        │  │
│  └──────────────────┬────────────────────────────┘  │
│                     │                               │
│  ┌──────────────────┴────────────────────────────┐  │
│  │            Application Shell                  │  │
│  │    (event loop, focus mgmt, keybindings)      │  │
│  └──┬──────────┬──────────┬──────────┬───────────┘  │
│     │          │          │          │               │
│  ┌──┴───┐  ┌──┴───┐  ┌──┴───┐  ┌──┴─────────┐    │
│  │  UI  │  │ Core │  │  LSP │  │  Plugins   │    │
│  │Layer │  │Engine│  │Client│  │  (WASM)    │    │
│  └──────┘  └──────┘  └──────┘  └────────────┘    │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │           Security Boundary Layer             │  │
│  │   (path validation, permissions, sandboxing)  │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌───────────────────────────────────────────────┐  │
│  │          Web Companion (Future)               │  │
│  │   (localhost server, WebSocket bridge)        │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## Domain Map

### Core Engine (`kryon-core`)
- Text buffer management (ropey)
- Cursor/selection model (multi-cursor)
- Undo/redo (command pattern)
- Diff computation
- Syntax tree integration (tree-sitter)
- Workspace model
- File operations (safe, validated)
- Search engine
- Theme/layout state
- Session persistence

### UI Layer (`kryon-tui`)
- Terminal rendering (ratatui)
- Layout system (panels, splits, tabs)
- Input routing (keyboard + mouse)
- Focus management
- Widget library (editor, tree, palette, statusbar)
- Responsive layout adaptation

### Language Services (`kryon-lsp`)
- LSP client implementation
- Completion, hover, diagnostics
- Go-to-definition, references
- Rename, formatting, code actions

### Plugin System (`kryon-plugin`)
- WASM runtime (wasmtime)
- Plugin manifest parsing
- Capability-based permissions
- Extension API (commands, views, themes)

### Security (`kryon-security`)
- Path normalization & validation
- Permission model
- Subprocess safety
- Secret redaction

### CLI (`kryon-cli`)
- Entry point, argument parsing
- Mode selection (edit, web, settings)
- Shell completions

## Key Patterns

1. **Elm Architecture**: Action → State Update → Render
2. **Command Pattern**: All operations as undoable commands
3. **Trait Abstraction**: Core traits for testability
4. **Message Passing**: Channels between components
5. **Incremental Rendering**: Diff-based terminal updates

## File Structure (Planned)

```
kryon/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── kryon-core/         # Core engine (buffer, cursor, commands)
│   ├── kryon-tui/          # Terminal UI (ratatui-based)
│   ├── kryon-lsp/          # LSP client
│   ├── kryon-plugin/       # WASM plugin runtime
│   ├── kryon-security/     # Security primitives
│   └── kryon-cli/          # Binary entry point
├── config/                 # Default configs, themes
├── docs/                   # Architecture docs
├── tests/                  # Integration tests
└── .specify/               # Constitution & standards
```
