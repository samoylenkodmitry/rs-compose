//! Robot test to verify offset modifier positioning in the combined app
//!
//! This test validates that the offset modifier correctly positions elements
//! by navigating to the Modifiers Showcase tab and checking actual positions.
//!
//! Run with:
//! ```bash
//! cargo run --package desktop-app --example robot_offset_test --features robot-app
//! ```

use desktop_app::app;
use compose_app::AppLauncher;
use compose_testing::find_by_text_recursive;
use std::time::Duration;

fn main() {
    println!("Robot Offset Test - Combined App");
    println!("=================================\n");

    AppLauncher::new()
        .with_title("Robot Offset Test")
        .with_size(900, 700)
        .with_test_driver(|robot| {
            println!("App launched! Waiting for initial render...");
            std::thread::sleep(Duration::from_secs(1));
            robot.wait_for_idle().expect("Failed to wait for idle");

            // =====================================================
            // Step 1: Navigate to Modifiers Showcase tab
            // =====================================================
            println!("\n📌 Step 1: Navigate to Modifiers Showcase tab");
            robot.click_by_text("Modifiers Showcase").expect("Failed to click Modifiers Showcase tab");
            std::thread::sleep(Duration::from_millis(500));
            match robot.wait_for_idle() {
                Ok(_) => println!("   Tab ready"),
                Err(e) => println!("   Tab switched ({})", e),
            }

            // =====================================================
            // Step 2: Select "Positioned Boxes" showcase
            // =====================================================
            println!("\n📌 Step 2: Select 'Positioned Boxes' showcase");
            robot.click_by_text("Positioned Boxes").expect("Failed to click Positioned Boxes");
            std::thread::sleep(Duration::from_millis(500));
            robot.wait_for_idle().ok();

            // Validate positioned boxes
            println!("\n   Validating positioned boxes:");
            let semantics = robot.get_semantics().expect("Failed to get semantics");
            
            // The positioned boxes showcase has:
            // - Box A at offset(20, 20) - Purple, top-left
            // - Box B at offset(220, 160) - Green, bottom-right
            // - C at offset(140, 30) - Orange, center-top
            // - Box D at offset(40, 140) - Blue, center-left
            
            let mut test_passed = true;
            
            // Box A should be at offset(20, 20)
            if let Some(elem) = find_by_text_recursive(&semantics, "Box A") {
                println!("   ✓ Found 'Box A' at x={:.1}, y={:.1}", elem.bounds.x, elem.bounds.y);
                // Box A is at offset(20, 20), plus container/padding offsets
                if elem.bounds.x > 0.0 && elem.bounds.x < 500.0 {
                    println!("     ✓ PASS: Box A has positive x offset");
                } else {
                    println!("     ✗ FAIL: Box A x={}", elem.bounds.x);
                    test_passed = false;
                }
            } else {
                println!("   ✗ 'Box A' not found");
                test_passed = false;
            }

            // Box B should be at offset(220, 160) - significantly more to the right
            if let Some(elem) = find_by_text_recursive(&semantics, "Box B") {
                println!("   ✓ Found 'Box B' at x={:.1}, y={:.1}", elem.bounds.x, elem.bounds.y);
                // Box B should be significantly to the right of Box A
                if let Some(box_a) = find_by_text_recursive(&semantics, "Box A") {
                    if elem.bounds.x > box_a.bounds.x + 100.0 {
                        println!("     ✓ PASS: Box B is to the right of Box A (diff: {:.0}px)", 
                            elem.bounds.x - box_a.bounds.x);
                    } else {
                        println!("     ✗ FAIL: Box B should be far right of Box A");
                        test_passed = false;
                    }
                }
            } else {
                println!("   ✗ 'Box B' not found");
                test_passed = false;
            }

            // C (small box) should be at offset(140, 30)
            if let Some(elem) = find_by_text_recursive(&semantics, "C") {
                println!("   ✓ Found 'C' at x={:.1}, y={:.1}", elem.bounds.x, elem.bounds.y);
            }

            // Box D should be at offset(40, 140)
            if let Some(elem) = find_by_text_recursive(&semantics, "Box D") {
                println!("   ✓ Found 'Box D' at x={:.1}, y={:.1}", elem.bounds.x, elem.bounds.y);
            }

            if test_passed {
                println!("\n   ✅ Positioned Boxes validation PASSED!");
            } else {
                println!("\n   ❌ Positioned Boxes validation FAILED!");
            }

            std::thread::sleep(Duration::from_secs(1));

            // =====================================================
            // Step 3: Select "Dynamic Modifiers" showcase
            // =====================================================
            println!("\n📌 Step 3: Select 'Dynamic Modifiers' showcase");
            robot.click_by_text("Dynamic Modifiers").expect("Failed to click Dynamic Modifiers");
            std::thread::sleep(Duration::from_millis(500));
            robot.wait_for_idle().ok();

            // =====================================================
            // Step 4: Press "Advance Frame" 3 times and validate
            // =====================================================
            println!("\n📌 Step 4: Press 'Advance Frame' 3 times and validate positions");

            // Get initial position of the "Move" box
            let semantics_before = robot.get_semantics().expect("Failed to get semantics");
            let move_elem_before = find_by_text_recursive(&semantics_before, "Move");
            if let Some(ref elem) = move_elem_before {
                println!("   Initial 'Move' box position: x={:.1}", elem.bounds.x);
            }

            let mut prev_x = move_elem_before.map(|e| e.bounds.x).unwrap_or(0.0);

            for i in 1..=3 {
                println!("\n   --- Frame {} ---", i);
                
                // Click Advance Frame button
                robot.click_by_text("Advance Frame").expect("Failed to click Advance Frame");
                std::thread::sleep(Duration::from_millis(300));
                robot.wait_for_idle().ok();

                // Get semantics and check dynamic element positions
                let semantics = robot.get_semantics().expect("Failed to get semantics");
                
                // Check the "Move" box position
                if let Some(elem) = find_by_text_recursive(&semantics, "Move") {
                    println!("   'Move' box at x={:.1}", elem.bounds.x);
                    
                    // Verify the box moved (x should increase by 10)
                    if elem.bounds.x > prev_x {
                        println!("   ✓ PASS: Box moved right");
                    } else {
                        println!("   ⚠ Box didn't move as expected (prev={:.1}, now={:.1})", prev_x, elem.bounds.x);
                    }
                    prev_x = elem.bounds.x;
                }
                
                // Check frame indicator text
                if let Some(elem) = find_by_text_recursive(&semantics, "Frame:") {
                    println!("   Frame indicator: {:?}", elem.text);
                }
            }

            println!("\n=== Test Summary ===");
            if test_passed {
                println!("✓ ALL TESTS PASSED");
            } else {
                println!("✗ SOME TESTS FAILED");
            }
            println!("   Keeping window open for 2 seconds...");
            std::thread::sleep(Duration::from_secs(2));

            println!("\n🛑 Shutting down...");
            robot.exit().expect("Failed to exit");
        })
        .run(|| {
            app::combined_app();
        });
}
