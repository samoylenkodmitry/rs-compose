# Modifier System Migration Tracker

## Status: ⚠️ MOSTLY COMPLETE (with known shortcuts)

The modifier system migration has made significant progress. All widgets (`Button`, `Text`, `Spacer`)
now use `LayoutNode` and the modifier chain reconciliation system is functional with capability-based
invalidation. However, several shortcuts keep us from real Jetpack Compose parity:

- `Modifier::then` still clones the entire element vector every time, so modifier composition is
  `O(n²)` instead of the persistent `CombinedModifier` structure Kotlin uses.
- The reconciled modifier node chain is only consulted to build `ResolvedModifiers`; the layout/draw
  pipeline never invokes `LayoutModifierNode::measure` or `DrawModifierNode::draw`.
- **Text still relies on `TextMeasurePolicy`** rather than a modifier node, so content, rendering,
  and semantics live outside the node architecture.

See "Known Shortcuts" for the outstanding work.

## Completed Work

1. ✅ **Wire the new dispatch queues into the host/runtime.** The app shell now
   calls `process_pointer_repasses` and `process_focus_invalidations` during frame processing
   (see [AppShell::run_dispatch_queues](crates/compose-app-shell/src/lib.rs#L237-L275)). Nodes
   that mark `needs_pointer_pass` / `needs_focus_sync` now have those flags cleared by the
   runtime, completing the invalidation cycle similar to Jetpack Compose's FocusInvalidationManager.

2. ✅ **Remove the legacy widget-specific nodes.** All widgets now use `LayoutNode`:
   - **Spacer** → `LayoutNode` with `LeafMeasurePolicy`
   - **Text** → `LayoutNode` with `TextMeasurePolicy`
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
  - `TextMeasurePolicy` - ⚠️ **shortcut**: stores text content (should be in modifier node)
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

### Text Implementation (Architecture Mismatch with Jetpack Compose)

**Current Implementation:**
```rust
// In crates/compose-ui/src/widgets/text.rs
Text(value, modifier) → Layout(modifier, TextMeasurePolicy::new(text), || {})
```

Text content is stored in `TextMeasurePolicy` and extracted via a `text_content()` method added to
the `MeasurePolicy` trait.

**Problem:**
- Violates separation of concerns - `MeasurePolicy` is for measurement, not content storage
- Pollutes `MeasurePolicy` trait with domain-specific methods
- **Doesn't match Jetpack Compose architecture**

**How Jetpack Compose Does It:**
```kotlin
// In androidx.compose.foundation.text.BasicText
Layout(modifier.then(TextStringSimpleElement(...)), EmptyMeasurePolicy)
```

Text content lives in `TextStringSimpleNode` which implements:
- `LayoutModifierNode` (measure)
- `DrawModifierNode` (draw)
- `SemanticsModifierNode` (semantics)

**Additional Gaps:**
- `TextModifierElement::update` cannot request invalidations, so text/style changes rely on
  rebuilding the entire LayoutNode to refresh layout/draw/semantics.
- `TextModifierNode::draw` is empty and measurement uses a monospaced fake; Kotlin uses
  `ParagraphLayoutCache` and exposes `getTextLayoutResult`.
- Semantics only set `content_description = text` rather than the richer properties provided by
  `TextStringSimpleNode`.

**Proper Fix:**
1. Create `TextModifierNode: LayoutModifierNode + DrawModifierNode + SemanticsModifierNode`
2. Create `TextModifierElement` that produces `TextModifierNode` and can request invalidations when
   text/style change
3. Update `Text()` to use modifier-based text: `Layout(modifier.textModifier(text), EmptyMeasurePolicy, || {})`
4. Remove `text_content()` from `MeasurePolicy` trait
5. Delete `TextMeasurePolicy`
6. Implement a real text measurer/drawer (ParagraphLayoutCache analogue) and semantics contract

**Reference:**
- See [modifier_match_with_jc.md](modifier_match_with_jc.md) for detailed architecture comparison
- JC source: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/modifiers/TextStringSimpleNode.kt`

## Remaining Work

### Make Modifier Composition Persistent
- Introduce a `CombinedModifier`-style representation so `Modifier::then` is `O(1)` and retains
  sharing between recompositions.
- Ensure `fold_in`, `fold_out`, `any`, and `all` traverse the combined structure correctly.

### Execute Modifier Nodes During Layout/Draw
- Run `LayoutModifierNode::measure`, `DrawModifierNode::draw`, pointer, semantics, and focus
  callbacks in the reconciled order instead of mirroring data into `ResolvedModifiers`.
- Delete the ad-hoc padding/size/background accumulation once the real nodes power layout.

### Critical: Fix Text Implementation
**Priority: High** - Required for true Jetpack Compose parity

The text implementation needs to be refactored to match JC's architecture:
1. Implement `TextModifierNode` with `LayoutModifierNode`, `DrawModifierNode`, and `SemanticsModifierNode` traits
2. Create `TextModifierElement` that creates `TextModifierNode`
3. Add `.textModifier(text, style, ...)` extension to `Modifier`
4. Update `Text()` widget to use modifier-based approach instead of `TextMeasurePolicy`
5. Remove `text_content()` from `MeasurePolicy` trait
6. Update rendering/semantics system to extract text from modifier chain and expose the same
   semantics contract (`text`, `getTextLayoutResult`, translation toggles)
7. Wire invalidation hooks so `TextModifierElement::update` can request layout/draw/semantics updates
8. Delete `TextMeasurePolicy` once migration is complete

This will properly separate concerns and align with Jetpack Compose's design where content lives
in modifier nodes, not measure policies.

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
