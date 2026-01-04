//! Graph builder for constructing the code graph from parsed nodes.
//!
//! The builder takes CodeNodes and resolves their references into
//! actual graph edges.

use crate::edge::{Edge, EdgeKind};
use crate::graph::ArborGraph;
use arbor_core::CodeNode;
use std::collections::HashMap;

/// Builds an ArborGraph from parsed code nodes.
///
/// The builder handles the two-pass process:
/// 1. Add all nodes to the graph
/// 2. Resolve references into edges
pub struct GraphBuilder {
    graph: ArborGraph,
    /// Maps qualified names to node IDs for edge resolution.
    name_to_id: HashMap<String, String>,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self {
            graph: ArborGraph::new(),
            name_to_id: HashMap::new(),
        }
    }

    /// Adds nodes from a file to the graph.
    ///
    /// Call this for each parsed file, then call `resolve_edges`
    /// when all files are added.
    pub fn add_nodes(&mut self, nodes: Vec<CodeNode>) {
        for node in nodes {
            let id = node.id.clone();
            let name = node.name.clone();
            let qualified = node.qualified_name.clone();

            self.graph.add_node(node);

            // Track names for edge resolution
            self.name_to_id.insert(name.clone(), id.clone());
            self.name_to_id.insert(qualified, id);
        }
    }

    /// Resolves references into actual graph edges.
    ///
    /// This is the second pass after all nodes are added. It looks up
    /// reference names and creates edges where targets exist.
    pub fn resolve_edges(&mut self) {
        // Collect all the edge additions first to avoid borrow issues
        let mut edges_to_add = Vec::new();

        for node in self.graph.nodes() {
            let from_id = &node.id;

            for reference in &node.references {
                // Try to find the target node
                if let Some(to_id) = self.name_to_id.get(reference) {
                    if from_id != to_id {
                        edges_to_add.push((from_id.clone(), to_id.clone(), reference.clone()));
                    }
                }
            }
        }

        // Now add the edges
        for (from_id, to_id, _ref_name) in edges_to_add {
            if let (Some(from_idx), Some(to_idx)) =
                (self.graph.get_index(&from_id), self.graph.get_index(&to_id))
            {
                self.graph
                    .add_edge(from_idx, to_idx, Edge::new(EdgeKind::Calls));
            }
        }
    }

    /// Finishes building and returns the graph.
    pub fn build(mut self) -> ArborGraph {
        self.resolve_edges();
        self.graph
    }

    /// Builds without resolving edges (for incremental updates).
    pub fn build_without_resolve(self) -> ArborGraph {
        self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_core::NodeKind;

    #[test]
    fn test_builder_adds_nodes() {
        let mut builder = GraphBuilder::new();

        let node1 = CodeNode::new("foo", "foo", NodeKind::Function, "test.rs");
        let node2 = CodeNode::new("bar", "bar", NodeKind::Function, "test.rs");

        builder.add_nodes(vec![node1, node2]);
        let graph = builder.build();

        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_builder_resolves_edges() {
        let mut builder = GraphBuilder::new();

        let caller = CodeNode::new("caller", "caller", NodeKind::Function, "test.rs")
            .with_references(vec!["callee".to_string()]);
        let callee = CodeNode::new("callee", "callee", NodeKind::Function, "test.rs");

        builder.add_nodes(vec![caller, callee]);
        let graph = builder.build();

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }
}
