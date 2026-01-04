//! Directory indexing.
//!
//! Walks directories to find and parse source files, building
//! the initial code graph.

use arbor_core::{parse_file, CodeNode};
use arbor_graph::{ArborGraph, GraphBuilder};
use ignore::WalkBuilder;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Result of indexing a directory.
pub struct IndexResult {
    /// The built graph.
    pub graph: ArborGraph,

    /// Number of files processed.
    pub files_indexed: usize,

    /// Number of nodes extracted.
    pub nodes_extracted: usize,

    /// Time taken in milliseconds.
    pub duration_ms: u64,

    /// Files that failed to parse.
    pub errors: Vec<(String, String)>,
}

/// Indexes a directory and returns the code graph.
///
/// This walks all source files, parses them, and builds the
/// relationship graph. It respects .gitignore patterns.
///
/// # Example
///
/// ```no_run
/// use arbor_watcher::index_directory;
/// use std::path::Path;
///
/// let result = index_directory(Path::new("./src")).unwrap();
/// println!("Indexed {} files, {} nodes", result.files_indexed, result.nodes_extracted);
/// ```
pub fn index_directory(root: &Path) -> Result<IndexResult, std::io::Error> {
    let start = Instant::now();
    let mut builder = GraphBuilder::new();
    let mut files_indexed = 0;
    let mut nodes_extracted = 0;
    let mut errors = Vec::new();

    info!("Starting index of {}", root.display());

    // Walk the directory, respecting .gitignore
    let walker = WalkBuilder::new(root)
        .hidden(true) // Skip hidden files
        .git_ignore(true) // Respect .gitignore
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker.filter_map(Result::ok) {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Check if it's a supported file type
        let extension = match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => ext,
            None => continue,
        };

        if !arbor_core::languages::is_supported(extension) {
            continue;
        }

        debug!("Parsing {}", path.display());

        match parse_file(path) {
            Ok(nodes) => {
                nodes_extracted += nodes.len();
                files_indexed += 1;
                builder.add_nodes(nodes);
            }
            Err(e) => {
                warn!("Failed to parse {}: {}", path.display(), e);
                errors.push((path.display().to_string(), e.to_string()));
            }
        }
    }

    let graph = builder.build();
    let duration = start.elapsed();

    info!(
        "Indexed {} files ({} nodes) in {:?}",
        files_indexed, nodes_extracted, duration
    );

    Ok(IndexResult {
        graph,
        files_indexed,
        nodes_extracted,
        duration_ms: duration.as_millis() as u64,
        errors,
    })
}

/// Parses a single file and returns its nodes.
pub fn parse_single_file(path: &Path) -> Result<Vec<CodeNode>, arbor_core::ParseError> {
    parse_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_index_empty_directory() {
        let dir = tempdir().unwrap();
        let result = index_directory(dir.path()).unwrap();
        assert_eq!(result.files_indexed, 0);
        assert_eq!(result.nodes_extracted, 0);
    }

    #[test]
    fn test_index_with_rust_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");

        fs::write(
            &file_path,
            r#"
            pub fn hello() {
                println!("Hello!");
            }
        "#,
        )
        .unwrap();

        let result = index_directory(dir.path()).unwrap();
        assert_eq!(result.files_indexed, 1);
        assert!(result.nodes_extracted > 0);
    }
}
