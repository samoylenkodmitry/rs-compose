# Modifier System Migration Tracker

## Status: Still In Progress

Recent work modernised many modifier builder helpers so they chain via `self.then(...)`, and
the pointer/focus dispatch queues are now integrated into the app shell runtime. The codebase
still contains legacy widget nodes (`ButtonNode`, `TextNode`, `SpacerNode`) and manual semantics
fallbacks that need migration to complete the parity with Jetpack Compose.

## High-Priority Gaps

1. ✅ **COMPLETED: Wire the new dispatch queues into the host/runtime.** The app shell now
   calls `process_pointer_repasses` and `process_focus_invalidations` during frame processing
   (see [AppShell::run_dispatch_queues](crates/compose-app-shell/src/lib.rs#L237-L275)). Nodes
   that mark `needs_pointer_pass` / `needs_focus_sync` now have those flags cleared by the
   runtime, completing the invalidation cycle similar to Jetpack Compose's FocusInvalidationManager.
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

- ✅ **DONE:** `crates/compose-app-shell` now drains pointer/focus queues each frame and calls
  the appropriate `LayoutNode` methods to clear `needs_pointer_pass` / `needs_focus_sync`.
- Convert the remaining widget nodes to emit layout/subcompose nodes with modifier-driven
  behaviour, then delete the `ButtonNode`, `TextNode`, and `SpacerNode` code paths along with
  the metadata fallbacks in `layout/mod.rs`.
- Audit `LayoutNodeData` creation so every code path uses `LayoutNodeData::new(...)` with
  `modifier_slices`, and remove the places that call `Modifier::empty().resolved_modifiers()`
  as a stand-in.
- Expand the pointer/focus integration tests to drive events through a render scene so we can
  verify the async pointer handlers and focus callbacks actually fire.
