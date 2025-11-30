use compose_app_shell::{default_root_key, AppShell};
use compose_core::Key;
use compose_render_pixels::scene::{DrawShape, HitRegion, TextDraw};
use compose_render_pixels::PixelsRenderer;
use compose_ui_graphics::Rect;

/// Headless harness that wraps an [`AppShell`] with the pixels renderer to enable
/// black-box robot style tests against real applications.
///
/// The robot exposes pointer interactions (move, press, release, click), frame
/// stepping, and snapshotting utilities that let tests assert on the rendered
/// scene (visible texts, rectangles, and hit regions).
pub struct RobotApp {
    shell: AppShell<PixelsRenderer>,
}

impl RobotApp {
    /// Launch a new robot-controlled application using the default root key.
    pub fn launch(content: impl FnMut() + 'static) -> Self {
        Self::launch_with_key(default_root_key(), content)
    }

    /// Launch a new robot-controlled application with an explicit root key.
    pub fn launch_with_key(root_key: Key, content: impl FnMut() + 'static) -> Self {
        let renderer = PixelsRenderer::new();
        let shell = AppShell::new(renderer, root_key, content);
        Self { shell }
    }

    /// Update the viewport used for layout.
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.shell.set_viewport(width, height);
    }

    /// Run a single frame update, processing recompositions, layout, and scene rebuilds.
    pub fn step(&mut self) {
        self.shell.update();
    }

    /// Drive the application until no further redraws are requested or the iteration
    /// limit is reached.
    pub fn pump_until_idle(&mut self, max_iterations: usize) {
        for _ in 0..max_iterations {
            if !self.shell.needs_redraw() {
                break;
            }
            self.shell.update();
        }
    }

    /// Move the virtual pointer to the provided coordinates, dispatching pointer move
    /// events to any hit targets.
    pub fn move_pointer(&mut self, x: f32, y: f32) -> bool {
        let moved = self.shell.set_cursor(x, y);
        self.shell.update();
        moved
    }

    /// Press the virtual pointer at the provided coordinates.
    pub fn press(&mut self, x: f32, y: f32) -> bool {
        self.shell.set_cursor(x, y);
        let pressed = self.shell.pointer_pressed();
        self.shell.update();
        pressed
    }

    /// Release the virtual pointer at the provided coordinates.
    pub fn release(&mut self, x: f32, y: f32) -> bool {
        self.shell.set_cursor(x, y);
        let released = self.shell.pointer_released();
        self.shell.update();
        released
    }

    /// Convenience helper that presses and then releases the pointer at the provided
    /// coordinates.
    pub fn click(&mut self, x: f32, y: f32) -> bool {
        self.shell.set_cursor(x, y);
        let pressed = self.shell.pointer_pressed();
        let released = self.shell.pointer_released();
        self.shell.update();
        pressed || released
    }

    /// Capture a snapshot of the current render scene for assertions.
    pub fn snapshot(&mut self) -> SceneSnapshot {
        SceneSnapshot::from_scene(self.shell.scene())
    }

    /// Shut down the robot-controlled application. Dropping the instance will
    /// clean up the underlying shell; this is provided for clarity in tests.
    pub fn close(self) {}
}

/// Immutable snapshot of a rendered scene containing draw operations and hit regions.
#[derive(Clone)]
pub struct SceneSnapshot {
    texts: Vec<TextDraw>,
    shapes: Vec<DrawShape>,
    hits: Vec<HitRegion>,
}

impl SceneSnapshot {
    pub(crate) fn from_scene(scene: &compose_render_pixels::Scene) -> Self {
        Self {
            texts: scene.texts.clone(),
            shapes: scene.shapes.clone(),
            hits: scene.hits.clone(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn from_wgpu_scene(scene: &compose_render_wgpu::Scene) -> Self {
        Self {
            texts: scene
                .texts
                .iter()
                .map(|text| TextDraw {
                    rect: text.rect,
                    text: text.text.clone(),
                    color: text.color,
                    scale: text.scale,
                    z_index: text.z_index,
                    clip: text.clip,
                })
                .collect(),
            shapes: scene
                .shapes
                .iter()
                .map(|shape| DrawShape {
                    rect: shape.rect,
                    brush: shape.brush.clone(),
                    shape: shape.shape,
                    z_index: shape.z_index,
                    clip: shape.clip,
                })
                .collect(),
            hits: scene
                .hits
                .iter()
                .map(|hit| HitRegion {
                    rect: hit.rect,
                    shape: hit.shape,
                    click_actions: Vec::new(),
                    pointer_inputs: Vec::new(),
                    z_index: hit.z_index,
                    hit_clip: hit.hit_clip,
                })
                .collect(),
        }
    }

    pub(crate) fn from_robot_snapshot(snapshot: &compose_app::desktop::RobotSceneSnapshot) -> Self {
        Self {
            texts: snapshot
                .texts
                .iter()
                .map(|text| TextDraw {
                    rect: text.rect,
                    text: text.text.clone(),
                    color: text.color,
                    scale: text.scale,
                    z_index: text.z_index,
                    clip: text.clip,
                })
                .collect(),
            shapes: snapshot
                .shapes
                .iter()
                .map(|shape| DrawShape {
                    rect: shape.rect,
                    brush: shape.brush.clone(),
                    shape: shape.shape,
                    z_index: shape.z_index,
                    clip: shape.clip,
                })
                .collect(),
            hits: snapshot
                .hits
                .iter()
                .map(|hit| HitRegion {
                    rect: hit.rect,
                    shape: hit.shape,
                    click_actions: Vec::new(),
                    pointer_inputs: Vec::new(),
                    z_index: hit.z_index,
                    hit_clip: hit.hit_clip,
                })
                .collect(),
        }
    }

    /// Visible texts recorded in the scene.
    pub fn texts(&self) -> &[TextDraw] {
        &self.texts
    }

    /// Rectangles produced by draw primitives in the scene.
    pub fn shapes(&self) -> &[DrawShape] {
        &self.shapes
    }

    /// Hit regions that accept pointer interaction.
    pub fn hits(&self) -> &[HitRegion] {
        &self.hits
    }

    /// Iterator over the string contents of all visible texts.
    pub fn text_values(&self) -> impl Iterator<Item = &str> {
        self.texts.iter().map(|text| text.text.as_str())
    }

    /// Rectangles for texts that exactly match the provided value.
    pub fn text_rects(&self, value: &str) -> Vec<Rect> {
        self.texts
            .iter()
            .filter(move |text| text.text == value)
            .map(|text| Rect {
                x: text.rect.x,
                y: text.rect.y,
                width: text.rect.width,
                height: text.rect.height,
            })
            .collect()
    }

    /// Returns the deepest hit region that contains the provided point, if any.
    pub fn hit_at(&self, x: f32, y: f32) -> Option<HitRegion> {
        self.hits
            .iter()
            .filter(|hit| hit.contains(x, y))
            .max_by(|a, b| a.z_index.cmp(&b.z_index))
            .cloned()
    }
}

impl std::fmt::Debug for SceneSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneSnapshot")
            .field("texts", &self.texts.len())
            .field("shapes", &self.shapes.len())
            .field("hits", &self.hits.len())
            .finish()
    }
}

/// Compute the center point for a rectangle.
pub fn rect_center(rect: &Rect) -> (f32, f32) {
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

#[cfg(test)]
#[path = "tests/robot_tests.rs"]
mod tests;
