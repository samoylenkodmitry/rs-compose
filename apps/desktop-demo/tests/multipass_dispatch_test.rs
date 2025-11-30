//! Robot test for multi-pass pointer event dispatch verification

use compose_testing::{ComposeTestRule, SemanticsMatcher};
use compose_ui::widgets::{Box, Text};
use compose_ui::Modifier;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_multi_pass_handler_receives_events() {
    // Track that handlers receive events
    let received = Rc::new(RefCell::new(false));
    
    let rule = ComposeTestRule::new({
        let received = received.clone();
        move || {
            let received = received.clone();
            Box(
                Modifier::empty()
                    .clickable(move || {
                        *received.borrow_mut() = true;
                    }),
                || {
                    Text("Clickable Text");
                },
            );
        }
    });
    
    // Find and click the text
    let node = rule.find(SemanticsMatcher::text("Clickable Text"));
    node.perform_click();
    
    rule.await_idle();
    
    // Verify the handler was triggered
    assert!(
        *received.borrow(),
        "Handler should receive click event through multi-pass dispatch"
    );
}

#[test]
fn test_existing_scroll_still_works() {
    // This verifies that our multi-pass changes don't break existing functionality
    use compose_foundation::ScrollState;
    use compose_ui::widgets::Column;
    
    let scroll_state = ScrollState::new();
    let scroll_clone = scroll_state.clone();
    
    let rule = ComposeTestRule::new(move || {
        Column(
            Modifier::empty().vertical_scroll(scroll_clone.clone()),
            || {
                for i in 0..20 {
                    Text(format!("Item {}", i));
                }
            },
        );
    });
    
    let initial_scroll = scroll_state.value();
    
    // Find the scrollable content
    let list = rule.find(SemanticsMatcher::text("Item 0"));
    
    // Perform scroll gesture (swipe)
    list.perform_scroll_by(0.0, -100.0);
    
    rule.await_idle();
    
    // Scroll position should have changed
    assert_ne!(
        scroll_state.value(),
        initial_scroll,
        "Scroll should work with multi-pass dispatch"
    );
}

