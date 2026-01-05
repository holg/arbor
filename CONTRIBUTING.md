# Contributing to Arbor

First off, thanks for considering contributing to Arbor. It's people like you that make this project possible.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Pull Request Process](#pull-request-process)
- [Adding Language Support](#adding-language-support)
- [Style Guide](#style-guide)

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you're expected to uphold this standard. Report unacceptable behavior to the maintainers.

## Getting Started

### What We're Looking For

- **Bug fixes** — something isn't working as expected
- **New language parsers** — help us support more ecosystems
- **Performance improvements** — make the indexer even faster
- **Documentation** — clarify confusing sections, add examples
- **Visualizer enhancements** — new shaders, interactions, layouts

### What to Avoid

- Large architectural changes without discussion first
- Vendoring dependencies unnecessarily
- Breaking changes to the protocol without a migration path

## Development Setup

### Prerequisites

You'll need these installed:

- **Rust** (1.70 or later) — [rustup.rs](https://rustup.rs)
- **Flutter** (3.0 or later) — [flutter.dev](https://flutter.dev/docs/get-started/install)
- **Node.js** (for testing TypeScript parsing) — [nodejs.org](https://nodejs.org)

### Clone and Build

```bash
# Clone the repo
git clone https://github.com/Anandb71/arbor.git
cd arbor

# Build the Rust crates
cd crates
cargo build

# Run tests to make sure everything works
cargo test --all

# Build the visualizer (optional)
cd ../visualizer
flutter pub get
flutter build windows  # or macos/linux

# Verify your environment
cd ../crates
cargo run -- check-health
```

### Running Locally

```bash
# Start the CLI in development mode
cd crates
cargo run --bin arbor-cli -- serve

# In another terminal, run the visualizer
cd visualizer
flutter run -d windows

# Or start everything together (MCP + Visualizer)
cargo run --bin arbor-cli -- bridge --viz
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feat/python-parser` — new feature
- `fix/watcher-memory-leak` — bug fix
- `docs/protocol-examples` — documentation
- `refactor/graph-query` — code cleanup

### Commit Messages

We follow conventional commits. Keep them short but descriptive:

```
feat(core): add Python class inheritance tracking

fix(watcher): handle symlink loops gracefully

docs: clarify WebSocket connection params
```

### Testing Your Changes

Always run the test suite before submitting:

```bash
cd crates
cargo test --all
cargo clippy --all -- -D warnings

cd ../visualizer
flutter test
flutter analyze
```

## Pull Request Process

1. **Fork the repo** and create your branch from `main`
2. **Make your changes** with appropriate tests
3. **Update documentation** if you're changing behavior
4. **Run the full test suite** and ensure it passes
5. **Submit your PR** with a clear description

### PR Template

When you open a PR, you'll see a template. Fill it out completely — it helps us review faster.

### Review Timeline

We try to review PRs within a few days. Complex changes might take longer. If you haven't heard anything in a week, feel free to ping us.

## Adding Language Support

Want to add support for a new language? Here's the process:

### 1. Add the Tree-sitter Grammar

In `crates/arbor-core/Cargo.toml`:

```toml
[dependencies]
tree-sitter-your-language = "0.20"
```

### 2. Create the Language Module

Create `crates/arbor-core/src/languages/your_language.rs`:

```rust
//! Parser implementation for YourLanguage.
//! 
//! Handles extraction of functions, classes, and imports from 
//! YourLanguage source files.

use crate::node::{CodeNode, NodeKind};
use crate::parser::LanguageParser;
use tree_sitter::Language;

pub struct YourLanguageParser;

impl LanguageParser for YourLanguageParser {
    fn language(&self) -> Language {
        tree_sitter_your_language::language()
    }

    fn extensions(&self) -> &[&str] {
        &["ext1", "ext2"]
    }

    fn extract_nodes(&self, tree: &tree_sitter::Tree, source: &str) -> Vec<CodeNode> {
        // Your extraction logic here
        vec![]
    }
}
```

### 3. Register the Parser

In `crates/arbor-core/src/languages/mod.rs`, add your language to the registry.

### 4. Add Tests

Create `crates/arbor-core/tests/your_language_test.rs` with representative test cases.

### 5. Update Documentation

Add your language to the table in README.md and any relevant docs.

## Style Guide

### Rust

We follow standard Rust conventions with a few preferences:

- Use `rustfmt` for formatting (run `cargo fmt`)
- Use `clippy` for linting (run `cargo clippy`)
- Prefer explicit error handling over `.unwrap()`
- Write doc comments for public APIs
- Keep functions focused and reasonably sized

```rust
// Good: clear, documented, handles errors
/// Parses a source file and extracts all code nodes.
/// 
/// Returns an empty vector if the file cannot be parsed.
pub fn parse_file(path: &Path) -> Result<Vec<CodeNode>, ParseError> {
    let source = fs::read_to_string(path)?;
    let parser = detect_language(path)?;
    Ok(parser.extract_nodes(&source))
}

// Avoid: cryptic, no docs, panics on error
pub fn parse(p: &Path) -> Vec<CodeNode> {
    let s = fs::read_to_string(p).unwrap();
    detect_language(p).unwrap().extract_nodes(&s)
}
```

### Flutter/Dart

- Use `dart format` for formatting
- Use `flutter analyze` for linting
- Follow the [Effective Dart](https://dart.dev/guides/language/effective-dart) guide
- Keep widgets small and composable
- Use providers for state management (Riverpod preferred)

```dart
// Good: clear naming, single responsibility
class GraphNode extends StatelessWidget {
  final NodeData data;
  final VoidCallback onTap;

  const GraphNode({
    required this.data,
    required this.onTap,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: CustomPaint(
        painter: NodePainter(data),
      ),
    );
  }
}
```

### Comments

Write comments that explain *why*, not *what*. The code shows what's happening — comments should provide context.

```rust
// Meh: obvious from the code
// Increment the counter by one
counter += 1;

// Better: explains the why
// Tree-sitter uses 0-based byte offsets, but editors expect 1-based lines
let line = byte_offset_to_line(offset) + 1;
```

## Questions?

- Open a [GitHub Discussion](https://github.com/Anandb71/arbor/discussions) for general questions
- File an issue for bugs or feature requests
- Tag maintainers in your PR if you need guidance

Thanks for contributing. Let's build something great together.
