//! Robot testing with real app rendering
//!
//! This module provides robot testing that launches the actual desktop app
//! with real rendering, enabling screenshot testing, visual validation, etc.
//!
//! **Important:** This module wraps the REAL desktop app runner from `compose-app`,
//! ensuring tests use the exact same code path as the production app.

// Re-export the real desktop robot runner
pub use compose_app::desktop_robot::{run_with_robot, RobotAppHandle, RobotCommand};

use compose_app::AppSettings;

/// Create a robot-controlled app for testing
///
/// This launches the REAL desktop app with all production code paths,
/// but adds robot control on top for automated testing.
///
/// # Example
///
/// ```no_run
/// use compose_testing::robot_app::RobotApp;
///
/// let robot = RobotApp::launch(800, 600, || {
///     my_app();
/// });
///
/// robot.click(400.0, 300.0).unwrap();
/// std::thread::sleep(std::time::Duration::from_millis(500));
/// robot.screenshot("test.png").unwrap();
/// robot.shutdown().unwrap();
/// ```
pub struct RobotApp {
    handle: RobotAppHandle,
}

impl RobotApp {
    /// Launch the real app with robot control
    pub fn launch<F>(width: u32, height: u32, content: F) -> Self
    where
        F: FnMut() + 'static + Send,
    {
        let settings = AppSettings {
            window_title: "Robot Test App".into(),
            initial_width: width,
            initial_height: height,
            fonts: None,
            android_use_system_fonts: false,
        };

        let handle = run_with_robot(settings, content);

        Self { handle }
    }

    /// Click at the given coordinates (in physical pixels)
    pub fn click(&self, x: f32, y: f32) -> Result<(), String> {
        self.handle.click(x, y)
    }

    /// Move cursor to coordinates (in physical pixels)
    pub fn move_to(&self, x: f32, y: f32) -> Result<(), String> {
        self.handle.move_to(x, y)
    }

    /// Drag from one position to another
    pub fn drag(&self, from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> Result<(), String> {
        self.handle.drag(from_x, from_y, to_x, to_y)
    }

    /// Take a screenshot and save to file
    pub fn screenshot(&self, path: &str) -> Result<(), String> {
        self.handle.screenshot(path)
    }

    /// Shutdown the app
    pub fn shutdown(&self) -> Result<(), String> {
        self.handle.shutdown()
    }

    /// Wait for a duration (helper for tests)
    pub fn wait(&self, millis: u64) {
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires display
    fn test_robot_app_launches() {
        let robot = RobotApp::launch(800, 600, || {
            // Empty app
        });

        robot.wait(500);
        robot.shutdown().unwrap();
    }
}
