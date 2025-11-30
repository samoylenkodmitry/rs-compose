    /// Performs hit testing at the given position
    ///
    /// This traverses the layout tree recursively, checking if the position
    /// is within bounds, and collects pointer input modifier nodes.
    ///
    /// Following Jetpack Compose's NodeCoordinator.hitTest pattern.
    pub fn hit_test(
        &self,
        position: crate::Point,
        result: &mut crate::layout::hit_test_result::HitTestResult,
        applier: &compose_core::MemoryApplier,
        parent_global: Option<crate::Point>,
    ) {
        // Check if position is within this node's bounds
        let coords = self.coordinates.borrow();
        if !coords.contains_global(position, parent_global) {
            // Miss - position outside bounds
            return;
        }
        
        // Compute this node's global position for children
        let node_global = match parent_global {
            Some(parent) => crate::Point::new(
                parent.x + coords.position_in_parent.x,
                parent.y + coords.position_in_parent.y,
            ),
            None => coords.position_in_parent,
        };
        
        // Hit! Add this node if it has pointer input
        if self.has_pointer_input_modifier_nodes() {
            if let Some(id) = self.id.get() {
                result.add(id, true); // in_layer = true for actual hits
            }
        }
        
        // Recursively test children (back to front for proper z-order)
        for child_id in self.children.iter().rev() {
            if let Ok(()) = applier.with_node::<Self, _>(*child_id, |child_node| {
                child_node.hit_test(position, result, applier, Some(node_global));
            }) {
                // Child tested successfully
            }
        }
    }
    
    /// Updates the layout coordinates after measurement
    pub fn update_coordinates(&self, size: crate::Size, position: crate::Point) {
        let mut coords = self.coordinates.borrow_mut();
        coords.size = size;
        coords.set_position(position);
    }
