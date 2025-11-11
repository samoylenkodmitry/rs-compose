use std::sync::atomic::{AtomicBool, Ordering};

static RENDER_INVALIDATED: AtomicBool = AtomicBool::new(false);

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
