//! Integration tests for multi-pass pointer event dispatch

use compose_foundation::nodes::input::types::{PointerEvent, PointerEventKind};
use compose_ui_graphics::Point;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_simple_multi_pass_dispatch() {
    // Track how many times handler is called
    let call_count = Rc::new(RefCell::new(0));
    
    let call_count_clone = call_count.clone();
    let handler: Rc<dyn Fn(PointerEvent)> = Rc::new(move |_event| {
        *call_count_clone.borrow_mut() += 1;
    });
    
    let event = PointerEvent::new(vec![], None);
    
    // Simulate multi-pass dispatch (3 calls)
    handler(event.clone());
    handler(event.clone());
    handler(event.clone());
    
    // Verify handler was called 3 times (once per pass)
    assert_eq!(*call_count.borrow(), 3, "Handler should be called 3 times for multi-pass dispatch");
}

#[test]
fn test_consumption_persists_across_handler_calls() {
    let consumed_count = Rc::new(RefCell::new(0));
    
    let consumed_count_clone = consumed_count.clone();
    let handler: Rc<dyn Fn(PointerEvent)> = Rc::new(move |event| {
        if !event.is_consumed() {
            event.consume();
            *consumed_count_clone.borrow_mut() += 1;
        }
    });
    
    let event = PointerEvent::new(vec![], None);
    
    // Simulate multi-pass dispatch
    handler(event.clone());  // First call: should consume
    handler(event.clone());  // Second call: already consumed
    handler(event.clone());  // Third call: already consumed
    
    // Verify consumption only happened once (in first call)
    assert_eq!(*consumed_count.borrow(), 1, "Event should only be consumed once");
    assert!(event.is_consumed(), "Event should remain consumed");
}

#[test]
fn test_multiple_handlers_with_consumption() {
    let handler1_calls = Rc::new(RefCell::new(0));
    let handler2_calls = Rc::new(RefCell::new(0));
    
    let handler1_calls_clone = handler1_calls.clone();
    let handler1: Rc<dyn Fn(PointerEvent)> = Rc::new(move |event| {
        *handler1_calls_clone.borrow_mut() += 1;
        // First handler consumes on first call
        if *handler1_calls_clone.borrow() == 1 {
            event.consume();
        }
    });
    
    let handler2_calls_clone = handler2_calls.clone();
    let handler2: Rc<dyn Fn(PointerEvent)> = Rc::new(move |event| {
        if !event.is_consumed() {
            *handler2_calls_clone.borrow_mut() += 1;
        }
    });
    
    let event = PointerEvent::new(vec![], None);
    
    // Simulate dispatch order: handler1 three times, then handler2 three times
    handler1(event.clone());
    handler1(event.clone());
    handler1(event.clone());
    
    handler2(event.clone());
    handler2(event.clone());
    handler2(event.clone());
    
    assert_eq!(*handler1_calls.borrow(), 3, "Handler1 called 3 times");
    // Handler2 should not increment because event was consumed by handler1
    assert_eq!(*handler2_calls.borrow(), 0, "Handler2 should not process consumed event");
}
