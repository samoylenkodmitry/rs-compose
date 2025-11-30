use compose_ui_graphics::Point;
use std::collections::HashMap;
use std::rc::Rc;

pub type PointerId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerType {
    Mouse,
    Touch,
    Stylus,
    Eraser,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerPhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerEventKind {
    Down,
    Move,
    Up,
    Cancel,
    Enter,
    Exit,
    Unknown,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    Primary = 0,
    Secondary = 1,
    Middle = 2,
    Back = 3,
    Forward = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerButtons(u8);

impl PointerButtons {
    pub const NONE: Self = Self(0);

    pub fn new() -> Self {
        Self::NONE
    }

    pub fn with(mut self, button: PointerButton) -> Self {
        self.insert(button);
        self
    }

    pub fn insert(&mut self, button: PointerButton) {
        self.0 |= 1 << (button as u8);
    }

    pub fn remove(&mut self, button: PointerButton) {
        self.0 &= !(1 << (button as u8));
    }

    pub fn contains(&self, button: PointerButton) -> bool {
        (self.0 & (1 << (button as u8))) != 0
    }
}

impl Default for PointerButtons {
    fn default() -> Self {
        Self::NONE
    }
}

/// Data that describes a particular pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerInputEventData {
    pub id: PointerId,
    pub uptime: u64,
    pub position_on_screen: Point,
    pub position: Point,
    pub down: bool,
    pub pressure: f32,
    pub type_: PointerType,
    pub active_hover: bool,
    pub historical: Vec<HistoricalChange>,
    pub scroll_delta: Point,
    pub original_event_position: Point,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HistoricalChange {
    pub uptime: u64,
    pub position: Point,
    pub original_event_position: Point,
}

/// The normalized data structure for pointer input event information.
#[derive(Clone, Debug)]
pub struct PointerInputEvent {
    pub uptime: u64,
    pub pointers: Vec<PointerInputEventData>,
    pub motion_event: Option<Rc<dyn std::any::Any>>, // Opaque platform event
}

impl PartialEq for PointerInputEvent {
    fn eq(&self, other: &Self) -> bool {
        self.uptime == other.uptime && self.pointers == other.pointers
    }
}

impl PointerInputEvent {
    pub fn new(uptime: u64, pointers: Vec<PointerInputEventData>) -> Self {
        Self {
            uptime,
            pointers,
            motion_event: None,
        }
    }
}

/// Represents a pointer input event internally.
#[derive(Clone, Debug)]
pub struct InternalPointerEvent {
    pub changes: HashMap<PointerId, Rc<PointerInputChange>>,
    pub pointer_input_event: PointerInputEvent,
    pub suppress_movement_consumption: bool,
}

/// Describes a change in a pointer.
#[derive(Clone, Debug)]
pub struct PointerInputChange {
    pub id: PointerId,
    pub uptime: u64,
    pub position: Point,
    pub pressed: bool,
    pub pressure: f32,
    pub previous_uptime: u64,
    pub previous_position: Point,
    pub previous_pressed: bool,
    pub is_consumed: std::cell::Cell<bool>,
    pub type_: PointerType,
    pub historical: Vec<HistoricalChange>,
    pub scroll_delta: Point,
    pub original_event_position: Point,
}

impl PointerInputChange {
    pub fn is_consumed(&self) -> bool {
        self.is_consumed.get()
    }

    pub fn consume(&self) {
        self.is_consumed.set(true);
    }

    pub fn changed_to_down(&self) -> bool {
        !self.is_consumed() && !self.previous_pressed && self.pressed
    }

    pub fn changed_to_down_ignore_consumed(&self) -> bool {
        !self.previous_pressed && self.pressed
    }

    pub fn changed_to_up(&self) -> bool {
        !self.is_consumed() && self.previous_pressed && !self.pressed
    }

    pub fn changed_to_up_ignore_consumed(&self) -> bool {
        self.previous_pressed && !self.pressed
    }

    pub fn position_changed(&self) -> bool {
        self.position_change_internal(false) != Point::ZERO
    }

    pub fn position_changed_ignore_consumed(&self) -> bool {
        self.position_change_internal(true) != Point::ZERO
    }

    fn position_change_internal(&self, ignore_consumed: bool) -> Point {
        let offset = self.position - self.previous_position;
        if !ignore_consumed && self.is_consumed() {
            Point::ZERO
        } else {
            offset
        }
    }
}

/// Public PointerEvent exposed to modifiers.
#[derive(Clone, Debug)]
pub struct PointerEvent {
    pub changes: Vec<Rc<PointerInputChange>>,
    pub internal_pointer_event: Option<Rc<InternalPointerEvent>>,
}

impl PointerEvent {
    pub fn new(changes: Vec<Rc<PointerInputChange>>, internal: Option<Rc<InternalPointerEvent>>) -> Self {
        Self {
            changes,
            internal_pointer_event: internal,
        }
    }

    /// Helper to get the main pointer change (usually the first one).
    /// This is useful for simple single-pointer scenarios.
    pub fn main_pointer(&self) -> Option<&PointerInputChange> {
        self.changes.first().map(|c| c.as_ref())
    }

    pub fn kind(&self) -> PointerEventKind {
        // Map change state to kind roughly
        if let Some(change) = self.main_pointer() {
            if change.changed_to_down() {
                PointerEventKind::Down
            } else if change.changed_to_up() {
                PointerEventKind::Up
            } else {
                PointerEventKind::Move
            }
        } else {
            PointerEventKind::Unknown
        }
    }

    pub fn position(&self) -> Point {
        self.main_pointer().map(|c| c.position).unwrap_or(Point::ZERO)
    }

    pub fn is_consumed(&self) -> bool {
        self.changes.iter().any(|c| c.is_consumed())
    }

    pub fn consume(&self) {
        for change in &self.changes {
            change.consume();
        }
    }
    
    pub fn id(&self) -> PointerId {
        self.main_pointer().map(|c| c.id).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessResult {
    pub dispatched_to_a_pointer_input_modifier: bool,
    pub any_movement_consumed: bool,
    pub any_change_consumed: bool,
}

impl ProcessResult {
    pub const fn new(
        dispatched: bool,
        movement_consumed: bool,
        change_consumed: bool,
    ) -> Self {
        Self {
            dispatched_to_a_pointer_input_modifier: dispatched,
            any_movement_consumed: movement_consumed,
            any_change_consumed: change_consumed,
        }
    }
}

impl Default for ProcessResult {
    fn default() -> Self {
        Self::new(false, false, false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerEventPass {
    Initial,
    Main,
    Final,
}
