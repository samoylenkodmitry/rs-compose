use compose_foundation::{
    nodes::input::types::{PointerInputChange, PointerId, PointerType},
    PointerButtons, PointerEvent, PointerEventKind, PointerPhase,
};
use compose_ui_graphics::Point;
use winit::dpi::PhysicalPosition;
use std::rc::Rc;
use std::cell::Cell;

pub struct DesktopWinitPlatform {
    scale_factor: f64,
}

impl DesktopWinitPlatform {
    pub fn new(scale_factor: f64) -> Self {
        Self { scale_factor }
    }

    pub fn set_scale_factor(&mut self, factor: f64) {
        self.scale_factor = factor;
    }

    pub fn get_scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn pointer_position(&self, position: PhysicalPosition<f64>) -> Point {
        Point {
            x: (position.x / self.scale_factor) as f32,
            y: (position.y / self.scale_factor) as f32,
        }
    }

    pub fn pointer_event(
        &self,
        kind: PointerEventKind,
        position: PhysicalPosition<f64>,
    ) -> PointerEvent {
        let logical = self.pointer_position(position);
        
        // Determine pressed state based on kind
        let (pressed, previous_pressed) = match kind {
            PointerEventKind::Down => (true, false),
            PointerEventKind::Move => (true, true), // Assumption: move happens while pressed for drag
            PointerEventKind::Up => (false, true),
            _ => (false, false),
        };

        let change = Rc::new(PointerInputChange {
            id: 0, // Single pointer for now
            uptime: 0, // TODO: Pass timestamp
            position: logical,
            pressed,
            pressure: if pressed { 1.0 } else { 0.0 },
            previous_uptime: 0,
            previous_position: logical, // TODO: Track previous position
            previous_pressed,
            is_consumed: Cell::new(false),
            type_: PointerType::Mouse,
            historical: vec![],
            scroll_delta: Point::ZERO,
            original_event_position: logical,
        });

        PointerEvent::new(vec![change], None)
    }
}

impl Default for DesktopWinitPlatform {
    fn default() -> Self {
        Self::new(1.0)
    }
}
