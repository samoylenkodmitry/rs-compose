# Modifier System Migration Tracker

## Status: ⚠️ Coordinator chain foundation in place, modifier nodes need measure() implementations

The workspace now has `measure_through_modifier_chain()` that properly accesses the reconciled modifier
chain and calls `TextModifierNode::measure()`. However, the existing modifier nodes (PaddingNode,
SizeNode, OffsetNode) do NOT implement `LayoutModifierNode::measure()` - they only aggregate data into
`ResolvedModifiers`. The next step is to make these nodes participate directly in measurement, then
remove the `ResolvedModifiers` fallback.

## Completed Work

1. ✅ **Dispatch queues integrated.** `AppShell::run_dispatch_queues`
   (`crates/compose-app-shell/src/lib.rs#L237-L275`) now drains pointer/focus invalidations so the
   capability flags on `LayoutNode` match Jetpack Compose's lifecycle.
2. ✅ **Legacy widget nodes deleted.** `Button`, `Text`, and `Spacer` all emit `LayoutNode` +
   `MeasurePolicy`. The bespoke `measure_text`/`measure_button` helpers were removed.
3. ✅ **Centralized modifier reconciliation.** `ModifierNodeChain` and `ModifierChainHandle`
   reconcile node instances with capability tracking and modifier locals.
4. ✅ **Persistent `Modifier::then`.** `ModifierKind::Combined` provides the same persistent tree as
   Kotlin's `CombinedModifier` (`crates/compose-ui/src/modifier/mod.rs:235-382`).
5. ✅ **Coordinator chain foundation.** `measure_through_modifier_chain()`
   (`crates/compose-ui/src/layout/mod.rs:640-705`) properly accesses the modifier chain via
   `ModifierChainHandle`, iterates through layout modifier nodes, and calls `TextModifierNode::measure()`.

## Architecture Overview

- **Widgets**: Pure composables that emit `LayoutNode`s with policies such as `EmptyMeasurePolicy`,
  `LeafMeasurePolicy`, `FlexMeasurePolicy`, etc.
- **Modifier chain**: Public builders now chain via `self.then(...)`, the runtime flattens into
  `Vec<DynModifierElement>` and reconciles into `ModifierNodeChain`, but also collapses nodes into
  `ResolvedModifiers` for legacy layout code.
- **Measure pipeline**: `measure_layout_node()` calls `measure_through_modifier_chain()` which
  iterates the reconciled chain and measures `TextModifierNode` directly. Other layout modifiers
  (padding/size/offset) still fall back to `ResolvedModifiers` because PaddingNode/SizeNode/etc.
  don't implement `LayoutModifierNode::measure()` yet.
- **Text**: `Text()` adds `TextModifierElement` + `EmptyMeasurePolicy`, the element only stores a
  `String` and the node delegates to the monospaced fallback in `crates/compose-ui/src/text.rs`.
- **Invalidation**: Capability-based invalidations exist, but modifier nodes rarely trigger them
  because only TextModifierNode participates in measurement currently.

## Known Shortcuts

### Modifier chain still flattened each recomposition
- `ModifierChainHandle::update()` allocates fresh element/inspector vectors and rebuilds
  `ResolvedModifiers` on every pass (`crates/compose-ui/src/modifier/chain.rs:72-231`), so the
  persistent tree never reaches the runtime.
- Jetpack Compose walks the `CombinedModifier` tree directly and never materializes a snapshot struct.

### Layout modifiers don't implement measure()
- `measure_through_modifier_chain()` properly iterates the chain and calls node `measure()` methods,
  BUT the existing modifier nodes (PaddingNode, SizeNode, OffsetNode, FillNode, etc. in
  `crates/compose-ui/src/modifier_nodes.rs`) do NOT implement `LayoutModifierNode::measure()`.
- These nodes only store data fields that get aggregated into `ResolvedModifiers` via
  `ModifierChainHandle::compute_resolved()` (`modifier/chain.rs:173-231`).
- This means the data exists in TWO places: in the modifier nodes (unused during measurement) and
  in `ResolvedModifiers` (actually used by `measure_layout_node`).
- To fix: each node type needs a `measure()` implementation that manipulates constraints/size,
  then `measure_through_modifier_chain()` needs to downcast and call them.

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

### 1. Implement LayoutModifierNode::measure() for existing modifier nodes

**Current State:**
- ✅ `measure_through_modifier_chain()` exists and works (lines 640-705)
- ✅ `TextModifierNode::measure()` is called correctly
- ❌ PaddingNode, SizeNode, OffsetNode, FillNode don't implement `LayoutModifierNode::measure()`

**What to do:**
In `crates/compose-ui/src/modifier_nodes.rs`, make these nodes implement `LayoutModifierNode`:

a) **PaddingNode** (already marked as LAYOUT capability):
   ```rust
   impl LayoutModifierNode for PaddingNode {
       fn measure(&mut self, _context: &mut dyn ModifierNodeContext,
                  measurable: &dyn Measurable, constraints: Constraints) -> Size {
           // Subtract padding from constraints
           let padding = self.padding();
           let inner_constraints = Constraints {
               min_width: (constraints.min_width - padding.horizontal_sum()).max(0.0),
               max_width: (constraints.max_width - padding.horizontal_sum()).max(0.0),
               min_height: (constraints.min_height - padding.vertical_sum()).max(0.0),
               max_height: (constraints.max_height - padding.vertical_sum()).max(0.0),
           };

           // Measure wrapped content
           let placeable = measurable.measure(inner_constraints);

           // Add padding back to size
           Size {
               width: placeable.width() + padding.horizontal_sum(),
               height: placeable.height() + padding.vertical_sum(),
           }
       }
   }
   ```

b) **SizeNode** (already marked as LAYOUT capability):
   - Similar pattern: constrain the max/min dimensions, measure wrapped content, apply size constraints

c) **OffsetNode** (already marked as LAYOUT capability):
   - Pass through to wrapped content (offset affects placement, not measurement)

d) **Update `measure_through_modifier_chain()`** to handle these nodes:
   ```rust
   // After the TextModifierNode check, add:
   else if let Some(padding_node) = node.as_any_mut().downcast_mut::<PaddingNode>() {
       current_size = padding_node.measure(&mut context, &wrapped, current_constraints);
   }
   else if let Some(size_node) = node.as_any_mut().downcast_mut::<SizeNode>() {
       current_size = size_node.measure(&mut context, &wrapped, current_constraints);
   }
   // ... etc for other node types
   ```

### 2. Remove `ResolvedModifiers` from layout pipeline

Once step 1 is complete and all layout modifiers measure through their nodes:

a) **Remove padding/size/offset logic from `measure_layout_node()`**:
   - Delete the code that reads `resolved_modifiers.padding()` (lines ~760-770)
   - Delete the code that reads `resolved_modifiers.layout_properties()` for size/offset
   - Delete the constraint manipulation based on `ResolvedModifiers`

b) **Stop computing layout-related ResolvedModifiers**:
   - In `ModifierChainHandle::compute_resolved()` (`modifier/chain.rs:173-231`), remove the code
     that aggregates PaddingNode, SizeNode, OffsetNode data
   - Keep background/corner_shape/graphics_layer for now (used by draw pipeline)

c) **Verify all tests still pass** after the migration

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
