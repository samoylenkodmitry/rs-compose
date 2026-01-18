//! Hit path tracking for pointer input capture.
//!
//! This module implements a Jetpack Compose-style `HitPathTracker` that stores
//! stable `NodeId` references instead of caching `HitRegion` geometry.
//!
//! Key insight from JC: Cache **node identity**, not geometry. Fresh geometry
//! is resolved from the current scene on each dispatch, avoiding stale coordinates
//! during scroll/layout changes.

use cranpose_core::NodeId;
use std::collections::HashMap;

/// Pointer ID type for tracking multi-touch gestures.
/// Currently we only use a single primary pointer (id=0), but this design
/// supports future multi-touch expansion.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PointerId(pub u32);

impl PointerId {
    /// The primary pointer (mouse button 1, first touch)
    pub const PRIMARY: PointerId = PointerId(0);
}

/// Tracks which nodes were hit on PointerDown, keyed by pointer ID.
///
/// This mirrors Jetpack Compose's `HitPathTracker`:
/// - Stores stable `NodeId` references, NOT geometry
/// - Fresh geometry is resolved from the current scene on each dispatch
/// - Handler closures are preserved because they're `Rc` references
///
/// ## Design Rationale
///
/// The problem with caching `HitRegion` directly:
/// - `HitRegion` contains `rect` (geometry) from the frame when Down occurred
/// - When scroll moves content, layout re-runs, element positions change
/// - But cached `rect` still has old coordinates
/// - Local position computation uses stale geometry → wrong coordinates
///
/// The solution (matching JC):
/// - Cache only `NodeId` (stable identity) on Down
/// - On Move/Up, call `scene.find_target(node_id)` to get fresh `HitRegion`
/// - Fresh `HitRegion` has current geometry from this frame
/// - Handler closure is same `Rc`, so internal state (press_position) is preserved
pub struct HitPathTracker {
    /// Maps pointer IDs to the list of nodes hit on Down (ordered top-to-bottom by z-index)
    paths: HashMap<PointerId, Vec<NodeId>>,
}

impl HitPathTracker {
    /// Creates a new empty tracker.
    pub fn new() -> Self {
        Self {
            paths: HashMap::new(),
        }
    }

    /// Records which nodes were hit for a pointer.
    /// Called on PointerDown after hit-testing.
    ///
    /// The `node_ids` should be ordered by z-index (top-to-bottom) so that
    /// dispatch happens in the correct order for event consumption.
    pub fn add_hit_path(&mut self, pointer: PointerId, node_ids: Vec<NodeId>) {
        self.paths.insert(pointer, node_ids);
    }

    /// Gets the cached hit path for a pointer.
    /// Returns None if no path exists (no active gesture for this pointer).
    pub fn get_path(&self, pointer: PointerId) -> Option<&Vec<NodeId>> {
        self.paths.get(&pointer)
    }

    /// Removes and returns the hit path for a pointer.
    /// Called on PointerUp/Cancel to end the gesture.
    pub fn remove_path(&mut self, pointer: PointerId) -> Option<Vec<NodeId>> {
        self.paths.remove(&pointer)
    }

    /// Returns true if there's an active gesture for this pointer.
    pub fn has_path(&self, pointer: PointerId) -> bool {
        self.paths.contains_key(&pointer)
    }

    /// Clears all tracked paths. Called on gesture cancel.
    pub fn clear(&mut self) {
        self.paths.clear();
    }

    /// Returns true if there are any active gestures being tracked.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

impl Default for HitPathTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_path() {
        let mut tracker = HitPathTracker::new();
        let nodes: Vec<NodeId> = vec![1, 2, 3];

        tracker.add_hit_path(PointerId::PRIMARY, nodes.clone());

        assert!(tracker.has_path(PointerId::PRIMARY));
        assert_eq!(tracker.get_path(PointerId::PRIMARY), Some(&nodes));
    }

    #[test]
    fn test_remove_path() {
        let mut tracker = HitPathTracker::new();
        let nodes: Vec<NodeId> = vec![1];

        tracker.add_hit_path(PointerId::PRIMARY, nodes.clone());
        let removed = tracker.remove_path(PointerId::PRIMARY);

        assert_eq!(removed, Some(nodes));
        assert!(!tracker.has_path(PointerId::PRIMARY));
        assert!(tracker.is_empty());
    }

    #[test]
    fn test_clear() {
        let mut tracker = HitPathTracker::new();
        tracker.add_hit_path(PointerId(0), vec![1]);
        tracker.add_hit_path(PointerId(1), vec![2]);

        assert!(!tracker.is_empty());

        tracker.clear();

        assert!(tracker.is_empty());
        assert!(!tracker.has_path(PointerId(0)));
        assert!(!tracker.has_path(PointerId(1)));
    }

    #[test]
    fn test_multiple_pointers() {
        let mut tracker = HitPathTracker::new();
        let nodes1: Vec<NodeId> = vec![1];
        let nodes2: Vec<NodeId> = vec![2, 3];

        tracker.add_hit_path(PointerId(0), nodes1.clone());
        tracker.add_hit_path(PointerId(1), nodes2.clone());

        assert_eq!(tracker.get_path(PointerId(0)), Some(&nodes1));
        assert_eq!(tracker.get_path(PointerId(1)), Some(&nodes2));
    }
}
