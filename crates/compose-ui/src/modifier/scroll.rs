//! Scroll modifier extensions for Modifier.

use super::{inspector_metadata, Modifier};
use crate::scroll::{ScrollElement, ScrollState};


impl Modifier {
    /// Creates a horizontally scrollable modifier.
    ///
    /// # Arguments
    /// * `state` - The ScrollState to control scroll position
    /// * `reverse_scrolling` - If true, reverses the scroll direction in layout.
    ///   Note: This affects how scroll offset is applied to content (via `ScrollNode`),
    ///   NOT the drag direction. Drag gestures always follow natural touch semantics:
    ///   drag right = scroll left (content moves right under finger).
    ///
    /// # Example
    /// ```ignore
    /// let scroll_state = ScrollState::new(0.0);
    /// Row(
    ///     Modifier::empty().horizontal_scroll(scroll_state, false),
    ///     // ... content
    /// );
    /// ```
    pub fn horizontal_scroll(self, state: ScrollState, reverse_scrolling: bool) -> Self {
        self.then(scroll_impl(state, false, reverse_scrolling))
    }

    /// Creates a vertically scrollable modifier.
    ///
    /// # Arguments
    /// * `state` - The ScrollState to control scroll position
    /// * `reverse_scrolling` - If true, reverses the scroll direction in layout.
    ///   Note: This affects how scroll offset is applied to content (via `ScrollNode`),
    ///   NOT the drag direction. Drag gestures always follow natural touch semantics:
    ///   drag down = scroll up (content moves down under finger).
    pub fn vertical_scroll(self, state: ScrollState, reverse_scrolling: bool) -> Self {
        self.then(scroll_impl(state, true, reverse_scrolling))
    }
}

fn scroll_impl(state: ScrollState, is_vertical: bool, reverse_scrolling: bool) -> Modifier {
    // Add pointer input for drag handling FIRST
    let scroll_state = state.clone();
    let key = (state.id(), is_vertical);
    let pointer_input = Modifier::empty().pointer_input(
        key,
        move |scope| {
            let state = scroll_state.clone();
            async move {
                scope
                    .await_pointer_event_scope(|await_scope| async move {
                        loop {
                            let event = await_scope.await_pointer_event().await;
                            
                            // Read current drag state from persistent storage
                            let down_position = state.drag_down_position();
                            let last_position = state.drag_last_position();
                            let is_dragging = state.is_dragging();

                            match event.kind {
                                crate::modifier::PointerEventKind::Down => {
                                    state.set_drag_down_position(Some(event.position));
                                    state.set_drag_last_position(Some(event.position));
                                    state.set_is_dragging(false);
                                }
                                crate::modifier::PointerEventKind::Move => {
                                    // Safety check: if no button is pressed but we still think we're tracking
                                    // a drag, it means we missed the Up event (e.g., due to hit test at
                                    // cursor position instead of down position). Clear state.
                                    use compose_foundation::PointerButton;
                                    if !event.buttons.contains(PointerButton::Primary) && down_position.is_some() {
                                        state.set_drag_down_position(None);
                                        state.set_drag_last_position(None);
                                        state.set_is_dragging(false);
                                        continue;  // Skip rest of Move handling
                                    }
                                    
                                    if let (Some(down_pos), Some(last_pos)) = (down_position, last_position) {
                                        // Calculate total distance from down position
                                        let total_delta = if is_vertical {
                                            event.position.y - down_pos.y
                                        } else {
                                            event.position.x - down_pos.x
                                        };

                                        // Calculate incremental delta from last position
                                        let delta = if is_vertical {
                                            event.position.y - last_pos.y
                                        } else {
                                            event.position.x - last_pos.x
                                        };

                                        // Only start dragging if we've moved a minimum distance from down position
                                        // Using shared DRAG_THRESHOLD to stay consistent with clickable modifier
                                        if !is_dragging && total_delta.abs() > compose_foundation::DRAG_THRESHOLD {
                                            state.set_is_dragging(true);
                                            // Consume the event to prevent child buttons from handling it
                                            // This implements gesture disambiguation - scroll wins over clicks
                                            event.consume();
                                        }

                                        // Re-read is_dragging since we may have just set it
                                        let is_dragging = state.is_dragging();
                                        if is_dragging {
                                            // Negative because dragging down/right = scroll up/left
                                            let _ = state.dispatch_raw_delta(-delta);
                                            // Continue consuming events while dragging
                                            event.consume();
                                        }


                                        state.set_drag_last_position(Some(event.position));
                                    }
                                }
                                crate::modifier::PointerEventKind::Up
                                | crate::modifier::PointerEventKind::Cancel => {
                                    if is_dragging {
                                        event.consume();
                                    }
                                    state.set_drag_down_position(None);
                                    state.set_drag_last_position(None);
                                    state.set_is_dragging(false);
                                }
                            }
                        }
                    })
                    .await;
            }
        },
    );

    // Create the layout modifier (ScrollNode) AFTER
    let element = ScrollElement::new(state.clone(), is_vertical, reverse_scrolling);
    let layout_modifier = Modifier::with_element(element).with_inspector_metadata(
        inspector_metadata(
            if is_vertical {
                "verticalScroll"
            } else {
                "horizontalScroll"
            },
            move |info| {
                info.add_property("isVertical", is_vertical.to_string());
                info.add_property("reverseScrolling", reverse_scrolling.to_string());
            },
        ),
    );

    // Combine: pointer input THEN layout modifier
    // This way pointer input is earlier in the modifier chain
    pointer_input.then(layout_modifier)
}
