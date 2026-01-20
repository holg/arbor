//! Confidence scoring for impact analysis
//!
//! Provides explainable risk levels (Low/Medium/High) based on graph structure.

use crate::ImpactAnalysis;

/// Confidence level for an analysis result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    /// High confidence - well-connected node with clear edges
    High,
    /// Medium confidence - some uncertainty exists
    Medium,
    /// Low confidence - significant unknowns
    Low,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceLevel::High => write!(f, "High"),
            ConfidenceLevel::Medium => write!(f, "Medium"),
            ConfidenceLevel::Low => write!(f, "Low"),
        }
    }
}

/// Reasons explaining the confidence level
#[derive(Debug, Clone)]
pub struct ConfidenceExplanation {
    pub level: ConfidenceLevel,
    pub reasons: Vec<String>,
    pub suggestions: Vec<String>,
}

impl ConfidenceExplanation {
    /// Compute confidence from an impact analysis
    pub fn from_analysis(analysis: &ImpactAnalysis) -> Self {
        let mut reasons = Vec::new();
        let mut suggestions = Vec::new();

        let upstream_count = analysis.upstream.len();
        let downstream_count = analysis.downstream.len();
        let total = analysis.total_affected;

        // Determine base confidence from connectivity
        let level = if upstream_count == 0 && downstream_count == 0 {
            // Isolated node
            reasons.push("Node appears isolated (no detected connections)".to_string());
            suggestions.push("Verify if this is called dynamically or from external code".to_string());
            ConfidenceLevel::Low
        } else if upstream_count == 0 {
            // Entry point
            reasons.push("Node is an entry point (no internal callers)".to_string());
            reasons.push(format!("Has {} downstream dependencies", downstream_count));
            if downstream_count > 5 {
                suggestions.push("Consider impact on downstream dependencies".to_string());
                ConfidenceLevel::Medium
            } else {
                ConfidenceLevel::High
            }
        } else if downstream_count == 0 {
            // Leaf/utility node
            reasons.push("Node is a utility (no outgoing dependencies)".to_string());
            reasons.push(format!("Called by {} upstream nodes", upstream_count));
            ConfidenceLevel::High
        } else {
            // Connected node
            reasons.push(format!("{} callers, {} dependencies", upstream_count, downstream_count));

            if total > 20 {
                reasons.push("Large blast radius detected".to_string());
                suggestions.push("Consider breaking this change into smaller refactors".to_string());
                ConfidenceLevel::Medium
            } else if total > 50 {
                reasons.push("Very large blast radius".to_string());
                suggestions.push("This change affects a significant portion of the codebase".to_string());
                ConfidenceLevel::Low
            } else {
                reasons.push("Well-connected with manageable impact".to_string());
                ConfidenceLevel::High
            }
        };

        // Add structural insights
        if total > 0 {
            let direct_count = analysis.upstream.iter()
                .filter(|n| n.hop_distance == 1)
                .count();
            if direct_count > 0 {
                reasons.push(format!("{} nodes will break immediately", direct_count));
            }
        }

        // Standard disclaimer
        suggestions.push("Tests still recommended for behavioral verification".to_string());

        Self {
            level,
            reasons,
            suggestions,
        }
    }
}

/// Node role classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Entry point - receives control from outside
    EntryPoint,
    /// Utility - helper function called by others
    Utility,
    /// Core logic - central to the domain
    CoreLogic,
    /// Isolated - no detected connections
    Isolated,
    /// Adapter - boundary between layers
    Adapter,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::EntryPoint => write!(f, "Entry Point"),
            NodeRole::Utility => write!(f, "Utility"),
            NodeRole::CoreLogic => write!(f, "Core Logic"),
            NodeRole::Isolated => write!(f, "Isolated"),
            NodeRole::Adapter => write!(f, "Adapter"),
        }
    }
}

impl NodeRole {
    /// Determine role from impact analysis
    pub fn from_analysis(analysis: &ImpactAnalysis) -> Self {
        let has_upstream = !analysis.upstream.is_empty();
        let has_downstream = !analysis.downstream.is_empty();

        match (has_upstream, has_downstream) {
            (false, false) => NodeRole::Isolated,
            (false, true) => NodeRole::EntryPoint,
            (true, false) => NodeRole::Utility,
            (true, true) => {
                // Distinguish between adapter and core logic
                let upstream_count = analysis.upstream.len();
                let downstream_count = analysis.downstream.len();

                // Adapters typically have few callers but many dependencies (or vice versa)
                if upstream_count <= 2 && downstream_count > 5 {
                    NodeRole::Adapter
                } else if downstream_count <= 2 && upstream_count > 5 {
                    NodeRole::Adapter
                } else {
                    NodeRole::CoreLogic
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_level_display() {
        assert_eq!(ConfidenceLevel::High.to_string(), "High");
        assert_eq!(ConfidenceLevel::Medium.to_string(), "Medium");
        assert_eq!(ConfidenceLevel::Low.to_string(), "Low");
    }

    #[test]
    fn test_node_role_display() {
        assert_eq!(NodeRole::EntryPoint.to_string(), "Entry Point");
        assert_eq!(NodeRole::Utility.to_string(), "Utility");
        assert_eq!(NodeRole::CoreLogic.to_string(), "Core Logic");
        assert_eq!(NodeRole::Isolated.to_string(), "Isolated");
        assert_eq!(NodeRole::Adapter.to_string(), "Adapter");
    }
}
