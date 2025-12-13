# Review Mitigation Plan

**Origin:** [code_review.md](./code_review.md)  
**Date:** 2025-12-13

---

## Overview

This document provides a phased execution plan to address the issues identified in `code_review.md`. Each phase groups related fixes for atomic commits. Fixes are ordered by dependency (earlier phases enable later ones).

**Reference Architecture:** Patterns are derived from Jetpack Compose:
- `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/input/internal/CursorAnimationState.kt`
- `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/DelegatableNode.kt`
- `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/focus/FocusManager.kt`

---

## Phase 1: Layout Invalidation System (Blocks Everything)

**Issues:** [#2 Global Cache Nuking](./code_review.md#2-global-cache-nuking-on-every-edit-oapp-per-keystroke), [#4 `layout_dirty` Stuck True](./code_review.md#4-layout_dirty-can-get-stuck-true-forever)

### 1.1 Remove `invalidate_all_layout_caches()` from Text Input

**Files:**
- `crates/compose-ui/src/text_field_modifier_node.rs:287, 782`

**Jetpack Compose Pattern:**
```kotlin
// JC uses per-node invalidation via DelegatableNode extension:
fun DelegatableNode.invalidateMeasurement() {
    if (node.isAttached) {
        requireLayoutNode().invalidateMeasurements()
    }
}
```

**Implementation:**
```rust
// Replace:
crate::layout::invalidate_all_layout_caches();
crate::request_layout_invalidation();

// With:
// Option A: Use existing schedule_layout_repass
crate::schedule_layout_repass(self.node_id());
crate::request_render_invalidation();

// Option B: Add invalidate_measurement() to ModifierNodeContext
context.invalidate(InvalidationKind::Measure);
```

**Subtasks:**
- [ ] Add `node_id()` accessor to `TextFieldModifierNode` if not present
- [ ] Replace all `invalidate_all_layout_caches()` calls in text field code
- [ ] Keep `invalidate_all_layout_caches()` only for viewport resize

---

### 1.2 Fix `layout_dirty` Lifecycle

**Files:**
- `crates/compose-app-shell/src/lib.rs:663-670`

**Bug:** `layout_dirty` is only cleared in the "do layout" branch. The skip branch leaves it `true` forever.

**Implementation:**
```rust
// Line ~665-670, add clear in skip branch:
if !needs_layout {
    log::trace!("Skipping layout: tree is clean");
    self.layout_dirty = false;  // ← ADD THIS
    applier.clear_runtime_handle();
    return;
}
```

---

## Phase 2: Cursor Blink Timer (CPU/Battery Fix)

**Issues:** [#3 Render Forever](./code_review.md#3-cursor-blink-implemented-as-render-forever-cpubattery-killer)

### 2.1 Implement `CursorAnimationState`

**Reference:** `CursorAnimationState.kt` from JC

**Jetpack Compose Pattern:**
```kotlin
// JC uses coroutine delay(), NOT continuous redraw:
suspend fun snapToVisibleAndAnimate() {
    cursorAlpha = 1f
    while (true) {
        delay(500)
        cursorAlpha = 0f
        delay(500)
        cursorAlpha = 1f
    }
}
```

**Files to Create/Modify:**
- **NEW:** `crates/compose-ui/src/cursor_animation.rs`
- `crates/compose-ui/src/text_field_modifier_node.rs`
- `crates/compose-app-shell/src/lib.rs:142-148`
- `crates/compose-app/src/desktop.rs` (remove `ControlFlow::Poll`)

**Implementation:**
```rust
// cursor_animation.rs
pub struct CursorAnimationState {
    cursor_alpha: Cell<f32>,
    next_blink_time: Cell<Option<Instant>>,
}

impl CursorAnimationState {
    pub fn tick(&self, now: Instant) -> f32 {
        if let Some(next) = self.next_blink_time.get() {
            if now >= next {
                let new_alpha = if self.cursor_alpha.get() > 0.5 { 0.0 } else { 1.0 };
                self.cursor_alpha.set(new_alpha);
                self.next_blink_time.set(Some(now + Duration::from_millis(500)));
            }
        }
        self.cursor_alpha.get()
    }
    
    pub fn next_blink_time(&self) -> Option<Instant> {
        self.next_blink_time.get()
    }
}
```

**Subtasks:**
- [ ] Create `CursorAnimationState` struct
- [ ] Modify `needs_redraw()` to NOT return true for `has_focused_field()`
- [ ] Add `next_event_time()` to AppShell that returns min of all pending timers
- [ ] Use `ControlFlow::WaitUntil(next_event_time)` instead of `Poll`

---

## Phase 3: Focus Management (O(1) Dispatch)

**Issues:** [#7 Stale Focus Atomic](./code_review.md#7-focus-state-is-global-fragile-and-can-get-stuck-true), [#8 O(N) Key Dispatch](./code_review.md#8-keyclipboard-dispatch-is-on-tree-scan)

### 3.1 Fix Stale Focus Detection

**Files:**
- `crates/compose-ui/src/text_field_focus.rs:71-73`

**Implementation:**
```rust
pub fn has_focused_field() -> bool {
    FOCUSED_FIELD.with(|current| {
        let mut borrow = current.borrow_mut();
        if let Some(ref weak) = *borrow {
            if weak.upgrade().is_some() {
                return true;
            }
            // Weak is dead - clear it
            *borrow = None;
        }
        HAS_FOCUSED_FIELD.store(false, Ordering::SeqCst);
        false
    })
}
```

---

### 3.2 Store Focused NodeId for O(1) Dispatch

**Files:**
- `crates/compose-ui/src/text_field_focus.rs` (add NodeId storage)
- `crates/compose-app-shell/src/lib.rs:786-932` (replace tree scan)

**Implementation:**
```rust
// In text_field_focus.rs:
thread_local! {
    static FOCUSED_NODE: RefCell<Option<NodeId>> = RefCell::new(None);
}

pub fn focused_node_id() -> Option<NodeId> {
    FOCUSED_NODE.with(|n| *n.borrow())
}

// In AppShell::on_key_event:
if let Some(node_id) = compose_ui::focused_node_id() {
    // Direct dispatch to focused node - O(1)
    applier.with_node::<LayoutNode, _>(node_id, |layout_node| {
        layout_node.with_text_field_modifier_mut(|tf| tf.handle_key_event(event))
    })
}
```

---

## Phase 4: `TextFieldElement` Correctness

**Issues:** [#1 Eq/Hash Violation](./code_review.md#1-textfieldelement-violates-the-eqhash-contract)

### 4.1 Fix `PartialEq` to Compare State Identity

**Files:**
- `crates/compose-ui/src/text_field_modifier_node.rs:1013-1022`

**Implementation:**
```rust
impl PartialEq for TextFieldElement {
    fn eq(&self, other: &Self) -> bool {
        // Compare Rc pointer identity, not content
        self.state == other.state && self.cursor_color == other.cursor_color
    }
}

impl Hash for TextFieldElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by Rc pointer, matching PartialEq
        std::ptr::hash(Rc::as_ptr(&self.state.inner), state);
        self.cursor_color.0.to_bits().hash(state);
        self.cursor_color.1.to_bits().hash(state);
        self.cursor_color.2.to_bits().hash(state);
        self.cursor_color.3.to_bits().hash(state);
    }
}
```

**Note:** This requires exposing `inner` or adding `Rc::as_ptr` accessor to `TextFieldState`.

---

## Phase 5: `TextFieldState` Safety

**Issues:** [#6 Panic-Hostile Edit](./code_review.md#6-textfieldstateedit-is-panic--and-reentrancy-hostile)

### 5.1 Add RAII Guard for `is_editing`

**Files:**
- `crates/compose-foundation/src/text/state.rs:235-320`

**Implementation:**
```rust
struct EditGuard<'a> {
    inner: &'a RefCell<TextFieldStateInner>,
}

impl<'a> EditGuard<'a> {
    fn new(inner: &'a RefCell<TextFieldStateInner>) -> Self {
        inner.borrow_mut().is_editing = true;
        Self { inner }
    }
}

impl Drop for EditGuard<'_> {
    fn drop(&mut self) {
        self.inner.borrow_mut().is_editing = false;
    }
}

// In edit():
let _guard = EditGuard::new(&self.inner);
f(&mut buffer);
// _guard drops here, even on panic
```

---

### 5.2 Clone Listeners Before Notification

**Implementation:**
```rust
// Clone listener Rcs before calling:
if changed {
    let listeners: Vec<_> = {
        let inner = self.inner.borrow();
        inner.listeners.iter().cloned().collect()
    };
    for listener in listeners {
        listener(&new_value);
    }
}
```

---

### 5.3 Use `VecDeque` for Undo Stack

**Files:**
- `crates/compose-foundation/src/text/state.rs:53, 275`

```rust
use std::collections::VecDeque;

// Change:
undo_stack: VecDeque<TextFieldValue>,
redo_stack: VecDeque<TextFieldValue>,

// Replace remove(0) with:
inner.undo_stack.pop_front();
```

---

## Phase 6: Architecture Cleanup

### 6.1 Remove Widget-Specific Code from `slices.rs`

**Issues:** [#5 Slice Collection Layering](./code_review.md#5-slice-collection-does-widget-specific-logic--mutates-state)

**Files:**
- `crates/compose-ui/src/modifier/slices.rs:153-308`

**Goal:** Move text field cursor/selection rendering to the node's `draw()` method - do not mutate state during slice collection.

**Subtasks:**
- [ ] Remove `downcast_ref::<TextFieldModifierNode>()` from `collect_modifier_slices`
- [ ] Remove `set_content_offset()` call during slice collection
- [ ] Let `TextFieldModifierNode::draw()` handle cursor/selection rendering
- [ ] Pass padding as layout result, not scanned from modifiers

---

### 6.2 Fix Platform cfg for PRIMARY Selection

**Files:**
- `crates/compose-app-shell/src/lib.rs:497-524`

```rust
// Change:
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]

// To:
#[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
```

---

### 6.3 Fix `hit_test()` to Check Bounds

**Files:**
- `crates/compose-ui/src/text_field_modifier_node.rs:947-950`

```rust
fn hit_test(&self, x: f32, y: f32) -> bool {
    let size = self.measured_size.get();
    x >= 0.0 && x <= size.width && y >= 0.0 && y <= size.height
}
```

---

## Phase 7: Polish

### 7.1 Fix Clippy Warnings

```bash
cargo clippy --fix --lib -p compose-ui -p compose-app-shell
```

### 7.2 Centralize Magic Numbers

**Create:** `crates/compose-foundation/src/text/config.rs`

```rust
pub const DEFAULT_LINE_HEIGHT: f32 = 20.0;
pub const CURSOR_BLINK_MS: u64 = 500;
pub const DOUBLE_CLICK_MS: u128 = 500;
pub const CURSOR_WIDTH: f32 = 2.0;
pub const UNDO_CAPACITY: usize = 100;
```

### 7.3 Unicode-Aware Word Navigation

**Files:**
- `crates/compose-ui/src/text_field_modifier_node.rs:440-474`

Replace `is_ascii_alphanumeric()` with `char::is_alphanumeric()`.

---

## Verification Plan

### Automated Tests

```bash
# Run all workspace tests
cargo test --workspace

# Run text field specific tests
cargo test -p compose-ui text_field
cargo test -p compose-foundation text

# Run robot tests (requires display)
cargo run --package desktop-app --example robot_text_input --features robot-app
cargo run --package desktop-app --example robot_double_click --features robot-app
cargo run --package desktop-app --example robot_drag_selection --features robot-app
```

### Manual Verification

1. **CPU Usage Test:** Focus a text field, minimize the window or leave idle for 30 seconds. CPU should NOT be at 100%.

2. **Layout Performance Test:** Open the Recursive Layout demo at level 7. Focus the text field. Typing should not cause visible jank.

3. **Focus Lifecycle Test:** 
   - Focus text field
   - Switch tabs (component unmounts)
   - Verify `has_focused_field()` returns false
   - Verify CPU is idle

---

## Execution Order

| Phase | Effort | Blocks |
|-------|--------|--------|
| Phase 1 | 2h | Phase 2, 3 |
| Phase 2 | 3h | - |
| Phase 3 | 2h | - |
| Phase 4 | 1h | - |
| Phase 5 | 1h | - |
| Phase 6 | 3h | - |
| Phase 7 | 1h | - |

**Total Estimated:** ~13 hours

Phases 2-5 can be parallelized after Phase 1 is complete.
