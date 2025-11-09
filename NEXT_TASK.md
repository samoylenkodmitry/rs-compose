# Next Task: NodeChain Delegation, Modifier Locals & Semantics Parity

## Context
Coroutine-backed `Modifier.pointerInput`/`clickable` are live, renderers source draw/pointer slices from reconciled `ModifierNodeChain`s, and legacy pointer closures are gone. The remaining gap to Jetpack Compose parity sits in the node chain itself (delegate links, parent pointers, modifier-local plumbing) and the semantics/modifier-local subsystems that rely on it. Kotlin’s reference lives under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui`, especially:

- `node/NodeChain.kt`, `node/DelegatableNode.kt`, `node/NodeKind.kt`
- `modifier/ModifierLocal*`, `semantics/*`, `semantics/SemanticsNode.kt`
- `Modifier.kt` for inspector/debug dumps

Matching those files 1:1 will unblock semantics/focus/modifier-local parity and allow us to delete the remaining `ModifierState` shims.

## Goals
1. Reintroduce Kotlin’s sentinel head/tail chain model *without unsafe blocks* while providing `parent`, `child`, and `aggregateChildKindSet` data so traversal mirrors `DelegatableNode`.
2. Port modifier local infrastructure (`ModifierLocalNode`, `ModifierLocalManager`, lookups/invalidation) and semantics participation (`SemanticsModifierNode`, configuration merges, tree invalidations).
3. Add Kotlin-style diagnostics (`Modifier.toString()`, chain dumps, capability masks) guarded behind a debug flag so future regressions are easy to inspect.
4. Wire the new plumbing into layout/render/semantics builders so modifier locals and semantics consume reconciled nodes instead of `RuntimeNodeMetadata`.

## Suggested Steps
1. **Study Kotlin reference**  
   - Review `NodeChain.kt`, `DelegatableNode.kt`, and helpers in `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node`.  
   - Audit how Kotlin stores sentinel nodes, delegate links, and capability aggregation; note the `node.parent`, `child`, and `kindSet` mechanics.
2. **Implement safe sentinel/delegate structure**  
   - Create a safe Rust equivalent by storing sentinel entries inside `ModifierNodeChain` (e.g., boxed structs held inside `Rc<RefCell<_>>`) so we can mutate neighbor pointers without `unsafe`.  
   - Ensure every `ModifierNode` can access its parent/child and aggregate capability masks (needed by modifier locals and semantics traversal).  
   - Add focused tests covering reordering, keyed reuse, and traversal order in `crates/compose-foundation/src/tests/modifier_tests.rs`.
3. **Port modifier locals**  
   - Mirror `ModifierLocalProvider`, `ModifierLocalNode`, and `ModifierLocalManager` from Kotlin inside Compose-RS (likely under `crates/compose-ui/src/modifier_locals`).  
   - Expose lookup APIs (`ModifierLocalProvider { }`, `ModifierLocalConsumer { }`) and ensure invalidations bubble through the new node chain capabilities.  
   - Add unit tests similar to Kotlin’s `ModifierLocalTest`.
4. **Port semantics stack**  
   - Implement `SemanticsModifierNode`, configuration merging, and semantics invalidation as Kotlin does (`semantics/SemanticsNode.kt`, `SemanticsOwner`).  
   - Update `crates/compose-ui/src/layout/mod.rs` and renderers to build semantics trees directly from modifier nodes (drop reliance on cached `RuntimeNodeMetadata`).  
   - Add parity tests (e.g., clickable semantics, custom properties) mirroring Android’s `SemanticsModifierNodeTest`.
5. **Diagnostics & cleanup**  
   - Add `COMPOSE_DEBUG_MODIFIERS` / `compose_ui::debug::log_modifier_chain` that prints node order, capability masks, and modifier locals (similar to `Modifier.toString()` in Kotlin).  
   - Remove the remaining `ModifierState` responsibilities once modifier locals/semantics no longer need metadata shims.

## Definition of Done
- `ModifierNodeChain` offers safe head/tail sentinels, parent/child links, and delegate traversal without `unsafe`. Tests cover reordering, keyed reuse, and capability aggregation.
- Modifier locals (`ModifierLocalNode`, providers/consumers, invalidation) and semantics (`SemanticsModifierNode`, `SemanticsOwner`) exist in Compose-RS and match Kotlin behaviors via targeted tests.
- Layout/render/semantics builders consume modifier locals and semantics directly from reconciled node chains; no reliance on `RuntimeNodeMetadata` for these concerns.
- Debug hooks (`Modifier::toString`, chain dumps, optional logging) exist behind a feature/flag for tracing capability masks and node order.
- Workspace `cargo test` (and targeted `cargo test -p compose-ui`) remain green.
