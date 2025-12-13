//! Observable state holder for text field content.
//!
//! Matches Jetpack Compose's `TextFieldState` from
//! `compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/input/TextFieldState.kt`.

use super::{TextFieldBuffer, TextRange};
use std::cell::RefCell;
use std::rc::Rc;

/// Immutable snapshot of text field content.
///
/// This represents the text, selection, and composition state at a point in time.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextFieldValue {
    /// The text content
    pub text: String,
    /// Current selection or cursor position
    pub selection: TextRange,
    /// IME composition range, if any
    pub composition: Option<TextRange>,
}

impl TextFieldValue {
    /// Creates a new value with the given text and cursor at end.
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let len = text.len();
        Self {
            text,
            selection: TextRange::cursor(len),
            composition: None,
        }
    }

    /// Creates a value with specified text and selection.
    pub fn with_selection(text: impl Into<String>, selection: TextRange) -> Self {
        let text = text.into();
        let selection = selection.coerce_in(text.len());
        Self {
            text,
            selection,
            composition: None,
        }
    }
}

type ChangeListener = Box<dyn Fn(&TextFieldValue)>;

/// Maximum capacity for undo stack
const UNDO_CAPACITY: usize = 100;

/// Inner state for TextFieldState
struct TextFieldStateInner {
    /// Current value
    value: TextFieldValue,
    /// Flag to prevent concurrent edits  
    is_editing: bool,
    /// Listeners to notify on changes
    listeners: Vec<ChangeListener>,
    /// Undo stack - previous states to restore
    undo_stack: Vec<TextFieldValue>,
    /// Redo stack - states undone that can be redone
    redo_stack: Vec<TextFieldValue>,
}

/// Observable state holder for text field content.
///
/// This is the primary API for managing text field state. All edits go through
/// the [`edit`](Self::edit) method which provides a mutable buffer.
///
/// # Example
///
/// ```
/// use compose_foundation::text::TextFieldState;
///
/// let state = TextFieldState::new("Hello");
///
/// // Edit the text
/// state.edit(|buffer| {
///     buffer.place_cursor_at_end();
///     buffer.insert(", World!");
/// });
///
/// assert_eq!(state.text(), "Hello, World!");
/// ```
///
/// # Thread Safety
///
/// `TextFieldState` uses `Rc<RefCell<...>>` internally and is not thread-safe.
/// It should only be used from the main thread.
#[derive(Clone)]
pub struct TextFieldState {
    inner: Rc<RefCell<TextFieldStateInner>>,
}

impl std::fmt::Debug for TextFieldState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.borrow();
        f.debug_struct("TextFieldState")
            .field("text", &inner.value.text)
            .field("selection", &inner.value.selection)
            .finish()
    }
}

impl TextFieldState {
    /// Creates a new text field state with the given initial text.
    pub fn new(initial_text: impl Into<String>) -> Self {
        let value = TextFieldValue::new(initial_text);
        Self {
            inner: Rc::new(RefCell::new(TextFieldStateInner {
                value,
                is_editing: false,
                listeners: Vec::new(),
                undo_stack: Vec::new(),
                redo_stack: Vec::new(),
            })),
        }
    }

    /// Creates a state with initial text and selection.
    pub fn with_selection(initial_text: impl Into<String>, selection: TextRange) -> Self {
        let value = TextFieldValue::with_selection(initial_text, selection);
        Self {
            inner: Rc::new(RefCell::new(TextFieldStateInner {
                value,
                is_editing: false,
                listeners: Vec::new(),
                undo_stack: Vec::new(),
                redo_stack: Vec::new(),
            })),
        }
    }

    /// Returns the current text content.
    pub fn text(&self) -> String {
        self.inner.borrow().value.text.clone()
    }

    /// Returns the current selection range.
    pub fn selection(&self) -> TextRange {
        self.inner.borrow().value.selection
    }

    /// Returns the current composition (IME) range, if any.
    pub fn composition(&self) -> Option<TextRange> {
        self.inner.borrow().value.composition
    }
    
    /// Copies the selected text without modifying the clipboard.
    /// Returns the selected text, or None if no selection.
    pub fn copy_selection(&self) -> Option<String> {
        let inner = self.inner.borrow();
        let selection = inner.value.selection;
        if selection.collapsed() {
            return None;
        }
        let start = selection.min();
        let end = selection.max();
        Some(inner.value.text[start..end].to_string())
    }

    /// Returns the current value snapshot.
    pub fn value(&self) -> TextFieldValue {
        self.inner.borrow().value.clone()
    }

    /// Adds a listener that is called when the value changes.
    ///
    /// Returns the listener index for removal.
    pub fn add_listener(&self, listener: impl Fn(&TextFieldValue) + 'static) -> usize {
        let mut inner = self.inner.borrow_mut();
        let index = inner.listeners.len();
        inner.listeners.push(Box::new(listener));
        index
    }

    /// Sets the selection directly without going through undo stack.
    /// Use this for transient selection changes like during drag selection.
    pub fn set_selection(&self, selection: TextRange) {
        let mut inner = self.inner.borrow_mut();
        let len = inner.value.text.len();
        inner.value.selection = selection.coerce_in(len);
    }

    /// Returns true if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.inner.borrow().undo_stack.is_empty()
    }

    /// Returns true if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.inner.borrow().redo_stack.is_empty()
    }

    /// Undoes the last edit.
    /// Returns true if undo was performed.
    pub fn undo(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some(previous_state) = inner.undo_stack.pop() {
            // Save current state to redo stack (clone first to avoid borrow conflict)
            let current = inner.value.clone();
            inner.redo_stack.push(current);
            inner.value = previous_state;
            true
        } else {
            false
        }
    }

    /// Redoes the last undone edit.
    /// Returns true if redo was performed.
    pub fn redo(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if let Some(redo_state) = inner.redo_stack.pop() {
            // Save current state to undo stack (clone first to avoid borrow conflict)
            let current = inner.value.clone();
            inner.undo_stack.push(current);
            inner.value = redo_state;
            true
        } else {
            false
        }
    }

    /// Edits the text field content.
    ///
    /// The provided closure receives a mutable buffer that can be used to
    /// modify the text and selection. After the closure returns, the changes
    /// are committed and listeners are notified.
    ///
    /// # Panics
    ///
    /// Panics if called while already editing (no concurrent or nested edits).
    pub fn edit<F>(&self, f: F)
    where
        F: FnOnce(&mut TextFieldBuffer),
    {
        // Check for concurrent edits
        {
            let inner = self.inner.borrow();
            if inner.is_editing {
                panic!("TextFieldState does not support concurrent or nested editing");
            }
        }

        // Mark as editing
        self.inner.borrow_mut().is_editing = true;

        // Create buffer from current value
        let current = self.value();
        let mut buffer = TextFieldBuffer::with_selection(&current.text, current.selection);
        if let Some(comp) = current.composition {
            buffer.set_composition(Some(comp));
        }

        // Execute the edit
        f(&mut buffer);

        // Build new value
        let new_value = TextFieldValue {
            text: buffer.text().to_string(),
            selection: buffer.selection(),
            composition: buffer.composition(),
        };

        // Only update and notify if changed
        let changed = new_value != current;
        
        {
            let mut inner = self.inner.borrow_mut();
            if changed {
                // Save current state to undo stack before applying changes
                if inner.undo_stack.len() >= UNDO_CAPACITY {
                    inner.undo_stack.remove(0); // Remove oldest
                }
                inner.undo_stack.push(current.clone());
                // Clear redo stack on new edit (can't redo after new changes)
                inner.redo_stack.clear();
                
                inner.value = new_value.clone();
            }
            inner.is_editing = false;
        }

        // Notify listeners outside of borrow
        if changed {
            let listeners: Vec<_> = {
                let inner = self.inner.borrow();
                inner.listeners.iter().map(|_| ()).collect()
            };
            // Re-borrow to call listeners (they may need to read state)
            for i in 0..listeners.len() {
                let listener = {
                    let inner = self.inner.borrow();
                    // Get a reference - we need to handle this carefully
                    if i < inner.listeners.len() {
                        // Call listener with current value
                        let value = inner.value.clone();
                        drop(inner);
                        // Now call with value outside of borrow
                        let inner = self.inner.borrow();
                        if i < inner.listeners.len() {
                            Some((i, value))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some((idx, value)) = listener {
                    let inner = self.inner.borrow();
                    if idx < inner.listeners.len() {
                        (inner.listeners[idx])(&value);
                    }
                }
            }
        }
    }

    /// Sets the text and places cursor at end.
    pub fn set_text(&self, text: impl Into<String>) {
        let text = text.into();
        self.edit(|buffer| {
            buffer.clear();
            buffer.insert(&text);
        });
    }

    /// Sets the text and selects all.
    pub fn set_text_and_select_all(&self, text: impl Into<String>) {
        let text = text.into();
        self.edit(|buffer| {
            buffer.clear();
            buffer.insert(&text);
            buffer.select_all();
        });
    }
}

impl Default for TextFieldState {
    fn default() -> Self {
        Self::new("")
    }
}

impl PartialEq for TextFieldState {
    fn eq(&self, other: &Self) -> bool {
        // Compare by Rc pointer identity - same state instance
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_has_cursor_at_end() {
        let state = TextFieldState::new("Hello");
        assert_eq!(state.text(), "Hello");
        assert_eq!(state.selection(), TextRange::cursor(5));
    }

    #[test]
    fn edit_updates_text() {
        let state = TextFieldState::new("Hello");
        state.edit(|buffer| {
            buffer.place_cursor_at_end();
            buffer.insert(", World!");
        });
        assert_eq!(state.text(), "Hello, World!");
    }

    #[test]
    fn edit_updates_selection() {
        let state = TextFieldState::new("Hello");
        state.edit(|buffer| {
            buffer.select_all();
        });
        assert_eq!(state.selection(), TextRange::new(0, 5));
    }

    #[test]
    fn set_text_replaces_content() {
        let state = TextFieldState::new("Hello");
        state.set_text("Goodbye");
        assert_eq!(state.text(), "Goodbye");
        assert_eq!(state.selection(), TextRange::cursor(7));
    }

    #[test]
    #[should_panic(expected = "concurrent or nested editing")]
    fn nested_edit_panics() {
        let state = TextFieldState::new("Hello");
        let state_clone = state.clone();
        state.edit(move |_buffer| {
            state_clone.edit(|_| {}); // This should panic
        });
    }

    #[test]
    fn listener_is_called_on_change() {
        use std::cell::Cell;
        use std::rc::Rc;

        let state = TextFieldState::new("Hello");
        let called = Rc::new(Cell::new(false));
        let called_clone = called.clone();
        
        state.add_listener(move |_value| {
            called_clone.set(true);
        });

        state.edit(|buffer| {
            buffer.insert("!");
        });

        assert!(called.get());
    }
}

