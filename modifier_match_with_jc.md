# Modifier Migration Reality Check

The modifier API surface is moving in the right direction (builder helpers now chain via
`self.then`, `ModifierNodeChain` has capability tracking, and the node-backed factories live in
`crates/compose-ui/src/modifier_nodes.rs`). This document records the current status and remaining
gaps before we can claim full parity with Jetpack Compose.

---

## Current Snapshot

- ✅ `ModifierNodeChain` reconciliation, capability masks, and helper macros exist in
  `crates/compose-foundation/src/modifier.rs` and are used by the built-in nodes under
  `crates/compose-ui/src/modifier_nodes.rs`.
- ✅ Public modifier builders (padding/background/fill/etc.) now consume `self` and use `then(...)`
  so callers can fluently chain them without reaching for ad-hoc constructors.
- ✅ Pointer/focus invalidation managers (`crates/compose-ui/src/pointer_dispatch.rs` and
  `crates/compose-ui/src/focus_dispatch.rs`) are now invoked by the app shell runtime during frame
  processing. The `process_pointer_repasses` / `process_focus_invalidations` functions are called
  in `AppShell::run_dispatch_queues()`, and the `needs_pointer_pass` / `needs_focus_sync` flags on
  `LayoutNode` are properly cleared after processing, matching Jetpack Compose's invalidation pattern.
- ✅ **Legacy widget-specific nodes removed.** `ButtonNode`, `TextNode`, and `SpacerNode` have been
  deleted. All widgets now use `LayoutNode` with appropriate measure policies.
- ✅ **Centralized modifier resolution.** The legacy `measure_spacer`, `measure_text`, and
  `measure_button` functions that rebuilt modifiers via `Modifier::empty().resolved_modifiers()` have
  been removed. All measurement goes through the unified `measure_layout_node` path.
- ✅ **Metadata fallbacks removed.** `runtime_metadata_for` and `compute_semantics_for_node` only
  handle `LayoutNode` and `SubcomposeLayoutNode`, ensuring consistent modifier chain traversal.
- ⚠️ **Modifier::then copies chains eagerly.** Each call to `Modifier::then` clones both element and
  inspector vectors (`crates/compose-ui/src/modifier/mod.rs:333-347`), so building long chains is
  `O(n²)` and loses the structural sharing provided by Jetpack Compose's `CombinedModifier`.
- ⚠️ **Modifier nodes never execute.** `ModifierChainHandle::compute_resolved` simply downcasts known
  node types and copies their fields into `ResolvedModifiers`
  (`crates/compose-ui/src/modifier/chain.rs:160-205`); the measure/draw pipeline never calls
  `LayoutModifierNode::measure` or `DrawModifierNode::draw`, so only hard-coded nodes affect layout.
  Text currently works only because `measure_layout_node` special-cases `TextModifierNode` and bypasses
  the rest of the chain.
- ⚠️ **Text modifier node still skeletal.** `Text()` now installs `TextModifierElement` +
  `EmptyMeasurePolicy`, but `TextModifierElement::update` still cannot request invalidations,
  `TextModifierNode::draw` is empty, semantics only populate `content_description`, and real glyph
  measurement/drawing is delegated to the GPU renderer crates. Until modifier nodes can talk to that
  external paragraph measurer/cache we cannot match Jetpack Compose's behavior.
- ⚠️ Tests under `crates/compose-ui/src/tests/pointer_input_integration_test.rs` simply assert node
  counts; no integration test actually drives pointer events through `HitTestTarget`.

---

## Known Shortcuts

### Modifier Chain Efficiency

**Current Behavior:**
- `Modifier::then` clones the entire element vector on every call.
- Inspector metadata `Vec` is cloned alongside, forcing new allocations even when two modifiers are
  already shared.

**Problem:**
- Building a modifier like `Modifier.padding().background().clickable()...` copies the entire chain
  each time, which is far more expensive than Jetpack Compose's persistent `CombinedModifier`.
- Structural sharing is lost, so equality checks fall back to pointer equality and large allocations
  surface during recomposition.

**Reference:** `crates/compose-ui/src/modifier/mod.rs:333-347`.

**Desired Fix:**
- Mirror Kotlin's `CombinedModifier` tree: keep a lightweight node that references an outer and inner
  modifier rather than rebuilding vectors.
- Preserve sharing so `Modifier.then` stays `O(1)` and fold/any/all traversals work against a stable
  structure.

### Modifier Nodes Bypassed

**Current Behavior:**
- `ModifierChainHandle::update` reconciles nodes but the layout/rendering pipeline immediately
  collapses them into `ResolvedModifiers` by downcasting known types.
- Measuring a `LayoutNode` only reads the resolved padding/size/offset fields
  (`crates/compose-ui/src/layout/mod.rs:677-811`); it never asks modifier nodes to measure/draw.

**Problem:**
- Custom modifier nodes (and even built-in ones) cannot influence measurement or drawing unless they
  are explicitly mirrored inside `compute_resolved`, which defeats the purpose of the node system.
- Pointer/focus/semantics nodes that declare capabilities never run, so the architecture still
  behaves like the pre-node "resolved property bag" system.

**Desired Fix:**
1. Thread the reconciled `ModifierNodeChain` through measurement/draw so `LayoutModifierNode::measure`
   and `DrawModifierNode::draw` are invoked in order.
2. Remove the `ResolvedModifiers` downcasting shortcut once the real pipeline is in place.

### Text Implementation Architecture Mismatch

**Current (Shortcut) Implementation:**
- `Text()` installs a modifier node (`TextModifierElement`) and uses `EmptyMeasurePolicy`.
- `measure_layout_node` searches for `TextModifierNode` and calls its `measure` implementation instead
  of executing the whole chain.
- `TextModifierNode` stores only the string, measures it through the shared text metrics service, and
  leaves `draw()`/semantics mostly empty.
- Actual glyph measurement + drawing live in the GPU renderer crates (`compose-render/*`). The text
  modifier does not yet push structured instructions to that renderer, so measurement/draw/semantics
  rely on a monospaced fallback rather than the GPU library.

**Problem:**
- The modifier node can’t request invalidations, baselines, or semantics beyond
  `content_description`, so recomposition must rebuild the layout node to reflect text changes.
- The layout pipeline still has to special-case Text because no generic modifier-node execution exists.
- We do not expose the GPU paragraph cache to modifier nodes, so we can neither match Kotlin’s
  `ParagraphLayoutCache` behavior nor report accurate typography metrics.

**Jetpack Compose Architecture:**
```kotlin
// In BasicText.kt
Layout(finalModifier, EmptyMeasurePolicy)

// Where finalModifier includes:
TextStringSimpleElement(text, style, ...) // Creates TextStringSimpleNode
```

**TextStringSimpleNode** implements:
- `LayoutModifierNode` - for measurement
- `DrawModifierNode` - for drawing
- `SemanticsModifierNode` - for semantics

Text content lives in the **modifier node** which talks to `ParagraphLayoutCache`.

**Proper Fix Required:**
1. Keep `Text()` modifier-based but run modifier nodes generically so Text no longer needs bespoke
   handling inside `measure_layout_node`.
2. Let `TextModifierElement::update` request layout/draw/semantics invalidations.
3. Integrate the GPU paragraph measurer/renderer with `TextModifierNode` so measurement and draw
   reuse the external cache instead of a stub.
4. Implement baselines + full semantics (`text`, `getTextLayoutResult`, translation toggles).
5. Remove any leftover MeasurePolicy shortcuts once modifier nodes own the content pipeline.

**Reference Files:**
- `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt`
- `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/modifiers/TextStringSimpleNode.kt`
- GPU renderer text paths under `crates/compose-render/*` (metrics + draw submitted from Rust side)

---

## Work Remaining Before Full Parity

1. ✅ **COMPLETED: Hook up the dispatch queues.**
2. ✅ **COMPLETED: Delete the widget-specific node types.**
3. ✅ **COMPLETED: Centralize resolved modifier data.**
4. ⚠️ **Make `Modifier` composition persistent.**
   - Reintroduce a `CombinedModifier`-style structure so `then` is `O(1)`.
   - Ensure fold/any/all traverse the structure without cloning vectors.
5. ⚠️ **Drive layout/draw/pointer work through modifier nodes.**
   - Invoke `LayoutModifierNode::measure`, `DrawModifierNode::draw`, and other capability-specific
     hooks instead of relying on the `ResolvedModifiers` snapshot.
   - Remove the hard-coded padding/size/background aggregation once nodes run the pipeline.
6. ⚠️ **Finish the Text modifier pipeline.**
   - Remove the layout special-case by executing modifier chains directly.
   - Allow `TextModifierElement::update` to trigger invalidations so text/style changes flow without
     rebuilding layout nodes.
   - Bridge the GPU renderer’s paragraph measurement/draw APIs into `TextModifierNode` so we expose
     accurate metrics/semantics similar to `ParagraphLayoutCache`.
7. **Add real integration coverage.**
   - Extend the pointer/focus tests to synthesize events through `HitTestTarget` so we can verify
     suspending pointer handlers, `Modifier.clickable`, and focus callbacks operate end-to-end.

---

## Jetpack Compose References

Use these upstream files while implementing the remaining pieces:

| Area | Kotlin Source | Compose-RS Target |
| --- | --- | --- |
| Modifier API | `androidx/compose/ui/Modifier.kt` | `crates/compose-ui/src/modifier/mod.rs` |
| Node lifecycle | `ModifierNodeElement.kt`, `DelegatableNode.kt` | `crates/compose-foundation/src/modifier.rs` |
| Text modifier nodes | `foundation/text/modifiers/TextStringSimpleNode.kt` | `crates/compose-ui/src/text_modifier_node.rs` |
| Text widget | `foundation/text/BasicText.kt` | `crates/compose-ui/src/widgets/text.rs` |
| Layout modifier | `ui/layout/LayoutModifier.kt` | `crates/compose-foundation/src/modifier.rs` (LayoutModifierNode) |
| Pointer input | `ui/input/pointer/*` | `crates/compose-ui/src/modifier/pointer_input.rs` |
| Focus system | `FocusInvalidationManager.kt`, `FocusOwner.kt` | `crates/compose-ui/src/modifier/focus.rs` + dispatch managers |
| Semantics | `semantics/*` | `crates/compose-ui/src/semantics` |

Keep this document up to date as we chip away at the remaining tasks so reviewers can clearly see
which parts of the Kotlin contract are satisfied.
