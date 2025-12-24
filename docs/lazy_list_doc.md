# LazyList Implementation

Last Updated: 2025-12-26

Virtualized lazy layouts for Compose-RS with 1:1 API and architecture parity with Jetpack Compose (JC). This document tracks current alignment gaps, refactor tasks, and verification steps.

---

## Status

| Feature | Status | Notes |
|---------|--------|-------|
| SubcomposeLayout | WIP | JC-style reuse + precompose path in place; validate edge cases |
| LazyColumn/LazyRow | WIP | Layout size now constrained to measured content; arrangement applied only when no extra items |
| LazyListState | WIP | Stats derived from SubcomposeState |
| LazyListIntervalContent | OK | Matches JC interval model |
| SlotReusePool | WIP | Foundation pool still present; SubcomposeState also tracks reuse |
| Lifecycle (compose/dispose) | WIP | Dispose non-retained slots immediately; verify detach behavior |
| PrefetchScheduler | BLOCKED | Needs idle-time scheduler; no UI tracking yet |
| Scrollable constraints | OK | LazyList asserts on infinite constraints (JC parity) |
| measure_lazy_list | OK | JC scroll/backfill flow + visible/beyond-bounds separation |
| canScrollForward/Backward | OK | Parity |
| IntrinsicSize modifiers | OK | Wrap-content uses IntrinsicSize; no unbounded constraints |
| Item animations | BLOCKED | Needs coroutine API |
| Fling/animated scroll | BLOCKED | Needs coroutine API |

---

## Architecture

### Files

| File | Purpose |
|------|---------|
| `compose-foundation/src/lazy/lazy_list_state.rs` | Scroll state + stats |
| `compose-foundation/src/lazy/lazy_list_scope.rs` | DSL + IntervalContent |
| `compose-foundation/src/lazy/lazy_list_measure.rs` | Measurement algorithm |
| `compose-foundation/src/lazy/prefetch.rs` | Prefetch scheduler |
| `compose-ui/src/widgets/lazy_list.rs` | LazyColumn/LazyRow |
| `compose-ui/src/subcompose_layout.rs` | SubcomposeLayout |
| `compose-ui/src/modifier/scroll.rs` | Scroll gestures |
| `compose-core/src/subcompose.rs` | SubcomposeState + lifecycle |

### JC Reference (`/media/huge/composerepo/`)

| JC File | Path |
|---------|------|
| SubcomposeLayout | `compose/ui/ui/src/commonMain/.../SubcomposeLayout.kt` |
| LazyLayout | `compose/foundation/.../lazy/layout/LazyLayout.kt` |
| LazyListMeasure | `compose/foundation/.../lazy/LazyListMeasure.kt` |
| LazyLayoutPrefetchState | `compose/foundation/.../lazy/layout/LazyLayoutPrefetchState.kt` |

---

## Code Review Findings (LazyList & Subcompose)

Priority is aligned with architectural risk and user impact.

### P0 (Architecture)

1. Dual slot reuse systems (SubcomposeState vs SlotReusePool) are redundant and not integrated.
   - Action: remove SlotReusePool and use JC-style reuse from SubcomposeState only.
2. Subcompose parent links can go stale (active children updated without parent pointer reconciliation).
   - Action: reconcile parent pointers when SubcomposeLayout updates active children.
3. Item root mismatch: lazy items may return multiple nodes but the measure code assumes a single root.
   - Action: support multiple roots by stacking placeables along the main axis (JC behavior).
4. Layout measurement unbounded children when size was unspecified, breaking virtualization.
   - Action: always respect parent constraints; implement IntrinsicSize for wrap-content; add LazyList infinite-constraint guard (JC parity).

### P1 (Performance / Correctness)

1. Reusable node eviction uses O(n) operations and is policy-inconsistent.
   - Action: use VecDeque for O(1) and align retain/dispose logic with JC policy.
2. Prefetch work happens during measure instead of idle time.
   - Action: schedule precompose work outside the measure pass using a prefetch runner.
3. Skipped groups can flatten item hierarchies in subcompose (child nodes become roots).
   - Action: force subtree recomposition on reuse; keep parent pointers accurate.

### P2 (Maintainability)

1. Precomposed count recalculated with O(n) scans.
   - Action: track incrementally in SubcomposeState.
2. measure_lazy_list_optimized is unused.
   - Action: remove or integrate once new prefetch flow is in place.

---

## Current Regressions (Needs Fix)

None observed in latest robot runs.

## Resolved Regressions

1. Recursive Layout controls missing after Modifiers Showcase navigation.
   - Repro: Modifiers Showcase → Recursive Layout.
   - Robot: `robot_recursive_layout` (pass).
2. LazyList item row layout broken after Modifiers Showcase navigation.
   - Repro: Modifiers Showcase → Lazy List.
   - Robot: `robot_lazy_list_after_modifiers` (pass).
3. Positioned Boxes header duplicated after LazyList navigation.
   - Repro: Lazy List → Modifiers Showcase → Positioned Boxes.
   - Robot: `robot_positioned_boxes_after_lazy_list` (pass).
4. LazyList "h: 48px" duplicated after End → Start.
   - Repro: Lazy List → End → Start.
   - Robot: `robot_lazy_list_end_start_dup` (pass).
5. End scroll left blank space at bottom when beyond-bounds items were present.
   - Cause: extra-items-before offsets leaked into visible placements.
   - Fix: reset main-axis cursor before placing visible items.
   - Robot: `robot_lazy_list_end_alignment` (pass).
6. LazyList viewport height shrank inside the demo tab, clipping items.
   - Cause: LazyList tab bypassed the scroll container, so the Column constrained the 400dp
     list to remaining height.
   - Fix: restore the scrollable tab container and keep the LazyColumn height constrained.
   - Robot: `robot_lazy_list` (LazyListViewport height check).

---

## Mitigations In Progress

- SlotTable group length no longer shrinks (prevents orphaned slots after large structural changes).
- SubcomposeLayout reconciles parent pointers when active children change.
- Live node tracking prevents cascade removal when parents are disposed; live descendants are detached before parent removal.

---

---

## JC Alignment Notes

Key JC behavior to match (sources above):

- Slot reuse is owned by SubcomposeLayoutState (no separate pool in foundation).
- Reuse uses per-content-type caps (default 7) and compatibility checks.
- Slots not retained by policy are disposed immediately and removed from the tree.
- Prefetching schedules precomposition work via a scheduler (idle time), not inside measure.
- Lazy items may emit multiple layout nodes; measure stacks placeables along the main axis.
- Layout size uses the measured main-axis span (currentMainAxisOffset), not total content size.
- Visible items info excludes beyond-bounds items; placement includes both.

---

## Plan (Active)

1. DONE: Align `measure_lazy_list` with JC scroll/backfill flow; keep visible vs beyond-bounds items distinct.
2. DONE: Update placement to use constrained layout sizes + JC spare-space rules (no extra items).
3. DONE: Run cargo fmt/test/clippy/tree + robot tests; update verification stamp.
4. DONE: Stop unbounding constraints; add IntrinsicSize wrap-content support; add LazyList constraint check + viewport semantics for robot bounds validation.

---

## Verification

Robot tests (run independently):

```
cargo run --package desktop-app --example robot_lazy_list --features robot-app
cargo run --package desktop-app --example robot_lazy_lifecycle --features robot-app
cargo run --package desktop-app --example robot_lazy_perf_validation --features robot-app
cargo run --package desktop-app --example robot_lazy_complex_scroll --features robot-app
cargo run --package desktop-app --example robot_lazy_list_after_modifiers --features robot-app
cargo run --package desktop-app --example robot_lazy_list_end_start_dup --features robot-app
cargo run --package desktop-app --example robot_lazy_list_end_alignment --features robot-app
cargo run --package desktop-app --example robot_positioned_boxes_after_lazy_list --features robot-app
cargo run --package desktop-app --example robot_recursive_layout --features robot-app
```

Workspace verification:

```
cargo fmt
cargo test --workspace
cargo clippy --workspace
cargo tree --duplicates
```

Last verified: 2025-12-26 (cargo fmt, cargo test, cargo clippy, cargo tree --duplicates, robot_lazy_list bounds validation, robot_lazy_lifecycle, robot_lazy_perf_validation, robot_lazy_list_end_start_dup, robot_lazy_list_end_alignment)
