use compose_core::{mutableStateOf, remember, MutableState};
use compose_macros::composable;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollAxis {
    Horizontal,
    Vertical,
}

/// Simple scroll position holder backed by snapshot state.
#[derive(Clone, Debug)]
pub struct ScrollState {
    id: u64,
    offset: MutableState<f32>,
    max_offset: MutableState<f32>,
}

impl ScrollState {
    /// Create a scroll state starting at the provided offset.
    pub fn new(initial: f32) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            offset: mutableStateOf(initial),
            max_offset: mutableStateOf(0.0),
        }
    }

    pub fn value(&self) -> f32 {
        self.offset.get()
    }

    pub fn max_value(&self) -> f32 {
        self.max_offset.get()
    }

    /// Update the allowed scroll extent and clamp the current position.
    pub fn update_bounds(&self, max_value: f32) {
        let clamped = max_value.max(0.0);
        self.max_offset.set(clamped);
        self.offset.update(|value| {
            if *value > clamped {
                *value = clamped;
            }
        });
    }

    /// Apply a raw delta to the scroll position, returning the consumed amount.
    pub fn dispatch_raw_delta(&self, delta: f32) -> f32 {
        let max_value = self.max_offset.get();
        self.offset.update(|value| {
            let new_value = (*value + delta).clamp(0.0, max_value.max(0.0));
            let consumed = new_value - *value;
            *value = new_value;
            consumed
        })
    }
}

#[composable]
pub fn remember_scroll_state(initial: f32) -> ScrollState {
    remember(move || ScrollState::new(initial)).with(|state| state.clone())
}

impl PartialEq for ScrollState {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ScrollState {}

impl Hash for ScrollState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_test_composition;

    #[test]
    fn dispatch_raw_delta_clamps_to_bounds() {
        run_test_composition(|| {
            let state = remember_scroll_state(0.0);
            state.update_bounds(10.0);

            assert_eq!(state.dispatch_raw_delta(5.0), 5.0);
            assert_eq!(state.value(), 5.0);

            // Exceeding max clamps to the configured bound
            assert_eq!(state.dispatch_raw_delta(10.0), 5.0);
            assert_eq!(state.value(), 10.0);

            // Negative deltas clamp at zero
            assert_eq!(state.dispatch_raw_delta(-15.0), -10.0);
            assert_eq!(state.value(), 0.0);
        });
    }
}
