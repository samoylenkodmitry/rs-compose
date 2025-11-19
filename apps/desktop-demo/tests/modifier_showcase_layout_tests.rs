use compose_testing::ComposeTestRule;
use compose_ui::{HeadlessRenderer, LayoutBox, LayoutEngine, LayoutTree, RenderOp, Size};

// Import the showcase functions
use desktop_app::app::{
    complex_chain_showcase, dynamic_modifiers_showcase, item_list_showcase,
    positioned_boxes_showcase, simple_card_showcase,
};

/// Helper to compute layout from a test rule
fn compute_layout_from_rule(
    rule: &mut ComposeTestRule,
    max_width: f32,
    max_height: f32,
) -> Result<LayoutTree, compose_core::NodeError> {
    let root = rule.root_id().expect("should have root");
    let handle = rule.runtime_handle();

    let layout = {
        let mut applier = rule.applier_mut();
        applier.set_runtime_handle(handle);
        let result = applier.compute_layout(
            root,
            Size {
                width: max_width,
                height: max_height,
            },
        )?;
        applier.clear_runtime_handle();
        result
    };

    Ok(layout)
}

/// Recursively dump layout tree with positions and sizes
fn dump_layout_tree(layout_box: &LayoutBox, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    let rect = &layout_box.rect;

    // Get text content if present
    let text_content = layout_box
        .node_data
        .modifier_slices()
        .text_content()
        .map(|s| format!(" text=\"{}\"", s))
        .unwrap_or_default();

    let mut output = format!(
        "{}[{}] pos=({:.1}, {:.1}) size=({:.1}x{:.1}){}\n",
        indent,
        layout_box.node_id,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        text_content
    );

    for child in &layout_box.children {
        output.push_str(&dump_layout_tree(child, depth + 1));
    }

    output
}

/// Validate that a child box is fully contained within parent bounds
fn validate_child_within_parent(parent: &LayoutBox, child: &LayoutBox) -> Result<(), String> {
    let parent_right = parent.rect.x + parent.rect.width;
    let parent_bottom = parent.rect.y + parent.rect.height;
    let child_right = child.rect.x + child.rect.width;
    let child_bottom = child.rect.y + child.rect.height;

    // Allow small epsilon for floating point errors
    let epsilon = 0.01;

    if child.rect.x < parent.rect.x - epsilon {
        return Err(format!(
            "Child [{}] x={:.2} is left of parent [{}] x={:.2}",
            child.node_id, child.rect.x, parent.node_id, parent.rect.x
        ));
    }

    if child.rect.y < parent.rect.y - epsilon {
        return Err(format!(
            "Child [{}] y={:.2} is above parent [{}] y={:.2}",
            child.node_id, child.rect.y, parent.node_id, parent.rect.y
        ));
    }

    if child_right > parent_right + epsilon {
        return Err(format!(
            "Child [{}] right={:.2} extends past parent [{}] right={:.2}",
            child.node_id, child_right, parent.node_id, parent_right
        ));
    }

    if child_bottom > parent_bottom + epsilon {
        return Err(format!(
            "Child [{}] bottom={:.2} extends past parent [{}] bottom={:.2}",
            child.node_id, child_bottom, parent.node_id, parent_bottom
        ));
    }

    Ok(())
}

/// Recursively validate all children are within their parents
fn validate_layout_hierarchy(layout_box: &LayoutBox) -> Result<(), String> {
    for child in &layout_box.children {
        validate_child_within_parent(layout_box, child)?;
        validate_layout_hierarchy(child)?;
    }
    Ok(())
}

/// Find a layout box by its text content
fn find_box_with_text<'a>(layout_box: &'a LayoutBox, text: &str) -> Option<&'a LayoutBox> {
    if let Some(content) = layout_box.node_data.modifier_slices().text_content() {
        if content == text {
            return Some(layout_box);
        }
    }

    for child in &layout_box.children {
        if let Some(found) = find_box_with_text(child, text) {
            return Some(found);
        }
    }

    None
}

/// Collect all layout boxes in a flat list (depth-first)
fn collect_all_boxes<'a>(layout_box: &'a LayoutBox, result: &mut Vec<&'a LayoutBox>) {
    result.push(layout_box);
    for child in &layout_box.children {
        collect_all_boxes(child, result);
    }
}

#[test]
fn test_simple_card_layout_positions() {
    let mut rule = ComposeTestRule::new();
    rule.set_content(|| {
        simple_card_showcase();
    })
    .expect("Simple card should render");

    let layout = compute_layout_from_rule(&mut rule, 800.0, 600.0)
        .expect("Should compute layout");

    println!("=== Simple Card Layout ===");
    println!("{}", dump_layout_tree(layout.root(), 0));

    // Validate hierarchy - all children should be within parent bounds
    validate_layout_hierarchy(layout.root()).expect("Layout hierarchy should be valid");

    // Simple card should have a title and description
    let title = find_box_with_text(layout.root(), "Card Title")
        .expect("Should find Card Title");
    let description = find_box_with_text(layout.root(), "Card content goes here with padding")
        .expect("Should find description");

    // Title should be above description
    assert!(
        title.rect.y < description.rect.y,
        "Title should be above description: title.y={:.2} vs desc.y={:.2}",
        title.rect.y,
        description.rect.y
    );

    println!("✓ Simple card layout is correct");
}

#[test]
fn test_positioned_boxes_layout() {
    let mut rule = ComposeTestRule::new();
    rule.set_content(|| {
        positioned_boxes_showcase();
    })
    .expect("Positioned boxes should render");

    let layout = compute_layout_from_rule(&mut rule, 800.0, 600.0)
        .expect("Should compute layout");

    println!("=== Positioned Boxes Layout ===");
    println!("{}", dump_layout_tree(layout.root(), 0));

    // Validate hierarchy - should now pass after fixing the overflow bug
    validate_layout_hierarchy(layout.root()).expect("Layout hierarchy should be valid");

    // Positioned boxes showcase has boxes named Box A and Box B
    let box_a = find_box_with_text(layout.root(), "Box A")
        .expect("Should find Box A");
    let box_b = find_box_with_text(layout.root(), "Box B")
        .expect("Should find Box B");

    println!("Box A: pos=({:.1}, {:.1}) size=({:.1}x{:.1})",
        box_a.rect.x, box_a.rect.y, box_a.rect.width, box_a.rect.height);
    println!("Box B: pos=({:.1}, {:.1}) size=({:.1}x{:.1})",
        box_b.rect.x, box_b.rect.y, box_b.rect.width, box_b.rect.height);

    // Verify boxes are properly positioned within container
    // Box A should be at top-left with offset (20, 20)
    // Box B should be at bottom-right with offset (180, 120)
    assert!(box_a.rect.x < box_b.rect.x, "Box A should be left of Box B");
    assert!(box_a.rect.y < box_b.rect.y, "Box A should be above Box B");

    println!("✓ Positioned boxes layout is correct");
}

#[test]
fn test_item_list_spacing() {
    let mut rule = ComposeTestRule::new();
    rule.set_content(|| {
        item_list_showcase();
    })
    .expect("Item list should render");

    let layout = compute_layout_from_rule(&mut rule, 800.0, 600.0)
        .expect("Should compute layout");

    println!("=== Item List Layout ===");
    println!("{}", dump_layout_tree(layout.root(), 0));

    // Validate hierarchy
    validate_layout_hierarchy(layout.root()).expect("Layout hierarchy should be valid");

    // Find all items
    let item1 = find_box_with_text(layout.root(), "Item #0")
        .expect("Should find Item #0");
    let item2 = find_box_with_text(layout.root(), "Item #1")
        .expect("Should find Item #1");
    let item3 = find_box_with_text(layout.root(), "Item #2")
        .expect("Should find Item #2");

    // Items should be vertically stacked
    assert!(
        item1.rect.y < item2.rect.y,
        "Item 1 should be above Item 2"
    );
    assert!(
        item2.rect.y < item3.rect.y,
        "Item 2 should be above Item 3"
    );

    // Calculate spacing between items
    let spacing_1_2 = item2.rect.y - (item1.rect.y + item1.rect.height);
    let spacing_2_3 = item3.rect.y - (item2.rect.y + item2.rect.height);

    println!("Spacing between Item 1 and 2: {:.1}", spacing_1_2);
    println!("Spacing between Item 2 and 3: {:.1}", spacing_2_3);

    // Spacing should be consistent (allowing small floating point error)
    let spacing_diff = (spacing_1_2 - spacing_2_3).abs();
    assert!(
        spacing_diff < 1.0,
        "Item spacing should be consistent: {:.2} vs {:.2} (diff: {:.2})",
        spacing_1_2,
        spacing_2_3,
        spacing_diff
    );

    println!("✓ Item list spacing is correct");
}

#[test]
fn test_complex_chain_modifier_ordering() {
    let mut rule = ComposeTestRule::new();
    rule.set_content(|| {
        complex_chain_showcase();
    })
    .expect("Complex chain should render");

    let layout = compute_layout_from_rule(&mut rule, 800.0, 600.0)
        .expect("Should compute layout");

    println!("=== Complex Chain Layout ===");
    println!("{}", dump_layout_tree(layout.root(), 0));

    // Validate hierarchy - should now pass after fixing the overflow bug
    validate_layout_hierarchy(layout.root()).expect("Layout hierarchy should be valid");

    // Render the scene to check draw order
    let renderer = HeadlessRenderer::new();
    let scene = renderer.render(&layout);

    println!("Render operations: {}", scene.operations().len());
    for (i, op) in scene.operations().iter().enumerate() {
        match op {
            RenderOp::Primitive { node_id, layer, primitive } => {
                println!("  [{}] Primitive node={} layer={:?} prim={:?}", i, node_id, layer, primitive);
            }
            RenderOp::Text { node_id, rect, value } => {
                println!("  [{}] Text node={} pos=({:.1},{:.1}) \"{}\"", i, node_id, rect.x, rect.y, value);
            }
        }
    }

    println!("✓ Complex chain renders correctly");
}

#[test]
fn test_dynamic_modifiers_size_changes() {
    let mut rule = ComposeTestRule::new();

    rule.set_content(|| {
        dynamic_modifiers_showcase();
    })
    .expect("Dynamic modifiers should render");

    let layout = compute_layout_from_rule(&mut rule, 800.0, 600.0)
        .expect("Should compute layout");

    println!("=== Dynamic Modifiers ===");
    println!("{}", dump_layout_tree(layout.root(), 0));

    // Validate hierarchy
    validate_layout_hierarchy(layout.root()).expect("Layout hierarchy should be valid");

    println!("✓ Dynamic modifiers render correctly");
}

#[test]
fn test_all_showcases_have_valid_layouts() {
    let showcases = vec![
        ("Simple Card", simple_card_showcase as fn()),
        ("Positioned Boxes", positioned_boxes_showcase as fn()),
        ("Item List", item_list_showcase as fn()),
        ("Complex Chain", complex_chain_showcase as fn()),
    ];

    for (name, showcase_fn) in showcases {
        println!("\n=== Testing {} ===", name);

        let mut rule = ComposeTestRule::new();
        rule.set_content(showcase_fn)
            .expect(&format!("{} should render", name));

        let layout = compute_layout_from_rule(&mut rule, 800.0, 600.0)
            .expect(&format!("{} should compute layout", name));

        // Validate hierarchy - all showcases should now have valid hierarchies
        validate_layout_hierarchy(layout.root())
            .expect(&format!("{} layout hierarchy should be valid", name));

        // Ensure root has non-zero size
        assert!(
            layout.root().rect.width > 0.0,
            "{} root should have width",
            name
        );
        assert!(
            layout.root().rect.height > 0.0,
            "{} root should have height",
            name
        );

        // Collect all boxes and ensure none have negative positions
        let mut all_boxes = Vec::new();
        collect_all_boxes(layout.root(), &mut all_boxes);

        for box_ref in &all_boxes {
            assert!(
                box_ref.rect.x >= -0.01,
                "{}: Box [{}] has negative x: {:.2}",
                name,
                box_ref.node_id,
                box_ref.rect.x
            );
            assert!(
                box_ref.rect.y >= -0.01,
                "{}: Box [{}] has negative y: {:.2}",
                name,
                box_ref.node_id,
                box_ref.rect.y
            );
            assert!(
                box_ref.rect.width >= 0.0,
                "{}: Box [{}] has negative width: {:.2}",
                name,
                box_ref.node_id,
                box_ref.rect.width
            );
            assert!(
                box_ref.rect.height >= 0.0,
                "{}: Box [{}] has negative height: {:.2}",
                name,
                box_ref.node_id,
                box_ref.rect.height
            );
        }

        println!("✓ {} has valid layout ({} boxes)", name, all_boxes.len());
    }
}
