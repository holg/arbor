//! Centrality ranking for code nodes.
//!
//! We use a simplified PageRank variant to score nodes by their
//! architectural significance. Nodes that are called by many
//! others rank higher.

use crate::graph::{ArborGraph, NodeId};
use std::collections::HashMap;

/// Stores centrality scores after computation.
#[derive(Debug, Default)]
pub struct CentralityScores {
    scores: HashMap<NodeId, f64>,
}

impl CentralityScores {
    /// Gets the score for a node.
    pub fn get(&self, id: NodeId) -> f64 {
        self.scores.get(&id).copied().unwrap_or(0.0)
    }

    /// Converts to a HashMap for storage in the graph.
    pub fn into_map(self) -> HashMap<NodeId, f64> {
        self.scores
    }
}

/// Computes centrality scores for all nodes in the graph.
///
/// This is a simplified PageRank that:
/// 1. Initializes all nodes with equal score
/// 2. Iteratively distributes scores along edges
/// 3. Applies damping to prevent score concentration
///
/// # Arguments
///
/// * `graph` - The graph to analyze
/// * `iterations` - Number of iterations (10-20 is usually enough)
/// * `damping` - Damping factor (0.85 is standard)
pub fn compute_centrality(graph: &ArborGraph, iterations: usize, damping: f64) -> CentralityScores {
    let node_count = graph.node_count();
    if node_count == 0 {
        return CentralityScores::default();
    }

    // Initialize scores
    let initial_score = 1.0 / node_count as f64;
    let mut scores: HashMap<NodeId, f64> = graph
        .node_indexes()
        .map(|idx| (idx, initial_score))
        .collect();

    // Count outgoing edges for each node
    let mut out_degree: HashMap<NodeId, usize> = HashMap::new();
    for idx in graph.node_indexes() {
        let callees = graph.get_callees(idx);
        out_degree.insert(idx, callees.len().max(1)); // Avoid division by zero
    }

    // Iterate
    for _ in 0..iterations {
        let mut new_scores: HashMap<NodeId, f64> = HashMap::new();

        for idx in graph.node_indexes() {
            // Base score (random jump)
            let base = (1.0 - damping) / node_count as f64;

            // Score from callers
            let callers = graph.get_callers(idx);
            let incoming: f64 = callers
                .iter()
                .filter_map(|caller| {
                    let caller_idx = graph.get_index(&caller.id)?;
                    let caller_score = scores.get(&caller_idx)?;
                    let caller_out = *out_degree.get(&caller_idx)? as f64;
                    Some(caller_score / caller_out)
                })
                .sum();

            new_scores.insert(idx, base + damping * incoming);
        }

        scores = new_scores;
    }

    // Normalize to [0, 1] range
    let max_score = scores.values().cloned().fold(0.0f64, f64::max);
    if max_score > 0.0 {
        for score in scores.values_mut() {
            *score /= max_score;
        }
    }

    CentralityScores { scores }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::{Edge, EdgeKind};
    use arbor_core::{CodeNode, NodeKind};

    #[test]
    fn test_centrality_empty_graph() {
        let graph = ArborGraph::new();
        let scores = compute_centrality(&graph, 10, 0.85);
        assert!(scores.scores.is_empty());
    }

    #[test]
    fn test_centrality_single_node() {
        let mut graph = ArborGraph::new();
        let node = CodeNode::new("foo", "foo", NodeKind::Function, "test.rs");
        graph.add_node(node);

        let scores = compute_centrality(&graph, 10, 0.85);
        assert_eq!(scores.scores.len(), 1);
    }

    #[test]
    fn test_centrality_popular_node_ranks_higher() {
        let mut graph = ArborGraph::new();

        // Create a "popular" function called by many others
        let popular = CodeNode::new("popular", "popular", NodeKind::Function, "test.rs");
        let popular_idx = graph.add_node(popular);

        // Create callers
        for i in 0..5 {
            let caller = CodeNode::new(
                format!("caller{}", i),
                format!("caller{}", i),
                NodeKind::Function,
                "test.rs",
            );
            let caller_idx = graph.add_node(caller);
            graph.add_edge(caller_idx, popular_idx, Edge::new(EdgeKind::Calls));
        }

        let scores = compute_centrality(&graph, 20, 0.85);

        // The popular node should have the highest score
        let popular_score = scores.get(popular_idx);
        assert!(popular_score > 0.5, "Popular node should rank high");
    }
}
