//! CLI command implementations.

use arbor_graph::compute_centrality;
use arbor_server::{ArborServer, ServerConfig};
use arbor_watcher::index_directory;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Initialize Arbor in a directory.
pub fn init(path: &Path) -> Result<()> {
    let arbor_dir = path.join(".arbor");

    if arbor_dir.exists() {
        println!("{} Already initialized", "✓".green());
        return Ok(());
    }

    fs::create_dir_all(&arbor_dir)?;

    // Create a default config file
    let config_path = arbor_dir.join("config.json");
    let default_config = serde_json::json!({
        "version": "1.0",
        "languages": ["typescript", "rust", "python"],
        "ignore": ["node_modules", "target", "dist", "__pycache__"]
    });

    fs::write(&config_path, serde_json::to_string_pretty(&default_config)?)?;

    println!("{} Initialized Arbor in {}", "✓".green(), path.display());
    println!("  Run {} to index your codebase", "arbor index".cyan());

    Ok(())
}

/// Index a directory and build the code graph.
pub fn index(path: &Path, output: Option<&Path>) -> Result<()> {
    println!("{}", "Indexing codebase...".cyan());

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}")?);
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner.set_message("Scanning files...");

    let result = index_directory(path)?;

    spinner.finish_and_clear();

    // Print results
    println!(
        "{} Indexed {} files ({} nodes) in {}ms",
        "✓".green(),
        result.files_indexed.to_string().cyan(),
        result.nodes_extracted.to_string().cyan(),
        result.duration_ms
    );

    // Show any errors
    if !result.errors.is_empty() {
        println!("\n{} files with parse errors:", "⚠".yellow());
        for (file, error) in result.errors.iter().take(5) {
            println!("  {} - {}", file.red(), error);
        }
        if result.errors.len() > 5 {
            println!("  ... and {} more", result.errors.len() - 5);
        }
    }

    // Export if requested
    if let Some(out_path) = output {
        export_graph(&result.graph, out_path)?;
    }

    Ok(())
}

fn export_graph(graph: &arbor_graph::ArborGraph, path: &Path) -> Result<()> {
    let nodes: Vec<_> = graph.nodes().collect();

    let export = serde_json::json!({
        "version": "1.0",
        "stats": {
            "nodeCount": graph.node_count(),
            "edgeCount": graph.edge_count()
        },
        "nodes": nodes
    });

    fs::write(path, serde_json::to_string_pretty(&export)?)?;
    println!("{} Exported to {}", "✓".green(), path.display());

    Ok(())
}

/// Query the code graph.
pub fn query(query: &str, limit: usize) -> Result<()> {
    // For now, we need to re-index. In a real implementation,
    // we'd load from a persisted graph or connect to the server.
    let path = std::env::current_dir()?;
    let result = index_directory(&path)?;

    let matches: Vec<_> = result.graph.search(query).into_iter().take(limit).collect();

    if matches.is_empty() {
        println!("No matches found for \"{}\"", query);
        return Ok(());
    }

    println!("Found {} matches:\n", matches.len());

    for node in matches {
        println!(
            "  {} {} {}",
            node.kind.to_string().yellow(),
            node.qualified_name.cyan(),
            format!("({}:{})", node.file, node.line_start).dimmed()
        );
        if let Some(ref sig) = node.signature {
            println!("    {}", sig.dimmed());
        }
    }

    Ok(())
}

/// Start the Arbor server.
pub async fn serve(port: u16, path: &Path) -> Result<()> {
    println!("{}", "Starting Arbor server...".cyan());

    // Index the codebase first
    let result = index_directory(path)?;
    let mut graph = result.graph;

    // Compute centrality
    let scores = compute_centrality(&graph, 20, 0.85);
    graph.set_centrality(scores.into_map());

    println!(
        "{} Indexed {} files ({} nodes)",
        "✓".green(),
        result.files_indexed,
        result.nodes_extracted
    );

    let addr = format!("127.0.0.1:{}", port).parse()?;
    let config = ServerConfig { addr };
    let server = ArborServer::new(graph, config);

    println!("{} Listening on ws://127.0.0.1:{}", "✓".green(), port);
    println!("  Press {} to stop", "Ctrl+C".cyan());

    server.run().await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Export the graph to JSON.
pub fn export(path: &Path, output: &Path) -> Result<()> {
    let result = index_directory(path)?;
    export_graph(&result.graph, output)?;
    Ok(())
}

/// Show index status.
pub fn status(path: &Path) -> Result<()> {
    let arbor_dir = path.join(".arbor");

    if !arbor_dir.exists() {
        println!("{} Arbor not initialized in this directory", "✗".red());
        println!("  Run {} to initialize", "arbor init".cyan());
        return Ok(());
    }

    // Quick index to get stats
    let result = index_directory(path)?;

    println!("{}", "Arbor Status".cyan().bold());
    println!();
    println!("  {} {}", "Files:".dimmed(), result.files_indexed);
    println!("  {} {}", "Nodes:".dimmed(), result.nodes_extracted);
    println!("  {} {}", "Edges:".dimmed(), result.graph.edge_count());
    println!("  {} TypeScript, Rust, Python", "Languages:".dimmed());

    Ok(())
}
