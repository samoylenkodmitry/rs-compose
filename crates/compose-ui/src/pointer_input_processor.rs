//! Pointer Input Event Processor - placeholder for full implementation

use crate::input::HitPathTracker;
use compose_foundation::nodes::input::types::PointerEvent;

/// Processes pointer input events for the composition tree
/// TODO: Full implementation following Kotlin's PointerInputEventProcessor
pub struct PointerInputEventProcessor {
    hit_path_tracker: HitPathTracker,
    is_processing: bool,
}

impl PointerInputEventProcessor {
    pub fn new() -> Self {
        Self {
            hit_path_tracker: HitPathTracker::new(),
            is_processing: false,
        }
    }
}
