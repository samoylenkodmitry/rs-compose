# Modifier System Migration Tracker

## Status: ⚠️ Persistent modifiers landed, node pipeline still bypassed

The workspace now exposes `ModifierKind`-backed persistent composition and every widget goes through
`LayoutNode` + `MeasurePolicy`. However, modifier nodes are still collapsed into `ResolvedModifiers`
and the layout/draw pipeline never executes them. Text relies on a monospaced stub and semantics
still hinge on `content_description`. The next iteration must focus on wiring the node pipeline and
bringing `Text` closer to Jetpack Compose’s `BasicText`.

## Completed Work

1. ✅ **Dispatch queues integrated.** `AppShell::run_dispatch_queues`
   (`crates/compose-app-shell/src/lib.rs#L237-L275`) now drains pointer/focus invalidations so the
   capability flags on `LayoutNode` match Jetpack Compose’s lifecycle.
2. ✅ **Legacy widget nodes deleted.** `Button`, `Text`, and `Spacer` all emit `LayoutNode` +
   `MeasurePolicy`. The bespoke `measure_text`/`measure_button` helpers were removed.
3. ✅ **Centralized modifier reconciliation.** `ModifierNodeChain` and `ModifierChainHandle`
   reconcile node instances with capability tracking and modifier locals.
4. ✅ **Persistent `Modifier::then`.** `ModifierKind::Combined` provides the same persistent tree as
   Kotlin’s `CombinedModifier` (`crates/compose-ui/src/modifier/mod.rs:235-382`).

## Architecture Overview

- **Widgets**: Pure composables that emit `LayoutNode`s with policies such as `EmptyMeasurePolicy`,
  `LeafMeasurePolicy`, `FlexMeasurePolicy`, etc.
- **Modifier chain**: Public builders now chain via `self.then(...)`, but the runtime still flattens
  the tree into `Vec<DynModifierElement>` each update (`crates/compose-ui/src/modifier/chain.rs:72-95`)
  and collapses nodes into `ResolvedModifiers`.
- **Measure pipeline**: `measure_layout_node()` reads padding/size/offset from
  `resolved_modifiers` (`crates/compose-ui/src/layout/mod.rs:725-880`) and falls back to a
  `try_measure_with_layout_modifiers()` hack that only recognizes `TextModifierNode`.
- **Text**: `Text()` adds `TextModifierElement` + `EmptyMeasurePolicy`, but the element only stores a
  `String` and the node delegates to the monospaced fallback in `crates/compose-ui/src/text.rs`.
- **Invalidation**: Capability-based invalidations exist, yet modifier nodes rarely trigger them
  because we never call into nodes during layout/draw.

## Known Shortcuts

### Modifier chain still flattened each recomposition
- `ModifierChainHandle::update()` allocates fresh element/inspector vectors and rebuilds
  `ResolvedModifiers` on every pass (`crates/compose-ui/src/modifier/chain.rs:72-231`), so the
  persistent tree never reaches the runtime.
- Jetpack Compose walks the `CombinedModifier` tree directly and never materializes a snapshot struct.

### Layout/draw/pointer pipelines bypass nodes
- `measure_layout_node()` manipulates constraints strictly via `ResolvedModifiers`, while the new
  `ModifierNodeMeasurable` wrapper (`crates/compose-ui/src/layout/modifier_measurable.rs`) is unused.
- `try_measure_with_layout_modifiers()` special-cases `TextModifierNode` and ignores every other
  `LayoutModifierNode`.
- Padding/background/offset logic still lives in `ResolvedModifiers`, so modifier nodes cannot own
  those behaviors.

### Text modifier pipeline gap
- `TextModifierElement` only takes a `String`
  (`crates/compose-ui/src/text_modifier_node.rs:167-205`), so style/overflow/font resolver/minLines/
  maxLines cannot flow to the node.
- `TextModifierNode::measure()` delegates to the monospaced stub in `crates/compose-ui/src/text.rs`
  and never talks to the GPU paragraph cache. `draw()` is empty and semantics only set
  `content_description`, which is later scraped in `runtime_metadata_for()`.
- The widget signature (`crates/compose-ui/src/widgets/text.rs:125-143`) exposes neither style nor
  callbacks like `onTextLayout`, so parity with Jetpack Compose’s `BasicText` isn’t possible yet.

## Remaining Work

### 1. Build the layout modifier coordinator chain
- In `measure_layout_node()` (`crates/compose-ui/src/layout/mod.rs:725-880`), walk the reconciled
  modifier chain (`layout_node.modifier_chain()`) to collect every `LayoutModifierNode`.
- Create a `ContentMeasurable` that wraps the `MeasurePolicy` + child measurables. Then wrap it in
  `ModifierNodeMeasurable` instances (outermost → innermost) so each node can call
  `measure()`/intrinsics on the wrapped measurable, mirroring
  `androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`.
- Replace `try_measure_with_layout_modifiers()` with this chain. Once the chain works, delete the
  text-only special case and feed placement data back into `MeasureResult`.

### 2. Remove `ResolvedModifiers` from layout/draw
- Move padding/size/offset/background behavior into modifier nodes (PaddingNode, SizeNode, etc.)
  instead of aggregating them in `ModifierChainHandle::compute_resolved()`.
- Update layout/draw/pointer pipelines to inspect the reconciled node chain directly so there is no
  shadow snapshot to keep in sync. `ResolvedModifiers` should eventually disappear.

### 3. Finish the Text modifier pipeline
- Expand `TextModifierElement` to carry the same arguments as Kotlin’s `TextStringSimpleElement`
  (string vs annotated string, `TextStyle`, `FontFamily.Resolver`, `TextOverflow`, softWrap,
  minLines/maxLines, `ColorProducer`, auto-size hooks, placeholders, selection controller).
- Store a paragraph cache / GPU renderer handle inside `TextModifierNode`, call into the GPU
  renderer crates for measurement + draw, and issue `invalidateMeasurement`/`invalidateDraw`/
  `invalidateSemantics` whenever text/style/metrics change.
- Implement real semantics (`text`, `getTextLayoutResult`, text substitution toggles) instead of the
  `content_description` placeholder, then remove the metadata hack in
  `crates/compose-ui/src/layout/mod.rs:1633-1655`.
- Update `Text` (and later `BasicText`) to expose Jetpack Compose’s API surface and plumb arguments
  through the modifier element.

### 4. Testing & integration cleanup
- Expand pointer/focus integration tests so they dispatch events through `HitTestTarget` and observe
  modifier-driven behavior instead of counting nodes.
- After each major change, run `cargo test > 1.tmp 2>&1` and inspect the log before iterating.

## References

- Kotlin modifier pipeline: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/Modifier.kt`
- Node coordinator chain: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`
- Text reference: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt`
  and `.../text/modifiers/TextStringSimpleNode.kt`
- Detailed parity checklist: [`modifier_match_with_jc.md`](modifier_match_with_jc.md)
