//! Query result types.
//!
//! These structs represent the results of various graph queries.
//! They're designed to be easily serializable for the protocol.

use arbor_core::CodeNode;
use serde::{Deserialize, Serialize};

/// Result of an impact analysis query.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImpactResult {
    /// The target node being analyzed.
    pub target: NodeInfo,

    /// Nodes that depend on the target.
    pub dependents: Vec<DependentInfo>,

    /// Total count of affected nodes.
    pub total_affected: usize,

    /// Time taken to compute (milliseconds).
    pub query_time_ms: u64,
}

/// Result of a context query.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    /// Matched nodes, ranked by relevance.
    pub nodes: Vec<NodeInfo>,

    /// Total token count of included source.
    pub total_tokens: usize,

    /// Time taken (milliseconds).
    pub query_time_ms: u64,
}

/// Basic information about a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub file: String,
    pub line_start: u32,
    pub line_end: u32,
    pub signature: Option<String>,
    pub centrality: f64,
}

impl From<&CodeNode> for NodeInfo {
    fn from(node: &CodeNode) -> Self {
        Self {
            id: node.id.clone(),
            name: node.name.clone(),
            qualified_name: node.qualified_name.clone(),
            kind: node.kind.to_string(),
            file: node.file.clone(),
            line_start: node.line_start,
            line_end: node.line_end,
            signature: node.signature.clone(),
            centrality: 0.0, // Will be filled in by the graph
        }
    }
}

/// Information about a dependent node.
#[derive(Debug, Serialize, Deserialize)]
pub struct DependentInfo {
    pub node: NodeInfo,
    pub relationship: String,
    pub depth: usize,
}
