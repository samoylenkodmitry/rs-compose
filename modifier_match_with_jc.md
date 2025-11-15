# Modifier Migration Reality Check

The modifier API surface is moving in the right direction (builder helpers now chain via
`self.then`, `ModifierNodeChain` has capability tracking, and the node-backed factories live in
`crates/compose-ui/src/modifier_nodes.rs`). However, the branch does **not** currently deliver the
parity described in the README/NEXT_TASK files. This document records the gaps so we can close them
before declaring the migration "done".

---

## Current Snapshot

- ✅ `ModifierNodeChain` reconciliation, capability masks, and helper macros exist in
  `crates/compose-foundation/src/modifier.rs` and are used by the built-in nodes under
  `crates/compose-ui/src/modifier_nodes.rs`.
- ✅ Public modifier builders (padding/background/fill/etc.) now consume `self` and use `then(...)`
  so callers can fluently chain them without reaching for ad-hoc constructors.
- ⚠️ Runtime consumers still rely on legacy fallbacks:
  - `LayoutNodeData` snapshots are only produced for `LayoutNode` / `SubcomposeLayoutNode`. Code
    paths such as `runtime_metadata_for` in `crates/compose-ui/src/layout/mod.rs` still clone
    modifiers from `ButtonNode`, `TextNode`, and `SpacerNode` instead of using reconciled node data.
  - `measure_spacer` builds resolved modifiers via `Modifier::empty().resolved_modifiers()` each
    time rather than pulling from the owning node.
- ⚠️ Pointer/focus invalidation managers (`crates/compose-ui/src/pointer_dispatch.rs` and
  `crates/compose-ui/src/focus_dispatch.rs`) are never invoked by the runtime. The only references
  to `process_pointer_repasses` / `process_focus_invalidations` live in the unit tests, meaning the
  new `needs_pointer_pass` / `needs_focus_sync` flags on `LayoutNode` can never clear in practice.
- ⚠️ `ButtonNode`, `TextNode`, and `SpacerNode` still implement the `Node` trait directly and
  bypass the modifier chain completely, so "legacy" behaviour is still present in the tree.
- ⚠️ Tests under `crates/compose-ui/src/tests/pointer_input_integration_test.rs` simply assert node
  counts; no integration test actually drives pointer events through `HitTestTarget`.

---

## Work Remaining Before Parity Claims

1. **Hook up the dispatch queues.**
   - Drain `process_pointer_repasses` / `process_focus_invalidations` from the app shell each frame
     and update the corresponding `LayoutNode` so `needs_pointer_pass` / `needs_focus_sync` can be
     cleared without forcing a layout pass.
   - Propagate the updated modifier slices or focus state to the renderer/hit-test structures.
2. **Delete the widget-specific node types.**
   - Rebuild `Button`, `Text`, and `Spacer` on top of `LayoutNode`/`SubcomposeLayoutNode` so
     metadata, semantics, and modifier snapshots all flow through the same path.
   - Remove the `RuntimeNodeMetadata` fallbacks once no caller needs them.
3. **Centralise resolved modifier data.**
   - Ensure every layout-tree builder passes the reconciled `modifier_slices`/`resolved_modifiers`
     into `LayoutNodeData::new(...)` and delete helper calls like
     `Modifier::empty().resolved_modifiers()` from the hot paths.
4. **Add real integration coverage.**
   - Extend the pointer/focus tests to synthesize events through `HitTestTarget` so we can verify
     suspending pointer handlers, `Modifier.clickable`, and focus callbacks operate end-to-end.
5. **Document the true status.**
   - README/NEXT_TASK should reflect the above reality until the missing pieces are implemented.

---

## Jetpack Compose References

Use these upstream files while implementing the remaining pieces:

| Area | Kotlin Source | Compose-RS Target |
| --- | --- | --- |
| Modifier API | `androidx/compose/ui/Modifier.kt` | `crates/compose-ui/src/modifier/mod.rs` |
| Node lifecycle | `ModifierNodeElement.kt`, `DelegatableNode.kt` | `crates/compose-foundation/src/modifier.rs` |
| Pointer input | `ui/input/pointer/*` | `crates/compose-ui/src/modifier/pointer_input.rs` |
| Focus system | `FocusInvalidationManager.kt`, `FocusOwner.kt` | `crates/compose-ui/src/modifier/focus.rs` + dispatch managers |
| Semantics | `semantics/*` | `crates/compose-ui/src/semantics` |

Keep this document up to date as we chip away at the remaining tasks so reviewers can clearly see
which parts of the Kotlin contract are satisfied.
