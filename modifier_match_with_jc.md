# Modifier ≈ Jetpack Compose Parity Plan

Goal: match Jetpack Compose’s `Modifier` API surface and `Modifier.Node` runtime semantics so Kotlin samples and mental models apply 1:1 in Compose-RS.

---

## Current Gaps (Compose-RS)
- `Modifier` is still an Rc-backed builder with cached layout/draw/pointer state for legacy APIs. Renderers now read reconciled node slices, but `ModifierState` remains as a compatibility shim until every modifier factory is node-backed.
- `compose_foundation::ModifierNodeChain` lacks Kotlin’s sentinel head/tail nodes, delegate links, and coroutine-aware lifecycle from `DelegatableNode`/`NodeChain`, so modifier locals, semantics, and nested traversal order still differ.
- `ModifierNode` contracts do not yet expose coroutine scopes, modifier-local plumbing, semantics participation, or the restartable pointer-input lifecycle (`AwaitPointerEventScope`) that Android provides.

## Jetpack Compose Reference Anchors
- `Modifier.kt`: immutable interface (`EmptyModifier`, `CombinedModifier`) plus `foldIn`, `foldOut`, `any`, `all`, `then`.
- `ModifierNodeElement.kt`: node-backed elements with `create`/`update`/`key`/`equals`/`hashCode`/inspector hooks.
- `NodeChain.kt`, `DelegatableNode.kt`, `NodeKind.kt`: sentinel-based chain, capability masks, delegate links, targeted invalidations, and traversal helpers.
- Pointer input stack under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/input/pointer`.

## Recent Progress
- `ModifierNodeChain` exposes typed iterators (`layout_nodes()`, `draw_nodes()`, `pointer_input_nodes()`), and `LayoutNode`/`SubcomposeLayoutNode` publish helper methods so subsystems can traverse reconciled nodes without touching `ModifierState`.
- Renderers (pixels, wgpu, headless) and pointer dispatch now collect draw commands, clip flags, and pointer handlers from node slices via `ModifierNodeSlices`, aligning draw order and hit testing with Kotlin’s `NodeCoordinator`.
- Core modifier factories (`padding`, `background`, `draw*`, `clipToBounds`, `pointerInput`, `clickable`) are node-backed, and `ModifierState` is only used to keep legacy helpers alive while migrations finish.

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
1. **Pointer-input coroutine parity**  
   - Port `PointerInputModifierNode`, `AwaitPointerEventScope`, and restart/cancellation semantics from the Kotlin sources under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/input/pointer`.  
   - Rebase `Modifier.pointerInput`, `Modifier.clickable`, and gesture modifiers (tap/press/drag/scroll) on coroutine-backed nodes; remove the remaining pointer closures from `ModifierState`.  
   - Add tests mirroring `PointerInputModifierNodeTest` to prove cancellation, restart, and consumption order match Android.
2. **NodeChain lifecycle + coordinator plumbing**  
   - Add sentinel head/tail nodes, parent/child links, and delegate pointers so each node can reach its neighbors without reallocations (see `NodeChain.kt`, `DelegatableNode.kt`).  
   - Introduce a coordinator-like runtime that sequences `onAttach`/`onDetach`, hands coroutine scopes to nodes, and exposes traversal APIs for semantics and modifier locals in addition to layout/draw/pointer.
3. **Semantics + modifier locals**  
   - Port `SemanticsModifierNode`, focus/hover semantics, and modifier-local propagation (Kotlin’s `ModifierLocalNode`, `ModifierLocalManager`).  
   - Update `SemanticsTree` construction and invalidations to read directly from reconciled node slices rather than `RuntimeNodeMetadata`.
4. **Diagnostics + conformance tests**  
   - Add `COMPOSE_DEBUG_MODIFIERS` tracing (chain dumps, capability masks, invalidation kinds, coroutine lifecycle) to help spot regressions.  
   - Mirror Kotlin’s modifier/node-chain tests and snapshot dumps so traversal ordering, capability masks, and coroutine restarts stay in lock-step with `androidx.compose.ui`.

## Kotlin Reference Playbook
| Area | Kotlin Source | Compose-RS Target |
| --- | --- | --- |
| Modifier API | `androidx/compose/ui/Modifier.kt` | `crates/compose-ui/src/modifier/mod.rs` |
| Node elements & lifecycle | `ModifierNodeElement.kt`, `DelegatableNode.kt` | `crates/compose-foundation/src/modifier.rs` + `compose-ui` node impls |
| Node chain diffing | `NodeChain.kt`, `NodeCoordinator.kt` | `crates/compose-foundation/src/modifier.rs`, upcoming coordinator module |
| Pointer input | `input/pointer/*` | `crates/compose-ui/src/modifier/pointer_input.rs` |
| Semantics | `semantics/*`, `SemanticsNode.kt` | `crates/compose-ui/src/semantics` (to be ported) |

Always cross-check behavior against the Kotlin sources under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui` to ensure parity.
