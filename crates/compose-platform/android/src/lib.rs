use compose_ui_graphics::Point;

/// Platform abstraction for Android.
///
/// This type manages platform-specific conversions (e.g., density, pointer coordinates)
/// and provides a bridge between Android's event system and Compose's logical coordinate space.
#[derive(Debug, Clone)]
pub struct AndroidPlatform {
    scale_factor: f64,
}

impl Default for AndroidPlatform {
    fn default() -> Self {
        Self { scale_factor: 1.0 }
    }
}

impl AndroidPlatform {
    /// Creates a new Android platform with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the platform's scale factor.
    ///
    /// This should be called when the device density changes.
    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    /// Converts a physical pointer position into logical coordinates.
    ///
    /// Android provides pointer positions in physical pixels; this method
    /// scales them back to logical pixels using the platform's current scale factor.
    pub fn pointer_position(&self, physical_x: f64, physical_y: f64) -> Point {
        let scale = self.scale_factor;
        Point {
            x: (physical_x / scale) as f32,
            y: (physical_y / scale) as f32,
        }
    }

    /// Returns the current scale factor (density).
    pub fn scale_factor(&self) -> f32 {
        self.scale_factor as f32
    }
}

#[cfg(feature = "surface")]
/// Creates a wgpu surface from an Android native window.
///
/// This function encapsulates all the unsafe window handle creation code.
///
/// # Safety
///
/// The native window pointer must be valid for the duration of the surface's lifetime.
pub unsafe fn create_wgpu_surface(
    instance: &wgpu::Instance,
    native_window: &ndk::native_window::NativeWindow,
) -> Result<wgpu::Surface<'static>, wgpu::CreateSurfaceError> {
    use raw_window_handle::{
        AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
    };
    use std::ptr::NonNull;

    let window_handle = AndroidNdkWindowHandle::new(
        NonNull::new(native_window.ptr().as_ptr() as *mut _).expect("Null window pointer"),
    );
    let display_handle = AndroidDisplayHandle::new();

    let raw_window_handle = RawWindowHandle::AndroidNdk(window_handle);
    let raw_display_handle = RawDisplayHandle::Android(display_handle);

    let target = wgpu::SurfaceTargetUnsafe::RawHandle {
        raw_display_handle,
        raw_window_handle,
    };

    instance.create_surface_unsafe(target)
}
