<p align="center">
  <img src="https://raw.githubusercontent.com/Anandb71/arbor/main/docs/assets/arbor-logo.svg" alt="Arbor" width="60" height="60" />
</p>

<h1 align="center">arbor-graph</h1>

<p align="center">
  <strong>Graph engine for Arbor</strong><br>
  <em>The Code Property Graph that LLMs can navigate</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/arbor-graph"><img src="https://img.shields.io/crates/v/arbor-graph?style=flat-square&color=blue" alt="Crates.io" /></a>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License" />
</p>

---

## Overview

`arbor-graph` is the heart of [Arbor](https://github.com/Anandb71/arbor). It manages:

- **Graph Schema**: Nodes (code entities) + Edges (relationships)
- **Symbol Table**: Cross-file FQN resolution
- **Persistence**: Sled-backed incremental storage
- **Queries**: Path finding, impact analysis, context retrieval

## Features

| Feature | Description |
|---------|-------------|
| `petgraph` core | Stable, fast in-memory graph |
| Global Symbol Table | Resolve imports across files |
| Sled Store | ACID-compliant persistence |
| `find_path` | A* shortest path between nodes |
| Serialization | `bincode` for compact storage |

## Architecture

```
arbor-core (parse) → arbor-graph (store) → arbor-server (expose)
                          ↓
                    ArborGraph
                    ├── nodes: HashMap<NodeId, CodeEntity>
                    ├── edges: Vec<(NodeId, NodeId, EdgeKind)>
                    └── symbol_table: SymbolTable
```

## Usage

This crate is used internally. For most use cases:

```bash
cargo install arbor-graph-cli
```

## Links

- **Main Repository**: [github.com/Anandb71/arbor](https://github.com/Anandb71/arbor)
