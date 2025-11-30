//! Pointer input dispatcher plumbing.
//!
//! This module currently contains a lightweight placeholder implementation
//! that will evolve alongside the event routing system. The goal is to make
//! it easy for platform integrations to enqueue pointer events and have them
//! dispatched to modifier nodes.

use crate::nodes::input::types::{PointerEvent, PointerId};
use std::cell::RefCell;

/// Queue for pointer events that need to be processed.
#[derive(Default)]
pub(crate) struct PointerEventQueue {
    queue: RefCell<Vec<(PointerId, PointerEvent)>>,
}

impl PointerEventQueue {
    pub fn new() -> Self {
        Self { queue: RefCell::new(Vec::new()) }
    }

    pub fn push_event(&self, event: PointerEvent) {
        self.queue.borrow_mut().push((event.id(), event));
    }

    pub fn drain<F>(&self, mut handler: F)
    where
        F: FnMut(PointerEvent),
    {
        let mut queue = self.queue.borrow_mut();
        for (_id, event) in queue.drain(..) {
            handler(event);
        }
    }

    pub fn clear(&self) {
        self.queue.borrow_mut().clear();
    }

    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}
