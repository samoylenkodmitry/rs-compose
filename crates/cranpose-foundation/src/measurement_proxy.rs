//! Public measurement proxy API for layout modifier nodes.
//!
//! This module provides the extensibility mechanism for layout modifiers to supply
//! their own measurement proxies. This enables custom layout modifiers to work with
//! the coordinator chain while respecting Rust's borrow checker constraints.
//!
//! # Architecture
//!
//! In Jetpack Compose (Kotlin), coordinators can hold direct references to live
//! modifier nodes. In Rust, we need to work around the borrow checker by extracting
//! a measurement proxy that captures the node's state at the time of measurement.
//!
//! Each `LayoutModifierNode` can optionally provide a `MeasurementProxy` via the
//! `create_measurement_proxy()` method. This proxy is a snapshot of the node's
//! configuration that can perform measurement without holding a borrow to the
//! modifier chain.

use crate::{Constraints, Measurable, ModifierNodeContext};
use cranpose_ui_layout::LayoutModifierMeasureResult;

/// Trait for measurement proxies that can perform measurement operations
/// without holding a borrow to the modifier chain.
///
/// Measurement proxies are created by `LayoutModifierNode::create_measurement_proxy()`
/// and used by coordinators to perform measurement while avoiding borrow checker issues.
pub trait MeasurementProxy {
    /// Performs measurement of the wrapped content with the given constraints.
    ///
    /// This method is equivalent to `LayoutModifierNode::measure()` but operates
    /// on a snapshot of the node's configuration.
    ///
    /// Returns a `LayoutModifierMeasureResult` containing:
    /// - The size this modifier will occupy
    /// - The offset at which to place the wrapped content
    fn measure_proxy(
        &self,
        context: &mut dyn ModifierNodeContext,
        wrapped: &dyn Measurable,
        constraints: Constraints,
    ) -> LayoutModifierMeasureResult;

    /// Returns the minimum intrinsic width of the wrapped content.
    fn min_intrinsic_width_proxy(&self, wrapped: &dyn Measurable, height: f32) -> f32;

    /// Returns the maximum intrinsic width of the wrapped content.
    fn max_intrinsic_width_proxy(&self, wrapped: &dyn Measurable, height: f32) -> f32;

    /// Returns the minimum intrinsic height of the wrapped content.
    fn min_intrinsic_height_proxy(&self, wrapped: &dyn Measurable, width: f32) -> f32;

    /// Returns the maximum intrinsic height of the wrapped content.
    fn max_intrinsic_height_proxy(&self, wrapped: &dyn Measurable, width: f32) -> f32;
}
