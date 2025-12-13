//! Text field modifier node for editable text input.
//!
//! This module implements the modifier node for `BasicTextField`, following
//! Jetpack Compose's `CoreTextFieldNode` architecture.
//!
//! The node handles:
//! - **Layout**: Measures text content and returns appropriate size
//! - **Draw**: Renders text, cursor, and selection highlights
//! - **Pointer Input**: Handles tap to position cursor, drag for selection
//! - **Semantics**: Provides text content for accessibility
//!
//! # Architecture
//!
//! Unlike display-only `TextModifierNode`, this node:
//! - References a `TextFieldState` for mutable text
//! - Tracks focus state for cursor visibility
//! - Handles pointer events for cursor positioning

use compose_foundation::text::{TextFieldState, TextRange};
use compose_foundation::{
    Constraints, DelegatableNode, DrawModifierNode, DrawScope, InvalidationKind,
    LayoutModifierNode, Measurable, ModifierNode, ModifierNodeContext, ModifierNodeElement,
    NodeCapabilities, NodeState, PointerEvent, PointerEventKind, PointerInputNode,
    SemanticsConfiguration, SemanticsNode, Size,
};
use compose_ui_graphics::{Brush, Color, Rect};
use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Default cursor color (white - visible on dark backgrounds)
const DEFAULT_CURSOR_COLOR: Color = Color(1.0, 1.0, 1.0, 1.0);

/// Default selection highlight color (light blue with transparency)
const DEFAULT_SELECTION_COLOR: Color = Color(0.0, 0.5, 1.0, 0.3);

/// Cursor width in pixels
const CURSOR_WIDTH: f32 = 2.0;

/// Modifier node for editable text fields.
///
/// This node is the core of `BasicTextField`, handling:
/// - Text measurement and layout
/// - Cursor and selection rendering
/// - Pointer input for cursor positioning
pub struct TextFieldModifierNode {
    /// The text field state (shared)
    state: TextFieldState,
    /// Whether this field is currently focused (shared with handler)
    is_focused: Rc<RefCell<bool>>,
    /// Whether the cursor is currently visible (for blinking)
    cursor_visible: bool,
    /// Cursor brush color
    cursor_brush: Brush,
    /// Selection highlight brush
    _selection_brush: Brush,
    /// Cached text value for change detection
    cached_text: String,
    /// Cached selection for change detection
    cached_selection: TextRange,
    /// Node state for delegation
    node_state: NodeState,
    /// Measured size cache
    measured_size: Cell<Size>,
    /// Content offset from left (padding) for accurate click positioning
    /// Updated during slices collection, used by click handler
    content_offset: Rc<Cell<f32>>,
    /// Drag anchor position (byte offset) for click-drag selection
    /// Set on mouse down, used during drag to extend selection
    drag_anchor: Rc<Cell<Option<usize>>>,
    /// Last click time for double/triple-click detection
    last_click_time: Rc<Cell<Option<instant::Instant>>>,
    /// Click count (1=single, 2=double, 3=triple)
    click_count: Rc<Cell<u8>>,
    /// Desired column for Up/Down navigation - remembers original column
    /// when navigating through lines of different lengths
    desired_column: Cell<Option<usize>>,
    /// Cached pointer input handler
    cached_handler: Rc<dyn Fn(PointerEvent)>,
}

impl std::fmt::Debug for TextFieldModifierNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextFieldModifierNode")
            .field("text", &self.state.text())
            .field("is_focused", &*self.is_focused.borrow())
            .finish()
    }
}

impl TextFieldModifierNode {
    /// Creates a new text field modifier node.
    pub fn new(state: TextFieldState) -> Self {
        let value = state.value();
        let is_focused = Rc::new(RefCell::new(false));
        let content_offset = Rc::new(Cell::new(0.0_f32));
        let drag_anchor = Rc::new(Cell::new(None::<usize>));
        let last_click_time = Rc::new(Cell::new(None::<instant::Instant>));
        let click_count = Rc::new(Cell::new(0_u8));
        let cached_handler = Self::create_handler(
            state.clone(),
            is_focused.clone(),
            content_offset.clone(),
            drag_anchor.clone(),
            last_click_time.clone(),
            click_count.clone(),
        );
        
        Self {
            state,
            is_focused,
            cursor_visible: true,
            cursor_brush: Brush::solid(DEFAULT_CURSOR_COLOR),
            _selection_brush: Brush::solid(DEFAULT_SELECTION_COLOR),
            cached_text: value.text,
            cached_selection: value.selection,
            node_state: NodeState::new(),
            measured_size: Cell::new(Size {
                width: 0.0,
                height: 0.0,
            }),
            content_offset,
            drag_anchor,
            last_click_time,
            click_count,
            desired_column: Cell::new(None),
            cached_handler,
        }
    }
    
    /// Creates the pointer input handler closure.
    /// Double-click timeout in milliseconds
    const DOUBLE_CLICK_MS: u128 = 500;
    
    fn create_handler(
        state: TextFieldState,
        is_focused: Rc<RefCell<bool>>,
        content_offset: Rc<Cell<f32>>,
        drag_anchor: Rc<Cell<Option<usize>>>,
        last_click_time: Rc<Cell<Option<instant::Instant>>>,
        click_count: Rc<Cell<u8>>,
    ) -> Rc<dyn Fn(PointerEvent)> {
        // Default line height (same as used in measurement)
        const LINE_HEIGHT: f32 = 20.0;
        
        // Helper to find character position from x,y coordinates for multiline
        let find_char_pos = move |click_x: f32, click_y: f32, text: &str| -> usize {
            if text.is_empty() {
                return 0;
            }
            
            // Split text into lines
            let lines: Vec<&str> = text.split('\n').collect();
            
            // Determine which line was clicked based on y coordinate
            let line_index = ((click_y / LINE_HEIGHT).floor() as usize).min(lines.len().saturating_sub(1));
            
            // Get the clicked line
            let line = lines.get(line_index).unwrap_or(&"");
            
            // Find byte offset at the start of this line
            let mut line_start_byte = 0;
            for i in 0..line_index {
                line_start_byte += lines[i].len() + 1; // +1 for newline
            }
            
            // Find character position within the line
            let mut best_pos = 0;
            let mut min_distance = f32::MAX;
            
            for i in 0..=line.chars().count() {
                let byte_pos = line.char_indices()
                    .take(i)
                    .last()
                    .map(|(idx, c)| idx + c.len_utf8())
                    .unwrap_or(0);
                
                let prefix = &line[..byte_pos.min(line.len())];
                let width = crate::text::measure_text(prefix).width;
                let distance = (width - click_x).abs();
                
                if distance < min_distance {
                    min_distance = distance;
                    best_pos = byte_pos;
                }
            }
            
            // Return byte position in the full text
            line_start_byte + best_pos
        };
        
        // Helper to find word boundaries around a byte position
        fn find_word_boundaries(text: &str, pos: usize) -> (usize, usize) {
            if text.is_empty() {
                return (0, 0);
            }
            
            // Find start of word (scan backwards for word char)
            let mut start = pos;
            for (idx, c) in text[..pos.min(text.len())].char_indices().rev() {
                if c.is_alphanumeric() || c == '_' {
                    start = idx;
                } else if start != pos {
                    break;
                }
            }
            
            // Find end of word (scan forwards for word char)
            let mut end = pos;
            for (idx, c) in text[pos.min(text.len())..].char_indices() {
                let actual_idx = pos + idx;
                if c.is_alphanumeric() || c == '_' {
                    end = actual_idx + c.len_utf8();
                } else if end != pos {
                    break;
                }
            }
            
            (start, end)
        }
        
        Rc::new(move |event: PointerEvent| {
            let padding_offset = content_offset.get();
            let click_x = (event.position.x - padding_offset).max(0.0);
            let click_y = event.position.y.max(0.0);
            
            match event.kind {
                PointerEventKind::Down => {
                    // Request focus
                    crate::text_field_focus::request_focus(is_focused.clone());
                    
                    let now = instant::Instant::now();
                    let text = state.text();
                    let pos = find_char_pos(click_x, click_y, &text);
                    
                    // Detect double-click
                    let is_double_click = if let Some(last) = last_click_time.get() {
                        now.duration_since(last).as_millis() < Self::DOUBLE_CLICK_MS
                    } else {
                        false
                    };
                    
                    if is_double_click {
                        // Increment click count for potential triple-click
                        let count = click_count.get() + 1;
                        click_count.set(count.min(3));
                        
                        if count >= 3 {
                            // Triple-click: select all
                            state.edit(|buffer| {
                                buffer.select_all();
                            });
                            // Set drag anchor to start for select-all drag
                            drag_anchor.set(Some(0));
                        } else if count >= 2 {
                            // Double-click: select word
                            let (word_start, word_end) = find_word_boundaries(&text, pos);
                            state.edit(|buffer| {
                                buffer.select(TextRange::new(word_start, word_end));
                            });
                            // Set drag anchor to word boundaries for word-extend drag
                            drag_anchor.set(Some(word_start));
                        }
                    } else {
                        // Single click: reset click count, place cursor
                        click_count.set(1);
                        drag_anchor.set(Some(pos));
                        state.edit(|buffer| {
                            buffer.place_cursor_before_char(pos);
                        });
                    }
                    
                    last_click_time.set(Some(now));
                    event.consume();
                }
                PointerEventKind::Move => {
                    // If we have a drag anchor, extend selection during drag
                    if let Some(anchor) = drag_anchor.get() {
                        if *is_focused.borrow() {
                            let text = state.text();
                            let current_pos = find_char_pos(click_x, click_y, &text);
                            
                            // Update selection directly (without undo stack push)
                            state.set_selection(TextRange::new(anchor, current_pos));
                            
                            // Invalidate layout to trigger repaint
                            crate::layout::invalidate_all_layout_caches();
                            
                            event.consume();
                        }
                    }
                }
                PointerEventKind::Up => {
                    // Clear drag anchor on mouse up
                    drag_anchor.set(None);
                }
                _ => {}
            }
        })
    }

    /// Creates a node with custom cursor color.
    pub fn with_cursor_color(mut self, color: Color) -> Self {
        self.cursor_brush = Brush::solid(color);
        self
    }

    /// Sets the focus state.
    pub fn set_focused(&mut self, focused: bool) {
        let current = *self.is_focused.borrow();
        if current != focused {
            *self.is_focused.borrow_mut() = focused;
            self.cursor_visible = focused; // Show cursor when focused
        }
    }

    /// Returns whether the field is focused.
    pub fn is_focused(&self) -> bool {
        *self.is_focused.borrow()
    }

    /// Returns the current text.
    pub fn text(&self) -> String {
        self.state.text()
    }

    /// Returns the current selection.
    pub fn selection(&self) -> TextRange {
        self.state.selection()
    }

    /// Returns the cursor brush for rendering.
    pub fn cursor_brush(&self) -> Brush {
        self.cursor_brush.clone()
    }
    
    /// Returns the selection brush for rendering selection highlight.
    pub fn selection_brush(&self) -> Brush {
        self._selection_brush.clone()
    }
    
    /// Inserts text at the current cursor position (for paste operations).
    pub fn insert_text(&mut self, text: &str) {
        self.state.edit(|buffer| {
            buffer.insert(text);
        });
    }
    
    /// Copies the selected text and returns it (for web copy operation).
    /// Returns None if no selection.
    pub fn copy_selection(&self) -> Option<String> {
        self.state.copy_selection()
    }
    
    /// Cuts the selected text: copies and deletes it.
    /// Returns the cut text, or None if no selection.
    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.copy_selection();
        if text.is_some() {
            self.state.edit(|buffer| {
                buffer.delete(buffer.selection());
            });
        }
        text
    }
    
    /// Returns a clone of the text field state for use in draw closures.
    /// This allows reading selection at DRAW time rather than LAYOUT time.
    pub fn get_state(&self) -> compose_foundation::text::TextFieldState {
        self.state.clone()
    }
    
    /// Updates the content offset (padding.left) for accurate click-to-position cursor placement.
    /// Called from slices collection where padding is known.
    pub fn set_content_offset(&self, offset: f32) {
        self.content_offset.set(offset);
    }

    /// Measures the text content.
    fn measure_text_content(&self) -> Size {
        let text = self.state.text();
        let metrics = crate::text::measure_text(&text);
        Size {
            width: metrics.width,
            height: metrics.height,
        }
    }

    /// Updates cached state and returns true if changed.
    fn update_cached_state(&mut self) -> bool {
        let value = self.state.value();
        let text_changed = value.text != self.cached_text;
        let selection_changed = value.selection != self.cached_selection;

        if text_changed {
            self.cached_text = value.text;
        }
        if selection_changed {
            self.cached_selection = value.selection;
        }

        text_changed || selection_changed
    }

    /// Positions cursor at a given x offset within the text.
    pub fn position_cursor_at_offset(&self, x_offset: f32) {
        let text = self.state.text();
        if text.is_empty() {
            self.state.edit(|buffer| {
                buffer.place_cursor_at_start();
            });
            return;
        }

        // Simple character-based positioning
        // In a real implementation, this would use proper text layout metrics
        let char_width = self.measured_size.get().width / text.len().max(1) as f32;
        let char_index = (x_offset / char_width).round() as usize;
        let clamped_index = char_index.min(text.len());

        // Find the actual byte offset at this character index
        let byte_offset = text
            .char_indices()
            .nth(clamped_index)
            .map(|(i, _)| i)
            .unwrap_or(text.len());

        self.state.edit(|buffer| {
            buffer.place_cursor_before_char(byte_offset);
        });
    }

    /// Handles a keyboard event.
    ///
    /// Returns `true` if the event was consumed, `false` otherwise.
    pub fn handle_key_event(&mut self, event: &crate::key_event::KeyEvent) -> bool {
        use crate::key_event::{KeyCode, KeyEventType};

        // Helper: find position at start of previous word (for Ctrl+Left)
        fn find_word_start(text: &str, pos: usize) -> usize {
            if pos == 0 || text.is_empty() {
                return 0;
            }
            let bytes = text.as_bytes();
            let mut i = pos.min(text.len());
            // Skip any whitespace/punctuation before (moving left)
            while i > 0 && !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_' {
                i -= 1;
            }
            // Now scan back through word chars
            while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
                i -= 1;
            }
            i
        }
        
        // Helper: find position at end of next word (for Ctrl+Right)
        fn find_word_end(text: &str, pos: usize) -> usize {
            let len = text.len();
            if pos >= len || text.is_empty() {
                return len;
            }
            let bytes = text.as_bytes();
            let mut i = pos;
            // Skip any whitespace/punctuation (moving right)
            while i < len && !bytes[i].is_ascii_alphanumeric() && bytes[i] != b'_' {
                i += 1;
            }
            // Now scan forward through word chars
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            i
        }

        // Only handle key down events
        if event.event_type != KeyEventType::KeyDown {
            return false;
        }

        // Only handle events when focused
        if !self.is_focused() {
            return false;
        }

        let consumed = match event.key_code {
            // Enter - insert newline for multiline support
            KeyCode::Enter => {
                self.state.edit(|buffer| {
                    buffer.insert("\n");
                });
                true
            }

            // Character input - insert text
            _ if !event.text.is_empty() && !event.modifiers.command_or_ctrl() => {
                self.state.edit(|buffer| {
                    buffer.insert(&event.text);
                });
                true
            }

            // Backspace - delete character before cursor
            KeyCode::Backspace => {
                self.state.edit(|buffer| {
                    buffer.delete_before_cursor();
                });
                true
            }

            // Delete - delete character after cursor
            KeyCode::Delete => {
                self.state.edit(|buffer| {
                    buffer.delete_after_cursor();
                });
                true
            }

            // Arrow Left - move cursor or extend selection left
            KeyCode::ArrowLeft => {
                self.desired_column.set(None); // Clear vertical navigation memory
                if event.modifiers.command_or_ctrl() && !event.modifiers.shift {
                    // Ctrl+Left: jump to start of previous word
                    let text = self.state.text();
                    let cursor_pos = self.state.selection().start;
                    let target_pos = find_word_start(&text, cursor_pos);
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(target_pos);
                    });
                } else if event.modifiers.shift {
                    // Shift+Left: extend selection left
                    self.state.edit(|buffer| {
                        buffer.extend_selection_left();
                    });
                } else {
                    // Plain Left: move cursor (collapse selection)
                    let text = self.state.text();
                    let selection = self.state.selection();
                    // If there's a selection, collapse to the left edge
                    let target_pos = if !selection.collapsed() {
                        selection.min()
                    } else if selection.start > 0 {
                        // Find previous character boundary
                        text[..selection.start]
                            .char_indices()
                            .last()
                            .map(|(i, _)| i)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(target_pos);
                    });
                }
                true
            }

            // Arrow Right - move cursor or extend selection right
            KeyCode::ArrowRight => {
                self.desired_column.set(None); // Clear vertical navigation memory
                if event.modifiers.command_or_ctrl() && !event.modifiers.shift {
                    // Ctrl+Right: jump to end of next word
                    let text = self.state.text();
                    let cursor_pos = self.state.selection().end;
                    let target_pos = find_word_end(&text, cursor_pos);
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(target_pos);
                    });
                } else if event.modifiers.shift {
                    // Shift+Right: extend selection right
                    self.state.edit(|buffer| {
                        buffer.extend_selection_right();
                    });
                } else {
                    // Plain Right: move cursor (collapse selection)
                    let text = self.state.text();
                    let selection = self.state.selection();
                    // If there's a selection, collapse to the right edge
                    let target_pos = if !selection.collapsed() {
                        selection.max()
                    } else if selection.end < text.len() {
                        // Find next character boundary
                        text[selection.end..]
                            .char_indices()
                            .nth(1)
                            .map(|(i, _)| selection.end + i)
                            .unwrap_or(text.len())
                    } else {
                        text.len()
                    };
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(target_pos);
                    });
                }
                true
            }

            // Arrow Up - move cursor to previous line (Shift extends selection)
            KeyCode::ArrowUp => {
                let text = self.state.text();
                let selection = self.state.selection();
                // For selection extension, use cursor position (end of selection when selecting forward)
                let cursor_pos = selection.end;
                
                // Find current line info
                let text_before = &text[..cursor_pos.min(text.len())];
                let current_line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                let current_column = cursor_pos - current_line_start;
                
                // Use desired_column if set, otherwise set it to current column
                let column = self.desired_column.get().unwrap_or_else(|| {
                    self.desired_column.set(Some(current_column));
                    current_column
                });
                
                // Calculate target position on previous line
                let target_pos = if current_line_start == 0 {
                    // On line 0 - go to start
                    self.desired_column.set(None);
                    0
                } else {
                    // Find previous line start
                    let prev_line_end = current_line_start - 1;
                    let prev_line_start = text[..prev_line_end].rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let prev_line_len = prev_line_end - prev_line_start;
                    let target_column = column.min(prev_line_len);
                    prev_line_start + target_column
                };
                
                if event.modifiers.shift {
                    // Shift+Up: extend selection to target position
                    let anchor = selection.start;
                    self.state.edit(|buffer| {
                        buffer.select(TextRange::new(anchor, target_pos));
                    });
                } else {
                    // Plain Up: move cursor
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(target_pos);
                    });
                }
                true
            }

            // Arrow Down - move cursor to next line (Shift extends selection)
            KeyCode::ArrowDown => {
                let text = self.state.text();
                let selection = self.state.selection();
                // For selection extension, use cursor position (end of selection)
                let cursor_pos = selection.end;
                
                // Find current line info
                let text_before = &text[..cursor_pos.min(text.len())];
                let current_line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                let current_column = cursor_pos - current_line_start;
                
                // Use desired_column if set, otherwise set it to current column
                let column = self.desired_column.get().unwrap_or_else(|| {
                    self.desired_column.set(Some(current_column));
                    current_column
                });
                
                // Find end of current line (next \n or end of text)
                let current_line_end = text[cursor_pos..].find('\n')
                    .map(|i| cursor_pos + i)
                    .unwrap_or(text.len());
                
                // Calculate target position
                let target_pos = if current_line_end >= text.len() {
                    // No next line - go to end
                    self.desired_column.set(None);
                    text.len()
                } else {
                    // Next line starts after the \n
                    let next_line_start = current_line_end + 1;
                    let next_line_end = text[next_line_start..].find('\n')
                        .map(|i| next_line_start + i)
                        .unwrap_or(text.len());
                    let next_line_len = next_line_end - next_line_start;
                    let target_column = column.min(next_line_len);
                    next_line_start + target_column
                };
                
                if event.modifiers.shift {
                    // Shift+Down: extend selection to target position
                    let anchor = selection.start;
                    self.state.edit(|buffer| {
                        buffer.select(TextRange::new(anchor, target_pos));
                    });
                } else {
                    // Plain Down: move cursor
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(target_pos);
                    });
                }
                true
            }

            // Home - move cursor to start of line (Ctrl+Home for document start)
            KeyCode::Home => {
                self.desired_column.set(None); // Clear vertical navigation memory
                if event.modifiers.command_or_ctrl() {
                    // Ctrl+Home: go to document start
                    self.state.edit(|buffer| {
                        buffer.place_cursor_at_start();
                    });
                } else {
                    // Home: go to current line start
                    let text = self.state.text();
                    let cursor_pos = self.state.selection().start;
                    let text_before = &text[..cursor_pos.min(text.len())];
                    let line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(line_start);
                    });
                }
                true
            }

            // End - move cursor to end of line (Ctrl+End for document end)
            KeyCode::End => {
                self.desired_column.set(None); // Clear vertical navigation memory
                if event.modifiers.command_or_ctrl() {
                    // Ctrl+End: go to document end
                    self.state.edit(|buffer| {
                        buffer.place_cursor_at_end();
                    });
                } else {
                    // End: go to current line end
                    let text = self.state.text();
                    let cursor_pos = self.state.selection().start;
                    let line_end = text[cursor_pos..].find('\n')
                        .map(|i| cursor_pos + i)
                        .unwrap_or(text.len());
                    self.state.edit(|buffer| {
                        buffer.place_cursor_before_char(line_end);
                    });
                }
                true
            }

            // Ctrl+A - select all
            KeyCode::A if event.modifiers.command_or_ctrl() => {
                self.state.edit(|buffer| {
                    buffer.select_all();
                });
                true
            }

            // Ctrl+C/X/V - DO NOT handle here!
            // Let these bubble to platform layer:
            // - Desktop: handled in AppShell::on_key_event() with arboard
            // - Web: browser fires native copy/paste/cut events

            // Ctrl+Z - undo
            KeyCode::Z if event.modifiers.command_or_ctrl() && !event.modifiers.shift => {
                self.state.undo();
                true
            }

            // Ctrl+Y or Ctrl+Shift+Z - redo
            KeyCode::Y if event.modifiers.command_or_ctrl() => {
                self.state.redo();
                true
            }
            KeyCode::Z if event.modifiers.command_or_ctrl() && event.modifiers.shift => {
                self.state.redo();
                true
            }



            // Not handled
            _ => false,
        };
        
        // Request layout and render invalidation when text changes
        // Layout invalidation ensures text box resizes to fit new content
        if consumed {
            // Directly invalidate layout caches to force re-measurement
            crate::layout::invalidate_all_layout_caches();
            crate::request_layout_invalidation();
            crate::request_render_invalidation();
        }
        
        consumed
    }
}

impl DelegatableNode for TextFieldModifierNode {
    fn node_state(&self) -> &NodeState {
        &self.node_state
    }
}

impl ModifierNode for TextFieldModifierNode {
    fn on_attach(&mut self, context: &mut dyn ModifierNodeContext) {
        context.invalidate(InvalidationKind::Layout);
        context.invalidate(InvalidationKind::Draw);
        context.invalidate(InvalidationKind::Semantics);
    }

    fn as_draw_node(&self) -> Option<&dyn DrawModifierNode> {
        Some(self)
    }

    fn as_draw_node_mut(&mut self) -> Option<&mut dyn DrawModifierNode> {
        Some(self)
    }

    fn as_layout_node(&self) -> Option<&dyn LayoutModifierNode> {
        Some(self)
    }

    fn as_layout_node_mut(&mut self) -> Option<&mut dyn LayoutModifierNode> {
        Some(self)
    }

    fn as_semantics_node(&self) -> Option<&dyn SemanticsNode> {
        Some(self)
    }

    fn as_semantics_node_mut(&mut self) -> Option<&mut dyn SemanticsNode> {
        Some(self)
    }

    fn as_pointer_input_node(&self) -> Option<&dyn PointerInputNode> {
        Some(self)
    }

    fn as_pointer_input_node_mut(&mut self) -> Option<&mut dyn PointerInputNode> {
        Some(self)
    }
}

impl LayoutModifierNode for TextFieldModifierNode {
    fn measure(
        &self,
        _context: &mut dyn ModifierNodeContext,
        _measurable: &dyn Measurable,
        constraints: Constraints,
    ) -> compose_ui_layout::LayoutModifierMeasureResult {
        // Measure the text content
        let text_size = self.measure_text_content();

        // Add minimum height for empty text (cursor needs space)
        let min_height = if text_size.height < 1.0 {
            20.0 // Default line height
        } else {
            text_size.height
        };

        // Constrain to provided constraints
        let width = text_size
            .width
            .max(constraints.min_width)
            .min(constraints.max_width);
        let height = min_height
            .max(constraints.min_height)
            .min(constraints.max_height);

        let size = Size { width, height };
        self.measured_size.set(size);

        compose_ui_layout::LayoutModifierMeasureResult::with_size(size)
    }

    fn min_intrinsic_width(&self, _measurable: &dyn Measurable, _height: f32) -> f32 {
        self.measure_text_content().width
    }

    fn max_intrinsic_width(&self, _measurable: &dyn Measurable, _height: f32) -> f32 {
        self.measure_text_content().width
    }

    fn min_intrinsic_height(&self, _measurable: &dyn Measurable, _width: f32) -> f32 {
        self.measure_text_content().height.max(20.0)
    }

    fn max_intrinsic_height(&self, _measurable: &dyn Measurable, _width: f32) -> f32 {
        self.measure_text_content().height.max(20.0)
    }
}

impl DrawModifierNode for TextFieldModifierNode {
    fn draw(&mut self, _context: &mut dyn ModifierNodeContext, draw_scope: &mut dyn DrawScope) {
        // Draw cursor if focused
        if self.is_focused() {
            let text = self.state.text();
            let selection = self.state.selection();
            
            // Calculate cursor x position based on text before cursor
            let text_before_cursor = &text[..selection.start.min(text.len())];
            let cursor_x = crate::text::measure_text(text_before_cursor).width;
            
            // Get the height from the measured size or use a default
            let height = draw_scope.size().height.max(16.0);
            
            // Draw cursor line
            let cursor_rect = Rect {
                x: cursor_x,
                y: 0.0,
                width: CURSOR_WIDTH,
                height,
            };
            
            draw_scope.draw_rect_at(cursor_rect, self.cursor_brush.clone());
        }
    }
}

impl SemanticsNode for TextFieldModifierNode {
    fn merge_semantics(&self, config: &mut SemanticsConfiguration) {
        let text = self.state.text();
        config.content_description = Some(text);
        // TODO: Add editable text semantics properties
        // - is_editable = true
        // - text_selection_range = self.state.selection()
    }
}

impl PointerInputNode for TextFieldModifierNode {
    fn on_pointer_event(
        &mut self,
        _context: &mut dyn ModifierNodeContext,
        event: &PointerEvent,
    ) -> bool {
        match event.kind {
            PointerEventKind::Down => {
                // Focus this text field on click/tap
                if !self.is_focused() {
                    self.set_focused(true);
                }
                
                // Position cursor at click location
                self.position_cursor_at_offset(event.position.x);
                
                // Consume the event
                event.consume();
                true
            }
            _ => false,
        }
    }

    fn hit_test(&self, _x: f32, _y: f32) -> bool {
        // Always participate in hit testing
        true
    }

    fn pointer_input_handler(&self) -> Option<Rc<dyn Fn(PointerEvent)>> {
        // Return cached handler for pointer input dispatch
        Some(self.cached_handler.clone())
    }
}

// ============================================================================
// TextFieldElement - Creates and updates TextFieldModifierNode
// ============================================================================

/// Element that creates and updates `TextFieldModifierNode` instances.
///
/// This follows the modifier element pattern where the element is responsible for:
/// - Creating new nodes (via `create`)
/// - Updating existing nodes when properties change (via `update`)
/// - Declaring capabilities (LAYOUT | DRAW | SEMANTICS)
#[derive(Clone)]
pub struct TextFieldElement {
    /// The text field state
    state: TextFieldState,
    /// Cursor color
    cursor_color: Color,
}

impl TextFieldElement {
    /// Creates a new text field element.
    pub fn new(state: TextFieldState) -> Self {
        Self {
            state,
            cursor_color: DEFAULT_CURSOR_COLOR,
        }
    }

    /// Creates an element with custom cursor color.
    pub fn with_cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = color;
        self
    }
}

impl std::fmt::Debug for TextFieldElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextFieldElement")
            .field("text", &self.state.text())
            .field("cursor_color", &self.cursor_color)
            .finish()
    }
}

impl Hash for TextFieldElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the text content for change detection
        self.state.text().hash(state);
        // Hash cursor color
        self.cursor_color.0.to_bits().hash(state);
        self.cursor_color.1.to_bits().hash(state);
        self.cursor_color.2.to_bits().hash(state);
        self.cursor_color.3.to_bits().hash(state);
    }
}

impl PartialEq for TextFieldElement {
    fn eq(&self, _other: &Self) -> bool {
        // Type matching is sufficient - node will be updated via update() method
        // This matches JC behavior where nodes are reused for same-type elements,
        // preserving is_focused state for proper keyboard event handling
        true
    }
}

impl Eq for TextFieldElement {}

impl ModifierNodeElement for TextFieldElement {
    type Node = TextFieldModifierNode;

    fn create(&self) -> Self::Node {
        TextFieldModifierNode::new(self.state.clone()).with_cursor_color(self.cursor_color)
    }

    fn update(&self, node: &mut Self::Node) {
        // Update the state reference
        node.state = self.state.clone();
        node.cursor_brush = Brush::solid(self.cursor_color);
        
        // Recreate the cached handler with the new state but same is_focused and content_offset
        node.cached_handler = TextFieldModifierNode::create_handler(
            node.state.clone(),
            node.is_focused.clone(),
            node.content_offset.clone(),
            node.drag_anchor.clone(),
            node.last_click_time.clone(),
            node.click_count.clone(),
        );

        // Check if content changed and update cache
        if node.update_cached_state() {
            // Content changed - node will need layout/draw invalidation
            // This happens automatically through the modifier reconciliation
        }
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::LAYOUT | NodeCapabilities::DRAW | NodeCapabilities::SEMANTICS | NodeCapabilities::POINTER_INPUT
    }

    fn always_update(&self) -> bool {
        // Always update to capture new state/handler while preserving focus state
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_field_node_creation() {
        let state = TextFieldState::new("Hello");
        let node = TextFieldModifierNode::new(state);
        assert_eq!(node.text(), "Hello");
        assert!(!node.is_focused());
    }

    #[test]
    fn text_field_node_focus() {
        let state = TextFieldState::new("Test");
        let mut node = TextFieldModifierNode::new(state);
        assert!(!node.is_focused());

        node.set_focused(true);
        assert!(node.is_focused());
        assert!(node.cursor_visible);

        node.set_focused(false);
        assert!(!node.is_focused());
    }

    #[test]
    fn text_field_element_creates_node() {
        let state = TextFieldState::new("Hello World");
        let element = TextFieldElement::new(state);

        let node = element.create();
        assert_eq!(node.text(), "Hello World");
    }

    #[test]
    fn text_field_element_equality() {
        let state1 = TextFieldState::new("Hello");
        let state2 = TextFieldState::new("Hello");
        let state3 = TextFieldState::new("World");

        let elem1 = TextFieldElement::new(state1);
        let elem2 = TextFieldElement::new(state2);
        let elem3 = TextFieldElement::new(state3);

        // All TextFieldElements are equal for type-based node reuse
        // This ensures focused state is preserved across recompositions
        assert_eq!(elem1, elem2);
        assert_eq!(elem1, elem3); // Changed from assert_ne - now equal for node reuse
    }

    /// Test that cursor draw command position is calculated correctly.
    /// 
    /// This test verifies that when we measure text width for cursor position:
    /// 1. The cursor x position = width of text before cursor
    /// 2. For text at cursor end, x = full text width
    #[test]
    fn test_cursor_x_position_calculation() {
        // Test that text measurement works correctly for cursor positioning
        
        // Empty text - cursor should be at x=0
        let empty_width = crate::text::measure_text("").width;
        assert!(empty_width.abs() < 0.1, "Empty text should have 0 width, got {}", empty_width);
        
        // Non-empty text - cursor at end should be at text width
        let hi_width = crate::text::measure_text("Hi").width;
        assert!(hi_width > 0.0, "Text 'Hi' should have positive width: {}", hi_width);
        
        // Partial text - cursor after 'H' should be at width of 'H'
        let h_width = crate::text::measure_text("H").width;
        assert!(h_width > 0.0, "Text 'H' should have positive width");
        assert!(h_width < hi_width, "'H' width {} should be less than 'Hi' width {}", h_width, hi_width);
        
        // Verify TextFieldState selection tracks cursor correctly
        let state = TextFieldState::new("Hi");
        assert_eq!(state.selection().start, 2, "Cursor should be at position 2 (end of 'Hi')");
        
        // The text before cursor at position 2 in "Hi" is "Hi" itself
        let text = state.text();
        let cursor_pos = state.selection().start;
        let text_before_cursor = &text[..cursor_pos.min(text.len())];
        assert_eq!(text_before_cursor, "Hi");
        
        // So cursor x = width of "Hi"
        let cursor_x = crate::text::measure_text(text_before_cursor).width;
        assert!((cursor_x - hi_width).abs() < 0.1, 
            "Cursor x {} should equal 'Hi' width {}", cursor_x, hi_width);
    }

    /// Test cursor is created when focused node is in slices.
    #[test]  
    fn test_focused_node_creates_cursor() {
        let state = TextFieldState::new("Test");
        let element = TextFieldElement::new(state.clone());
        let mut node = element.create();
        
        // Initially not focused
        assert!(!node.is_focused());
        
        // Set focus
        *node.is_focused.borrow_mut() = true;
        assert!(node.is_focused());
        
        // Verify the node has correct text
        assert_eq!(node.text(), "Test");
        
        // Verify selection is at end
        assert_eq!(node.selection().start, 4);
    }
}
