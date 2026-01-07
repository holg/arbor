<p align="center">
  <img src="https://raw.githubusercontent.com/Anandb71/arbor/main/docs/assets/arbor-logo.svg" alt="Arbor" width="60" height="60" />
</p>

<h1 align="center">arbor-server</h1>

<p align="center">
  <strong>WebSocket server for Arbor</strong><br>
  <em>Real-time graph queries for IDEs and visualizers</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/arbor-server"><img src="https://img.shields.io/crates/v/arbor-server?style=flat-square&color=blue" alt="Crates.io" /></a>
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License" />
</p>

---

## Overview

`arbor-server` exposes the [Arbor](https://github.com/Anandb71/arbor) graph over WebSocket, enabling:

- **VS Code Extension**: Live code highlighting
- **Logic Forest Visualizer**: Real-time graph rendering
- **Custom Clients**: Any tool that speaks JSON-RPC

## Protocol

**Default**: `ws://localhost:7432`

| Method | Description |
|--------|-------------|
| `discover` | Find architectural entry points |
| `impact` | Calculate blast radius of changes |
| `context` | Get ranked context for AI prompts |
| `graph.subscribe` | Stream live graph updates |
| `spotlight` | Highlight a node across clients |

## Message Format

```json
{
  "jsonrpc": "2.0",
  "method": "context",
  "params": { "node": "auth::validate", "depth": 2 },
  "id": 1
}
```

## Usage

```bash
cargo install arbor-graph-cli
arbor serve  # Starts on ws://localhost:7432
```

## Links

- **Main Repository**: [github.com/Anandb71/arbor](https://github.com/Anandb71/arbor)
- **Protocol Docs**: [docs/PROTOCOL.md](https://github.com/Anandb71/arbor/blob/main/docs/PROTOCOL.md)
