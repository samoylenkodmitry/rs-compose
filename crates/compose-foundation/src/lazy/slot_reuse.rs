//! Slot tracking for lazy layouts.
//!
//! Tracks composed item slots for statistics and lifecycle management.
//!
//! **Note**: Currently tracks metadata only. Actual slot reuse (recycling
//! composed nodes via SubcomposeLayout) is not yet implemented. In Rust,
//! item cleanup is handled by ownership when nodes go out of scope,
//! unlike JC which needs explicit GC-aware recycling.

use std::collections::HashMap;

/// Default number of slots to keep for reuse.
/// Matches JC/RecyclerView's default cache size.
pub const DEFAULT_REUSE_SLOT_COUNT: usize = 7;

/// Policy for reusing composed item slots.
#[derive(Clone, Debug)]
pub struct SlotReusePolicy {
    /// Maximum number of slots to keep for each content type.
    pub max_slots_per_type: usize,

    /// Whether slot reuse is enabled.
    pub enabled: bool,
}

impl Default for SlotReusePolicy {
    fn default() -> Self {
        Self {
            max_slots_per_type: DEFAULT_REUSE_SLOT_COUNT,
            enabled: true,
        }
    }
}

impl SlotReusePolicy {
    /// Creates a policy with the specified slot count.
    pub fn new(max_slots_per_type: usize) -> Self {
        Self {
            max_slots_per_type,
            enabled: true,
        }
    }

    /// Disables slot reuse.
    pub fn disabled() -> Self {
        Self {
            max_slots_per_type: 0,
            enabled: false,
        }
    }
}

/// A reusable slot that can hold a composed item.
#[derive(Debug, Clone)]
pub struct ReusableSlot {
    /// The slot's unique key.
    pub key: u64,

    /// Content type for type-safe reuse.
    pub content_type: Option<u64>,

    /// The node ID of the composed content.
    pub node_id: usize,

    /// Whether this slot is currently in use.
    pub in_use: bool,
}

/// Pool of reusable slots organized by content type.
#[derive(Debug, Default)]
pub struct SlotReusePool {
    /// Available slots grouped by content type.
    /// Key is content_type (0 = default), value is list of available slots.
    available_slots: HashMap<u64, Vec<ReusableSlot>>,

    /// All slots currently in use.
    in_use_slots: HashMap<u64, ReusableSlot>,

    /// Policy controlling reuse behavior.
    policy: SlotReusePolicy,
}

impl SlotReusePool {
    /// Creates a new pool with the default policy.
    pub fn new() -> Self {
        Self::with_policy(SlotReusePolicy::default())
    }

    /// Creates a pool with the specified policy.
    pub fn with_policy(policy: SlotReusePolicy) -> Self {
        Self {
            available_slots: HashMap::new(),
            in_use_slots: HashMap::new(),
            policy,
        }
    }

    /// Attempts to get a reusable slot for the given content type.
    /// Returns None if no matching slot is available.
    ///
    /// This returns the slot metadata but does NOT remove it from the available pool.
    /// For true slot reuse (where you want to recompose the existing slot),
    /// use [`try_take_reusable`] instead.
    pub fn try_get_slot(&mut self, content_type: Option<u64>) -> Option<ReusableSlot> {
        if !self.policy.enabled {
            return None;
        }

        let type_key = content_type.unwrap_or(0);

        if let Some(slots) = self.available_slots.get_mut(&type_key) {
            slots.pop()
        } else {
            None
        }
    }

    /// Attempts to take a reusable slot for ACTUAL reuse.
    ///
    /// This is used for true slot reuse where we want to reuse an existing
    /// composed node tree instead of creating a new one. The returned tuple
    /// contains:
    /// - The original slot key (to be used as the SlotId for subcomposition)
    /// - The node ID of the existing root node
    ///
    /// JC Pattern: `SubcomposeLayoutState.takeNodeFromReusables()`
    ///
    /// # Algorithm
    /// 1. First tries to find an exact slot ID match (same key)
    /// 2. Falls back to finding a compatible content type match
    ///
    /// When a slot is taken, it's removed from the available pool and should
    /// be re-registered via [`mark_in_use`] after subcomposition.
    pub fn try_take_reusable(
        &mut self,
        target_slot_id: u64,
        content_type: Option<u64>,
    ) -> Option<(u64, usize)> {
        if !self.policy.enabled {
            return None;
        }

        let type_key = content_type.unwrap_or(0);

        // First, try exact match (same slot ID in available pool)
        if let Some(slots) = self.available_slots.get_mut(&type_key) {
            if let Some(pos) = slots.iter().position(|s| s.key == target_slot_id) {
                let slot = slots.remove(pos);
                return Some((slot.key, slot.node_id));
            }
        }

        // Second, try content-type-compatible match (any slot with matching type)
        // This is the key for true cross-slot reuse
        if let Some(slots) = self.available_slots.get_mut(&type_key) {
            if let Some(slot) = slots.pop() {
                return Some((slot.key, slot.node_id));
            }
        }

        None
    }

    /// Checks if there's a reusable slot available without taking it.
    pub fn has_reusable(&self, content_type: Option<u64>) -> bool {
        if !self.policy.enabled {
            return false;
        }
        let type_key = content_type.unwrap_or(0);
        self.available_slots
            .get(&type_key)
            .is_some_and(|slots| !slots.is_empty())
    }

    /// Returns a slot to the pool for reuse.
    pub fn return_slot(&mut self, mut slot: ReusableSlot) {
        if !self.policy.enabled {
            return;
        }

        slot.in_use = false;
        let type_key = slot.content_type.unwrap_or(0);

        // Remove from in-use
        self.in_use_slots.remove(&slot.key);

        // Add to available if under limit
        let slots = self.available_slots.entry(type_key).or_default();
        if slots.len() < self.policy.max_slots_per_type {
            slots.push(slot);
        }
        // Otherwise, let the slot be dropped (disposed)
    }

    /// Marks a slot as in use with the given key.
    pub fn mark_in_use(&mut self, key: u64, content_type: Option<u64>, node_id: usize) {
        let slot = ReusableSlot {
            key,
            content_type,
            node_id,
            in_use: true,
        };
        self.in_use_slots.insert(key, slot);
    }

    /// Gets a slot that's currently in use by key.
    pub fn get_in_use(&self, key: u64) -> Option<&ReusableSlot> {
        self.in_use_slots.get(&key)
    }

    /// Releases all slots that are no longer visible.
    /// Items in `visible_keys` stay in use, others go to available pool.
    pub fn release_non_visible(&mut self, visible_keys: &[u64]) {
        // Convert to HashSet for O(1) lookup instead of O(n)
        let visible_set: std::collections::HashSet<u64> = visible_keys.iter().copied().collect();

        let to_release: Vec<u64> = self
            .in_use_slots
            .keys()
            .filter(|k| !visible_set.contains(k))
            .copied()
            .collect();

        for key in to_release {
            if let Some(slot) = self.in_use_slots.remove(&key) {
                // Inline the return logic to avoid double-remove
                let type_key = slot.content_type.unwrap_or(0);
                let slots = self.available_slots.entry(type_key).or_default();
                if slots.len() < self.policy.max_slots_per_type {
                    let mut available_slot = slot;
                    available_slot.in_use = false;
                    slots.push(available_slot);
                }
            }
        }
    }

    /// Returns the number of available slots.
    pub fn available_count(&self) -> usize {
        self.available_slots.values().map(|v| v.len()).sum()
    }

    /// Returns the number of slots in use.
    pub fn in_use_count(&self) -> usize {
        self.in_use_slots.len()
    }

    /// Clears all slots from the pool.
    pub fn clear(&mut self) {
        self.available_slots.clear();
        self.in_use_slots.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_reuse() {
        let mut pool = SlotReusePool::new();

        // Mark some slots in use
        pool.mark_in_use(1, None, 100);
        pool.mark_in_use(2, None, 101);

        assert_eq!(pool.in_use_count(), 2);
        assert_eq!(pool.available_count(), 0);

        // Release one slot
        let slot = pool.get_in_use(1).unwrap().clone();
        pool.return_slot(slot);

        assert_eq!(pool.in_use_count(), 1);
        assert_eq!(pool.available_count(), 1);

        // Try to get a reusable slot
        let reused = pool.try_get_slot(None);
        assert!(reused.is_some());
        assert_eq!(reused.unwrap().key, 1);
    }

    #[test]
    fn test_content_type_matching() {
        let mut pool = SlotReusePool::new();

        // Create slots with different content types
        pool.mark_in_use(1, Some(100), 1000);
        pool.mark_in_use(2, Some(200), 1001);

        let slot1 = pool.get_in_use(1).unwrap().clone();
        let slot2 = pool.get_in_use(2).unwrap().clone();

        pool.return_slot(slot1);
        pool.return_slot(slot2);

        // Should get matching content type
        let reused = pool.try_get_slot(Some(100));
        assert!(reused.is_some());
        assert_eq!(reused.unwrap().content_type, Some(100));

        // Wrong type returns None
        let wrong_type = pool.try_get_slot(Some(300));
        assert!(wrong_type.is_none());
    }

    #[test]
    fn test_release_non_visible() {
        let mut pool = SlotReusePool::new();

        pool.mark_in_use(1, None, 100);
        pool.mark_in_use(2, None, 101);
        pool.mark_in_use(3, None, 102);

        // Only key 2 is visible
        pool.release_non_visible(&[2]);

        assert_eq!(pool.in_use_count(), 1);
        assert_eq!(pool.available_count(), 2);
        assert!(pool.get_in_use(2).is_some());
    }

    #[test]
    fn test_slot_limit() {
        let policy = SlotReusePolicy::new(2);
        let mut pool = SlotReusePool::with_policy(policy);

        // Create more slots than limit
        for i in 0..5 {
            pool.mark_in_use(i, None, i as usize);
        }

        // Release all
        pool.release_non_visible(&[]);

        // Should only keep 2
        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn test_try_take_reusable_exact_match() {
        let mut pool = SlotReusePool::new();

        // Create and release slots
        pool.mark_in_use(100, Some(1), 1000);
        pool.mark_in_use(200, Some(1), 2000);
        pool.release_non_visible(&[]);

        assert_eq!(pool.available_count(), 2);

        // Exact match: should get the slot with key 100
        let result = pool.try_take_reusable(100, Some(1));
        assert!(result.is_some());
        let (key, node_id) = result.unwrap();
        assert_eq!(key, 100);
        assert_eq!(node_id, 1000);

        // Pool should now have 1 available
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_try_take_reusable_compatible_type() {
        let mut pool = SlotReusePool::new();

        // Create and release a slot with content type 1
        pool.mark_in_use(100, Some(1), 1000);
        pool.release_non_visible(&[]);

        // Request for a DIFFERENT key (999) but same content type (1)
        // Should get the compatible slot (cross-slot reuse!)
        let result = pool.try_take_reusable(999, Some(1));
        assert!(result.is_some());
        let (original_key, node_id) = result.unwrap();
        assert_eq!(original_key, 100); // Returns the ORIGINAL key
        assert_eq!(node_id, 1000);

        // Pool should now be empty
        assert_eq!(pool.available_count(), 0);
    }

    #[test]
    fn test_try_take_reusable_wrong_type() {
        let mut pool = SlotReusePool::new();

        // Create and release a slot with content type 1
        pool.mark_in_use(100, Some(1), 1000);
        pool.release_non_visible(&[]);

        // Request for content type 2 - should NOT match
        let result = pool.try_take_reusable(100, Some(2));
        assert!(result.is_none());

        // Slot should still be available
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_has_reusable() {
        let mut pool = SlotReusePool::new();

        assert!(!pool.has_reusable(Some(1)));

        pool.mark_in_use(100, Some(1), 1000);
        pool.release_non_visible(&[]);

        assert!(pool.has_reusable(Some(1)));
        assert!(!pool.has_reusable(Some(2)));
        assert!(!pool.has_reusable(None));
    }
}
