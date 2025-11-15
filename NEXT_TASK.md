# Modifier System Migration Tracker

## Status: ⚠️ MOSTLY COMPLETE (with known shortcuts)

The modifier system migration has made significant progress. All widgets (`Button`, `Text`, `Spacer`)
now use `LayoutNode` and the modifier chain reconciliation system is functional with capability-based
invalidation. However, several shortcuts keep us from real Jetpack Compose parity:

- `Modifier::then` still clones the entire element vector every time, so modifier composition is
  `O(n²)` instead of the persistent `CombinedModifier` structure Kotlin uses.
- The reconciled modifier node chain is only consulted to build `ResolvedModifiers`; the layout/draw
  pipeline never invokes `LayoutModifierNode::measure` or `DrawModifierNode::draw` (Text works only
  because we special-case its modifier node).
- **Text modifier nodes remain skeletal.** `Text()` now uses `EmptyMeasurePolicy` + `TextModifierElement`,
  but measurement/draw/semantics rely on a GPU renderer shortcut, the node cannot invalidate itself,
  and layout still has to special-case Text instead of running modifier nodes in order.

See "Known Shortcuts" for the outstanding work.

## Completed Work

1. ✅ **Wire the new dispatch queues into the host/runtime.** The app shell now
   calls `process_pointer_repasses` and `process_focus_invalidations` during frame processing
   (see [AppShell::run_dispatch_queues](crates/compose-app-shell/src/lib.rs#L237-L275)). Nodes
   that mark `needs_pointer_pass` / `needs_focus_sync` now have those flags cleared by the
   runtime, completing the invalidation cycle similar to Jetpack Compose's FocusInvalidationManager.

2. ✅ **Remove the legacy widget-specific nodes.** All widgets now use `LayoutNode`:
   - **Spacer** → `LayoutNode` with `LeafMeasurePolicy`
   - **Text** → `LayoutNode` with `EmptyMeasurePolicy` + `TextModifierElement` (GPU renderer still
     handles the real glyph rendering/measuring)
   - **Button** → `LayoutNode` with `FlexMeasurePolicy::column`

   Legacy `ButtonNode`, `TextNode`, and `SpacerNode` types have been deleted.

3. ✅ **Stop rebuilding modifier snapshots ad-hoc.** All modifier resolution now happens through
   the reconciled `ModifierNodeChain`. The legacy `measure_spacer`, `measure_text`, and
   `measure_button` functions that called `Modifier::empty().resolved_modifiers()` have been
   removed. All measurement goes through the unified `measure_layout_node` path.

4. ✅ **Remove metadata fallbacks.** The `runtime_metadata_for` and `compute_semantics_for_node`
   functions no longer special-case legacy node types. They only handle `LayoutNode` and
   `SubcomposeLayoutNode`, ensuring consistent modifier chain traversal.

## Architecture Overview

The codebase mostly follows Jetpack Compose's modifier system design:

- **Widgets as Composables**: `Button`, `Text`, `Spacer` are pure composable functions
- **LayoutNode-based**: All widgets emit `LayoutNode` with appropriate `MeasurePolicy`
- **Measure Policies**:
  - `EmptyMeasurePolicy` - used by Text; relies on `TextModifierNode` + the GPU renderer for real text
    measurement/drawing (still missing invalidation + semantics parity)
  - `LeafMeasurePolicy` - for leaf nodes with fixed intrinsic size
  - `FlexMeasurePolicy` - for row/column layouts (used by Button)
  - `BoxMeasurePolicy` - for box layouts
- **Modifier Chain**:
  - Modifiers reconcile through `ModifierNodeChain`
  - ⚠️ `Modifier::then` clones both the element list and inspector metadata every call instead of
    composing persistently
  - ⚠️ Layout/draw code reads `ResolvedModifiers` snapshots rather than executing the reconciled nodes
- **Invalidation**: Capability-based invalidation (layout, draw, pointer, focus, semantics)

## Known Shortcuts

### Modifier::then Copies the Chain
- `Modifier::then` allocates new `Vec`s for both elements and inspector data each time we append.
- Building a chain like `Modifier.padding().background().clickable()` repeatedly clones previous
  entries, making recomposition slower than Kotlin's `CombinedModifier` structure.
- **Fix:** Mirror Jetpack Compose's persistent composition so `then` is `O(1)` and sharing is
  preserved.

### Modifier Nodes Never Participate in Layout/Draw
- `ModifierChainHandle::compute_resolved` downcasts a shortlist of node types and copies their data
  into `ResolvedModifiers`. The layout pipeline then reads padding/size/background from that struct
  and never calls `LayoutModifierNode::measure`, `DrawModifierNode::draw`, etc.
- Custom modifiers (or even built-in ones outside the shortlist) cannot affect measurement, drawing,
  pointer input, or semantics.
- **Fix:** Thread the reconciled node chain through layout/draw/pointer dispatch so capability
  interfaces actually run, then delete the `ResolvedModifiers` shadow copy.

### Text Modifier Pipeline (Architecture Mismatch)

**Current Implementation:**
- `Text()` composes `Layout(modifier_then_text_element, EmptyMeasurePolicy, || {})`.
- `measure_layout_node` looks for `TextModifierNode` and calls its `measure` directly instead of
  executing every `LayoutModifierNode` in the chain.
- `TextModifierNode` stores the string and asks the shared text-metrics service (currently backed by
  a monospaced fallback) for width/height. Actual glyph measurement/drawing happens inside the GPU
  renderer crates, not inside the modifier node.
- `draw()` and semantics remain placeholders (content description only).

**Problem:**
- The modifier node cannot request layout/draw/semantics invalidations, forcing us to rebuild the
  layout node when text changes.
- Layout/draw still bypass the reconciled chain, so Text remains a one-off in the engine.
- Without exposing the GPU renderer's paragraph cache to `TextModifierNode`, we cannot report
  baselines, selection info, or accurate typography metrics like Jetpack Compose's
  `ParagraphLayoutCache`.

**How Jetpack Compose Does It:**
```kotlin
Layout(modifier.then(TextStringSimpleElement(...)), EmptyMeasurePolicy)
```
`TextStringSimpleNode` implements `LayoutModifierNode`, `DrawModifierNode`, and
`SemanticsModifierNode` and talks directly to `ParagraphLayoutCache`.

**Proper Fix:**
1. Execute modifier nodes generically so Text no longer needs a special path in `measure_layout_node`.
2. Let `TextModifierElement::update` schedule invalidations when text/style changes.
3. Integrate the GPU renderer's paragraph measurement + draw commands so the modifier node produces
   real metrics (and forwards draw instructions) instead of using a monospaced stub.
4. Implement Jetpack Compose–style semantics hooks (`text`, `getTextLayoutResult`, translation
   toggles) and baselines.

**Reference:**
- See [modifier_match_with_jc.md](modifier_match_with_jc.md) for detailed architecture comparison.
- JC source: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/modifiers/TextStringSimpleNode.kt`.
- GPU renderer text paths: `crates/compose-render/*`.

## Remaining Work

### Make Modifier Composition Persistent
- Introduce a `CombinedModifier`-style representation so `Modifier::then` is `O(1)` and retains
  sharing between recompositions.
- Ensure `fold_in`, `fold_out`, `any`, and `all` traverse the combined structure correctly.

### Execute Modifier Nodes During Layout/Draw
- Run `LayoutModifierNode::measure`, `DrawModifierNode::draw`, pointer, semantics, and focus
  callbacks in the reconciled order instead of mirroring data into `ResolvedModifiers`.
- Delete the ad-hoc padding/size/background accumulation once the real nodes power layout.

### Critical: Finish the Text Modifier Pipeline
**Priority: High** - Required for true Jetpack Compose parity

We now have a modifier-based Text, but it still relies on shortcuts:
1. Remove the layout special-case by executing modifier chains so `LayoutModifierNode::measure`
   runs for every node (Text included).
2. Teach `TextModifierElement::update` how to request layout/draw/semantics invalidations when
   text/style/metrics change.
3. Hook `TextModifierNode` up to the GPU renderer's paragraph measurement + draw APIs so it can
   deliver real metrics (baselines, multi-line sizes) and enqueue draw commands instead of the
   monospaced fallback.
4. Flesh out semantics: expose `text`, `getTextLayoutResult`, translation toggles, and selection
   hooks just like `TextStringSimpleNode`.
5. Add regression tests that validate modifier-driven measurement/draw once the GPU-backed pipeline
   is wired up.

### Testing
- ✅ Legacy node tests marked as `#[ignore]` and stubbed (need rewrite using semantics/layout tree)
- Integration tests for pointer/focus events should be expanded to verify end-to-end behavior
- Add tests for text modifier node once implemented

### Future Enhancements
- Additional measure policies for more complex layouts
- Performance optimization of modifier chain reconciliation (goes away once `then` is persistent)
- More comprehensive integration tests

## References

See [modifier_match_with_jc.md](modifier_match_with_jc.md) for the original migration plan
and Jetpack Compose behavioral parity requirements.
