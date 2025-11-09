# Next Task: Pointer-Input Coroutine & Node Lifecycle Parity

## Context
Renderers, pointer dispatch, and layout now traverse reconciled `ModifierNodeChain` slices, but we still lack Kotlin’s coroutine-backed pointer input machinery and the sentinel-based node lifecycle that powers modifier locals and semantics. Kotlin’s implementation lives under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui`, especially:

- `input/pointer/PointerInputModifierNode.kt`, `SuspendingPointerInputFilter.kt`, `AwaitPointerEventScope.kt`
- `ModifierNodeElement.kt`, `DelegatableNode.kt`, `NodeChain.kt`

Matching these files 1:1 will let us delete the remaining `ModifierState` pointer hooks, introduce restartable pointer scopes, and unblock semantics/modifier-local parity.

## Goals
1. Port the coroutine-driven pointer input lifecycle (await scope, restart, cancellation, resume on attach/detach) and expose it through `Modifier.pointerInput`.
2. Re-implement clickable/gesture modifiers so they create real `PointerInputModifierNode`s and stop storing closures in `ModifierState`.
3. Extend `ModifierNodeChain` with sentinel head/tail nodes and delegate links so nodes can reach parents/children just like Kotlin’s `DelegatableNode`; this coordinator wiring must host coroutine scopes for pointer nodes.
4. Add tests proving pointer cancellation/restart order, coroutine lifetime, and node traversal matches the reference implementation.

## Suggested Steps
1. **Study Kotlin reference**  
   - Read `PointerInputModifierNode.kt`, `SuspendingPointerInputFilter.kt`, and `AwaitPointerEventScope.kt` under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/input/pointer`.  
   - Mirror the public surface (suspending `awaitPointerEventScope`, `PointerInputModifierNode` trait) in `crates/compose-ui/src/modifier/pointer_input.rs`.
2. **Port coroutine scaffolding**  
   - Introduce a `PointerInputModifierNode` trait (or extend `PointerInputNode`) with `onPointerEvent` + coroutine scope injection.  
   - Implement the coroutine dispatcher that starts when nodes attach, restarts when keys change, and cancels on detach/reset.
3. **Update modifier factories**  
   - Rewrite `Modifier.pointerInput` and `Modifier.clickable` (plus any gesture helpers in `modifier_nodes.rs`) to create the new nodes. Remove unused `ModifierState` pointer fields.  
   - Ensure capability masks and invalidations fire (`InvalidationKind::PointerInput`) via node contexts.
4. **Add sentinel + delegate wiring**  
   - Extend `ModifierNodeChain` (`crates/compose-foundation/src/modifier.rs`) with sentinel head/tail nodes, `parent`/`child` pointers, and delegate links so pointer nodes can walk to siblings/parents (needed for modifier locals & semantics).  
   - Add unit tests covering traversal order and capability aggregation with the new structure.
5. **Tests & parity validation**  
   - Add coroutine-focused tests under `crates/compose-ui/src/tests/modifier_nodes_tests.rs` and `modifier/pointer_input.rs` mirroring Kotlin’s `PointerInputModifierNodeTest`.  
   - Add integration tests that simulate pointer cancellation/restart sequences via `compose_render` scenes to ensure behaviors match Android.

## Definition of Done
- `Modifier.pointerInput`, `Modifier.clickable`, and gesture helpers create coroutine-backed nodes; no pointer closures remain in `ModifierState`.
- Pointer nodes receive coroutine scopes that start on attach, restart on key change, and cancel on detach/reset, matching Kotlin’s lifecycle (validated via tests).
- `ModifierNodeChain` has sentinel head/tail nodes and delegate links; traversal helpers (layout/draw/pointer) operate on this structure, and new tests cover the topology.
- New unit/integration tests prove pointer cancellation/restart ordering and node traversal parity; `cargo test -p compose-ui` and workspace `cargo test` stay green.
