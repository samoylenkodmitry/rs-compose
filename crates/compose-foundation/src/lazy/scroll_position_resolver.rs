//! Scroll position resolution for lazy list measurement.
//!
//! This module handles scroll position calculation and offset normalization,
//! including jump optimization for large scrolls.

use super::lazy_list_measure::LazyListMeasureConfig;
use super::lazy_list_state::LazyListState;

/// Resolves and normalizes scroll position for lazy list measurement.
///
/// Handles:
/// - Consuming pending scroll-to-item requests
/// - Applying scroll deltas
/// - Jump optimization for large backward/forward scrolls
/// - Offset normalization across item boundaries
pub struct ScrollPositionResolver<'a> {
    state: &'a LazyListState,
    config: &'a LazyListMeasureConfig,
    items_count: usize,
    effective_viewport_size: f32,
}

/// Result of scroll position resolution.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedScrollPosition {
    /// Index of the first visible item.
    pub first_item_index: usize,
    /// Scroll offset within the first visible item.
    pub first_item_scroll_offset: f32,
}

impl<'a> ScrollPositionResolver<'a> {
    /// Creates a new ScrollPositionResolver.
    pub fn new(
        state: &'a LazyListState,
        config: &'a LazyListMeasureConfig,
        items_count: usize,
        effective_viewport_size: f32,
    ) -> Self {
        Self {
            state,
            config,
            items_count,
            effective_viewport_size,
        }
    }

    /// Resolves the scroll position from state and pending requests, then normalizes it.
    ///
    /// This is the main entry point that:
    /// 1. Gets initial position from scroll-to request or current state
    /// 2. Applies pending scroll delta
    /// 3. Normalizes backward scroll offsets
    /// 4. Normalizes forward scroll offsets
    pub fn resolve_and_normalize(&self) -> ResolvedScrollPosition {
        // Get initial position from pending request or current state
        let (mut index, mut offset) = self.get_initial_position();

        // Apply pending scroll delta
        let scroll_delta = self.state.consume_scroll_delta();
        offset -= scroll_delta; // Negate: drag down (-delta) => increase offset

        // Normalize backward scroll (negative offset)
        (index, offset) = self.normalize_backward(index, offset);

        // Clamp to valid range
        index = index.min(self.items_count.saturating_sub(1));
        offset = offset.max(0.0);

        // Normalize forward scroll (large positive offset)
        (index, offset) = self.normalize_forward(index, offset);

        ResolvedScrollPosition {
            first_item_index: index,
            first_item_scroll_offset: offset,
        }
    }

    /// Gets initial position from pending scroll-to request or current state.
    fn get_initial_position(&self) -> (usize, f32) {
        if let Some((target_index, target_offset)) = self.state.consume_scroll_to_index() {
            let clamped = target_index.min(self.items_count.saturating_sub(1));
            (clamped, target_offset)
        } else {
            (
                self.state
                    .first_visible_item_index()
                    .min(self.items_count.saturating_sub(1)),
                self.state.first_visible_item_scroll_offset(),
            )
        }
    }

    /// Normalizes backward scroll offset using jump optimization.
    ///
    /// When scrolling backward by more than a viewport, uses jump optimization
    /// to avoid O(n) loop for large flings.
    fn normalize_backward(&self, mut index: usize, mut offset: f32) -> (usize, f32) {
        if offset >= 0.0 || index == 0 {
            return (index, offset);
        }

        let average_size = self.state.average_item_size();

        // Jump optimization for large backward scrolls
        if average_size > 0.0 && offset < -self.effective_viewport_size {
            let pixels_to_jump = (-offset) - self.effective_viewport_size;
            let items_to_jump =
                (pixels_to_jump / (average_size + self.config.spacing)).floor() as usize;

            if items_to_jump > 0 {
                let actual_jump = items_to_jump.min(index);
                if actual_jump > 0 {
                    index -= actual_jump;
                    offset += actual_jump as f32 * (average_size + self.config.spacing);
                }
            }
        }

        // Fine-tune one item at a time for remaining offset
        while offset < 0.0 && index > 0 {
            index -= 1;
            let estimated_size = self
                .state
                .get_cached_size(index)
                .unwrap_or_else(|| self.state.average_item_size());
            offset += estimated_size + self.config.spacing;
        }

        (index, offset)
    }

    /// Normalizes forward scroll offset by skipping items.
    ///
    /// When offset is large, estimates items to skip to avoid measuring
    /// items that won't be visible.
    fn normalize_forward(&self, mut index: usize, mut offset: f32) -> (usize, f32) {
        if offset <= 0.0 {
            return (index, offset);
        }

        let average_size = self.state.average_item_size();
        if average_size <= 0.0 {
            return (index, offset);
        }

        // Keep a buffer to avoid over-skipping due to size variance
        let buffer_pixels = self.effective_viewport_size;
        if offset > buffer_pixels {
            let pixels_to_skip = offset - buffer_pixels;
            // Include spacing in jump calculation to match backward path logic
            // Without this, index/offset drifts on large forward scrolls when spacing > 0
            let item_size_with_spacing = average_size + self.config.spacing;
            let items_to_skip = (pixels_to_skip / item_size_with_spacing).floor() as usize;

            if items_to_skip > 0 {
                let max_skip = self.items_count.saturating_sub(1).saturating_sub(index);
                let actual_skip = items_to_skip.min(max_skip);

                if actual_skip > 0 {
                    index += actual_skip;
                    offset -= actual_skip as f32 * item_size_with_spacing;
                }
            }
        }

        (index, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::lazy_list_state::test_helpers::{
        new_lazy_list_state, new_lazy_list_state_with_position, with_test_runtime,
    };

    #[test]
    fn test_resolve_from_default_state() {
        with_test_runtime(|| {
            let state = new_lazy_list_state();
            let config = LazyListMeasureConfig::default();
            let resolver = ScrollPositionResolver::new(&state, &config, 100, 500.0);

            let pos = resolver.resolve_and_normalize();
            assert_eq!(pos.first_item_index, 0);
            assert_eq!(pos.first_item_scroll_offset, 0.0);
        });
    }

    #[test]
    fn test_resolve_with_initial_position() {
        with_test_runtime(|| {
            let state = new_lazy_list_state_with_position(5, 25.0);
            let config = LazyListMeasureConfig::default();
            let resolver = ScrollPositionResolver::new(&state, &config, 100, 500.0);

            let pos = resolver.resolve_and_normalize();
            assert_eq!(pos.first_item_index, 5);
            assert_eq!(pos.first_item_scroll_offset, 25.0);
        });
    }

    #[test]
    fn test_resolve_clamps_beyond_items_count() {
        with_test_runtime(|| {
            let state = new_lazy_list_state_with_position(50, 0.0);
            let config = LazyListMeasureConfig::default();
            // Only 10 items, but positioned at 50
            let resolver = ScrollPositionResolver::new(&state, &config, 10, 500.0);

            let pos = resolver.resolve_and_normalize();
            assert_eq!(pos.first_item_index, 9); // Clamped to last item
        });
    }

    #[test]
    fn test_scroll_to_request() {
        with_test_runtime(|| {
            let state = new_lazy_list_state();
            state.scroll_to_item(10, 15.0);
            let config = LazyListMeasureConfig::default();
            let resolver = ScrollPositionResolver::new(&state, &config, 100, 500.0);

            let pos = resolver.resolve_and_normalize();
            assert_eq!(pos.first_item_index, 10);
            assert_eq!(pos.first_item_scroll_offset, 15.0);
        });
    }
}
