# Scrolling in Compose-RS: Findings & Implementation Plan

This document outlines the current state of input handling in Compose-RS, identifies gaps compared to Jetpack Compose, and proposes a plan to implement scrolling support.

**Reference:** [Deep Dive into Jetpack Compose Scrolling](./SCROLL_IN_COMPOSE_KT.md)

## 1. Current State Analysis

### ✅ The Good: Async/Await
The biggest risk was whether Rust's `async/await` could replicate Kotlin's `suspend` coroutines for input handling.
*   **Finding:** It works! `PointerInputScope` is already implemented using Rust futures.
*   **Code:** `crates/compose-ui/src/modifier/pointer_input.rs` implements `await_pointer_event_scope` which allows writing async input handlers that look almost identical to Kotlin.

### ❌ The Bad: Primitive Event Dispatch
The current event dispatch system is insufficient for scrolling.
*   **Retained Mode Hit Testing:** `RenderScene` stores a flat list of `HitRegion`s.
*   **No Hierarchy:** There is no tree traversal for events. `AppShell` calls `hit_test` which returns a single `HitRegion` (the top-most one).
*   **No Interception:** A parent (like `Scrollable`) cannot intercept events meant for a child (like a `Button`). In the current system, if a Button is on top, it eats the event, and the Scrollable never sees it.

### ⚠️ The Missing: Core Scrolling Components
The following components are completely missing:
1.  **`Modifier.scrollable`**: The high-level modifier.
2.  **`ScrollState`**: State holder for scroll position.
3.  **`ScrollNode`**: The layout modifier that physically moves content.
4.  **`PointerInputChange.consume()`**: No way to mark an event as "handled" to stop propagation.

## 2. Implementation Plan

### Phase 1: Event Dispatch Refactor (The Foundation)
We must move from "Flat Hit Regions" to "Tree-Based Dispatch".

1.  **Update `HitTestResult`**:
    *   Instead of returning `Option<HitRegion>`, `hit_test` should return a `HitTestResult` containing a **chain** of nodes.
    *   This chain should represent the path from the root down to the leaf node that was hit.

2.  **Implement `PointerInputEventProcessor`**:
    *   Create a processor that takes the `HitTestResult` and dispatches the event.
    *   Implement the **3-Pass System** (or at least 2 passes):
        *   **Initial Pass (Tunneling)**: Root -> Leaf. Allows parents to intercept (crucial for scrolling).
        *   **Main Pass (Bubbling)**: Leaf -> Root. Standard event handling.

3.  **Update `PointerEvent`**:
    *   Add `is_consumed` flag (or similar mechanism) to `PointerInputChange`.
    *   Ensure changes propagate through the chain.

### Phase 2: Scroll Components
Once the foundation is ready, we can implement the scrolling logic.

1.  **`ScrollState`**:
    *   Implement `ScrollState` struct backed by `MutableState<i32>`.
    *   Add `dispatch_raw_delta(delta: f32)` method.

2.  **`ScrollNode` (Layout Modifier)**:
    *   Create a `LayoutModifierNode` that takes `ScrollState`.
    *   **Measure**: Pass infinite constraints to child in the scroll direction.
    *   **Layout**: Read `ScrollState.value` and place child at `-value` offset.

3.  **`ScrollableNode` (Input Modifier)**:
    *   Create a `PointerInputNode` that uses `await_pointer_event_scope`.
    *   Detect drag gestures.
    *   Update `ScrollState` via `dispatch_raw_delta`.
    *   **Crucial**: Use the "Initial Pass" to monitor events even if a child button is pressed, and "steal" the stream if a drag threshold is passed.

## 3. Comparison Summary

| Feature | Jetpack Compose | Current Compose-RS | Target Compose-RS |
| :--- | :--- | :--- | :--- |
| **Hit Test** | Tree Traversal | Flat List (Z-index) | Tree Traversal |
| **Dispatch** | 3-Pass (Tunnel/Bubble) | Direct Callback | 3-Pass (Tunnel/Bubble) |
| **Async** | Coroutines | Futures (Done) | Futures (Done) |
| **Scroll** | `Scrollable` + `ScrollNode` | None | `Scrollable` + `ScrollNode` |
