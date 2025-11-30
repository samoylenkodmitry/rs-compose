use compose_app::desktop::{run_with_robot, DesktopRobotApp, DesktopRobotError, RobotFrameCapture};
use compose_app::AppSettings;

use crate::robot::SceneSnapshot;

pub type WgpuRobotError = DesktopRobotError;

/// Robot harness that drives the real desktop runtime, WGPU renderer, and event
/// loop for black-box testing of full applications.
pub struct WgpuRobotApp {
    inner: DesktopRobotApp,
}

impl WgpuRobotApp {
    /// Launch a robot-controlled application using the provided viewport size
    /// and renderer fonts.
    pub fn launch_with_fonts(
        width: u32,
        height: u32,
        fonts: &'static [&'static [u8]],
        content: impl FnMut() + Send + 'static,
    ) -> Result<Self, DesktopRobotError> {
        Self::launch_internal(width, height, Some(fonts), content)
    }

    /// Launch a robot-controlled application using the provided viewport size
    /// without bundled fonts. Text rendering will fail unless your UI draws
    /// only shapes.
    pub fn launch(
        width: u32,
        height: u32,
        content: impl FnMut() + Send + 'static,
    ) -> Result<Self, DesktopRobotError> {
        Self::launch_internal(width, height, None, content)
    }

    fn launch_internal(
        width: u32,
        height: u32,
        fonts: Option<&'static [&'static [u8]]>,
        content: impl FnMut() + Send + 'static,
    ) -> Result<Self, DesktopRobotError> {
        let settings = AppSettings {
            initial_width: width,
            initial_height: height,
            fonts,
            ..AppSettings::default()
        };

        let app = run_with_robot(settings, content)?;
        app.set_viewport(width as f32, height as f32)?;

        Ok(Self { inner: app })
    }

    /// Resize the viewport used for layout.
    pub fn set_viewport(&self, width: f32, height: f32) -> Result<(), DesktopRobotError> {
        self.inner.set_viewport(width, height)
    }

    /// Drive the app until no redraws are requested or the iteration limit is reached.
    pub fn pump_until_idle(&self, max_iterations: usize) -> Result<(), DesktopRobotError> {
        self.inner.pump_until_idle(max_iterations)
    }

    /// Move the virtual pointer to the provided coordinates, dispatching pointer move
    /// events to any hit targets.
    pub fn move_pointer(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.inner.move_pointer(x, y)
    }

    /// Press the virtual pointer at the provided coordinates.
    pub fn press(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.inner.press(x, y)
    }

    /// Release the virtual pointer at the provided coordinates.
    pub fn release(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.inner.release(x, y)
    }

    /// Convenience helper that presses and then releases the pointer at the provided
    /// coordinates.
    pub fn click(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.inner.click(x, y)
    }

    /// Capture a snapshot of the current render scene for assertions.
    pub fn snapshot(&self) -> Result<SceneSnapshot, DesktopRobotError> {
        let snapshot = self.inner.snapshot()?;
        Ok(SceneSnapshot::from_robot_snapshot(&snapshot))
    }

    /// Capture the currently rendered frame into RGBA bytes suitable for
    /// screenshot comparisons.
    pub fn capture_frame(&self) -> Result<FrameCapture, DesktopRobotError> {
        let frame = self.inner.capture_frame()?;
        Ok(FrameCapture::from_robot_capture(frame))
    }

    /// Shut down the robot-controlled application.
    pub fn close(self) -> Result<(), DesktopRobotError> {
        self.inner.close()
    }
}

/// In-memory screenshot of a rendered frame.
pub struct FrameCapture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl FrameCapture {
    /// Raw RGBA pixels for the captured frame.
    pub fn rgba(&self) -> &[u8] {
        &self.pixels
    }

    fn from_robot_capture(capture: RobotFrameCapture) -> Self {
        Self {
            width: capture.width,
            height: capture.height,
            pixels: capture.pixels,
        }
    }
}
