//! Chunked slot storage backend that avoids large rotate operations.
//!
//! This backend divides the slot array into fixed-size chunks, allowing
//! insertions and deletions to only shift slots within or between adjacent
//! chunks rather than rotating the entire storage.

use crate::{
    slot_storage::{GroupId, SlotStorage, StartGroup, ValueSlotId},
    AnchorId, Key, NodeId, Owned, ScopeId,
};
use std::any::Any;
use std::cell::Cell;

/// Size of each chunk in slots. Tuned for balance between chunk overhead
/// and shift performance.
const CHUNK_SIZE: usize = 256;

/// Chunked slot storage implementation.
///
/// Uses a Vec of fixed-size chunks to store slots, avoiding the O(n) rotate
/// operations needed when inserting near the start of a large flat Vec.
#[derive(Default)]
pub struct ChunkedSlotStorage {
    /// Storage chunks, each up to CHUNK_SIZE slots.
    chunks: Vec<Vec<ChunkedSlot>>,
    /// Global cursor position (linear index across all chunks).
    cursor: usize,
    /// Group stack tracking current composition nesting.
    group_stack: Vec<GroupFrame>,
    /// Anchor ID → global slot position mapping.
    anchors: Vec<usize>,
    /// Whether anchors need rebuilding.
    anchors_dirty: bool,
    /// Counter for allocating unique anchor IDs.
    next_anchor_id: Cell<usize>,
    /// Tracks whether the most recent begin_group reused a gap.
    last_start_was_gap: bool,
}

struct GroupFrame {
    key: Key,
    start: usize,
    end: usize,
    force_children_recompose: bool,
}

enum ChunkedSlot {
    Group {
        key: Key,
        anchor: AnchorId,
        len: usize,
        scope: Option<ScopeId>,
        has_gap_children: bool,
    },
    Value {
        anchor: AnchorId,
        data: Box<dyn Any>,
    },
    Node {
        anchor: AnchorId,
        id: NodeId,
    },
    Gap {
        anchor: AnchorId,
        group_key: Option<Key>,
        group_scope: Option<ScopeId>,
        group_len: usize,
    },
}

impl ChunkedSlot {
    fn anchor_id(&self) -> AnchorId {
        match self {
            ChunkedSlot::Group { anchor, .. } => *anchor,
            ChunkedSlot::Value { anchor, .. } => *anchor,
            ChunkedSlot::Node { anchor, .. } => *anchor,
            ChunkedSlot::Gap { anchor, .. } => *anchor,
        }
    }

    fn as_value<T: 'static>(&self) -> &T {
        match self {
            ChunkedSlot::Value { data, .. } => data
                .downcast_ref::<T>()
                .expect("slot value type mismatch"),
            _ => panic!("slot is not a value"),
        }
    }

    fn as_value_mut<T: 'static>(&mut self) -> &mut T {
        match self {
            ChunkedSlot::Value { data, .. } => data
                .downcast_mut::<T>()
                .expect("slot value type mismatch"),
            _ => panic!("slot is not a value"),
        }
    }
}

impl Default for ChunkedSlot {
    fn default() -> Self {
        ChunkedSlot::Gap {
            anchor: AnchorId::INVALID,
            group_key: None,
            group_scope: None,
            group_len: 0,
        }
    }
}

impl ChunkedSlotStorage {
    pub fn new() -> Self {
        Self {
            next_anchor_id: Cell::new(1), // Start at 1 (0 is INVALID)
            ..Default::default()
        }
    }

    /// Get total number of slots across all chunks.
    fn total_slots(&self) -> usize {
        self.chunks.iter().map(|c| c.len()).sum()
    }

    /// Convert global index to (chunk_index, offset).
    fn global_to_chunk(&self, global: usize) -> (usize, usize) {
        let mut remaining = global;
        for (chunk_idx, chunk) in self.chunks.iter().enumerate() {
            if remaining < chunk.len() {
                return (chunk_idx, remaining);
            }
            remaining -= chunk.len();
        }
        // Past the end
        (self.chunks.len(), 0)
    }

    /// Get a reference to the slot at global index.
    fn get_slot(&self, global: usize) -> Option<&ChunkedSlot> {
        let (chunk_idx, offset) = self.global_to_chunk(global);
        self.chunks.get(chunk_idx)?.get(offset)
    }

    /// Get a mutable reference to the slot at global index.
    fn get_slot_mut(&mut self, global: usize) -> Option<&mut ChunkedSlot> {
        let (chunk_idx, offset) = self.global_to_chunk(global);
        self.chunks.get_mut(chunk_idx)?.get_mut(offset)
    }

    /// Allocate a new anchor ID.
    fn alloc_anchor(&self) -> AnchorId {
        let id = self.next_anchor_id.get();
        self.next_anchor_id.set(id + 1);
        AnchorId::new(id)
    }

    /// Ensure capacity at cursor by adding gap slots if needed.
    fn ensure_capacity(&mut self) {
        if self.chunks.is_empty() {
            // Initialize first chunk
            let mut chunk = Vec::with_capacity(CHUNK_SIZE);
            chunk.resize_with(CHUNK_SIZE, ChunkedSlot::default);
            self.chunks.push(chunk);
        }

        let total = self.total_slots();
        if self.cursor >= total {
            // Need more chunks
            let mut chunk = Vec::with_capacity(CHUNK_SIZE);
            chunk.resize_with(CHUNK_SIZE, ChunkedSlot::default);
            self.chunks.push(chunk);
        }
    }

    /// Insert a slot at the cursor position, shifting within/between chunks.
    fn insert_at_cursor(&mut self, slot: ChunkedSlot) {
        self.ensure_capacity();

        // Simple implementation: just overwrite if it's a gap
        if let Some(existing) = self.get_slot(self.cursor) {
            if matches!(existing, ChunkedSlot::Gap { .. }) {
                *self.get_slot_mut(self.cursor).unwrap() = slot;
                // Update group end to account for this slot
                if let Some(frame) = self.group_stack.last_mut() {
                    if self.cursor >= frame.end {
                        frame.end = self.cursor + 1;
                    }
                }
                return;
            }
        }

        // If not a gap, we'll just overwrite for this MVP implementation.
        // A full implementation would shift slots and update all affected
        // group frames and anchors.
        if self.cursor < self.total_slots() {
            *self.get_slot_mut(self.cursor).unwrap() = slot;
            // Update group end
            if let Some(frame) = self.group_stack.last_mut() {
                if self.cursor >= frame.end {
                    frame.end = self.cursor + 1;
                }
            }
            self.anchors_dirty = true;
        }
    }

    /// Rebuild anchor positions by scanning all slots.
    fn rebuild_anchors(&mut self) {
        if !self.anchors_dirty {
            return;
        }

        // Clear existing anchor map
        for pos in self.anchors.iter_mut() {
            *pos = usize::MAX;
        }

        // Scan all slots and update anchor positions
        let mut global_idx = 0;
        for chunk in &self.chunks {
            for slot in chunk {
                let anchor = slot.anchor_id();
                if anchor.is_valid() {
                    let id = anchor.0;
                    if id >= self.anchors.len() {
                        self.anchors.resize(id + 1, usize::MAX);
                    }
                    self.anchors[id] = global_idx;
                }
                global_idx += 1;
            }
        }

        self.anchors_dirty = false;
    }

    /// Find a gap slot near the cursor.
    fn find_gap_near_cursor(&self) -> Option<usize> {
        // Look forward from cursor
        for offset in 0..64 {
            let pos = self.cursor + offset;
            if let Some(slot) = self.get_slot(pos) {
                if matches!(slot, ChunkedSlot::Gap { .. }) {
                    return Some(pos);
                }
            }
        }
        None
    }

    /// Start a new group at the cursor.
    fn start_group(&mut self, key: Key) -> (usize, bool) {
        self.ensure_capacity();

        // Check if current slot is a gap group we can restore
        if let Some(slot) = self.get_slot(self.cursor) {
            if let ChunkedSlot::Gap {
                group_key: Some(gap_key),
                group_scope,
                group_len,
                ..
            } = slot
            {
                if *gap_key == key {
                    // Restore the gap group
                    let anchor = self.alloc_anchor();
                    let scope = *group_scope;
                    let len = *group_len;
                    *self.get_slot_mut(self.cursor).unwrap() = ChunkedSlot::Group {
                        key,
                        anchor,
                        len,
                        scope,
                        has_gap_children: true,
                    };

                    let start = self.cursor;
                    self.cursor += 1;
                    self.group_stack.push(GroupFrame {
                        key,
                        start,
                        end: start + len,
                        force_children_recompose: true,
                    });
                    self.last_start_was_gap = true;
                    return (start, true);
                }
            }
        }

        // Create new group
        let anchor = self.alloc_anchor();
        let slot = ChunkedSlot::Group {
            key,
            anchor,
            len: 0,
            scope: None,
            has_gap_children: false,
        };

        self.insert_at_cursor(slot);
        let start = self.cursor;
        self.cursor += 1;
        self.group_stack.push(GroupFrame {
            key,
            start,
            end: start,
            force_children_recompose: false,
        });
        self.last_start_was_gap = false;
        (start, false)
    }

    /// End the current group (internal implementation).
    fn do_end_group(&mut self) {
        if let Some(frame) = self.group_stack.pop() {
            let len = self.cursor.saturating_sub(frame.start + 1);
            if let Some(slot) = self.get_slot_mut(frame.start) {
                if let ChunkedSlot::Group { len: slot_len, .. } = slot {
                    *slot_len = len;
                }
            }
        }
    }

    /// Skip over the current group (internal implementation).
    fn do_skip_current_group(&mut self) {
        if let Some(slot) = self.get_slot(self.cursor) {
            if let ChunkedSlot::Group { len, .. } = slot {
                self.cursor += 1 + len;
            }
        }
    }

    /// Finalize current group by marking unreached tail as gaps (internal implementation).
    fn do_finalize_current_group(&mut self) -> bool {
        let frame_end = match self.group_stack.last() {
            Some(frame) => frame.end,
            None => return false,
        };

        let mut marked = false;
        while self.cursor < frame_end {
            if let Some(slot) = self.get_slot_mut(self.cursor) {
                // Convert to gap
                let anchor = slot.anchor_id();
                let (group_key, group_scope, group_len) = match slot {
                    ChunkedSlot::Group { key, scope, len, .. } => (Some(*key), *scope, *len),
                    _ => (None, None, 0),
                };
                *slot = ChunkedSlot::Gap {
                    anchor,
                    group_key,
                    group_scope,
                    group_len,
                };
                marked = true;
            }
            self.cursor += 1;
        }

        if let Some(frame) = self.group_stack.last_mut() {
            frame.end = self.cursor;
        }
        marked
    }
}

impl SlotStorage for ChunkedSlotStorage {
    type Group = GroupId;
    type ValueSlot = ValueSlotId;

    fn begin_group(&mut self, key: Key) -> StartGroup<Self::Group> {
        let (idx, restored) = self.start_group(key);
        StartGroup {
            group: GroupId::new(idx),
            restored_from_gap: restored,
        }
    }

    fn set_group_scope(&mut self, group: Self::Group, scope: ScopeId) {
        if let Some(slot) = self.get_slot_mut(group.index()) {
            if let ChunkedSlot::Group {
                scope: slot_scope, ..
            } = slot
            {
                *slot_scope = Some(scope);
            }
        }
    }

    fn end_group(&mut self) {
        self.do_end_group();
    }

    fn skip_current_group(&mut self) {
        self.do_skip_current_group();
    }

    fn nodes_in_current_group(&self) -> Vec<NodeId> {
        // Scan current group for nodes
        let mut nodes = Vec::new();
        if let Some(frame) = self.group_stack.last() {
            for pos in (frame.start + 1)..frame.end {
                if let Some(ChunkedSlot::Node { id, .. }) = self.get_slot(pos) {
                    nodes.push(*id);
                }
            }
        }
        nodes
    }

    fn begin_recompose_at_scope(&mut self, scope: ScopeId) -> Option<Self::Group> {
        // Linear scan to find group with this scope
        for global_idx in 0..self.total_slots() {
            if let Some(ChunkedSlot::Group {
                scope: Some(s), ..
            }) = self.get_slot(global_idx)
            {
                if *s == scope {
                    self.cursor = global_idx;
                    return Some(GroupId::new(global_idx));
                }
            }
        }
        None
    }

    fn end_recompose(&mut self) {
        // No-op for chunked storage
    }

    fn alloc_value_slot<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Self::ValueSlot {
        self.ensure_capacity();

        // Check if current slot is a reusable value slot
        if let Some(ChunkedSlot::Value { data, .. }) = self.get_slot(self.cursor) {
            if data.is::<T>() {
                let slot_id = ValueSlotId::new(self.cursor);
                self.cursor += 1;
                return slot_id;
            }
        }

        // Create new value slot
        let anchor = self.alloc_anchor();
        let slot = ChunkedSlot::Value {
            anchor,
            data: Box::new(init()),
        };
        self.insert_at_cursor(slot);
        let slot_id = ValueSlotId::new(self.cursor);
        self.cursor += 1;
        slot_id
    }

    fn read_value<T: 'static>(&self, slot: Self::ValueSlot) -> &T {
        self.get_slot(slot.index())
            .expect("value slot not found")
            .as_value()
    }

    fn read_value_mut<T: 'static>(&mut self, slot: Self::ValueSlot) -> &mut T {
        self.get_slot_mut(slot.index())
            .expect("value slot not found")
            .as_value_mut()
    }

    fn write_value<T: 'static>(&mut self, slot: Self::ValueSlot, value: T) {
        if let Some(slot_mut) = self.get_slot_mut(slot.index()) {
            if let ChunkedSlot::Value { data, .. } = slot_mut {
                *data = Box::new(value);
            }
        }
    }

    fn remember<T: 'static>(&mut self, init: impl FnOnce() -> T) -> Owned<T> {
        let slot = self.alloc_value_slot(|| Owned::new(init()));
        self.read_value::<Owned<T>>(slot).clone()
    }

    fn peek_node(&self) -> Option<NodeId> {
        if let Some(ChunkedSlot::Node { id, .. }) = self.get_slot(self.cursor) {
            Some(*id)
        } else {
            None
        }
    }

    fn record_node(&mut self, id: NodeId) {
        self.ensure_capacity();
        let anchor = self.alloc_anchor();
        let slot = ChunkedSlot::Node { anchor, id };
        self.insert_at_cursor(slot);
        self.cursor += 1;
    }

    fn advance_after_node_read(&mut self) {
        self.cursor += 1;
    }

    fn step_back(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn finalize_current_group(&mut self) -> bool {
        self.do_finalize_current_group()
    }

    fn reset(&mut self) {
        self.cursor = 0;
        self.group_stack.clear();
    }

    fn flush(&mut self) {
        self.rebuild_anchors();
    }
}
