use compose_testing::{ComposeTestRule, has_text};
use compose_ui::{
    LinearArrangement, Box, Modifier, Row, RowSpec, Size,
};
use compose_foundation::scroll::ScrollState;
use compose_foundation::PointerEventKind;
use std::rc::Rc;

#[test]
fn test_row_horizontal_scroll() {
    let scroll_state = Rc::new(std::cell::RefCell::new(None));
    let scroll_state_clone = scroll_state.clone();

    let mut rule = ComposeTestRule::new();
    rule.set_content(move || {
        // Create ScrollState with initial value 0
        let state = compose_core::remember(|| ScrollState::new(0));
        *scroll_state_clone.borrow_mut() = Some(state.clone());

        Row(
            Modifier::empty()
                .size(Size::new(200.0, 50.0)) // Fixed size container
                .horizontal_scroll(state.borrow().clone()),
            RowSpec::new().horizontal_arrangement(LinearArrangement::SpacedBy(10.0)),
            || {
                // Add enough children to overflow 200.0
                // 5 children * 50 width + 4 gaps * 10 = 250 + 40 = 290 width
                for i in 0..5 {
                    Box(
                        Modifier::empty().size(Size::new(50.0, 50.0)),
                        Default::default(),
                        move || {
                            compose_ui::widgets::Text(format!("Item {}", i), Modifier::empty());
                        },
                    );
                }
            },
        );
    });

    rule.await_idle();

    let state = scroll_state.borrow().as_ref().unwrap().clone();
    // Use borrow() to access ScrollState methods on Owned<ScrollState>
    assert_eq!(state.borrow().value(), 0, "Initial scroll should be 0");
    assert!(state.borrow().max_value() > 0, "Max scroll should be > 0 (was {})", state.borrow().max_value());

    // Find the first item
    let mut item = rule.on_node(has_text("Item 0"));
    
    // Get bounds to calculate scroll target
    let bounds = item.get_bounds();
    let center_x = bounds.x + bounds.width / 2.0;
    let center_y = bounds.y + bounds.height / 2.0;

    // Perform scroll gesture (drag left by 50px to scroll right)
    item.perform_touch_input(|scope| {
        scope.down(None, None); // Down at center
        scope.move_to(center_x - 50.0, center_y);
        scope.up();
    });
    
    rule.await_idle();

    assert!(state.borrow().value() > 0, "Should have scrolled (value was {})", state.borrow().value());
}
