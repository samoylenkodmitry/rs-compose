//! Interactive robot demonstration
//!
//! This example shows how to use robot testing to automate interactions
//! while the app is running. You can watch the robot control the app.
//!
//! Run with:
//! ```bash
//! cargo run --package desktop-app --example robot_interactive --features robot-app
//! ```

use desktop_app::app;
use compose_testing::robot_app::RobotApp;
use std::time::Duration;

fn main() {
    println!("=== Robot Interactive Demo ===");
    println!("This demo shows robot testing in action.");
    println!("Watch the window - the robot will interact with the app automatically.\n");

    let robot = RobotApp::launch(800, 600, || {
        app::combined_app();
    });

    std::thread::sleep(Duration::from_millis(500));
    println!("✓ App launched\n");

    // Workflow 1: Counter interactions
    println!("--- Workflow 1: Counter App ---");
    std::thread::sleep(Duration::from_secs(1));

    println!("Moving cursor to increment button...");
    robot.move_to(150.0, 560.0).expect("Failed to move");
    std::thread::sleep(Duration::from_millis(500));

    println!("Clicking increment button 3 times...");
    for i in 1..=3 {
        println!("  Click {}", i);
        robot.click(150.0, 560.0).expect("Failed to click");
        std::thread::sleep(Duration::from_millis(400));
    }

    std::thread::sleep(Duration::from_secs(1));
    println!("✓ Counter workflow complete\n");

    // Workflow 2: Tab navigation
    println!("--- Workflow 2: Tab Navigation ---");
    std::thread::sleep(Duration::from_secs(1));

    let tabs = vec![
        ("Async Runtime", 400.0, 50.0),
        ("Modifiers Showcase", 800.0, 50.0),
        ("Counter App", 70.0, 50.0),
    ];

    for (name, x, y) in tabs {
        println!("Switching to '{}' tab...", name);
        robot.click(x, y).expect("Failed to click tab");
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(800));
    }

    println!("✓ Tab navigation complete\n");

    // Workflow 3: Drag interaction
    println!("--- Workflow 3: Drag Gesture ---");
    std::thread::sleep(Duration::from_secs(1));

    println!("Performing drag from (200, 300) to (500, 400)...");
    robot.drag(200.0, 300.0, 500.0, 400.0).expect("Failed to drag");
    robot.wait_frames(10).expect("Failed to wait");
    std::thread::sleep(Duration::from_secs(1));
    println!("✓ Drag complete\n");

    // Keep window open
    println!("--- Demo Complete ---");
    println!("Window will stay open for 10 seconds...");
    println!("(Press Ctrl+C to exit early)\n");

    for remaining in (1..=10).rev() {
        println!("Closing in {} seconds...", remaining);
        std::thread::sleep(Duration::from_secs(1));
    }

    println!("\nShutting down...");
    robot.shutdown().expect("Failed to shutdown");
    println!("Done!");
}
