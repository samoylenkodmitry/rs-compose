//! Robot demonstration - watch the robot interact with the app
//!
//! Run with:
//! ```bash
//! cargo run --package desktop-app --example robot_demo --features robot-app
//! ```

use desktop_app::app;
use compose_testing::robot_app::RobotApp;
use std::time::Duration;

fn main() {
    println!("Launching app with robot control...");

    let robot = RobotApp::launch(800, 600, || {
        app::combined_app();
    });

    println!("App launched! Starting robot interactions in 1 second...");
    std::thread::sleep(Duration::from_secs(1));

    // Click the increment button 5 times
    println!("Clicking increment button 5 times...");
    for i in 1..=5 {
        println!("  Click {}/5", i);
        robot.click(150.0, 560.0).expect("Failed to click");
        robot.wait_frames(5).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(300));
    }

    println!("Switching to Async Runtime tab...");
    robot.click(400.0, 50.0).expect("Failed to click tab");
    robot.wait_frames(10).expect("Failed to wait");
    std::thread::sleep(Duration::from_secs(1));

    println!("Switching to Modifiers Showcase tab...");
    robot.click(800.0, 50.0).expect("Failed to click tab");
    robot.wait_frames(10).expect("Failed to wait");
    std::thread::sleep(Duration::from_secs(1));

    println!("Going back to Counter App tab...");
    robot.click(70.0, 50.0).expect("Failed to click tab");
    robot.wait_frames(10).expect("Failed to wait");
    std::thread::sleep(Duration::from_secs(1));

    println!("Performing drag gesture...");
    robot.drag(200.0, 300.0, 500.0, 400.0).expect("Failed to drag");
    robot.wait_frames(10).expect("Failed to wait");
    std::thread::sleep(Duration::from_secs(1));

    println!("Demo complete! Keeping window open for 5 more seconds...");
    std::thread::sleep(Duration::from_secs(5));

    println!("Shutting down...");
    robot.shutdown().expect("Failed to shutdown");
}
