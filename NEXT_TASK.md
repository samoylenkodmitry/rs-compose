# Next Task: Mask-Driven Visitors & Pointer/Focus Invalidations

## Context
`BasicModifierNodeContext` now records `ModifierInvalidation`s with capability masks and `LayoutNode::mark_needs_redraw()` mirrors `AndroidComposeView#invalidateLayers`, so DRAW-only nodes stop forcing layout. The remaining Kotlin parity gap lives in dispatch: `NodeChain` APIs in `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/NodeChain.kt` walk capability masks directly, while Compose-RS still relies on legacy `as_*` shortcuts and treats pointer/focus dirties as implicit layout work. We need to finish the mask-driven traversal story (per `Modifier.kt` + `ModifierNodeElement.kt`) and route pointer/focus invalidations through their dedicated managers.

## Current State
- `ModifierNodeChain::for_each_*_matching` and delegate-aware visitors exist, but higher-level systems (draw collection, pointer input stack, semantics, focus, modifier locals) still downcast via `as_draw_node`/`as_pointer_input_node`.
- `BasicModifierNodeContext` emits rich `ModifierInvalidation`s and `compose-app-shell` watches `request_render_invalidation()`, yet pointer/focus events remain tied to layout dirties instead of their Kotlin-style queues (`PointerInputDelegatingNode` in `androidx/compose/ui/input/pointer` uses `NodeKind.Pointer` checks).
- Docs/tests still tell downstream authors to override `as_*` helpers even though capability masks are the contract.

## Goals
1. **Mask-driven traversal everywhere** — All modifier consumers (draw, pointer, focus, semantics, modifier locals) use capability masks + delegate-aware visitors, enabling nodes that only set `NodeCapabilities` to participate without overriding helpers.
2. **Pointer & focus invalidation routing** — Pointer/focus nodes trigger dedicated queues (pointer repass, focus recomposition) without calling `mark_needs_measure/layout`, matching `NodeChain.invalidateKind(NodeKind.Pointer/Input/Foc)` semantics from Kotlin.
3. **API surface cleanup** — Deprecate mentions of `as_draw_node`/friends, ensure built-ins report accurate masks, and document the new contract for third parties.

## Jetpack Compose Reference
- `androidx/compose/ui/node/NodeChain.kt` & `DelegatableNode.kt` for capability-mask traversal.
- `androidx/compose/ui/Modifier.kt` / `ModifierNodeElement.kt` for the “capabilities define behavior” contract.
- Pointer/focus routing under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/input/pointer` and `androidx/compose/ui/focus/FocusTargetNode.kt`.

## Implementation Plan

### Phase 1 — Finish Mask-Driven Visitors
1. Introduce shims (e.g., `for_each_node_with(mask, visitor)`) to wrap `ModifierChainNodeRef` iteration à la Kotlin’s `forEachKind`.
2. Update draw slice collection, pointer input dispatch, semantics collection, focus search, and modifier-local manager to consume these shims instead of calling the `as_*` helpers.
3. Leave the `as_*` hooks as temporary compat no-ops, but add tests proving nodes that only set capability bits still run.

### Phase 2 — Pointer/Focus Invalidation Routing
1. Extend `ModifierInvalidation` plumbing so pointer/focus requests bubble out of `BasicModifierNodeContext` and reach `LayoutNode::dispatch_modifier_invalidations`.
2. Add `LayoutNode::mark_needs_pointer_pass()`/focus equivalent, and teach `compose-app-shell` (and other hosts) to act on those flags without toggling layout dirties.
3. Mirror Kotlin’s targeted behavior: pointer stacks request a new pass, focus manager queues recomposition, layout flags remain untouched.

### Phase 3 — Public Surface Polish
1. Update docs + examples to describe capability-driven nodes (drop `as_draw_node` references).
2. Audit built-in `ModifierNodeElement::capabilities()` to guarantee every node advertises pointers/focus/etc. correctly.
3. Add migration/regression tests for third-party nodes that rely solely on masks.

## Acceptance Criteria
- Draw, pointer, focus, semantics, and modifier-local traversals rely exclusively on capability masks & delegate-aware visitors.
- Pointer/focus invalidations no longer call `mark_needs_measure/layout`; they drive their dedicated managers like in Kotlin.
- Built-in docs/tests describe the capability contract, and third-party nodes that only set `NodeCapabilities` pass new regression suites.
- Workspace `cargo test` passes.
