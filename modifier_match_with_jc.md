# Modifier Migration Reality Check

The modifier API surface continues to move toward Jetpack Compose parity. Builder helpers now chain
via `self.then`, capability tracking exists in `ModifierNodeChain`, and node-backed factories live
in `crates/compose-ui/src/modifier_nodes.rs`. This document captures the current status and the gaps
that still prevent us from matching Jetpack Compose’s behavior.

---

## Current Snapshot

- ✅ `ModifierNodeChain` reconciliation, capability masks, and helper macros live in
  `crates/compose-foundation/src/modifier.rs` and power the built-in nodes in
  `crates/compose-ui/src/modifier_nodes.rs`.
- ✅ Public modifier builders (padding/background/fill/etc.) now take `self` and use `then(...)`
  so callers can fluently chain them without bespoke constructors.
- ✅ Pointer/focus invalidation managers (`crates/compose-ui/src/pointer_dispatch.rs` and
  `crates/compose-ui/src/focus_dispatch.rs`) run every frame via `AppShell::run_dispatch_queues`,
  clearing `needs_pointer_pass` / `needs_focus_sync` just like Jetpack Compose.
- ✅ Legacy widget-specific nodes (`ButtonNode`, `TextNode`, `SpacerNode`) are gone. All widgets emit
  `LayoutNode` instances with a `MeasurePolicy`.
- ✅ Modifier resolution is centralized. The old `measure_spacer/text/button` helpers that rebuilt
  modifiers have been deleted; everything flows through `measure_layout_node`.
- ✅ Metadata fallbacks were removed. `runtime_metadata_for` / `compute_semantics_for_node` only
  handle `LayoutNode` / `SubcomposeLayoutNode`, so modifier chains stay consistent.
- ✅ `Modifier::then` now builds a persistent Combined tree backed by `ModifierKind`
  (`crates/compose-ui/src/modifier/mod.rs:235-382`), matching Kotlin’s `CombinedModifier`.
- ⚠️ `ModifierChainHandle::update()` immediately flattens that tree into new `Vec`s and rebuilds a
  `ResolvedModifiers` snapshot on every pass (`crates/compose-ui/src/modifier/chain.rs:72-231`), so
  structural sharing stops at the API boundary.
- ⚠️ Layout/draw still partially bypass modifier nodes. `measure_layout_node()` now calls
  `measure_through_modifier_chain()` (`crates/compose-ui/src/layout/mod.rs:640-705`) which properly
  iterates through the modifier chain and measures `TextModifierNode`, but other `LayoutModifierNode`
  types (padding/size/offset) still fall back to `ResolvedModifiers` lookups.
- ⚠️ `TextModifierElement` still only stores a raw `String` (`crates/compose-ui/src/text_modifier_node.rs:167-205`);
  `TextModifierNode::measure()` delegates to a monospaced fallback
  (`text_modifier_node.rs:97-135` + `crates/compose-ui/src/text.rs:4-34`), `draw()` is empty, and
  semantics expose only `content_description`. The widget itself (`crates/compose-ui/src/widgets/text.rs:125-143`)
  lacks style/overflow/minLines/maxLines parameters, so parity with `BasicText` is far away.
- ⚠️ Tests under `crates/compose-ui/src/tests/pointer_input_integration_test.rs` still only assert
  node counts—no integration test actually drives pointer events through `HitTestTarget`.

---

## Known Shortcuts

### Modifier Chain Reconciliation Still Flattens

**Current Behavior:**
- Even though `Modifier::then` produces a persistent tree, `ModifierChainHandle::update()` calls
  `modifier.elements()` and `modifier.inspector_metadata()` (`crates/compose-ui/src/modifier/chain.rs:72-95`),
  cloning every element + inspector entry into new `Vec`s before passing them to
  `ModifierNodeChain::update_from_slice()`.
- The flattened elements are then collapsed into a `ResolvedModifiers` struct
  (`modifier/chain.rs:173-231`), reintroducing the eager snapshot the Kotlin runtime avoids.

**Problem:**
- Reconciliation is still `O(n)` allocations per recomposition, so the `ModifierKind` tree never
  delivers structural sharing.
- Because pipeline stages consume the `ResolvedModifiers` copy, no downstream code can actually use
  `ModifierNode`s during layout/draw/pointer dispatch.

**Desired Fix:**
- Iterate the Combined tree directly (via `foldIn`/`foldOut` equivalents) when updating the node
  chain so we don’t allocate intermediate vectors.
- Stop materializing `ResolvedModifiers` once layout/draw/pointer pipelines read directly from the
  reconciled node chain.

**Reference:** Kotlin builds `CombinedModifier` and lets `ModifierNodeChain.updateFrom()` walk it in
`androidx/compose/ui/Modifier.kt` and `androidx/compose/ui/node/ModifierNodeCoordinator.kt`.

### Modifier Nodes Partially Bypassed During Measure/Draw

**Current Behavior:**
- `measure_layout_node()` now calls `measure_through_modifier_chain()` which properly accesses the
  `ModifierNodeChain` and iterates through layout modifier nodes (`layout/mod.rs:640-705`).
- `TextModifierNode` is handled correctly - its `measure()` method is called and the result is used.
- Other `LayoutModifierNode` types return `Err(())` and fall back to `ResolvedModifiers` lookups.
- Padding/size/offset are still read from `ResolvedModifiers` in `measure_layout_node()` (lines 760-880).

**Problem:**
- The existing modifier nodes (PaddingNode, SizeNode, OffsetNode, etc.) in `crates/compose-ui/src/modifier_nodes.rs`
  do NOT implement `LayoutModifierNode::measure()`. They only store data that gets aggregated into
  `ResolvedModifiers` via `ModifierChainHandle::compute_resolved()`.
- This means padding/size/background are duplicated - both in modifier nodes (for aggregation) AND
  in `ResolvedModifiers` (for actual use during layout).
- Layout modifiers cannot wrap child measurables or participate in measurement, so we can't match
  Jetpack Compose's `LayoutModifierNodeCoordinator` chain.

**Remaining Work:**
1. Make PaddingNode, SizeNode, OffsetNode implement `LayoutModifierNode` trait and provide `measure()` implementations:
   - `PaddingNode::measure()` should subtract padding from constraints, measure wrapped content, add padding back to size
   - `SizeNode::measure()` should constrain the wrapped content size
   - `OffsetNode::measure()` should pass through to wrapped content (offset handled during placement)
2. Extend `measure_through_modifier_chain()` to handle these nodes (add downcasts after TextModifierNode check).
3. Once all modifier nodes are handled, remove the `ResolvedModifiers` lookups from `measure_layout_node()`.
4. Delete `compute_resolved()` from `ModifierChainHandle` once layout no longer needs `ResolvedModifiers`.

**Reference:** See `androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt` and
`androidx/compose/ui/node/NodeCoordinator.kt` for the Kotlin reference chain.

### Text Implementation Architecture Mismatch

**Current Implementation:**
- `TextModifierElement` only captures a `String`
  (`crates/compose-ui/src/text_modifier_node.rs:167-205`), so style/overflow/minLines/maxLines are
  lost before they reach the node.
- `TextModifierNode::measure()` delegates to a global monospaced stub
  (`crates/compose-ui/src/text.rs:4-34`), ignores constraints like softWrap/maxLines, never caches
  paragraphs, and cannot invalidate itself when data changes.
- `draw()` is empty and no GPU paragraph commands are emitted, so rendering happens elsewhere via
  ad-hoc inspection of the modifier chain.
- Semantics only sets `content_description`
  (`text_modifier_node.rs:151-155`), and `runtime_metadata_for()` relies on that description to
  infer `SemanticsRole::Text` (`crates/compose-ui/src/layout/mod.rs:1633-1655`).
- The widget itself only exposes `(value, modifier)` (`crates/compose-ui/src/widgets/text.rs:125-143`),
  so there is no way to pass the parameters that Jetpack Compose’s `BasicText` accepts
  (style/font resolver/overflow/minLines/maxLines/autoSize/etc.).

**Problem:**
- Without style/resolver parameters the modifier node cannot talk to the GPU paragraph measurer or
  report accurate metrics (baselines, line count, paragraph intrinsics).
- The node cannot request layout/draw/semantics invalidations when text/style changes, so we still
  rely on rebuilding the layout node.
- Accessibility lacks `SemanticsPropertyReceiver.text`, `getTextLayoutResult`, translation toggles,
  and text substitution support present in `TextStringSimpleNode`.

**Proper Fix Required:**
1. Mirror Kotlin’s `TextStringSimpleElement`: capture the full argument surface (string vs annotated
   string, style, `FontFamily.Resolver`, `TextOverflow`, `softWrap`, `minLines`, `maxLines`,
   optional `ColorProducer`, auto-size, placeholders).
2. Store a paragraph cache/renderer handle inside `TextModifierNode`, route measurement and drawing
   through the GPU renderer crates, and call `invalidateMeasurement()` / `invalidateDraw()` /
   `invalidateSemantics()` when properties change.
3. Replace the `content_description` fallback with real semantics properties (`text`,
   `getTextLayoutResult`, `isShowingTextSubstitution`, etc.).
4. Expand `Text` (and future `BasicText`) to expose the Kotlin API surface and pass those arguments
   through the modifier element.
5. Delete the runtime metadata hack once the semantics tree exposes `SemanticsRole::Text`
   naturally via `SemanticsModifierNode`.

**Reference Files:**
- `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt`
- `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/modifiers/TextStringSimpleNode.kt`
- GPU renderer text paths: `crates/compose-render/*`

---

## Work Remaining Before Full Parity

1. ✅ **Hook up the dispatch queues.**
2. ✅ **Delete the widget-specific node types.**
3. ✅ **Centralize resolved modifier data.**
4. ✅ **Make `Modifier::then` persistent.** (`ModifierKind` now mirrors `CombinedModifier`.)
5. ⚠️ **Leverage the persistent structure when reconciling.**
   - `ModifierChainHandle::update()` still clones flattened element/inspector vectors; rework it to
     walk the tree directly so modifier updates stay O(1) and inspector data stays shared.
6. ⚠️ **Drive layout/draw/pointer work through modifier nodes.** (In Progress - 40% complete)
   - ✅ `measure_through_modifier_chain()` foundation implemented (lines 640-705)
   - ✅ TextModifierNode measurement works through the chain
   - ❌ PaddingNode, SizeNode, OffsetNode don't implement `LayoutModifierNode::measure()` yet
   - ❌ Still need to remove `ResolvedModifiers` lookups from layout pipeline
   - ❌ Draw pipeline not started yet
7. ⚠️ **Finish the Text modifier pipeline.**
   - Mirror `TextStringSimpleNode`’s parameters, hook up GPU paragraph measurement/draw, provide
     semantics + baselines, and delete the metadata special cases.
8. **Add real integration coverage.**
   - Pointer/focus tests should drive events through `HitTestTarget` and validate modifier-driven
     behavior end to end once the pipelines run modifier nodes directly.

---

## Jetpack Compose References

| Area | Kotlin Source | Compose-RS Target |
| --- | --- | --- |
| Modifier API | `androidx/compose/ui/Modifier.kt` | `crates/compose-ui/src/modifier/mod.rs` |
| Modifier node chain | `ModifierNodeElement.kt`, `DelegatableNode.kt` | `crates/compose-foundation/src/modifier.rs` |
| Layout modifier execution | `androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt` | `crates/compose-ui/src/layout/mod.rs`, `crates/compose-ui/src/layout/modifier_measurable.rs` |
| Text modifier nodes | `foundation/text/modifiers/TextStringSimpleNode.kt` | `crates/compose-ui/src/text_modifier_node.rs` |
| Text widget | `foundation/text/BasicText.kt` | `crates/compose-ui/src/widgets/text.rs` |
| Layout modifier interface | `ui/layout/LayoutModifier.kt` | `crates/compose-foundation/src/modifier.rs` (LayoutModifierNode) |
| Pointer input | `ui/input/pointer/*` | `crates/compose-ui/src/modifier/pointer_input.rs` |
| Focus system | `FocusInvalidationManager.kt`, `FocusOwner.kt` | `crates/compose-ui/src/modifier/focus.rs` + dispatch managers |
| Semantics | `semantics/*` | `crates/compose-ui/src/semantics` |

Keep this document current so reviewers can see exactly which Kotlin contracts are satisfied.
