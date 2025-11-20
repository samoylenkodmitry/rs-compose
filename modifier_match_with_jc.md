# Modifier Migration Reality Check

Concise snapshot of how the modifier system differs from Jetpack Compose and what must change next.

## Current Snapshot (modifier-specific)

- **Direct measurement with placement offsets**: `LayoutModifierNode::measure` now returns a `LayoutModifierMeasureResult` that includes placement offsets, and `LayoutModifierCoordinator` stores that offset and applies it during `place`.
- **Unused proxy surface**: The `LayoutModifierNode` trait still exposes `create_measurement_proxy`, and built-in nodes implement proxies, but `LayoutModifierCoordinator` measures nodes directly and never consults the proxy API. The snapshot surface diverges from Kotlin without providing value today.
- **Flattened Resolution**: `ModifierChainHandle::compute_resolved` flattens standard modifiers (Padding, Size, Offset) into a single `ResolvedModifiers` struct. This loses ordering (e.g., `padding(10).background(...).padding(20)` becomes just 30 padding and one background).
- **Slice Coalescing**: `ModifierNodeSlices` collects draw commands and pointer inputs but reduces text content and graphics layers to "last write wins", preventing composition of these effects.

## Mismatches vs Jetpack Compose

- **Flattened layout semantics**: Kotlin traverses the node chain for layout behavior; the Rust path still aggregates padding/size/offset into `ResolvedModifiers`, discarding modifier order.
- **Draw/text composition gaps**: Kotlin composes multiple draw/text layers; `ModifierNodeSlices` coalesces text and graphics layers to the rightmost entry instead of composing them.
- **Stray proxy API**: Kotlin coordinators call nodes directly. Rust now measures nodes directly too, but the exposed `MeasurementProxy` API remains unused noise.

## Roadmap (integrates “open protocol” proposal)

1. **Eliminate layout flattening**
   - Route padding/size/offset/intrinsic behavior through layout nodes instead of `ResolvedModifiers`.
   - Add coverage for mixed chains (e.g., `padding.background.padding`) to ensure ordering is preserved.

2. **Make draw/text slices composable**
   - Allow multiple text and graphics-layer entries to stack instead of last-write-wins semantics.
   - Preserve chain order when emitting draw commands and pointer handlers.

3. **Resolve the measurement proxy story**
   - Either remove `MeasurementProxy` and related implementations or integrate it meaningfully (e.g., for borrow-safe async measurement). Right now it is unused surface area.
