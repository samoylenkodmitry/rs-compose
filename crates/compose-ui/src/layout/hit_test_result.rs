//! Hit Test Result
//!
//! Collects modifier nodes during hit testing, following Jetpack Compose's HitTestResult.

use compose_core::NodeId;

/// Result of a hit test, containing the nodes that were hit
#[derive(Default, Clone)]
pub struct HitTestResult {
    /// List of hit nodes in order (outer to inner)
    pub nodes: Vec<HitTestEntry>,
}

/// A single entry in the hit test result
#[derive(Clone)]
pub struct HitTestEntry {
    pub node_id: NodeId,
    /// Whether this hit is in the actual bounds (vs extended touch target)
    pub in_layer: bool,
}

impl HitTestResult {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
    
    /// Add a node to the hit test result
    pub fn add(&mut self, node_id: NodeId, in_layer: bool) {
        self.nodes.push(HitTestEntry { node_id, in_layer });
    }
    
    /// Clear all results
    pub fn clear(&mut self) {
        self.nodes.clear();
    }
    
    /// Check if any nodes were hit
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    
    /// Returns iterator over hit node IDs
    pub fn iter(&self) -> impl Iterator<Item = &HitTestEntry> {
        self.nodes.iter()
    }
}
