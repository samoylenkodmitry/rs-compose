# Modifier ≈ Jetpack Compose Parity Plan

Goal: match Jetpack Compose’s `Modifier` API surface and `Modifier.Node` runtime semantics so Kotlin samples and mental models apply 1:1 in Compose-RS.

---

## Current Gaps (Compose-RS)
- `Modifier` is still an Rc-backed builder with cached layout/draw state for legacy APIs. Renderers now read reconciled node slices, but `ModifierState` remains as a compatibility shim until every modifier factory is node-backed.
- `compose_foundation::ModifierNodeChain` still lacks Kotlin’s delegate links, parent references, and modifier-local/semantics plumbing from `DelegatableNode`/`NodeChain`, so traversal order and capability routing can diverge.
- Modifier locals, semantics participation, and diagnostics/parity tooling (`ModifierLocalManager`, `SemanticsModifierNode`, `NodeChain#trace`) are not yet implemented, leaving focus/semantics/modifier-local behaviors off from Android.

## Jetpack Compose Reference Anchors
- `Modifier.kt`: immutable interface (`EmptyModifier`, `CombinedModifier`) plus `foldIn`, `foldOut`, `any`, `all`, `then`.
- `ModifierNodeElement.kt`: node-backed elements with `create`/`update`/`key`/`equals`/`hashCode`/inspector hooks.
- `NodeChain.kt`, `DelegatableNode.kt`, `NodeKind.kt`: sentinel-based chain, capability masks, delegate links, targeted invalidations, and traversal helpers.
- Pointer input stack under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/input/pointer`.

## Recent Progress
- `ModifierNodeChain` exposes typed iterators (`layout_nodes()`, `draw_nodes()`, `pointer_input_nodes()`), and `LayoutNode`/`SubcomposeLayoutNode` publish helper methods so subsystems can traverse reconciled nodes without touching `ModifierState`.
- Renderers (pixels, wgpu, headless) and pointer dispatch now collect draw commands, clip flags, and pointer handlers from node slices via `ModifierNodeSlices`, aligning draw order and hit testing with Kotlin’s `NodeCoordinator`.
- Core modifier factories (`padding`, `background`, `draw*`, `clipToBounds`, `pointerInput`, `clickable`) are node-backed, and pointer input now runs on coroutine-driven `PointerInputScope`/`AwaitPointerEventScope` scaffolding that mirrors `SuspendingPointerInputFilter`. `ModifierState` is only used to keep legacy helpers alive while migrations finish.

## Migration Plan
1. **Mirror the `Modifier` data model (Kotlin: `Modifier.kt`)**  
   Keep the fluent API identical (fold helpers, `any`/`all`, inspector metadata) and delete the remaining runtime responsibilities of `ModifierState` once all factories are node-backed.
2. **Adopt `ModifierNodeElement` / `Modifier.Node` parity (Kotlin: `ModifierNodeElement.kt`)**  
   Implement the full lifecycle contract: `onAttach`, `onDetach`, `onReset`, coroutine scope ownership, and equality/key-driven reuse.
3. **Port `NodeChain` diff + capability plumbing (Kotlin: `NodeChain.kt`, `NodeKind.kt`)**  
   Add sentinel head/tail nodes, delegate links, parent pointers, and modifier-local storage so traversal/invalidation order matches Android exactly.
4. **Wire runtime subsystems through chains**  
   Layout/draw/pointer already read reconciled nodes; next up are semantics, focus, modifier locals, and coroutine-backed pointer input.
5. **Migrate modifier factories + diagnostics**  
   Reimplement all public factories on top of node elements, add Kotlin-style debug dumps/inspector metadata, and grow the test matrix to compare traversal order against the Android reference.

## Near-Term Next Steps
1. **NodeChain lifecycle + delegate plumbing**  
   - Re-introduce Kotlin’s sentinel-style head/tail nodes, parent links, and delegate chains without `unsafe`, matching `androidx/compose/ui/node/NodeChain.kt`.  
   - Ensure nodes expose `parent`, `child`, and capability masks so modifier locals, semantics, pointer hit-testing, and diagnostics can traverse exactly like `DelegatableNode`.
2. **Modifier locals + semantics parity**  
   - Port `ModifierLocalNode`, `ModifierLocalManager`, `SemanticsModifierNode`, and related invalidations.  
   - Update `SemanticsTree` building to read directly from modifier nodes, mirroring the Kotlin semantics pipeline under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/semantics`.
3. **Diagnostics + parity tooling**  
   - Add Kotlin-style debug dumps (`Modifier.toString()`, `chainToString`, capability masks, coroutine lifecycle traces) guarded by `COMPOSE_DEBUG_MODIFIERS`.  
   - Mirror Kotlin’s modifier/node-chain tests and snapshot dumps so traversal ordering, capability masks, and coroutine restarts stay in lock-step with `androidx.compose.ui`.
4. **Modifier factory cleanup**  
   - Finish migrating the remaining factories off `ModifierState`. Once complete, delete the legacy layout/draw caches and rely entirely on reconciled node chains/slices.

## Kotlin Reference Playbook
| Area | Kotlin Source | Compose-RS Target |
| --- | --- | --- |
| Modifier API | `androidx/compose/ui/Modifier.kt` | `crates/compose-ui/src/modifier/mod.rs` |
| Node elements & lifecycle | `ModifierNodeElement.kt`, `DelegatableNode.kt` | `crates/compose-foundation/src/modifier.rs` + `compose-ui` node impls |
| Node chain diffing | `NodeChain.kt`, `NodeCoordinator.kt` | `crates/compose-foundation/src/modifier.rs`, upcoming coordinator module |
| Pointer input | `input/pointer/*` | `crates/compose-ui/src/modifier/pointer_input.rs` |
| Semantics | `semantics/*`, `SemanticsNode.kt` | `crates/compose-ui/src/semantics` (to be ported) |

Always cross-check behavior against the Kotlin sources under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui` to ensure parity.
