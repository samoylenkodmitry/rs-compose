use compose_core::NodeId;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! {
    static LAYOUT_REPASS_MANAGER: RefCell<LayoutRepassManager> =
        RefCell::new(LayoutRepassManager::new());
}

/// Manages scoped layout invalidations for specific nodes.
///
/// Similar to PointerDispatchManager, this tracks which specific nodes
/// need layout invalidation rather than forcing a global invalidation.
struct LayoutRepassManager {
    dirty_nodes: HashSet<NodeId>,
}

impl LayoutRepassManager {
    fn new() -> Self {
        Self {
            dirty_nodes: HashSet::new(),
        }
    }

    fn schedule_repass(&mut self, node_id: NodeId) {
        self.dirty_nodes.insert(node_id);
    }

    fn has_pending_repass(&self) -> bool {
        !self.dirty_nodes.is_empty()
    }

    fn take_dirty_nodes(&mut self) -> Vec<NodeId> {
        self.dirty_nodes.drain().collect()
    }
}

/// Schedules a layout repass for a specific node.
///
/// This is the preferred way to invalidate layout for a single node (e.g., from scroll).
/// The app shell will call `process_layout_repasses` to bubble the dirty flag up the tree.
pub fn schedule_layout_repass(node_id: NodeId) {
    LAYOUT_REPASS_MANAGER.with(|manager| {
        manager.borrow_mut().schedule_repass(node_id);
    });
    // Also set the global flag so the app shell knows to process repasses
    LAYOUT_INVALIDATED.store(true, Ordering::Relaxed);
}

/// Returns true if any layout repasses are pending.
pub fn has_pending_layout_repasses() -> bool {
    LAYOUT_REPASS_MANAGER.with(|manager| manager.borrow().has_pending_repass())
}

/// Takes all pending layout repass node IDs.
///
/// The caller should iterate over these and call `bubble_layout_dirty` for each.
pub fn take_layout_repass_nodes() -> Vec<NodeId> {
    LAYOUT_REPASS_MANAGER.with(|manager| manager.borrow_mut().take_dirty_nodes())
}

static RENDER_INVALIDATED: AtomicBool = AtomicBool::new(false);
static POINTER_INVALIDATED: AtomicBool = AtomicBool::new(false);
static FOCUS_INVALIDATED: AtomicBool = AtomicBool::new(false);

/// Requests that the renderer rebuild the current scene.
pub fn request_render_invalidation() {
    RENDER_INVALIDATED.store(true, Ordering::Relaxed);
}

/// Returns true if a render invalidation was pending and clears the flag.
pub fn take_render_invalidation() -> bool {
    RENDER_INVALIDATED.swap(false, Ordering::Relaxed)
}

/// Returns true if a render invalidation is pending without clearing it.
pub fn peek_render_invalidation() -> bool {
    RENDER_INVALIDATED.load(Ordering::Relaxed)
}

/// Requests a new pointer-input pass without touching layout or draw dirties.
pub fn request_pointer_invalidation() {
    POINTER_INVALIDATED.store(true, Ordering::Relaxed);
}

/// Returns true if a pointer invalidation was pending and clears the flag.
pub fn take_pointer_invalidation() -> bool {
    POINTER_INVALIDATED.swap(false, Ordering::Relaxed)
}

/// Returns true if a pointer invalidation is pending without clearing it.
pub fn peek_pointer_invalidation() -> bool {
    POINTER_INVALIDATED.load(Ordering::Relaxed)
}

/// Requests a focus recomposition without affecting layout/draw dirties.
pub fn request_focus_invalidation() {
    FOCUS_INVALIDATED.store(true, Ordering::Relaxed);
}

/// Returns true if a focus invalidation was pending and clears the flag.
pub fn take_focus_invalidation() -> bool {
    FOCUS_INVALIDATED.swap(false, Ordering::Relaxed)
}

/// Returns true if a focus invalidation is pending without clearing it.
pub fn peek_focus_invalidation() -> bool {
    FOCUS_INVALIDATED.load(Ordering::Relaxed)
}

static LAYOUT_INVALIDATED: AtomicBool = AtomicBool::new(false);

/// Requests that layout be re-run.
pub fn request_layout_invalidation() {
    LAYOUT_INVALIDATED.store(true, Ordering::Relaxed);
}

/// Returns true if a layout invalidation was pending and clears the flag.
pub fn take_layout_invalidation() -> bool {
    LAYOUT_INVALIDATED.swap(false, Ordering::Relaxed)
}

/// Returns true if a layout invalidation is pending without clearing it.
pub fn peek_layout_invalidation() -> bool {
    LAYOUT_INVALIDATED.load(Ordering::Relaxed)
}
