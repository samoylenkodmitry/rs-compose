# Next Task: Wire Delegate Traversal Into Runtime Consumers

## Context
`ModifierNodeChain` now mirrors Jetpack Compose’s `NodeChain`: every `Modifier.Node` owns parent/child links, delegate stacks contribute to traversal order, and aggregate capability masks propagate through delegates without any `unsafe`. The remaining divergence is that higher-level systems (modifier locals, focus, pointer input, semantics, `ModifierChainHandle`) still treat the chain as a flat list. As a result, capability short-circuiting and ancestor lookups continue to rely on legacy metadata, and we must keep bespoke iterators such as `draw_nodes()` and `pointer_input_nodes()`.

Use the Kotlin sources under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui`—especially `node/NodeChain.kt`, `node/DelegatableNode.kt`, `node/ModifierLocalManager.kt`, `node/Focus*`, and `modifier/Modifier.kt`—to match the behavior and traversal contracts.

## Goals
1. Update every runtime consumer to use the delegate-aware traversal helpers exposed by `compose_foundation::ModifierNodeChain`.
2. Remove or deprecate the legacy entry-only iterators (`draw_nodes*`, `pointer_input_nodes*`, etc.) once equivalent delegate-based paths exist.
3. Ensure modifier locals, semantics extraction, pointer dispatch, and focus pipelines short-circuit based on `aggregate_child_capabilities` exactly like Kotlin’s `NodeChain`.
4. Extend diagnostics/tests to cover the new traversal flow (delegate depth, capability masks, ancestor lookups).

## Suggested Steps
1. **ModifierChainHandle + diagnostics**
   - Teach `ModifierChainHandle` (in `crates/compose-ui/src/modifier/chain.rs`) to call the new traversal helpers when generating resolved modifiers, computing capability bitmasks, and logging chains.
   - Remove direct access to `entries`/legacy iterators; keep the public surface identical.
2. **Modifier locals**
   - Update `crates/compose-ui/src/modifier/local.rs` so provider/consumer discovery, ancestor resolution, and invalidation bubbling walk delegates via capability masks (`NodeCapabilities::MODIFIER_LOCALS`).
   - Verify behavior against Kotlin’s `ModifierLocalManager` by porting relevant tests.
3. **Pointer input & focus**
   - In `crates/compose-ui/src/modifier/pointer_input.rs` and the focus utilities under `crates/compose-ui/src/widgets`, route hit-testing and ancestor traversal through the delegate-aware visitors. Ensure capability checks gate traversal the same way `androidx.compose.ui.node.NodeChain` does (see `visitChildren`, `visitAncestors` in the Kotlin sources).
4. **Semantics preparation**
   - Update any semantics helpers that still call `chain.semantics_nodes()` to instead filter via `for_each_forward_matching(NodeCapabilities::SEMANTICS, …)`. This unblocks the upcoming semantics tree rewrite.
5. **Cleanup & tests**
   - Delete or mark deprecated the legacy iterators (`draw_nodes`, `pointer_input_nodes`, etc.) once no call sites remain.
   - Add/adjust tests in `crates/compose-foundation/src/tests/modifier_tests.rs` and `crates/compose-ui/src/modifier/tests` to cover delegate traversal from each subsystem (modifier locals resolving through delegates, pointer input skipping chains without the capability bit, etc.).

## Definition of Done
- `ModifierChainHandle`, modifier locals, pointer input, focus, and semantics helpers no longer access `ModifierNodeChain.entries` directly; they rely on delegate-aware traversal APIs.
- Capability masks (`aggregate_child_capabilities`) short-circuit modifier locals, pointer input, focus, and semantics in the same scenarios covered by Kotlin’s `NodeChain`.
- Legacy iterators (`draw_nodes*`, `pointer_input_nodes*`, etc.) are removed or unused.
- New/updated tests verify delegate traversal for modifier locals, pointer input, focus, and semantics; `cargo test` across the workspace remains green.
