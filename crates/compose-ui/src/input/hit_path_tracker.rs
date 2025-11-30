//! HitPathTracker - Kotlin-aligned multi-pass dispatch
//!
//! Organizes pointers and PointerInputNodes into a hierarchy for multi-pass dispatch.
//! Following Jetpack Compose's HitPathTracker pattern exactly.

use compose_core::NodeId;
use compose_foundation::nodes::input::types::{PointerEvent, PointerEventPass, InternalPointerEvent};
use std::collections::HashMap;
use std::rc::Rc;

/// A node in the hit path tree representing a PointerInputNode
struct Node {
    /// The NodeId of the PointerInputNode
    node_id: NodeId,
    
    /// Pointer IDs that hit this node  
    pointer_ids: Vec<compose_foundation::nodes::input::types::PointerId>,
    
    /// Children of this node (inner nodes in the modifier chain)
    children: Vec<Node>,
    
    /// Cached pointer event for this node (built once, used in all passes)
   cached_event: Option<PointerEvent>,
}

impl Node {
    fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            pointer_ids: Vec::new(),
            children: Vec::new(),
            cached_event: None,
        }
    }
    
    /// Build cache for this node and children
    /// Returns true if there are relevant changes
    fn build_cache(&mut self, internal_event: &InternalPointerEvent) -> bool {
        // Build cache for children first
        let mut child_changed = false;
        for child in &mut self.children {
            child_changed |= child.build_cache(internal_event);
        }
        
        // Build event for this node from relevant changes
        let mut relevant_changes = Vec::new();
        for pointer_id in &self.pointer_ids {
            if let Some(change) = internal_event.changes.get(pointer_id) {
                relevant_changes.push(change.clone());
            }
        }
        
        if relevant_changes.is_empty() {
            self.pointer_ids.clear();
            self.children.clear();
            return child_changed;
        }
        
        // Clean up pointer IDs that weren't in the event
        self.pointer_ids.retain(|id| {
            internal_event.changes.contains_key(id)
        });
        
        // Create cached event
        self.cached_event = Some(PointerEvent::new(
            relevant_changes,
            Some(Rc::new(internal_event.clone())),
        ));
        
        true
    }
    
    /// Dispatch main event pass: Initial (tunneling) → children → Main (bubbling)
    fn dispatch_main_event_pass<F>(&mut self, dispatch_fn: &mut F) -> bool
    where
        F: FnMut(NodeId, &PointerEvent, PointerEventPass),
    {
        if self.cached_event.is_none() || self.pointer_ids.is_empty() {
            return false;
        }
        
        let event = self.cached_event.as_ref().unwrap();
        
        // 1. Dispatch Initial pass (tunneling)
        dispatch_fn(self.node_id, event, PointerEventPass::Initial);
        
        // 2. Dispatch to children
        for child in &mut self.children {
            child.dispatch_main_event_pass(dispatch_fn);
        }
        
        // 3. Dispatch Main pass (bubbling)
        dispatch_fn(self.node_id, event, PointerEventPass::Main);
        
        true
    }
    
    /// Dispatch final event pass: Final (tunneling) → children
    fn dispatch_final_event_pass<F>(&mut self, dispatch_fn: &mut F) -> bool
    where
        F: FnMut(NodeId, &PointerEvent, PointerEventPass),
    {
        if self.cached_event.is_none() || self.pointer_ids.is_empty() {
            return false;
        }
        
        let event = self.cached_event.as_ref().unwrap();
        
        // 1. Dispatch Final pass (tunneling)
        dispatch_fn(self.node_id, event, PointerEventPass::Final);
        
        // 2. Dispatch to children
        for child in &mut self.children {
            child.dispatch_final_event_pass(dispatch_fn);
        }
        
        // Clear cache after final pass
        self.cached_event = None;
        
        true
    }
    
    /// Clean up hits after dispatch
    fn cleanup_hits(&mut self, internal_event: &InternalPointerEvent) {
        // Remove pointer IDs for released pointers
        if let Some(event) = &self.cached_event {
 for change in &event.changes {
                if !change.pressed {
                    // Pointer released - remove it
                    self.pointer_ids.retain(|id| *id != change.id);
                }
            }
        }
        
        // Clean up children
        self.children.retain(|child| !child.pointer_ids.is_empty());
        
        for child in &mut self.children {
            child.cleanup_hits(internal_event);
        }
    }
}

/// Parent node containing child nodes
struct NodeParent {
    children: Vec<Node>,
}

impl NodeParent {
    fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }
    
    fn build_cache(&mut self, internal_event: &InternalPointerEvent) -> bool {
        let mut changed = false;
        for child in &mut self.children {
            changed |= child.build_cache(internal_event);
        }
        changed
    }
    
    fn dispatch_main_event_pass<F>(&mut self, dispatch_fn: &mut F) ->bool
    where
        F: FnMut(NodeId, &PointerEvent, PointerEventPass),
    {
        let mut dispatched = false;
        for child in &mut self.children {
            dispatched |= child.dispatch_main_event_pass(dispatch_fn);
        }
        dispatched
    }
    
    fn dispatch_final_event_pass<F>(&mut self, dispatch_fn: &mut F) -> bool
    where
        F: FnMut(NodeId, &PointerEvent, PointerEventPass),
    {
        let mut dispatched = false;
        for child in &mut self.children {
            dispatched |= child.dispatch_final_event_pass(dispatch_fn);
        }
        dispatched
    }
    
    fn cleanup_hits(&mut self, internal_event: &InternalPointerEvent) {
        // Remove children with no pointer IDs
        self.children.retain(|child| !child.pointer_ids.is_empty());
        
        for child in &mut self.children {
            child.cleanup_hits(internal_event);
        }
    }
}

/// Tracks hit paths and dispatches events through the hierarchy
pub struct HitPathTracker {
    /// Root parent node
    root: NodeParent,
    
    /// Map of pointer ID to nodes that were hit
    hit_pointer_ids_and_nodes: HashMap<compose_foundation::nodes::input::types::PointerId, Vec<NodeId>>,
    
    /// Whether we're currently dispatching
    dispatching_event: bool,
}

impl HitPathTracker {
    pub fn new() -> Self {
        Self {
            root: NodeParent::new(),
            hit_pointer_ids_and_nodes: HashMap::new(),
            dispatching_event: false,
        }
    }
    
    /// Add a hit path for a pointer ID
    ///
    /// This builds the Node tree from the hit test result.
    /// Nodes are ordered from ancestor to descendant.
    /// 
    /// Following Kotlin's addHitPath logic:
    /// - Merge with existing tree where possible (reuse nodes)
    /// - Create new Nodes for new paths
    /// - Track pointer IDs in nodes
    pub fn add_hit_path(
        &mut self,
        pointer_id: compose_foundation::nodes::input::types::PointerId,
        nodes: Vec<NodeId>,
    ) {
        if nodes.is_empty() {
            return;
        }
        
        // Clear tracking for this pointer (we'll rebuild it)
        self.hit_pointer_ids_and_nodes.remove(&pointer_id);
        
        // Build path using indices to avoid borrow checker issues
        // We'll navigate by index, creating nodes as needed
        let mut path_indices: Vec<usize> = Vec::with_capacity(nodes.len());
        
        // Phase 1: Find or create nodes, tracking indices
        for node_id in &nodes {
            // Navigate to the current level using path_indices
            let current_children = self.get_children_at_path(&path_indices);
            
            // Find or create node at this level
            let node_index = current_children
                .iter()
                .position(|n| n.node_id == *node_id)
                .unwrap_or_else(|| {
                    // Node doesn't exist - create it
                    let new_node = Node::new(*node_id);
                    let children = self.get_children_at_path_mut(&path_indices);
                    children.push(new_node);
                    children.len() - 1
                });
            
            path_indices.push(node_index);
        }
        
        // Phase 2: Add pointer ID to all nodes in the path
        for (depth, node_id) in nodes.iter().enumerate() {
            let path_to_node = &path_indices[..=depth];
            let node = self.get_node_at_path_mut(path_to_node);
            
            if !node.pointer_ids.contains(&pointer_id) {
                node.pointer_ids.push(pointer_id);
            }
            
            // Track this node for this pointer
            self.hit_pointer_ids_and_nodes
                .entry(pointer_id)
                .or_insert_with(Vec::new)
                .push(*node_id);
        }
    }
    
    /// Helper: Get immutable reference to children at a specific path (by indices)
    fn get_children_at_path(&self, path: &[usize]) -> &Vec<Node> {
        let mut current = &self.root.children;
        for &index in path {
            current = &current[index].children;
        }
        current
    }
    
    /// Helper: Get mutable reference to children at a specific path (by indices)
    fn get_children_at_path_mut(&mut self, path: &[usize]) -> &mut Vec<Node> {
        let mut current = &mut self.root.children;
        for &index in path {
            current = &mut current[index].children;
        }
        current
    }
    
    /// Helper: Get mutable reference to a node at a specific path (by indices)
    fn get_node_at_path_mut(&mut self, path: &[usize]) -> &mut Node {
        assert!(!path.is_empty(), "Path must not be empty");
        
        let mut current = &mut self.root.children;
        for &index in &path[..path.len() - 1] {
            current = &mut current[index].children;
        }
        &mut current[path[path.len() - 1]]
    }
    
    /// Dispatch changes through all hit paths
    /// 
    /// The caller must provide a dispatch function that handles the actual dispatch to layout nodes.
    /// This allows the HitPathTracker to remain agnostic of how nodes are accessed (no unsafe code).
    /// 
    /// Following Kotlin's pattern:
    /// 1. Build cache (filter relevant changes, create events)
    /// 2. Dispatch main pass (Initial → children → Main)
    /// 3. Dispatch final pass (Final → children)
    /// 4. Cleanup hits
    pub fn dispatch_changes<F>(
        &mut self,
        internal_event: &InternalPointerEvent,
        mut dispatch_fn: F,
    ) -> bool
    where
        F: FnMut(NodeId, &PointerEvent, PointerEventPass),
    {
        if self.dispatching_event {
            return false;
        }
        
        // 1. Build cache
        let changed = self.root.build_cache(internal_event);
        if !changed {
            return false;
        }
        
        self.dispatching_event = true;
        
        // 2. Dispatch main event pass
        let dispatched = self.root.dispatch_main_event_pass(&mut dispatch_fn);
        
        // 3. Dispatch final event pass
        let _final_dispatched = self.root.dispatch_final_event_pass(&mut dispatch_fn);
        
        // 4. Cleanup hits
        self.root.cleanup_hits(internal_event);
        
        self.dispatching_event = false;
        dispatched
    }
    
    /// Clear all tracked paths
    pub fn clear(&mut self) {
        self.root.children.clear();
        self.hit_pointer_ids_and_nodes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compose_core::NodeId;
    use compose_foundation::nodes::input::types::{InternalPointerEvent, PointerEvent, PointerEventPass, PointerInputChange, PointerId, PointerInputEvent};
    use compose_ui_graphics::Point;
    use std::cell::Cell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn test_add_hit_path_creates_tree() {
        let mut tracker = HitPathTracker::new();
        
        let node1: NodeId = 1;
        let node2: NodeId = 2;
        let node3: NodeId = 3;
        let pointer_id: PointerId = 0;
        
        tracker.add_hit_path(pointer_id, vec![node1, node2, node3]);
        
        // Root should have 1 child (node1)
        assert_eq!(tracker.root.children.len(), 1);
        assert_eq!(tracker.root.children[0].node_id, node1);
        assert!(tracker.root.children[0].pointer_ids.contains(&pointer_id));
    }
    
    #[test]
    fn test_add_hit_path_merges_shared_nodes() {
        let mut tracker = HitPathTracker::new();
        
        let node1: NodeId = 1;
        let node2: NodeId = 2;
        let pointer1: PointerId = 1;
        let pointer2: PointerId = 2;
        
        tracker.add_hit_path(pointer1, vec![node1, node2]);
        tracker.add_hit_path(pointer2, vec![node1, node2]);
        
        // Should have 1 child (shared node1)
        assert_eq!(tracker.root.children.len(), 1);
        // Both pointers in node1
        assert_eq!(tracker.root.children[0].pointer_ids.len(), 2);
    }
    
    #[test]
    fn test_dispatch_order_follows_kotlin_pattern() {
        use std::sync::{Arc, Mutex};
        
        let mut tracker = HitPathTracker::new();
        let node1: NodeId = 1;
        let node2: NodeId = 2;
        let pointer_id: PointerId = 1;
        
        tracker.add_hit_path(pointer_id, vec![node1, node2]);
        
        let log: Arc<Mutex<Vec<(NodeId, PointerEventPass)>>> = Arc::new(Mutex::new(Vec::new()));
        let log_clone = log.clone();
        
        let change = Rc::new(PointerInputChange {
            id: pointer_id,
            uptime: 0,
            position: Point::new(0.0, 0.0),
            pressed: true,
            pressure: 1.0,
            previous_uptime: 0,
            previous_position: Point::new(0.0, 0.0),
            previous_pressed: false,
            is_consumed: Cell::new(false),
            type_: compose_foundation::nodes::input::types::PointerType::Mouse,
            historical: Vec::new(),
            scroll_delta: Point::ZERO,
            original_event_position: Point::ZERO,
        });
        
        let mut changes = HashMap::new();
        changes.insert(pointer_id, change);
        
        let internal_event = InternalPointerEvent {
            changes,
            pointer_input_event: PointerInputEvent::new(0, Vec::new()),
            suppress_movement_consumption: false,
        };
        
        tracker.dispatch_changes(&internal_event, |node_id, _event, pass| {
            log_clone.lock().unwrap().push((node_id, pass));
        });
        
        let logged = log.lock().unwrap();
        
        // Verify correct order: Initial(1,2) -> Main(2,1) -> Final(1,2)
        assert_eq!(logged.len(), 6);
        assert_eq!(logged[0], (node1, PointerEventPass::Initial));
        assert_eq!(logged[1], (node2, PointerEventPass::Initial));
        assert_eq!(logged[2], (node2, PointerEventPass::Main));
        assert_eq!(logged[3], (node1, PointerEventPass::Main));
        assert_eq!(logged[4], (node1, PointerEventPass::Final));
        assert_eq!(logged[5], (node2, PointerEventPass::Final));
    }
}
