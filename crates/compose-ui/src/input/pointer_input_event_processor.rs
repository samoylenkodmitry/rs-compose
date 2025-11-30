use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use compose_core::NodeId;
use compose_foundation::nodes::input::types::{
    InternalPointerEvent, PointerId, PointerInputChange, PointerInputEvent,
    ProcessResult,
};
use compose_ui_graphics::Point;

use crate::input::hit_path_tracker::HitPathTracker;
use crate::widgets::nodes::layout_node::LayoutNode;

/// Orchestrates pointer input processing.
pub struct PointerInputEventProcessor {
    root_id: NodeId,
    hit_path_tracker: RefCell<HitPathTracker>,
    pointer_input_change_event_producer: RefCell<PointerInputChangeEventProducer>,
    is_processing: RefCell<bool>,
}

impl PointerInputEventProcessor {
    pub fn new(root_id: NodeId) -> Self {
        Self {
            root_id,
            hit_path_tracker: RefCell::new(HitPathTracker::new()),
            pointer_input_change_event_producer: RefCell::new(PointerInputChangeEventProducer::new()),
            is_processing: RefCell::new(false),
        }
    }

    pub fn process(&self, event: PointerInputEvent) -> ProcessResult {
        println!("[Processor] Processing on thread {:?}", std::thread::current().id());
        if *self.is_processing.borrow() {
            // Re-entrancy not supported
            return ProcessResult::default();
        }

        *self.is_processing.borrow_mut() = true;

        // Produce internal event (deltas)
        let internal_event = self
            .pointer_input_change_event_producer
            .borrow_mut()
            .produce(event);

        let mut is_hover = true;
        for change in internal_event.changes.values() {
            if change.pressed || change.previous_pressed {
                is_hover = false;
                break;
            }
        }

        // Hit testing for new down events
        // We need to access the root node from the registry
        use crate::widgets::nodes::layout_node::LAYOUT_NODE_REGISTRY;
        
        LAYOUT_NODE_REGISTRY.with(|registry| {
            let registry = registry.borrow();
            if let Some(entry) = registry.get(&self.root_id) {
                // SAFETY: Node is alive if in registry and will be valid for duration of borrow
                let root_node = unsafe { &*(entry.node as *const crate::widgets::nodes::layout_node::LayoutNode) };
                
                for change in internal_event.changes.values() {
                    if is_hover || change.changed_to_down_ignore_consumed() {
                        println!("[Processor] Performing hit test for change {:?} at {:?}", change.id, change.position);
                        let mut hit_result = crate::layout::hit_test_result::HitTestResult::new();
                        root_node.hit_test(change.position, &mut hit_result, None);
                        
                        println!("[Processor] Hit test result: {} nodes", hit_result.nodes.len());
                        
                        if !hit_result.is_empty() {
                            let nodes: Vec<compose_core::NodeId> = hit_result.nodes.iter().map(|e| e.node_id).collect();
                            self.hit_path_tracker.borrow_mut().add_hit_path(
                                change.id,
                                nodes,
                            );
                        }
                    }
                }
            } else {
                println!("[Processor] Root node #{} not found in registry!", self.root_id);
            }
        });

        // Dispatch changes with a callback that handles the actual LayoutNode dispatch
        // This is the only place we need unsafe - to access the registry
        
        let dispatched = LAYOUT_NODE_REGISTRY.with(|registry| {
            let registry_borrow = registry.borrow();
            
            // Dispatch callback that accesses LayoutNodes from the registry
            let dispatch_fn = |node_id: NodeId, event: &compose_foundation::PointerEvent, pass: compose_foundation::nodes::input::types::PointerEventPass| {
                println!("[Processor] Dispatching to node #{} pass {:?}", node_id, pass);
                if let Some(entry) = registry_borrow.get(&node_id) {
                    // SAFETY: Node is alive if in registry and will be valid for duration of borrow
                    // The registry ensures nodes are unregistered in their Drop impl
                    let layout_node = unsafe { &mut *(entry.node as *mut crate::widgets::nodes::layout_node::LayoutNode) };
                    layout_node.dispatch_pointer_event(event, pass);
                }
            };
            
            self.hit_path_tracker
                .borrow_mut()
                .dispatch_changes(&internal_event, dispatch_fn)
        });

        let any_movement_consumed = if internal_event.suppress_movement_consumption {
            false
        } else {
            internal_event
                .changes
                .values()
                .any(|c| c.position_changed_ignore_consumed() && c.is_consumed())
        };

        let any_change_consumed = internal_event.changes.values().any(|c| c.is_consumed());

        *self.is_processing.borrow_mut() = false;

        ProcessResult::new(dispatched, any_movement_consumed, any_change_consumed)
    }

    pub fn process_cancel(&self) {
        if !*self.is_processing.borrow() {
            self.pointer_input_change_event_producer.borrow_mut().clear();
            // self.hit_path_tracker.borrow_mut().process_cancel(); // TODO: Implement process_cancel in HitPathTracker
        }
    }
}

struct PointerInputChangeEventProducer {
    previous_pointer_input_data: HashMap<PointerId, PointerInputData>,
}

struct PointerInputData {
    uptime: u64,
    position_on_screen: Point,
    down: bool,
}

impl PointerInputChangeEventProducer {
    fn new() -> Self {
        Self {
            previous_pointer_input_data: HashMap::new(),
        }
    }

    fn produce(&mut self, event: PointerInputEvent) -> InternalPointerEvent {
        let mut changes = HashMap::new();

        for pointer in &event.pointers {
            let (previous_time, previous_position, previous_down) =
                if let Some(prev) = self.previous_pointer_input_data.get(&pointer.id) {
                    (prev.uptime, prev.position_on_screen, prev.down) // Note: Using screen pos as local for now, assuming root is at 0,0
                } else {
                    (pointer.uptime, pointer.position, false)
                };

            let change = Rc::new(PointerInputChange {
                id: pointer.id,
                uptime: pointer.uptime,
                position: pointer.position,
                pressed: pointer.down,
                pressure: pointer.pressure,
                previous_uptime: previous_time,
                previous_position,
                previous_pressed: previous_down,
                is_consumed: std::cell::Cell::new(false),
                type_: pointer.type_,
                historical: pointer.historical.clone(),
                scroll_delta: pointer.scroll_delta,
                original_event_position: pointer.original_event_position,
            });

            changes.insert(pointer.id, change);

            if pointer.down {
                self.previous_pointer_input_data.insert(
                    pointer.id,
                    PointerInputData {
                        uptime: pointer.uptime,
                        position_on_screen: pointer.position_on_screen,
                        down: pointer.down,
                    },
                );
            } else {
                self.previous_pointer_input_data.remove(&pointer.id);
            }
        }

        InternalPointerEvent {
            changes,
            pointer_input_event: event,
            suppress_movement_consumption: false,
        }
    }

    fn clear(&mut self) {
        self.previous_pointer_input_data.clear();
    }
}
