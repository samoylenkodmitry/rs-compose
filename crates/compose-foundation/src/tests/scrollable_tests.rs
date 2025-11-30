use crate::scrollable::{Orientation, ScrollablePointerInputNode, ScrollableState};
use crate::nodes::input::types::{PointerEvent, PointerEventKind, PointerEventPass, PointerId, PointerType};
use compose_ui_graphics::{Point, Size};
use std::cell::RefCell;
use std::rc::Rc;
use crate::PointerInputNode;

// Mock ScrollableState
struct MockScrollableState {
    consumed_delta: RefCell<f32>,
    is_scrolling: RefCell<bool>,
}

impl MockScrollableState {
    fn new() -> Self {
        Self {
            consumed_delta: RefCell::new(0.0),
            is_scrolling: RefCell::new(false),
        }
    }
}

impl ScrollableState for MockScrollableState {
    fn consume_scroll_delta(&self, delta: f32) -> f32 {
        *self.consumed_delta.borrow_mut() += delta;
        *self.is_scrolling.borrow_mut() = true;
        delta
    }

    fn is_scroll_in_progress(&self) -> bool {
        *self.is_scrolling.borrow()
    }
}

// Helper to create a pointer event
fn create_pointer_event(
    kind: PointerEventKind,
    position: Point,
    down: bool,
    id: PointerId,
) -> PointerEvent {
    let previous_pressed = match kind {
        PointerEventKind::Down => false,
        PointerEventKind::Move => true,
        PointerEventKind::Up => true,
        _ => false,
    };

    let change = Rc::new(crate::nodes::input::types::PointerInputChange {
        id,
        uptime: 0,
        position,
        pressed: down,
        pressure: if down { 1.0 } else { 0.0 },
        previous_uptime: 0,
        previous_position: position, // Simplified
        previous_pressed,
        is_consumed: std::cell::Cell::new(false),
        type_: PointerType::Mouse,
        historical: vec![],
        scroll_delta: Point::ZERO,
        original_event_position: position,
    });

    PointerEvent::new(
        vec![change],
        None,
    )
}

#[test]
fn test_scrollable_horizontal_drag() {
    let state = Rc::new(MockScrollableState::new());
    let mut node = ScrollablePointerInputNode::new(state.clone(), Orientation::Horizontal, true);
    
    // 1. Down event
    let down_event = create_pointer_event(
        PointerEventKind::Down,
        Point::new(100.0, 100.0),
        true,
        0,
    );
    
    // Should consume down event
    let consumed = node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &down_event,
        PointerEventPass::Main,
        Size::new(200.0, 200.0),
    );
    
    assert!(consumed, "Should consume Down event");
    assert!(down_event.changes[0].is_consumed.get(), "Event should be marked consumed");

    // 2. Move event (drag left -> scroll right)
    // Dragging from 100 to 90 (delta -10) means content moves left, so we scroll right (+10)
    let move_event = create_pointer_event(
        PointerEventKind::Move,
        Point::new(90.0, 100.0),
        true,
        0,
    );
    // Manually set previous position to simulate drag
    // In a real scenario, the tracker would handle this, but here we construct the event manually
    // The Scrollable node calculates delta from its own last_position, so we don't strictly need 
    // previous_position in the event to be correct for the node logic, but it's good practice.

    let consumed = node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &move_event,
        PointerEventPass::Main,
        Size::new(200.0, 200.0),
    );

    assert!(consumed, "Should consume Move event");
    assert_eq!(*state.consumed_delta.borrow(), 10.0, "Should have consumed +10.0 delta");

    // 3. Up event
    let up_event = create_pointer_event(
        PointerEventKind::Up,
        Point::new(90.0, 100.0),
        false,
        0,
    );

    let consumed = node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &up_event,
        PointerEventPass::Main,
        Size::new(200.0, 200.0),
    );

    assert!(consumed, "Should consume Up event");
}

#[test]
fn test_scrollable_vertical_drag() {
    let state = Rc::new(MockScrollableState::new());
    let mut node = ScrollablePointerInputNode::new(state.clone(), Orientation::Vertical, true);
    
    // 1. Down event
    let down_event = create_pointer_event(
        PointerEventKind::Down,
        Point::new(100.0, 100.0),
        true,
        0,
    );
    node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &down_event,
        PointerEventPass::Main,
        Size::new(200.0, 200.0),
    );

    // 2. Move event (drag up -> scroll down)
    // Dragging from 100 to 80 (delta -20) means content moves up, so we scroll down (+20)
    let move_event = create_pointer_event(
        PointerEventKind::Move,
        Point::new(100.0, 80.0),
        true,
        0,
    );

    node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &move_event,
        PointerEventPass::Main,
        Size::new(200.0, 200.0),
    );

    assert_eq!(*state.consumed_delta.borrow(), 20.0, "Should have consumed +20.0 delta");
}

#[test]
fn test_scrollable_ignore_wrong_pass() {
    let state = Rc::new(MockScrollableState::new());
    let mut node = ScrollablePointerInputNode::new(state.clone(), Orientation::Horizontal, true);
    
    let down_event = create_pointer_event(
        PointerEventKind::Down,
        Point::new(100.0, 100.0),
        true,
        0,
    );
    
    // Initial pass should be ignored
    let consumed = node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &down_event,
        PointerEventPass::Initial,
        Size::new(200.0, 200.0),
    );
    
    assert!(!consumed, "Should ignore Initial pass");
}

#[test]
fn test_scrollable_disabled() {
    let state = Rc::new(MockScrollableState::new());
    let mut node = ScrollablePointerInputNode::new(state.clone(), Orientation::Horizontal, false);
    
    let down_event = create_pointer_event(
        PointerEventKind::Down,
        Point::new(100.0, 100.0),
        true,
        0,
    );
    
    let consumed = node.on_pointer_event(
        &mut crate::modifier::BasicModifierNodeContext::new(),
        &down_event,
        PointerEventPass::Main,
        Size::new(200.0, 200.0),
    );
    
    assert!(!consumed, "Should ignore events when disabled");
}
