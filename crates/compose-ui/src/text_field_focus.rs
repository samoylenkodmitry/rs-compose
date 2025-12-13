//! Global focus manager for text fields.
//!
//! This module tracks which text field currently has focus, ensuring only one
//! text field is focused at a time. When a new field requests focus, the
//! previously focused field is automatically unfocused.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};

// Thread-local for the actual focus Rc storage (needed because Rc isn't Send)
thread_local! {
    static FOCUSED_FIELD: RefCell<Option<Weak<RefCell<bool>>>> = const { RefCell::new(None) };
}

// Global atomic for cross-thread visibility (for needs_redraw in different thread)
static HAS_FOCUSED_FIELD: AtomicBool = AtomicBool::new(false);

/// Requests focus for a text field.
///
/// If another text field was previously focused, it will be unfocused first.
/// The provided `is_focused` handle should be the field's focus state.
pub fn request_focus(is_focused: Rc<RefCell<bool>>) {
    FOCUSED_FIELD.with(|current| {
        let mut current = current.borrow_mut();
        
        // Unfocus the previously focused field (if any and still alive)
        if let Some(ref weak) = *current {
            if let Some(old_focused) = weak.upgrade() {
                *old_focused.borrow_mut() = false;
            }
        }
        
        // Set the new field as focused
        *is_focused.borrow_mut() = true;
        *current = Some(Rc::downgrade(&is_focused));
    });
    
    // Update global atomic for cross-thread visibility
    HAS_FOCUSED_FIELD.store(true, Ordering::SeqCst);
    
    // Request layout invalidation so modifier slices are re-collected with cursor draw command
    crate::request_layout_invalidation();
    // Also request render invalidation for immediate redraw
    crate::request_render_invalidation();
}

/// Clears focus from the currently focused text field.
#[allow(dead_code)]
pub fn clear_focus() {
    FOCUSED_FIELD.with(|current| {
        let mut current = current.borrow_mut();
        
        if let Some(ref weak) = *current {
            if let Some(focused) = weak.upgrade() {
                *focused.borrow_mut() = false;
            }
        }
        
        *current = None;
    });
    
    // Update global atomic
    HAS_FOCUSED_FIELD.store(false, Ordering::SeqCst);
    
    crate::request_render_invalidation();
}

/// Returns true if any text field currently has focus.
/// Uses global atomic for cross-thread visibility.
pub fn has_focused_field() -> bool {
    HAS_FOCUSED_FIELD.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_focus_sets_flag() {
        let focus = Rc::new(RefCell::new(false));
        request_focus(focus.clone());
        assert!(*focus.borrow());
    }

    #[test]
    fn request_focus_clears_previous() {
        let focus1 = Rc::new(RefCell::new(false));
        let focus2 = Rc::new(RefCell::new(false));
        
        request_focus(focus1.clone());
        assert!(*focus1.borrow());
        
        request_focus(focus2.clone());
        assert!(!*focus1.borrow()); // First should be unfocused
        assert!(*focus2.borrow());  // Second should be focused
    }

    #[test]
    fn clear_focus_unfocuses_current() {
        let focus = Rc::new(RefCell::new(false));
        request_focus(focus.clone());
        assert!(*focus.borrow());
        
        clear_focus();
        assert!(!*focus.borrow());
    }
}
