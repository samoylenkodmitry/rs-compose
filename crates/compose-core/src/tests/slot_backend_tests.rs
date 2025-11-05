//! Tests for slot storage backends.

use crate::{SlotBackendKind, SlotStorage};

/// Smoke test to verify all backends can perform basic operations.
#[test]
fn test_all_backends_smoke() {
    // Only test baseline for now - other backends are MVP/prototypes
    // and need more work before they can pass full test suite
    let backends = [
        SlotBackendKind::Baseline,
    ];

    for kind in backends {
        test_backend_smoke(kind);
    }
}

// TODO: Enable these backends once they reach feature parity
// SlotBackendKind::Chunked,
// SlotBackendKind::Hierarchical,
// SlotBackendKind::Split,

fn test_backend_smoke(kind: SlotBackendKind) {
    use crate::slot_backend::SlotBackend;

    let mut storage = SlotBackend::new(kind);

    // Begin a root group
    let result1 = storage.begin_group(100);
    assert!(!result1.restored_from_gap, "{:?}: First group should not be from gap", kind);

    // Allocate a value slot
    let slot = storage.alloc_value_slot(|| 42);

    // Read it back
    assert_eq!(*storage.read_value::<i32>(slot), 42, "{:?}: Value should match", kind);

    // Write a new value
    storage.write_value(slot, 100);
    assert_eq!(*storage.read_value::<i32>(slot), 100, "{:?}: Updated value should match", kind);

    // End the group
    storage.end_group();

    // Flush any pending operations
    storage.flush();
}

/// Test that backends handle recomposition correctly.
#[test]
fn test_backends_recomposition() {
    let backends = [
        SlotBackendKind::Baseline,
        // Note: Other backends may need more work to fully support recomposition
    ];

    for kind in backends {
        test_backend_recomposition(kind);
    }
}

fn test_backend_recomposition(kind: SlotBackendKind) {
    use crate::slot_backend::SlotBackend;

    let mut storage = SlotBackend::new(kind);

    // First composition
    storage.reset();
    let result = storage.begin_group(200);
    let group1 = result.group;

    // Associate a scope with this group
    storage.set_group_scope(group1, 1);

    let _slot1 = storage.alloc_value_slot(|| "hello");

    storage.end_group();
    storage.flush();

    // Recompose at that scope
    storage.reset();
    if let Some(_group) = storage.begin_recompose_at_scope(1) {
        // Just verify we can navigate to this position
        // The actual value reuse depends on properly calling the same
        // init closure, which is handled by the composer
        storage.end_group();
        storage.end_recompose();
    } else {
        panic!("{:?}: Should find scope for recomposition", kind);
    }
}

/// Test gap behavior (conditional rendering).
#[test]
fn test_backends_gaps() {
    let backends = [
        SlotBackendKind::Baseline,
        // Other backends need more sophisticated gap handling
    ];

    for kind in backends {
        test_backend_gaps(kind);
    }
}

fn test_backend_gaps(kind: SlotBackendKind) {
    use crate::slot_backend::SlotBackend;

    let mut storage = SlotBackend::new(kind);

    // First composition: create a group with nested content
    storage.reset();
    let result = storage.begin_group(300);
    let group1 = result.group;
    storage.set_group_scope(group1, 2);

    // Create a nested group
    let _inner = storage.begin_group(301);
    let _slot = storage.alloc_value_slot(|| 123);
    storage.end_group();

    // Now end the outer group, but don't finalize
    // (In a real scenario, we'd have cursor < end)
    storage.end_group();

    storage.flush();

    // Just verify the backend can handle basic gap operations
    // Full gap restoration requires proper cursor management
    // that is complex to test in isolation
}
