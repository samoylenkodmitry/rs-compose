# Modifier System Migration Tracker

## Status: ⚠️ Layout modifier coordinators exist but rehydrate built-ins each pass, skip unknown layout nodes, and have stub placement; draw/pointer/semantics still run off flattened `ResolvedModifiers`.

## Baseline (useful context)

- Modifier chain uses `ModifierKind::Combined`; reconciliation via `ModifierChainHandle` feeds a node
  chain with capability tracking.
- Widgets emit `LayoutNode` + `MeasurePolicy`; dispatch queues keep pointer/focus flags in sync.
- Built-in layout modifier nodes exist (padding/size/fill/offset/text).
- Layout measurement goes through a coordinator chain
  (`crates/compose-ui/src/layout/mod.rs:725`), but `LayoutModifierCoordinator` downcasts to
  built-ins and recreates them (`crates/compose-ui/src/layout/coordinator.rs:175`) instead of
  calling live nodes; placement is stubbed (`coordinator.rs:164`) and padding/offset is still
  accumulated separately as a workaround.

## Architecture Overview

- **Widgets**: Pure composables emitting `LayoutNode`s with policies.
- **Modifier chain**: Builders chain via `then`; reconciliation still flattens into element vectors
  and a `ResolvedModifiers` snapshot for layout/draw.
- **Measure pipeline**: Coordinator chain exists but rehydrates built-ins and skips unknown layout
  modifiers; falls back to `ResolvedModifiers` when no layout nodes.
- **Text**: `TextModifierElement` stores only a `String`; measure uses monospaced stub, draw empty,
  semantics use `content_description`, no invalidations on update.
- **Invalidation**: Capability flags exist but layout/draw/semantics invalidations mostly come from
  the flattened snapshot.

## Jetpack Compose reference cues (what we’re missing)

- `LayoutModifierNodeCoordinator` keeps a persistent handle to the live modifier node and measures
  it directly instead of cloning (`.../LayoutModifierNodeCoordinator.kt:37-195`).
- The same coordinator drives placement/draw/alignment and wraps the next coordinator for ordering
  (`LayoutModifierNodeCoordinator.kt:240-280`), so per-node state and lookahead are preserved.
- Node capabilities are honored per phase; draw/pointer/semantics are dispatched through the node
  chain rather than flattened snapshots.

## Unacceptable Gaps

### Modifier chain still flattened
- `ModifierChainHandle::update()` clones element/inspector vectors and rebuilds
  `ResolvedModifiers` every recomposition (`crates/compose-ui/src/modifier/chain.rs:71`), so the
  persistent `ModifierKind` tree is discarded; stacked properties collapse (padding summed,
  backgrounds last-write wins) and invalidations do not reuse node state.

### Coordinator chain gaps
- Only built-in padding/size/fill/offset/text measured; custom/stateful layout modifiers skipped,
  and nodes rehydrated each pass (`crates/compose-ui/src/layout/coordinator.rs:141,175`).
- Placement/draw/pointer/semantics/lookahead coordinators absent; runtime still depends on
  `ResolvedModifiers` snapshots and manual padding/offset accumulation.

### Text pipeline gap
- `TextModifierElement` is string-only (`crates/compose-ui/src/text_modifier_node.rs:166`), measure
  uses monospaced stub, draw empty, semantics via `content_description`, no invalidations on update;
  widget surface lacks style/overflow/min/max lines, etc.

## Remaining Work (phased)

### Phase 1: Core Architecture & Layout (blocking)
- [ ] Generalize coordinator construction: iterate all `NodeCapabilities::LAYOUT` entries and wrap
  them without downcasting to padding/size/fill/offset/text adapters.
- [ ] Refactor `LayoutModifierCoordinator`: drop the `NodeKind` snapshot, hold the live node handle
  from `ModifierNodeChain`, and measure via `node.as_layout_node().measure(...)`.
- [ ] Implement placement: accumulate offsets and propagate `place` through the coordinator chain.

### Phase 2: Rendering Pipeline & Visual Correctness
- [ ] Invalidate render on any tree structure change, not just constraint changes.
- [ ] Deduplicate click handling: remove legacy handler extraction and use
  `modifier_slices.click_handlers`.
- [ ] Render via modifier slices: iterate `modifier_slices.draw_commands` for draw order; stop using
  `ResolvedModifiers` for visuals; drop background/corner_shape/graphics_layer from `ResolvedModifiers`.

### Phase 3: Text Integration (node-driven)
- [ ] Implement `TextModifierNode::draw` to emit the renderer’s expected draw ops.
- [ ] Remove `content_description` fallback for text semantics; populate semantics from the text
  modifier node.

### Phase 4: Input & Focus Wiring
- [ ] Instantiate and process `FocusManager` invalidations in the app shell.
- [ ] Switch demo input to `Modifier.pointer_input((), handler)` so pointer flows through nodes.
- [ ] Expose a debug API like `Modifier.debug_chain(true)` for inspector logging.

### Phase 5: Standardization
- [ ] Deprecate static modifier factories; standardize on `Modifier::empty().foo(...)` chaining.
- [ ] Add integration tests for pointer/focus/text through `HitTestTarget`; run `cargo test > 1.tmp 2>&1`.

## References

- Kotlin modifier pipeline: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/Modifier.kt`
- Node coordinator chain: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`
- Text reference: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt`
  and `.../text/modifiers/TextStringSimpleNode.kt`
- Detailed parity checklist: [`modifier_match_with_jc.md`](modifier_match_with_jc.md)
