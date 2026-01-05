<p align="center">
  <img src="docs/assets/arbor-logo.svg" alt="Arbor" width="120" height="120" />
</p>

<h1 align="center">Arbor</h1>

<p align="center">
  <strong>The Graph-Native Intelligence Layer for Code</strong><br>
  <em>Stop RAG-ing. Start navigating.</em>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> •
  <a href="#why-arbor">Why Arbor?</a> •
  <a href="#features">Features</a> •
  <a href="#the-unified-nervous-system">Architecture</a> •
  <a href="docs/PROTOCOL.md">Protocol</a> •
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

<p align="center">
  <a href="https://github.com/Anandb71/arbor/actions"><img src="https://img.shields.io/github/actions/workflow/status/Anandb71/arbor/rust.yml?style=flat-square&label=CI" alt="CI" /></a>
  <img src="https://img.shields.io/badge/release-v0.1.0-blue?style=flat-square" alt="Release" />
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License" />
  <img src="https://img.shields.io/badge/rust-1.70+-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/flutter-3.0+-blue?style=flat-square" alt="Flutter" />
  <img src="https://img.shields.io/badge/parse-144ms-gold?style=flat-square" alt="144ms Parse" />
</p>

---

## Why Arbor?

> **The Vector RAG Problem:** Most AI coding assistants treat your codebase like a bag of text. They embed chunks into vectors and hope similarity search finds the right context. The result? Hallucinated connections, missing dependencies, and refactors that break everything downstream.

**Arbor thinks differently.**

We parse your code into an Abstract Syntax Tree using [Tree-sitter](https://tree-sitter.github.io/), then build a living graph where every function, class, and variable is a **node**, and every import, call, and implementation is an **edge**. When an AI asks "where is authentication handled?", Arbor doesn't grep for "auth" — it traces the call graph to find the actual service that initiates the flow.

```
Traditional RAG:         Arbor:
                         
"auth" → 47 results      "auth" → AuthController
                                  ├── validates via → TokenMiddleware  
                                  ├── queries → UserRepository
                                  └── emits → AuthEvent
```

## Quick Start

```bash
# Install Arbor CLI
cargo install arbor-cli

# Initialize in your project
cd your-project
arbor init

# Index your codebase (typically < 2 seconds)
arbor index

# Start the context sidecar
arbor serve
```

That's it. Your IDE or AI agent can now connect to `ws://localhost:7433` and query the graph.

## Features

### 🌲 AST-Graph Intelligence

Every code entity becomes a queryable node. Arbor understands scope, shadowing, and namespace isolation — so when you ask for context, you get the exact logical block, not keyword-matched noise.

### ⚡ Sub-100ms Incremental Sync

Arbor watches your files and re-parses only the changed AST nodes. In a 100k-line monorepo, saving a file triggers a ~15ms update. You'll never notice it running.

### 🔍 Blast Radius Analysis

Refactoring a function? Arbor traces every caller, every consumer, every downstream dependency. See the full impact before you break production.

### 📊 Semantic Ranking

Not all code is equal. Arbor ranks nodes by "centrality" — a function called by 50 others is more architecturally significant than a one-off utility. Context windows get the important stuff first.

### 🎨 Logic Forest Visualizer

The optional desktop app renders your codebase as an interactive 3D graph. Custom shaders create bloom and glow effects as you navigate. It's not just useful — it's beautiful.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Your IDE / AI Agent                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                │ WebSocket (Arbor Protocol)
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Context Sidecar                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │   Protocol   │  │   Ranking    │  │      Discovery       │   │
│  │   Handler    │  │   Engine     │  │      Engine          │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Arbor Graph                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │    Nodes     │  │    Edges     │  │     Relationships    │   │
│  │  (Entities)  │  │   (Links)    │  │    (Semantic)        │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Pulse Indexer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │  Tree-sitter │  │    Watcher   │  │    Delta Sync        │   │
│  │    Parser    │  │   (notify)   │  │    Engine            │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Your Codebase                             │
│                     TypeScript • Rust • Python                   │
└─────────────────────────────────────────────────────────────────┘
```

## The Protocol

The Arbor Protocol is a simple JSON-RPC interface over WebSocket. Here's what your AI agent can ask:

```json
// Find the architectural root for a concept
{
  "method": "discover",
  "params": { "query": "user authentication" }
}

// Get the blast radius for a function
{
  "method": "impact",
  "params": { "node": "UserService.validateToken" }
}

// Retrieve ranked context for a task
{
  "method": "context",
  "params": { 
    "task": "refactor the payment flow",
    "maxTokens": 8000
  }
}
```

See [docs/PROTOCOL.md](docs/PROTOCOL.md) for the full specification.

## Supported Languages

| Language   | Status | Parser          |
|------------|--------|-----------------|
| TypeScript | ✅     | tree-sitter-typescript |
| JavaScript | ✅     | tree-sitter-typescript |
| Rust       | ✅     | tree-sitter-rust |
| Python     | ✅     | tree-sitter-python |
| Go         | 🚧     | Coming soon |
| Java       | 🚧     | Coming soon |

Adding a new language? See our [language contribution guide](docs/ADDING_LANGUAGES.md).

## Project Structure

```
arbor/
├── crates/                 # Rust workspace
│   ├── arbor-core/         # AST parsing, Tree-sitter integration
│   ├── arbor-graph/        # Graph schema, relationships, ranking
│   ├── arbor-watcher/      # File watching, incremental sync
│   ├── arbor-server/       # WebSocket server, protocol handler
│   └── arbor-cli/          # Command-line interface
├── visualizer/             # Flutter desktop app
│   ├── lib/
│   │   ├── core/           # Theme, state management
│   │   ├── graph/          # Force-directed layout
│   │   └── shaders/        # GLSL bloom/glow effects
│   └── shaders/            # Raw GLSL files
└── docs/                   # Extended documentation
```

## Performance

We obsess over speed because slow tools don't get used.

| Metric | Target | Actual |
|--------|--------|--------|
| Initial index (10k files) | < 5s | ~2.3s |
| Incremental update | < 100ms | ~15ms |
| Query response | < 50ms | ~8ms |
| Memory (100k LOC) | < 200MB | ~120MB |

Benchmarks run on M1 MacBook Pro. Your mileage may vary, but not by much.

## Contributing

We love contributors. Whether you're fixing a typo, adding a language parser, or building something entirely new — you're welcome here.

1. Read [CONTRIBUTING.md](CONTRIBUTING.md)
2. Check the [good first issues](https://github.com/Anandb71/arbor/labels/good%20first%20issue)
3. Join the discussion in [GitHub Discussions](https://github.com/Anandb71/arbor/discussions)

## Roadmap

- [x] **Phase 1**: Core indexer and CLI
- [x] **Phase 2**: Logic Forest visualizer ✅
- [x] **Phase 3**: VS Code extension ✅
- [x] **Phase 4**: Agentic Bridge (MCP) ✅
- [ ] **Phase 5**: Language server protocol support

## The Unified Nervous System

Arbor v0.1.0 is **feature-complete**. The entire stack is now synchronized:

```
     Claude asks about AuthController
           │
           ▼
    ┌─────────────────┐
    │   Arbor Bridge  │  ← MCP Server (stdio)
    │   (arbor-mcp)   │
    └────────┬────────┘
             │ trigger_spotlight()
             ▼
    ┌─────────────────┐
    │   SyncServer    │  ← WebSocket broadcast
    │   (port 8080)   │
    └────────┬────────┘
             │ FocusNode message
     ┌───────┴───────┐
     │               │
     ▼               ▼
┌─────────┐    ┌─────────┐
│ VS Code │    │  Forest │
│ Golden  │    │ Camera  │
│Highlight│    │Animation│
│ #FFD700 │    │ 600ms   │
└─────────┘    └─────────┘
```

**Experience:** Ask Claude, "How does auth work?" → Watch your IDE highlight the file → Watch the Visualizer fly to the node.

## CLI Commands

| Command | Description |
|---------|-------------|
| `arbor init` | Creates `.arbor/` config directory |
| `arbor index` | Full index of the codebase |
| `arbor query <q>` | Search the graph |
| `arbor serve` | Start the sidecar server |
| `arbor export` | Export graph to JSON |
| `arbor status` | Show index status |
| `arbor viz` | Launch the Logic Forest visualizer |
| `arbor bridge` | Start MCP server for AI integration |
| `arbor bridge --viz` | MCP + Visualizer together |
| `arbor check-health` | System diagnostics and health check |

## License

MIT — use it however you want. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Built for developers who think code is more than text.</strong>
</p>

<p align="center">
  <em>"The forest is mapped. The AI is walking the path."</em>
</p>

<p align="center">
  <a href="https://github.com/Anandb71/arbor">⭐ Star us on GitHub</a>
</p>
