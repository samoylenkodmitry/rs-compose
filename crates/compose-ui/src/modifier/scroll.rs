use super::{inspector_metadata, Modifier};
use crate::modifier_nodes::ScrollElement;
use crate::scroll::{ScrollAxis, ScrollState};

impl Modifier {
    /// Make content scrollable in the given axis using the provided [ScrollState].
    ///
    /// This attaches both a layout modifier that offsets the content based on the
    /// scroll state and a pointer input handler that updates the state from drag
    /// gestures.
    pub fn scrollable(self, state: ScrollState, axis: ScrollAxis) -> Self {
        let inspector_state = state.clone();
        let modifier = Modifier::with_element(ScrollElement::new(state, axis))
            .with_inspector_metadata(inspector_metadata("scrollable", move |info| {
                info.add_property(
                    "axis",
                    match axis {
                        ScrollAxis::Horizontal => "Horizontal",
                        ScrollAxis::Vertical => "Vertical",
                    },
                );
                info.add_property("scrollOffset", format!("{:.2}", inspector_state.value()));
            }))
            .clip_to_bounds();

        self.then(modifier)
    }
}
