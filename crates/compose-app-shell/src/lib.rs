#![allow(clippy::type_complexity)]

use std::fmt::Debug;
// Use instant crate for cross-platform time support (native + WASM)
use instant::Instant;

use compose_core::{location_key, Applier, Composition, Key, MemoryApplier, NodeError};
use compose_foundation::{PointerButton, PointerButtons, PointerEvent, PointerEventKind};
use compose_render_common::{HitTestTarget, RenderScene, Renderer};
use compose_runtime_std::StdRuntime;
use compose_ui::{
    has_pending_focus_invalidations, has_pending_pointer_repasses, log_layout_tree,
    log_render_scene, log_screen_summary, peek_focus_invalidation, peek_pointer_invalidation,
    peek_render_invalidation, peek_layout_invalidation, process_focus_invalidations, process_pointer_repasses,
    request_render_invalidation, take_focus_invalidation, take_pointer_invalidation,
    take_render_invalidation, take_layout_invalidation, HeadlessRenderer, LayoutNode, LayoutTree, SemanticsTree,
};
use compose_ui_graphics::{Point, Size};

pub struct AppShell<R>
where
    R: Renderer,
{
    runtime: StdRuntime,
    composition: Composition<MemoryApplier>,
    renderer: R,
    cursor: (f32, f32),
    viewport: (f32, f32),
    buffer_size: (u32, u32),
    start_time: Instant,
    layout_tree: Option<LayoutTree>,
    semantics_tree: Option<SemanticsTree>,
    layout_dirty: bool,
    scene_dirty: bool,
    is_dirty: bool,
    /// Tracks which mouse buttons are currently pressed
    buttons_pressed: PointerButtons,
    /// Cached hit targets from pointer DOWN event.
    /// We store the actual HitTarget instances so the same handler closures
    /// (with their captured state like press_position) receive both Down and Up events.
    cached_hits: Vec<<<R as Renderer>::Scene as RenderScene>::HitTarget>,
}

impl<R> AppShell<R>
where
    R: Renderer,
    R::Error: Debug,
{
    pub fn new(mut renderer: R, root_key: Key, content: impl FnMut() + 'static) -> Self {
        let runtime = StdRuntime::new();
        let mut composition = Composition::with_runtime(MemoryApplier::new(), runtime.runtime());
        let build = content;
        if let Err(err) = composition.render(root_key, build) {
            log::error!("initial render failed: {err}");
        }
        renderer.scene_mut().clear();
        let mut shell = Self {
            runtime,
            composition,
            renderer,
            cursor: (0.0, 0.0),
            viewport: (800.0, 600.0),
            buffer_size: (800, 600),
            start_time: Instant::now(),
            layout_tree: None,
            semantics_tree: None,
            layout_dirty: true,
            scene_dirty: true,
            is_dirty: true,
            buttons_pressed: PointerButtons::NONE,
            cached_hits: Vec::new(),
        };
        shell.process_frame();
        shell
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
        self.layout_dirty = true;
        self.mark_dirty();
        self.process_frame();
    }

    pub fn set_buffer_size(&mut self, width: u32, height: u32) {
        self.buffer_size = (width, height);
    }

    pub fn buffer_size(&self) -> (u32, u32) {
        self.buffer_size
    }

    pub fn scene(&self) -> &R::Scene {
        self.renderer.scene()
    }

    pub fn renderer(&mut self) -> &mut R {
        &mut self.renderer
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_frame_waker(&mut self, waker: impl Fn() + Send + Sync + 'static) {
        self.runtime.set_frame_waker(waker);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_frame_waker(&mut self, waker: impl Fn() + Send + 'static) {
        self.runtime.set_frame_waker(waker);
    }

    pub fn clear_frame_waker(&mut self) {
        self.runtime.clear_frame_waker();
    }

    pub fn should_render(&self) -> bool {
        if self.layout_dirty
            || self.scene_dirty
            || peek_render_invalidation()
            || peek_pointer_invalidation()
            || peek_focus_invalidation()
            || peek_layout_invalidation()
        {
            return true;
        }
        self.runtime.take_frame_request() || self.composition.should_render()
    }

    /// Returns true if the shell needs to redraw (dirty flag or active animations).
    pub fn needs_redraw(&self) -> bool {
        self.is_dirty || self.has_active_animations()
    }

    /// Marks the shell as dirty, indicating a redraw is needed.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Returns true if there are active animations or pending recompositions.
    pub fn has_active_animations(&self) -> bool {
        self.runtime.take_frame_request() || self.composition.should_render()
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let frame_time = now
            .checked_duration_since(self.start_time)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.runtime.drain_frame_callbacks(frame_time);
        self.runtime.runtime_handle().drain_ui();
        if self.composition.should_render() {
            match self.composition.process_invalid_scopes() {
                Ok(changed) => {
                    if changed {
                        self.layout_dirty = true;
                        // Request render invalidation so the scene gets rebuilt
                        request_render_invalidation();
                    }
                }
                Err(NodeError::Missing { id }) => {
                    // Node was removed (likely due to conditional render or tab switch)
                    // This is expected when scopes try to recompose after their nodes are gone
                    log::debug!("Recomposition skipped: node {} no longer exists", id);
                    self.layout_dirty = true;
                    request_render_invalidation();
                }
                Err(err) => {
                    log::error!("recomposition failed: {err}");
                    self.layout_dirty = true;
                    request_render_invalidation();
                }
            }
        }
        self.process_frame();
        // Clear dirty flag after update (frame has been processed)
        self.is_dirty = false;
    }

    pub fn set_cursor(&mut self, x: f32, y: f32) -> bool {
        self.cursor = (x, y);
        
        // SOUNDNESS: During a gesture (button pressed), dispatch Move events to cached hits
        // to preserve gesture state consistency. Fresh hit-test is only for hover.
        
        // Debug assertion: if a button is pressed, we expect cached_hits to be populated
        // from pointer_pressed(). An empty cache with pressed buttons suggests a bug.
        debug_assert!(
            self.buttons_pressed == PointerButtons::NONE || !self.cached_hits.is_empty(),
            "buttons_pressed={:?} but cached_hits is empty. This suggests pointer_pressed() \
             was not called or hits were incorrectly cleared.",
            self.buttons_pressed
        );
        
        if self.buttons_pressed != PointerButtons::NONE && !self.cached_hits.is_empty() {
            // Gesture in progress: dispatch to cached targets (same HitRegion instances)
            let event = PointerEvent::new(
                PointerEventKind::Move,
                Point { x, y },
                Point { x, y },
            ).with_buttons(self.buttons_pressed);
            for hit in &self.cached_hits {
                hit.dispatch(event.clone());
                if event.is_consumed() {
                    break;
                }
            }
            self.mark_dirty();
            true
        } else {
            // No gesture in progress: use fresh hit-test for hover effects
            let hits = self.renderer.scene().hit_test(x, y);
            if !hits.is_empty() {
                let event = PointerEvent::new(
                    PointerEventKind::Move,
                    Point { x, y },
                    Point { x, y },
                ).with_buttons(self.buttons_pressed);
                for hit in hits {
                    hit.dispatch(event.clone());
                    if event.is_consumed() {
                        break;
                    }
                }
                self.mark_dirty();
                true
            } else {
                false
            }
        }
    }

    pub fn pointer_pressed(&mut self) -> bool {
        // Track button state
        self.buttons_pressed.insert(PointerButton::Primary);
        
        // Perform hit test and CACHE the results.
        // This is critical: we dispatch UP events to the SAME hit targets to preserve
        // gesture state (like press_position in ClickableNode).
        self.cached_hits = self.renderer.scene().hit_test(self.cursor.0, self.cursor.1);
        
        if !self.cached_hits.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Down,
                Point { x: self.cursor.0, y: self.cursor.1 },
                Point { x: self.cursor.0, y: self.cursor.1 },
            ).with_buttons(self.buttons_pressed);
            for hit in &self.cached_hits {
                hit.dispatch(event.clone());
                if event.is_consumed() {
                    break;
                }
            }
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn pointer_released(&mut self) -> bool {
        // UP events report buttons as "currently pressed" (after release),
        // matching typical platform semantics where primary is already gone.
        self.buttons_pressed.remove(PointerButton::Primary);
        let corrected_buttons = self.buttons_pressed;
        
        // CRITICAL: Dispatch UP event to CACHED hit targets (same instances as Down)
        if !self.cached_hits.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Up,
                Point { x: self.cursor.0, y: self.cursor.1 },
                Point { x: self.cursor.0, y: self.cursor.1 },
            ).with_buttons(corrected_buttons);
            for hit in &self.cached_hits {
                hit.dispatch(event.clone());
                if event.is_consumed() {
                    break;
                }
            }
            // Clear the cache after UP event
            self.cached_hits.clear();
            self.mark_dirty();
            true
        } else {
            false
        }
    }    
    /// Cancels any active gesture, dispatching Cancel events to cached targets.
    /// Call this when:
    /// - Window loses focus
    /// - Mouse leaves window while button pressed
    /// - Any other gesture abort scenario
    pub fn cancel_gesture(&mut self) {
        if !self.cached_hits.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Cancel,
                Point { x: self.cursor.0, y: self.cursor.1 },
                Point { x: self.cursor.0, y: self.cursor.1 },
            ).with_buttons(self.buttons_pressed);
            for hit in &self.cached_hits {
                hit.dispatch(event.clone());
            }
            self.cached_hits.clear();
            self.buttons_pressed = PointerButtons::NONE;
            self.mark_dirty();
        }
    }

    pub fn log_debug_info(&mut self) {
        println!("\n\n");
        println!("════════════════════════════════════════════════════════");
        println!("           DEBUG: CURRENT SCREEN STATE");
        println!("════════════════════════════════════════════════════════");

        if let Some(ref layout_tree) = self.layout_tree {
            log_layout_tree(layout_tree);
            let renderer = HeadlessRenderer::new();
            let render_scene = renderer.render(layout_tree);
            log_render_scene(&render_scene);
            log_screen_summary(layout_tree, &render_scene);
        } else {
            println!("No layout available");
        }

        println!("════════════════════════════════════════════════════════");
        println!("\n\n");
    }

    /// Get the current layout tree (for robot/testing)
    pub fn layout_tree(&self) -> Option<&LayoutTree> {
        self.layout_tree.as_ref()
    }

    /// Get the current semantics tree (for robot/testing)
    pub fn semantics_tree(&self) -> Option<&SemanticsTree> {
        self.semantics_tree.as_ref()
    }

    fn process_frame(&mut self) {
        self.run_layout_phase();
        self.run_dispatch_queues();
        self.run_render_phase();
    }

    fn run_layout_phase(&mut self) {
        // First, process scoped layout repasses (e.g., from scroll).
        // This bubbles dirty flags up from specific nodes without invalidating all caches.
        let repass_nodes = compose_ui::take_layout_repass_nodes();
        if !repass_nodes.is_empty() {
            let mut applier = self.composition.applier_mut();
            for node_id in repass_nodes {
                compose_core::bubble_layout_dirty(&mut *applier as &mut dyn compose_core::Applier, node_id);
            }
            drop(applier);
            self.layout_dirty = true;
        }
        
        // Global layout invalidation: fallback path for rare "layout algorithm changed" cases.
        // For local changes (like scroll), prefer schedule_layout_repass() which uses
        // the scoped repass_nodes path above—it just bubbles dirty flags without nuking
        // all cached measurements. This global hammer is expensive and should be avoided.
        let invalidation_requested = take_layout_invalidation();
        if invalidation_requested {
            // Force all layout measurements to be invalidated by incrementing the epoch
            compose_ui::invalidate_all_layout_caches();
            
            // ALSO mark the root Layout node as needing layout so needs_measure=true
            if let Some(root) = self.composition.root() {
                let mut applier = self.composition.applier_mut();
                if let Ok(node) = applier.get_mut(root) {
                    if let Some(layout_node) = node.as_any_mut().downcast_mut::<compose_ui::LayoutNode>() {
                        layout_node.mark_needs_layout();
                    }
                }
            }
            self.layout_dirty = true;
        }
        
        // Early exit if layout is not needed (viewport didn't change, etc.)
        if !self.layout_dirty {
            return;
        }



        let viewport_size = Size {
            width: self.viewport.0,
            height: self.viewport.1,
        };
        if let Some(root) = self.composition.root() {
            let handle = self.composition.runtime_handle();
            let mut applier = self.composition.applier_mut();
            applier.set_runtime_handle(handle);

            // Selective measure optimization: skip layout if tree is clean (O(1) check)
            let needs_layout =
                compose_ui::tree_needs_layout(&mut *applier, root).unwrap_or_else(|err| {
                    log::warn!(
                        "Cannot check layout dirty status for root #{}: {}",
                        root,
                        err
                    );
                    true // Assume dirty on error
                });

            if !needs_layout {
                // Tree is clean - skip layout computation and keep cached layout
                log::trace!("Skipping layout: tree is clean");
                self.layout_dirty = false;
                applier.clear_runtime_handle();
                return;
            }

            // Tree needs layout - compute it
            self.layout_dirty = false;
            
            // Ensure slots exist and borrow mutably (handled inside measure_layout via MemoryApplier)
            match compose_ui::measure_layout(&mut applier, root, viewport_size) {
                Ok(measurements) => {
                    self.semantics_tree = Some(measurements.semantics_tree().clone());
                    self.layout_tree = Some(measurements.into_layout_tree());
                    self.scene_dirty = true;
                }
                Err(err) => {
                    log::error!("failed to compute layout: {err}");
                    self.layout_tree = None;
                    self.semantics_tree = None;
                    self.scene_dirty = true;
                }
            }
            applier.clear_runtime_handle();
        } else {
            self.layout_tree = None;
            self.semantics_tree = None;
            self.scene_dirty = true;
            self.layout_dirty = false;
        }
    }

    fn run_dispatch_queues(&mut self) {
        // Process pointer input repasses
        // Similar to Jetpack Compose's pointer input invalidation processing,
        // we service nodes that need pointer input state updates without forcing layout/draw
        if has_pending_pointer_repasses() {
            let mut applier = self.composition.applier_mut();
            process_pointer_repasses(|node_id| {
                // Access the LayoutNode and clear its dirty flag
                let result = applier.with_node::<LayoutNode, _>(node_id, |layout_node| {
                    if layout_node.needs_pointer_pass() {
                        layout_node.clear_needs_pointer_pass();
                        log::trace!("Cleared pointer repass flag for node #{}", node_id);
                    }
                });
                if let Err(err) = result {
                    log::debug!(
                        "Could not process pointer repass for node #{}: {}",
                        node_id,
                        err
                    );
                }
            });
        }

        // Process focus invalidations
        // Mirrors Jetpack Compose's FocusInvalidationManager.invalidateNodes(),
        // processing nodes that need focus state synchronization
        if has_pending_focus_invalidations() {
            let mut applier = self.composition.applier_mut();
            process_focus_invalidations(|node_id| {
                // Access the LayoutNode and clear its dirty flag
                let result = applier.with_node::<LayoutNode, _>(node_id, |layout_node| {
                    if layout_node.needs_focus_sync() {
                        layout_node.clear_needs_focus_sync();
                        log::trace!("Cleared focus sync flag for node #{}", node_id);
                    }
                });
                if let Err(err) = result {
                    log::debug!(
                        "Could not process focus invalidation for node #{}: {}",
                        node_id,
                        err
                    );
                }
            });
        }
    }

    fn run_render_phase(&mut self) {
        let render_dirty = take_render_invalidation();
        let pointer_dirty = take_pointer_invalidation();
        let focus_dirty = take_focus_invalidation();
        if render_dirty || pointer_dirty || focus_dirty {
            self.scene_dirty = true;
        }
        if !self.scene_dirty {
            return;
        }
        self.scene_dirty = false;
        if let Some(layout_tree) = self.layout_tree.as_ref() {
            let viewport_size = Size {
                width: self.viewport.0,
                height: self.viewport.1,
            };
            if let Err(err) = self.renderer.rebuild_scene(layout_tree, viewport_size) {
                log::error!("renderer rebuild failed: {err:?}");
            }
        } else {
            self.renderer.scene_mut().clear();
        }
    }
}

impl<R> Drop for AppShell<R>
where
    R: Renderer,
{
    fn drop(&mut self) {
        self.runtime.clear_frame_waker();
    }
}

pub fn default_root_key() -> Key {
    location_key(file!(), line!(), column!())
}

#[cfg(test)]
#[path = "tests/app_shell_tests.rs"]
mod tests;
