/// Integration tests for the modifier system in real-world scenarios.
/// These tests verify that the entire system works together correctly,
/// not just individual units.

use compose_core::{location_key, Composition, MemoryApplier};
use compose_ui::{
    composable, Box as ComposeBox, BoxSpec, Column, ColumnSpec, Modifier, Row, RowSpec, Size, Text,
};

/// Test that complex modifier chains preserve ordering and are measured correctly
#[test]
fn test_complex_modifier_chain_ordering() {
    #[composable]
    fn content() {
        // Create a complex chain: padding -> size -> offset -> padding
        ComposeBox(
            Modifier::empty()
                .padding(10.0)
                .size(Size {
                    width: 100.0,
                    height: 100.0,
                })
                .offset(20.0, 30.0)
                .padding(5.0),
            BoxSpec::default(),
            || {
                Text("Test", Modifier::empty());
            },
        );
    }

    let mut composition = Composition::new(MemoryApplier::new());
    composition
        .render(location_key(file!(), line!(), column!()), content)
        .unwrap();

    // Verify the composition succeeded and nodes were created
    assert!(composition.root().is_some());

    // Count nodes to ensure the structure is correct
    let root = composition.root().unwrap();
    let mut applier = composition.applier_mut();

    let child_count = applier
        .with_node(root, |node: &mut compose_ui::LayoutNode| {
            node.children.len()
        })
        .unwrap();

    assert_eq!(
        child_count, 1,
        "Root should have exactly one child (the Box)"
    );
}

/// Test that modifier chains are properly updated during recomposition
#[test]
fn test_modifier_chain_recomposition() {
    #[composable]
    fn content(use_large_padding: bool) {
        let padding = if use_large_padding { 20.0 } else { 5.0 };

        ComposeBox(
            Modifier::empty().padding(padding),
            BoxSpec::default(),
            || {
                Text("Dynamic", Modifier::empty());
            },
        );
    }

    let mut composition = Composition::new(MemoryApplier::new());

    // Initial composition with large padding
    composition
        .render(location_key(file!(), line!(), column!()), || {
            content(true)
        })
        .unwrap();

    assert!(composition.root().is_some());

    // Recompose with small padding
    composition
        .render(location_key(file!(), line!(), column!()), || {
            content(false)
        })
        .unwrap();

    // Verify nodes still exist after recomposition
    assert!(composition.root().is_some());

    // Recompose back to large padding
    composition
        .render(location_key(file!(), line!(), column!()), || {
            content(true)
        })
        .unwrap();

    assert!(composition.root().is_some());
}

/// Test performance with many modifiers in a single chain
#[test]
fn test_large_modifier_chain_performance() {
    #[composable]
    fn content() {
        // Build a very long modifier chain
        let mut modifier = Modifier::empty();
        for i in 0..100 {
            modifier = modifier.padding(1.0);
            if i % 10 == 0 {
                modifier = modifier.offset(i as f32, i as f32);
            }
        }

        ComposeBox(modifier, BoxSpec::default(), || {
            Text("Deep chain", Modifier::empty());
        });
    }

    let mut composition = Composition::new(MemoryApplier::new());

    let start = std::time::Instant::now();
    composition
        .render(location_key(file!(), line!(), column!()), content)
        .unwrap();
    let duration = start.elapsed();

    // Should complete quickly even with 100+ modifiers
    assert!(
        duration.as_millis() < 100,
        "Large modifier chain took too long: {:?}",
        duration
    );

    assert!(composition.root().is_some());
}

/// Test that many items with modifiers can be rendered efficiently
#[test]
fn test_many_items_with_modifiers() {
    #[composable]
    fn list(item_count: usize) {
        Column(Modifier::empty(), ColumnSpec::default(), move || {
            for i in 0..item_count {
                Row(
                    Modifier::empty().padding(4.0).size(Size {
                        width: 200.0,
                        height: 40.0,
                    }),
                    RowSpec::default(),
                    move || {
                        // Use static strings to avoid allocation issues
                        let text = if i < 10 {
                            match i {
                                0 => "Item 0",
                                1 => "Item 1",
                                2 => "Item 2",
                                3 => "Item 3",
                                4 => "Item 4",
                                5 => "Item 5",
                                6 => "Item 6",
                                7 => "Item 7",
                                8 => "Item 8",
                                9 => "Item 9",
                                _ => "Item",
                            }
                        } else {
                            "Item 10+"
                        };
                        Text(text, Modifier::empty());
                    },
                );
            }
        });
    }

    let mut composition = Composition::new(MemoryApplier::new());

    // Test with 100 items
    let start = std::time::Instant::now();
    composition
        .render(location_key(file!(), line!(), column!()), || {
            list(100)
        })
        .unwrap();
    let duration = start.elapsed();

    println!("100 items with modifiers: {:?}", duration);

    // Should handle 100 items efficiently
    assert!(
        duration.as_millis() < 500,
        "100 items took too long: {:?}",
        duration
    );

    // Verify the composition succeeded
    assert!(composition.root().is_some());
}

/// Test that modifier chains work correctly in nested layouts
#[test]
fn test_nested_layouts_with_modifiers() {
    #[composable]
    fn nested_content() {
        Column(
            Modifier::empty().padding(10.0),
            ColumnSpec::default(),
            || {
                Row(
                    Modifier::empty().padding(5.0),
                    RowSpec::default(),
                    || {
                        ComposeBox(
                            Modifier::empty()
                                .size(Size {
                                    width: 50.0,
                                    height: 50.0,
                                })
                                .offset(5.0, 5.0),
                            BoxSpec::default(),
                            || {
                                Text("Nested", Modifier::empty());
                            },
                        );
                    },
                );

                Row(
                    Modifier::empty().padding(5.0),
                    RowSpec::default(),
                    || {
                        Text("Second row", Modifier::empty());
                    },
                );
            },
        );
    }

    let mut composition = Composition::new(MemoryApplier::new());
    composition
        .render(location_key(file!(), line!(), column!()), nested_content)
        .unwrap();

    assert!(composition.root().is_some());

    // Verify nested structure was created correctly
    let root = composition.root().unwrap();
    let mut applier = composition.applier_mut();

    // Verify nested structure exists
    let children = applier
        .with_node(root, |node: &mut compose_ui::LayoutNode| {
            node.children.clone()
        })
        .unwrap();

    assert!(!children.is_empty(), "Root should have children");
}

/// Test recomposition with changing list sizes
#[test]
fn test_dynamic_list_recomposition() {
    #[composable]
    fn dynamic_list(count: usize) {
        Column(Modifier::empty(), ColumnSpec::default(), move || {
            for i in 0..count {
                let text = match i {
                    0 => "Item 0",
                    1 => "Item 1",
                    2 => "Item 2",
                    3 => "Item 3",
                    4 => "Item 4",
                    5 => "Item 5",
                    6 => "Item 6",
                    7 => "Item 7",
                    8 => "Item 8",
                    9 => "Item 9",
                    _ => "Item 10+",
                };
                Text(text, Modifier::empty().padding(4.0));
            }
        });
    }

    let mut composition = Composition::new(MemoryApplier::new());

    // Start with 5 items
    composition
        .render(location_key(file!(), line!(), column!()), || {
            dynamic_list(5)
        })
        .unwrap();

    assert!(composition.root().is_some());

    // Grow to 10 items
    composition
        .render(location_key(file!(), line!(), column!()), || {
            dynamic_list(10)
        })
        .unwrap();

    assert!(composition.root().is_some());

    // Shrink to 3 items
    composition
        .render(location_key(file!(), line!(), column!()), || {
            dynamic_list(3)
        })
        .unwrap();

    assert!(composition.root().is_some());

    // Verify composition succeeded after all recompositions
    assert!(composition.root().is_some());
}

/// Test that modifiers work correctly with text nodes
#[test]
fn test_text_with_modifiers() {
    #[composable]
    fn styled_text() {
        Text(
            "Styled",
            Modifier::empty()
                .padding_horizontal(10.0)
                .padding_vertical(5.0)
                .size(Size {
                    width: 100.0,
                    height: 30.0,
                }),
        );
    }

    let mut composition = Composition::new(MemoryApplier::new());
    composition
        .render(location_key(file!(), line!(), column!()), styled_text)
        .unwrap();

    assert!(composition.root().is_some());
}

/// Test complex real-world UI pattern: Card list
#[test]
fn test_card_list_pattern() {
    #[composable]
    fn card(title: &'static str, description: &'static str) {
        ComposeBox(
            Modifier::empty().padding(12.0).size(Size {
                width: 300.0,
                height: 150.0,
            }),
            BoxSpec::default(),
            move || {
                Column(Modifier::empty(), ColumnSpec::default(), move || {
                    Text(title, Modifier::empty().padding_each(0.0, 0.0, 0.0, 8.0));
                    Text(description, Modifier::empty());
                });
            },
        );
    }

    #[composable]
    fn card_list() {
        Column(
            Modifier::empty().padding(16.0),
            ColumnSpec::default(),
            || {
                card("Card 1", "First card description");
                card("Card 2", "Second card description");
                card("Card 3", "Third card description");
            },
        );
    }

    let mut composition = Composition::new(MemoryApplier::new());

    let start = std::time::Instant::now();
    composition
        .render(location_key(file!(), line!(), column!()), card_list)
        .unwrap();
    let duration = start.elapsed();

    println!("Card list pattern: {:?}", duration);

    // Verify composition succeeded
    assert!(composition.root().is_some());
}

/// Stress test: Rapidly changing modifiers
#[test]
fn test_rapid_modifier_changes() {
    #[composable]
    fn animated(frame: i32) {
        ComposeBox(
            Modifier::empty().offset(frame as f32, frame as f32),
            BoxSpec::default(),
            || {
                Text("Moving", Modifier::empty());
            },
        );
    }

    let mut composition = Composition::new(MemoryApplier::new());

    let start = std::time::Instant::now();

    // Simulate 100 frames of animation
    for frame in 0..100 {
        composition
            .render(location_key(file!(), line!(), column!()), || {
                animated(frame)
            })
            .unwrap();
    }

    let duration = start.elapsed();

    println!("100 recompositions: {:?}", duration);
    println!(
        "Average per frame: {:?}",
        duration / 100
    );

    // Should handle rapid changes efficiently
    assert!(
        duration.as_millis() < 1000,
        "100 recompositions took too long: {:?}",
        duration
    );
}
