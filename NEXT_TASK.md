# Modifier System Migration Tracker

## Status: ⚠️ Adapter walk exists, but NodeCoordinator-style chaining is missing

`measure_through_modifier_chain()` now collects built-in layout nodes (padding/size/fill/offset/text)
and wraps the `MeasurePolicy` with temporary adapters, yet any unknown `LayoutModifierNode` forces a
fallback to `ResolvedModifiers`. The adapters re-instantiate fresh nodes each measure with a fresh
`BasicModifierNodeContext`, so invalidations/caches never reach `LayoutNode`, there is no
NodeCoordinator equivalent for draw/pointer/lookahead, and Text remains a string-only stub with
monospaced measurement, empty draw, and placeholder semantics.

## Completed Work

1. ✅ **Dispatch queues integrated.** `AppShell::run_dispatch_queues`
   (`crates/compose-app-shell/src/lib.rs#L237-L275`) now drains pointer/focus invalidations so the
   capability flags on `LayoutNode` match Jetpack Compose's lifecycle.
2. ✅ **Legacy widget nodes deleted.** `Button`, `Text`, and `Spacer` all emit `LayoutNode` +
   `MeasurePolicy`; bespoke `measure_*` helpers are gone.
3. ✅ **Centralized modifier reconciliation.** `ModifierNodeChain` + `ModifierChainHandle` reconcile
   node instances with capability tracking and modifier locals.
4. ✅ **Persistent `Modifier::then`.** `ModifierKind::Combined` mirrors Kotlin’s `CombinedModifier`
   (`crates/compose-ui/src/modifier/mod.rs:235-382`).
5. ✅ **Layout modifier node implementations.** Padding/Size/Offset/Fill nodes expose full measure +
   intrinsic logic (`crates/compose-ui/src/modifier_nodes.rs`), even though the pipeline doesn’t call
   them yet.

## Architecture Overview

- **Widgets**: Pure composables emitting `LayoutNode`s with policies (Empty/Flex/etc.).
- **Modifier chain**: Builders chain via `self.then(...)`, flatten into `Vec<DynModifierElement>`
  for reconciliation, and also collapse into a `ResolvedModifiers` snapshot for layout/draw.
- **Measure pipeline**: `measure_through_modifier_chain()` downcasts to built-in layout nodes and
  builds new adapters each pass (`crates/compose-ui/src/layout/mod.rs:953-1062`). Any unknown/custom
  `LayoutModifierNode` triggers a fallback to `ResolvedModifiers`, and the adapters never reuse
  `ModifierNode` state or invalidations.
- **Text**: `Text()` adds `TextModifierElement` + `EmptyMeasurePolicy`; the element stores only a
  `String`, the node measures via the monospaced stub in `crates/compose-ui/src/text.rs`, `draw()` is
  empty, `update()` cannot invalidate on text changes, and semantics just set `content_description`.
- **Invalidation**: Capability flags exist, but layout/draw/semantics invalidations come from the
  resolved snapshot rather than node-driven updates.

## Known Shortcuts

### Modifier chain still flattened each recomposition
- `ModifierChainHandle::update()` allocates fresh element/inspector vectors and rebuilds
  `ResolvedModifiers` on every pass (`crates/compose-ui/src/modifier/chain.rs:72-231`), so the
  persistent tree never reaches the runtime. Kotlin walks the `CombinedModifier` tree directly.

### Layout modifier nodes bypassed
- `measure_through_modifier_chain()` only recognizes built-in nodes and rebuilds temporary adapters
  (`crates/compose-ui/src/layout/mod.rs:953-1062`); custom layout modifiers fall back to
  `ResolvedModifiers` and reconciled nodes never receive real `ModifierNodeContext` calls. Any
  invalidations during adapter measurement are lost because the context is throwaway.
- `ModifierChainHandle::compute_resolved()` sums padding and overwrites later properties into a
  `ResolvedModifiers` snapshot (`crates/compose-ui/src/modifier/chain.rs:173-219`); stacked
  modifiers lose ordering and “last background wins.”
- `measure_layout_node()` still mutates constraints/offsets from that snapshot when the adapter walk
  bails out, there is no NodeCoordinator to share layout results with draw/pointer/semantics, and
  there is no lookahead/approach measurement hook.

### Text modifier pipeline gap
- `TextModifierElement` captures only a `String`
  (`crates/compose-ui/src/text_modifier_node.rs:167-205`) and cannot invalidate on updates; style/
  font resolver/overflow/softWrap/minLines/maxLines/color/auto-size/placeholders/selection cannot
  reach the node.
- `TextModifierNode` uses monospaced measurement, has an empty `draw()`, and only sets
  `content_description` semantics; Kotlin’s `TextStringSimpleNode` manages paragraph caches,
  baselines, text substitution, and `SemanticsPropertyReceiver.text` (`.../text/modifiers/TextStringSimpleNode.kt`).
- `TextStringSimpleNode` manually invalidates layout/draw/semantics around a `ParagraphLayoutCache`
  while `shouldAutoInvalidate` is false; our `update()` cannot issue invalidations because it never
  sees a live `ModifierNodeContext`.
- The widget API (`crates/compose-ui/src/widgets/text.rs:125-143`) exposes neither style nor
  callbacks like `onTextLayout`, so parity with `BasicText` isn’t possible.

## Remaining Work

### 1. Drive layout/draw/pointer through modifier nodes
- Build a `LayoutModifierNodeCoordinator`-style walk over `ModifierNodeChain`, wrapping measurables
  and invoking each reconciled `LayoutModifierNode` (not fresh adapters) in order, with placements
  and intrinsic queries, and wire those nodes to a real `ModifierNodeContext`.
- Surface draw/pointer/semantics nodes from the same chain, preserve modifier ordering for layers/
  clipping, and leave room for lookahead/approach measurement.
- Stop mutating constraints/offsets from `ResolvedModifiers` once nodes drive the pipeline and draw
  can consume the coordinator chain.

### 2. Preserve the persistent modifier tree during reconciliation
- Stop cloning intermediate element/inspector vectors; walk the `ModifierKind::Combined` tree
  directly when updating the chain/inspector snapshot so modifier updates stay O(1).

### 3. Finish the Text modifier pipeline
- Mirror Kotlin’s `TextStringSimpleElement` surface (style, `FontFamily.Resolver`, overflow,
  softWrap, minLines/maxLines, `ColorProducer`, auto-size, placeholders/selection hooks).
- Store a paragraph cache/renderer handle inside `TextModifierNode`, call into the existing external
  text renderer/paragraph library for measurement + draw, issue
  `invalidateMeasurement`/`invalidateDraw`/`invalidateSemantics` on updates (no auto invalidate), and
  expose real semantics (`text`, `getTextLayoutResult`, substitution toggles). Remove the runtime
  metadata fallback once semantics provide `SemanticsRole::Text`.
- Expand `Text`/`BasicText` to pass the full parameter set through the modifier element.

### 4. Testing & integration cleanup
- Add pointer/focus integration tests that dispatch through `HitTestTarget` instead of counting
  nodes; add text layout/draw/semantics assertions once the pipeline is wired.
- After each major change, run `cargo test > 1.tmp 2>&1` and inspect the log before iterating.

## References

- Kotlin modifier pipeline: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/Modifier.kt`
- Node coordinator chain: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`
- Text reference: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt`
  and `.../text/modifiers/TextStringSimpleNode.kt`
- Detailed parity checklist: [`modifier_match_with_jc.md`](modifier_match_with_jc.md)
