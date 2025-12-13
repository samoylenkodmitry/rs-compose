# Code Review: Text Field Implementation

**Date:** 2025-12-13  
**Scope:** ~5400 LOC across 32 files implementing `BasicTextField` and related text editing infrastructure

---

## Summary

This is a significant patch introducing complex stateful logic. While the implementation correctly follows Jetpack Compose patterns and provides impressive functionality (editing, selection, undo/redo, clipboard, multiline), **this review identifies stop-ship architectural flaws, performance bottlenecks, and correctness bugs** that will cause serious pain in production.

---

## 🛑 STOP-SHIP Issues

### 1. `TextFieldElement` Violates the `Eq`/`Hash` Contract

**Location:** `crates/compose-ui/src/text_field_modifier_node.rs:1001-1022`

```rust
impl Hash for TextFieldElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state.text().hash(state);  // Hash depends on text
        self.cursor_color.0.to_bits().hash(state);
    }
}

impl PartialEq for TextFieldElement {
    fn eq(&self, _other: &Self) -> bool {
        true  // <--- ALWAYS TRUE
    }
}
```

**The Problem:** `PartialEq` returns `true` for *everything*, while `Hash` depends on the current text and cursor color. **Equal keys must hash equal** - this is a correctness time bomb if this ever ends up in a `HashMap`/`HashSet`.

Additionally, `always_update()` is hardcoded `true`, guaranteeing churn even when nothing changed.

**Fix:** Make element identity stable and `Eq`/`Hash` consistent:
```rust
fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.state.inner, &other.state.inner) && 
    self.cursor_color == other.cursor_color
}
```
Or remove `Hash`/`Eq` if your diffing doesn't need hashed keys.

---

### 2. Global Cache Nuking on Every Edit (O(App) Per Keystroke)

**Location:** `crates/compose-ui/src/text_field_modifier_node.rs:782-784`

```rust
// Inside handle_key_event - runs on EVERY keystroke:
crate::layout::invalidate_all_layout_caches(); // <--- THIS IS A DISASTER
crate::request_layout_invalidation();
crate::request_render_invalidation();
```

Then `AppShell::run_layout_phase` responds by **invalidating all caches and force-marking root measure/layout**. That's **O(entire app tree)** work per keystroke. On a real tree this becomes instant jank.

**Fix:** Scoped repass. The text field should dirty *itself* (and maybe parent for intrinsic size), not the whole world:
```rust
crate::schedule_layout_repass(node_id);
crate::request_render_invalidation();
```

---

### 3. Cursor Blink Implemented as "Render Forever" (CPU/Battery Killer)

**Locations:** 
- `crates/compose-app-shell/src/lib.rs:142-148` 
- `crates/compose-app/src/desktop.rs` (ControlFlow::Poll)

```rust
pub fn needs_redraw(&self) -> bool {
    self.is_dirty || self.layout_dirty || self.has_active_animations()
        || compose_ui::has_focused_field()  // <--- CONTINUOUS REDRAW
}
```

**The Problem:** This makes cursor blink equivalent to *continuous redraw while any text field is focused*:
- `needs_redraw()` returns true if any field is focused
- Scene dirtiness is forced by `has_focused_field()`
- desktop-winit switches to `ControlFlow::Poll` when a field is focused

This is a **CPU/battery killer**, and it masks performance regressions because you're always redrawing.

**Fix:** Blink should schedule a redraw at the next transition (e.g., `WaitUntil(next_blink)` or a runtime timer), not spin the event loop.

---

### 4. `layout_dirty` Can Get Stuck True Forever

**Location:** `crates/compose-app-shell/src/lib.rs:663-670`

**The Problem:** The skip-layout optimization checks `needs_layout = tree_needs_layout_check || self.layout_dirty`, but `layout_dirty` is only cleared in the "do layout" branch. If there isn't a clear elsewhere, one keyboard event can **permanently disable your O(1) "tree clean" fast path**.

**Fix:** Ensure `layout_dirty` is cleared in both branches:
```rust
if !needs_layout {
    log::trace!("Skipping layout: tree is clean");
    self.layout_dirty = false;  // <-- ADD THIS
    applier.clear_runtime_handle();
    return;
}
```

---

### 5. Slice Collection Does Widget-Specific Logic + Mutates State

**Location:** `crates/compose-ui/src/modifier/slices.rs:153-178, 180-308`

```rust
// In collect_modifier_slices():
if let Some(text_field_node) = any.downcast_ref::<TextFieldModifierNode>() {
    text_field_node.set_content_offset(padding.left);  // <-- MUTATES STATE
    // ... injects cursor draw command
}
```

**The Problem:** This is a layering violation:
- Generic traversal now "knows" about a specific widget type
- Rendering/slice building now **mutates input geometry state** (order-dependent behavior)
- Padding is inferred by scanning modifiers, not derived from actual layout coordinates

**Fix:** The text field node should own its caret/selection rendering and coordinate mapping, based on real layout results—not "scan padding modifiers and hope it matches".

---

### 6. `TextFieldState::edit` Is Panic- and Reentrancy-Hostile

**Location:** `crates/compose-foundation/src/text/state.rs:235-320`

```rust
self.inner.borrow_mut().is_editing = true;
f(&mut buffer);  // <-- USER CLOSURE RUNS HERE
// ... on panic, is_editing stays true forever
inner.is_editing = false;
```

**Problems:**
1. `is_editing` is set, then user closure runs, then cleared - **no RAII**, so a panic leaves state permanently "editing"
2. Listeners are invoked while still holding internal borrows (`(inner.listeners[idx])(&value)`), making reentrant use likely to panic with `RefCell` borrow rules

**Fix:** Use RAII guard for `is_editing`, clone listeners for notification outside borrows:
```rust
struct EditGuard<'a>(&'a RefCell<TextFieldStateInner>);
impl Drop for EditGuard<'_> {
    fn drop(&mut self) { self.0.borrow_mut().is_editing = false; }
}
let _guard = EditGuard(&self.inner);
```

---

## 🔴 High-Impact Architecture Problems

### 7. Focus State Is Global, Fragile, and Can Get "Stuck True"

**Location:** `crates/compose-ui/src/text_field_focus.rs`

Focus is tracked via a thread-local weak handle *and* a global atomic bool. `request_focus()` sets the atomic to true. If a focused field is dropped without calling `clear_focus()`:
- `HAS_FOCUSED_FIELD = true` forever
- → Poll loop forever
- → **Redraw forever (100% CPU)**

**Fix:** "has focused field" must be derived from a live focused handle:
```rust
pub fn has_focused_field() -> bool {
    FOCUSED_FIELD.with(|current| {
        if let Some(weak) = current.borrow().as_ref() {
            if weak.upgrade().is_some() { return true; }
        }
        HAS_FOCUSED_FIELD.store(false, Ordering::SeqCst);
        false
    })
}
```
Or better: keep focus in the runtime/app-shell, not a global in `compose-ui`.

---

### 8. Key/Clipboard Dispatch Is O(N) Tree Scan

**Location:** `crates/compose-app-shell/src/lib.rs:786-932`

```rust
fn dispatch_key_to_text_fields(applier, node_id, event) -> bool {
    // Recursively walks ENTIRE node tree looking for focused fields
    for child_id in children {
        if dispatch_key_to_text_fields(applier, child_id, event) { ... }
    }
}
```

**The Problem:** 
- Keyboard dispatch recursively walks entire node tree
- Clipboard copy/cut also recurses
- If two nodes mistakenly say "focused", "first found in traversal order wins" unpredictably

**Fix:** Focus manager should store *the* focused text field (NodeId or direct handle), so key/clipboard dispatch is **O(1)**.

---

### 9. `hit_test()` Returns `true` Unconditionally (Steal All Clicks?)

**Location:** `crates/compose-ui/src/text_field_modifier_node.rs:947-950`

```rust
fn hit_test(&self, _x: f32, _y: f32) -> bool {
    true  // Always participate in hit testing
}
```

**The Problem:** If the engine consults `hit_test()` to build the hit path, this can break pointer routing globally - the text field may "steal" clicks from overlapping elements.

---

### 10. Platform cfg for PRIMARY Selection Is Wrong

**Location:** `crates/compose-app-shell/src/lib.rs:497-524`

```rust
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn set_primary_selection(&mut self, text: &str) {
    use arboard::{SetExtLinux, LinuxClipboardKind};
    // ...
}
```

**The Problem:** Linux-specific `LinuxClipboardKind::Primary` APIs are referenced without a Linux-only cfg. The surrounding cfg excludes wasm/android but **still includes macOS/Windows** where these APIs will fail or panic.

**Fix:** Add `target_os = "linux"` to the cfg.

---

### 11. O(N) Undo Stack

**Location:** `crates/compose-foundation/src/text/state.rs:275`

```rust
inner.undo_stack.remove(0);  // O(N) shift
```

**Fix:** Use `VecDeque::pop_front()` for O(1).

---

### 12. ASCII-Only Word Navigation

**Location:** `crates/compose-ui/src/text_field_modifier_node.rs:447`

```rust
!bytes[i - 1].is_ascii_alphanumeric()
```

**The Problem:** Ctrl+Left/Right treats non-ASCII characters (Cyrillic, Chinese, etc.) as word separators or may crash on UTF-8 boundaries.

**Fix:** Use `char::is_alphanumeric()` with proper UTF-8 iteration.

---

## 🟡 "Lazy API" Smells

### 13. Options Exist But Aren't Enforced

**Location:** `crates/compose-ui/src/widgets/basic_text_field.rs:61-77`

```rust
pub struct BasicTextFieldOptions {
    pub enabled: bool,      // NOT ENFORCED
    pub read_only: bool,    // NOT ENFORCED
    pub single_line: bool,  // NOT ENFORCED
}
```

The implementation passes cursor color but doesn't enforce behavioral constraints. This creates false confidence and future compatibility debt.

---

### 14. Robot "Tests" Are Not Real Tests

**Location:** `apps/desktop-demo/robot-runners/robot_text_input.rs`

```rust
std::thread::spawn(|| {
    std::thread::sleep(Duration::from_secs(30));
    std::process::exit(1);  // <-- "Test" exit
});
```

Process exit as test reporting makes CI integration problematic.

---

### 15. Magic Numbers Everywhere

| Constant | Locations |
|----------|-----------|
| `20.0` (line height) | `text_field_modifier_node.rs`, `slices.rs` |
| `500` (blink/click timeout) | Multiple files |
| `2.0` (cursor width) | Multiple files |

---

## Clippy Warnings (Must Fix)

```
warning: manual arithmetic check (slices.rs:250)
  → use `sel_start.saturating_sub(line_start)`

warning: needless_range_loop (text_field_modifier_node.rs:163)
  → use `for line in lines.iter().take(line_index)`

warning: question_mark (lib.rs:464, 478)
  → use `let root_id = self.composition.root()?;`
```

---

## ✅ What Works

1. **Modifier Architecture:** `TextFieldElement` (immutable) / `TextFieldModifierNode` (stateful) split is correct
2. **Draw-time Selection:** Selection rendered in Draw phase enables smooth drag updates without layout thrashing
3. **Clipboard Integration:** Persistent `arboard::Clipboard` addresses Linux X11 lifetime issue
4. **Cross-platform Timing:** Uses `instant` crate for WASM compatibility
5. **Robot Tests:** Comprehensive coverage of UI interactions

---

## Minimum Salvage Plan (Required Before Merge)

| # | Action | Severity |
|---|--------|----------|
| 1 | **Delete** "cursor blink == continuous redraw". Replace with blink scheduler that requests redraw only on transitions. Kill `ControlFlow::Poll` for this. | 🛑 |
| 2 | Replace all `invalidate_all_layout_caches()` in text input with **scoped layout repass** | 🛑 |
| 3 | Fix `TextFieldElement` `Eq`/`Hash`/`always_update`. Current state is structurally unsafe. | 🛑 |
| 4 | Remove text-field-specific hacks from `slices.rs` - no downcasts, no state mutation during slice building | 🛑 |
| 5 | Make focus + key dispatch **O(1)** by tracking focused field NodeId directly; delete tree scans | 🔴 |
| 6 | Fix `layout_dirty` lifecycle so it can't permanently disable "skip layout" | 🛑 |
| 7 | Fix `TextFieldState::edit` with **RAII guard** + safe listener notification (clone before calling) | 🔴 |
| 8 | Fix `has_focused_field()` to detect dead weak refs and update atomic | 🔴 |
| 9 | Add `target_os = "linux"` to PRIMARY selection cfg | 🟠 |
| 10 | Switch undo stack to `VecDeque` | 🟠 |
