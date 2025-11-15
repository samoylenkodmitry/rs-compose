# Modifier System Migration Tracker

## Status: Still In Progress

Recent work modernised many modifier builder helpers so they chain via `self.then(...)`, but
core runtime parity with Jetpack Compose is not complete yet. The codebase still contains
legacy widget nodes (`ButtonNode`, `TextNode`, `SpacerNode`) and manual semantics fallbacks,
and the new pointer/focus invalidation managers are never invoked outside of unit tests.

## High-Priority Gaps

1. **Wire the new dispatch queues into the host/runtime.** `schedule_pointer_repass` and
   `schedule_focus_invalidation` are exported from `compose_ui`, but `process_pointer_repasses`
   / `process_focus_invalidations` are never called by the app shell. Nodes mark
   `needs_pointer_pass` / `needs_focus_sync` yet nothing clears those flags, so the queues never
   drain.
2. **Remove the legacy widget-specific nodes.** Layout/runtime metadata paths still special-case
   `ButtonNode`, `TextNode`, and `SpacerNode`, pulling modifier information directly from those
   structs instead of the reconciled `LayoutNode` chain. Migrating those widgets onto standard
   layout nodes will let us delete a large amount of duplicate logic.
3. **Stop rebuilding modifier snapshots ad-hoc.** Functions such as
   `measure_spacer` in `layout/mod.rs` still call `Modifier::empty().resolved_modifiers()` which
   spins up a temporary chain every time. Resolved modifiers should come exclusively from the
   layout node data that already owns the reconciled chain.
4. **Tests/examples only validate structure.** Pointer integration tests currently just check
   node counts; they never synthesize pointer events through `HitTestTarget`. We still need
   proper integration coverage before claiming parity.

## Next Steps

- Teach `crates/compose-app-shell` to drain pointer/focus queues each frame and to call the
  appropriate `LayoutNode` methods so `needs_pointer_pass` / `needs_focus_sync` are cleared.
- Convert the remaining widget nodes to emit layout/subcompose nodes with modifier-driven
  behaviour, then delete the `ButtonNode`, `TextNode`, and `SpacerNode` code paths along with
  the metadata fallbacks in `layout/mod.rs`.
- Audit `LayoutNodeData` creation so every code path uses `LayoutNodeData::new(...)` with
  `modifier_slices`, and remove the places that call `Modifier::empty().resolved_modifiers()`
  as a stand-in.
- Expand the pointer/focus integration tests to drive events through a render scene so we can
  verify the async pointer handlers and focus callbacks actually fire.
