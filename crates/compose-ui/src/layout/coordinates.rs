//! Layout Coordinates
//!
//! Tracks position, size, and relationships for hit testing and coordinate transformations.
//! Follows Jetpack Compose's LayoutCoordinates model.

use crate::{Point, Size};
use std::rc::Rc;

/// Coordinates of a layout node within its parent and the composition.
///
/// This is similar to Jetpack Compose's LayoutCoordinates, tracking:
/// - Size of the node
/// - Position relative to parent  
/// - Global position for hit testing
#[derive(Debug, Clone)]
pub struct LayoutCoordinates {
    /// Size of this layout
    pub size: Size,
    
    /// Position relative to parent (set during placement)
    pub position_in_parent: Point,
    
    /// Global position (computed by traversing parent chain)
    /// This is lazily computed when needed for hit testing
    global_position: Rc<std::cell::RefCell<Option<Point>>>,
}

impl LayoutCoordinates {
    /// Creates new coordinates with the given size
    pub fn new(size: Size) -> Self {
        Self {
            size,
            position_in_parent: Point::new(0.0, 0.0),
            global_position: Rc::new(std::cell::RefCell::new(None)),
        }
    }
    
    /// Updates the position within parent (called during placement)
    pub fn set_position(&mut self, position: Point) {
        self.position_in_parent = position;
        // Invalidate cached global position
        *self.global_position.borrow_mut() = None;
    }
    
    /// Checks if a point (in global coordinates) is within this layout's bounds
    pub fn contains_global(&self, global_point: Point, parent_global: Option<Point>) -> bool {
        // Compute this node's global position
        let node_global = match parent_global {
            Some(parent) => Point::new(
                parent.x + self.position_in_parent.x,
                parent.y + self.position_in_parent.y,
            ),
            None => self.position_in_parent, // Root node
        };
        
        // Check if point is within bounds
        global_point.x >= node_global.x
            && global_point.x <= node_global.x + self.size.width
            && global_point.y >= node_global.y
            && global_point.y <= node_global.y + self.size.height
    }
    
    /// Converts a global point to local coordinates
    pub fn global_to_local(&self, global_point: Point, parent_global: Option<Point>) -> Point {
        let node_global = match parent_global {
            Some(parent) => Point::new(
                parent.x + self.position_in_parent.x,
                parent.y + self.position_in_parent.y,
            ),
            None => self.position_in_parent,
        };
        
        Point::new(
            global_point.x - node_global.x,
            global_point.y - node_global.y,
        )
    }
}

impl Default for LayoutCoordinates {
    fn default() -> Self {
        Self::new(Size { width: 0.0, height: 0.0 })
    }
}
