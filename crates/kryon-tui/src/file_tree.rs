//! File tree widget for the sidebar explorer.
//!
//! Scans a workspace directory and renders a navigable, collapsible
//! tree of files and directories.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

/// A single entry in the file tree (file or directory).
#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    /// Display name (file/dir name, not full path).
    pub name: String,
    /// Full path on disk.
    pub path: PathBuf,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Nesting depth (0 = root children).
    pub depth: usize,
    /// Whether this directory is expanded (ignored for files).
    pub expanded: bool,
    /// Indices of direct children in the `entries` vec (dirs only).
    pub children: Vec<usize>,
}

/// The file tree state for the sidebar explorer.
#[derive(Debug)]
pub struct FileTree {
    /// Workspace root path.
    pub root: PathBuf,
    /// Flat list of all entries (populated lazily via expand).
    entries: Vec<FileTreeEntry>,
    /// Flat list of currently visible entry indices (respecting collapse state).
    visible: Vec<usize>,
    /// Currently selected index in the `visible` list.
    pub selected: usize,
    /// Maximum scan depth to prevent runaway recursion.
    max_depth: usize,
}

impl FileTree {
    /// Create a new file tree rooted at the given path.
    ///
    /// Scans the first level of children immediately and auto-expands root.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        let mut tree = Self {
            root: root.to_path_buf(),
            entries: Vec::new(),
            visible: Vec::new(),
            selected: 0,
            max_depth: 10,
        };
        // Build root-level entries
        tree.scan_dir(root, 0);
        tree.sort_entries_at_depth(0);
        tree.rebuild_visible();
        tree
    }

    /// Scan a directory and add its children as entries.
    fn scan_dir(&mut self, dir: &Path, depth: usize) {
        if depth > self.max_depth {
            return;
        }
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };

        for entry in read_dir.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files/dirs (starting with .)
            if name.starts_with('.') {
                continue;
            }
            // Skip common build/dependency directories
            if metadata.is_dir()
                && matches!(
                    name.as_str(),
                    "target" | "node_modules" | "__pycache__" | ".git"
                )
            {
                continue;
            }

            self.entries.push(FileTreeEntry {
                name,
                path: entry.path(),
                is_dir: metadata.is_dir(),
                depth,
                expanded: false,
                children: Vec::new(),
            });
        }
    }

    /// Sort entries at a given depth: directories first, then alphabetical.
    fn sort_entries_at_depth(&mut self, depth: usize) {
        // Collect indices of entries at this depth
        let mut indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.depth == depth)
            .map(|(i, _)| i)
            .collect();

        indices.sort_by(|&a, &b| {
            let ea = &self.entries[a];
            let eb = &self.entries[b];
            // Directories first
            match (ea.is_dir, eb.is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => ea.name.to_lowercase().cmp(&eb.name.to_lowercase()),
            }
        });

        // Reorder entries at this depth by collecting and reinserting
        // For simplicity at depth 0, just sort the entire entries vec
        if depth == 0 {
            self.entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });
        }
    }

    /// Rebuild the flat visible list based on expanded state.
    fn rebuild_visible(&mut self) {
        self.visible.clear();
        self.rebuild_visible_recursive(0);
        // Clamp selected
        if !self.visible.is_empty() && self.selected >= self.visible.len() {
            self.selected = self.visible.len() - 1;
        }
    }

    /// Recursively add visible entries at the given depth.
    fn rebuild_visible_recursive(&mut self, depth: usize) {
        // Gather entries at this depth (we need indices)
        let indices: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.depth == depth)
            .map(|(i, _)| i)
            .collect();

        for idx in indices {
            // An entry is visible if all its ancestors are expanded.
            // For depth 0, always visible. For deeper entries, we check
            // if they were added as children of an expanded parent.
            if depth == 0 {
                self.visible.push(idx);
                if self.entries[idx].is_dir && self.entries[idx].expanded {
                    self.add_children_visible(idx);
                }
            }
        }
    }

    /// Add visible children of an expanded directory entry.
    fn add_children_visible(&mut self, parent_idx: usize) {
        let children = self.entries[parent_idx].children.clone();
        for child_idx in children {
            self.visible.push(child_idx);
            if self.entries[child_idx].is_dir && self.entries[child_idx].expanded {
                self.add_children_visible(child_idx);
            }
        }
    }

    /// Toggle expand/collapse of the directory at the given visible index.
    ///
    /// If the directory was not yet scanned, scan it now (lazy loading).
    pub fn toggle(&mut self, visible_idx: usize) {
        let Some(&entry_idx) = self.visible.get(visible_idx) else {
            return;
        };
        if !self.entries[entry_idx].is_dir {
            return;
        }

        let was_expanded = self.entries[entry_idx].expanded;
        self.entries[entry_idx].expanded = !was_expanded;

        if !was_expanded && self.entries[entry_idx].children.is_empty() {
            // Lazy-load children
            let path = self.entries[entry_idx].path.clone();
            let child_depth = self.entries[entry_idx].depth + 1;
            let start_idx = self.entries.len();

            self.scan_dir(&path, child_depth);

            // Sort the newly added children
            let end_idx = self.entries.len();
            if start_idx < end_idx {
                // Sort just the new children slice
                self.entries[start_idx..end_idx].sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });

                // Register children
                let child_indices: Vec<usize> = (start_idx..end_idx).collect();
                self.entries[entry_idx].children = child_indices;
            }
        }

        self.rebuild_visible();
    }

    /// Get the path of the entry at the given visible index.
    #[must_use]
    pub fn path_at(&self, visible_idx: usize) -> Option<&Path> {
        self.visible
            .get(visible_idx)
            .map(|&idx| self.entries[idx].path.as_path())
    }

    /// Whether the entry at the given visible index is a directory.
    #[must_use]
    pub fn is_dir_at(&self, visible_idx: usize) -> bool {
        self.visible
            .get(visible_idx)
            .is_some_and(|&idx| self.entries[idx].is_dir)
    }

    /// Move selection up.
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn move_down(&mut self) {
        if !self.visible.is_empty() && self.selected < self.visible.len() - 1 {
            self.selected += 1;
        }
    }

    /// Handle Enter key: toggle dir or return file path to open.
    #[must_use]
    pub fn activate(&mut self) -> Option<PathBuf> {
        if self.is_dir_at(self.selected) {
            self.toggle(self.selected);
            None
        } else {
            self.path_at(self.selected).map(Path::to_path_buf)
        }
    }

    /// Get the currently visible entries for rendering.
    ///
    /// Returns an iterator of `(name, is_dir, depth, expanded, is_selected)`.
    #[must_use]
    pub fn visible_entries(&self) -> Vec<FileTreeRenderEntry<'_>> {
        self.visible
            .iter()
            .enumerate()
            .map(|(vis_idx, &entry_idx)| {
                let entry = &self.entries[entry_idx];
                FileTreeRenderEntry {
                    name: &entry.name,
                    is_dir: entry.is_dir,
                    depth: entry.depth,
                    expanded: entry.expanded,
                    selected: vis_idx == self.selected,
                }
            })
            .collect()
    }

    /// Number of visible entries.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        self.visible.len()
    }

    /// Handle a mouse click at a given row within the tree area.
    /// `row` is 0-based relative to the tree rendering area (excluding title).
    /// `scroll_offset` is how many entries have been scrolled past.
    ///
    /// Returns `Some(path)` if a file was clicked (to open it),
    /// or `None` if a directory was toggled or click was out of range.
    pub fn handle_click(&mut self, row: usize, scroll_offset: usize) -> Option<PathBuf> {
        let visible_idx = scroll_offset + row;
        if visible_idx >= self.visible.len() {
            return None;
        }
        self.selected = visible_idx;
        if self.is_dir_at(visible_idx) {
            self.toggle(visible_idx);
            None
        } else {
            self.path_at(visible_idx).map(Path::to_path_buf)
        }
    }
}

/// Data for rendering a single file tree entry.
#[derive(Debug)]
pub struct FileTreeRenderEntry<'a> {
    /// Display name.
    pub name: &'a str,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Nesting depth.
    pub depth: usize,
    /// Whether this directory is expanded.
    pub expanded: bool,
    /// Whether this entry is currently selected.
    pub selected: bool,
}
