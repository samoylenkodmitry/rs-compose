# Modifier System Status - In Progress

## ✅ What is done
- `LayoutModifierNode::measure` returns `LayoutModifierMeasureResult` with placement offsets, enabling padding/offset implementations to drive placement.
- `LayoutModifierCoordinator` measures nodes directly via `Rc<RefCell<Box<dyn ModifierNode>>>` and applies the captured placement offset during `place`.

## ⚠️ Gaps to close
- `ResolvedModifiers` flatten padding/size/offset into aggregated values, losing modifier ordering.
- `ModifierNodeSlices` coalesces text and graphics layers to the rightmost entry instead of composing multiple layers.
- `MeasurementProxy` remains in the public API even though the coordinator never uses it, leaving dead surface area to maintain.

## Next Tasks

### 1) Remove layout flattening
- Route padding/size/offset/intrinsic behavior through layout nodes (and their coordinators) rather than the `ResolvedModifiers` accumulator.
- Add regression tests for mixed chains (`padding.background.padding`, overlapping offsets, etc.) to lock in ordering semantics.

### 2) Make draw/text slices composable
- Allow multiple text nodes and graphics layers to stack instead of overwriting each other.
- Preserve chain order when emitting draw commands and pointer handlers.

### 3) Decide the measurement proxy story
- Either remove `MeasurementProxy` and proxy implementations or integrate them meaningfully (e.g., to support borrow-safe async measurement).
- Update documentation and tests once the direction is chosen so the API surface matches runtime behavior.
