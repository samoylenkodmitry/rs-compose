//! Desktop runtime for Compose applications.
//!
//! This module provides the desktop event loop implementation using winit.

use crate::launcher::AppSettings;
use compose_app_shell::{default_root_key, AppShell};
use compose_platform_desktop_winit::DesktopWinitPlatform;
use compose_render_wgpu::{DrawShape, TextDraw, WgpuRenderer};
use compose_ui_graphics::{Rect, RoundedCornerShape};
use std::sync::{mpsc, Arc};
use std::thread;
use thiserror::Error;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::error::EventLoopError;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use winit::window::WindowBuilder;

/// Runs a desktop Compose application with wgpu rendering.
///
/// Called by `AppLauncher::run_desktop()`. This is the framework-level
/// entrypoint that manages the desktop event loop and rendering.
///
/// **Note:** Applications should use `AppLauncher` instead of calling this directly.
pub fn run(settings: AppSettings, content: impl FnMut() + 'static) -> ! {
    let event_loop = EventLoopBuilder::new()
        .build()
        .expect("failed to create event loop");
    let frame_proxy = event_loop.create_proxy();

    let initial_width = settings.initial_width;
    let initial_height = settings.initial_height;

    let window = Arc::new(
        WindowBuilder::new()
            .with_title(settings.window_title)
            .with_inner_size(LogicalSize::new(
                initial_width as f64,
                initial_height as f64,
            ))
            .build(&event_loop)
            .expect("failed to create window"),
    );

    // Initialize WGPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance
        .create_surface(window.clone())
        .expect("failed to create surface");

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("failed to find suitable adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Main Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        },
        None,
    ))
    .expect("failed to create device");

    let size = window.inner_size();
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(&device, &surface_config);

    // Create renderer with fonts from settings
    let mut renderer = if let Some(fonts) = settings.fonts {
        WgpuRenderer::new_with_fonts(fonts)
    } else {
        WgpuRenderer::new()
    };
    renderer.init_gpu(Arc::new(device), Arc::new(queue), surface_format);
    let initial_scale = window.scale_factor();
    renderer.set_root_scale(initial_scale as f32);

    let mut app = AppShell::new(renderer, default_root_key(), content);
    let mut platform = DesktopWinitPlatform::default();
    platform.set_scale_factor(initial_scale);

    app.set_frame_waker({
        let proxy = frame_proxy.clone();
        move || {
            let _ = proxy.send_event(());
        }
    });

    // Set buffer_size to physical pixels and viewport to logical dp
    app.set_buffer_size(size.width, size.height);
    let logical_width = size.width as f32 / initial_scale as f32;
    let logical_height = size.height as f32 / initial_scale as f32;
    app.set_viewport(logical_width, logical_height);

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);
        match event {
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    elwt.exit();
                }
                WindowEvent::Resized(new_size) => {
                    if new_size.width > 0 && new_size.height > 0 {
                        surface_config.width = new_size.width;
                        surface_config.height = new_size.height;
                        let device = app.renderer().device();
                        surface.configure(device, &surface_config);

                        let scale_factor = window.scale_factor();
                        let logical_width = new_size.width as f32 / scale_factor as f32;
                        let logical_height = new_size.height as f32 / scale_factor as f32;

                        app.set_buffer_size(new_size.width, new_size.height);
                        app.set_viewport(logical_width, logical_height);
                    }
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    platform.set_scale_factor(scale_factor);
                    app.renderer().set_root_scale(scale_factor as f32);

                    let new_size = window.inner_size();
                    if new_size.width > 0 && new_size.height > 0 {
                        surface_config.width = new_size.width;
                        surface_config.height = new_size.height;
                        let device = app.renderer().device();
                        surface.configure(device, &surface_config);

                        let logical_width = new_size.width as f32 / scale_factor as f32;
                        let logical_height = new_size.height as f32 / scale_factor as f32;

                        app.set_buffer_size(new_size.width, new_size.height);
                        app.set_viewport(logical_width, logical_height);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let logical = platform.pointer_position(position);
                    app.set_cursor(logical.x, logical.y);
                }
                WindowEvent::MouseInput {
                    state,
                    button: MouseButton::Left,
                    ..
                } => match state {
                    ElementState::Pressed => {
                        app.pointer_pressed();
                    }
                    ElementState::Released => {
                        app.pointer_released();
                    }
                },
                WindowEvent::KeyboardInput { event, .. } => {
                    use winit::keyboard::{KeyCode, PhysicalKey};
                    if event.state == ElementState::Pressed {
                        if let PhysicalKey::Code(KeyCode::KeyD) = event.physical_key {
                            app.log_debug_info();
                        }
                    }
                }
                WindowEvent::RedrawRequested => {
                    app.update();

                    let output = match surface.get_current_texture() {
                        Ok(output) => output,
                        Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
                            // Reconfigure surface with current window size
                            let size = window.inner_size();
                            if size.width > 0 && size.height > 0 {
                                surface_config.width = size.width;
                                surface_config.height = size.height;
                                let device = app.renderer().device();
                                surface.configure(device, &surface_config);
                            }
                            return;
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("Out of memory, exiting");
                            elwt.exit();
                            return;
                        }
                        Err(wgpu::SurfaceError::Timeout) => {
                            log::debug!("Surface timeout, skipping frame");
                            return;
                        }
                    };

                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    if let Err(err) =
                        app.renderer()
                            .render(&view, surface_config.width, surface_config.height)
                    {
                        log::error!("render failed: {err:?}");
                        return;
                    }

                    output.present();
                }
                _ => {}
            },
            Event::AboutToWait | Event::UserEvent(()) => {
                if app.needs_redraw() {
                    window.request_redraw();
                }
                // Use Poll for animations, Wait for idle
                if app.has_active_animations() {
                    elwt.set_control_flow(ControlFlow::Poll);
                } else {
                    elwt.set_control_flow(ControlFlow::Wait);
                }
            }
            _ => {}
        }
    });

    std::process::exit(0)
}

/// Resulting handle for a robot-controlled desktop application running with the
/// real renderer, event loop, and surface.
pub struct DesktopRobotApp {
    proxy: EventLoopProxy<RobotCommand>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl DesktopRobotApp {
    /// Move the virtual pointer to the provided logical coordinates.
    pub fn move_pointer(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.send_command(|respond_to| RobotCommand::MovePointer { x, y, respond_to })
    }

    /// Press the virtual pointer at the provided logical coordinates.
    pub fn press(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.send_command(|respond_to| RobotCommand::Press { x, y, respond_to })
    }

    /// Release the virtual pointer at the provided logical coordinates.
    pub fn release(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.send_command(|respond_to| RobotCommand::Release { x, y, respond_to })
    }

    /// Click (press + release) at the provided logical coordinates.
    pub fn click(&self, x: f32, y: f32) -> Result<bool, DesktopRobotError> {
        self.send_command(|respond_to| RobotCommand::Click { x, y, respond_to })
    }

    /// Resize the viewport to the provided logical size.
    pub fn set_viewport(&self, width: f32, height: f32) -> Result<(), DesktopRobotError> {
        self.send_command(|respond_to| RobotCommand::SetViewport {
            width,
            height,
            respond_to,
        })
    }

    /// Drive the app until it no longer requests redraws or the iteration limit is reached.
    pub fn pump_until_idle(&self, max_iterations: usize) -> Result<(), DesktopRobotError> {
        self.send_command(|respond_to| RobotCommand::PumpUntilIdle {
            max_iterations,
            respond_to,
        })
    }

    /// Snapshot the current render scene (texts, shapes, and hit regions).
    pub fn snapshot(&self) -> Result<RobotSceneSnapshot, DesktopRobotError> {
        self.send_command(RobotCommand::Snapshot)
    }

    /// Capture the latest rendered frame into RGBA bytes.
    pub fn capture_frame(&self) -> Result<RobotFrameCapture, DesktopRobotError> {
        self.send_command(RobotCommand::CaptureFrame)?
    }

    /// Shut down the application and block until the event loop exits.
    pub fn close(mut self) -> Result<(), DesktopRobotError> {
        let _ = self.send_command(|respond_to| RobotCommand::Close { respond_to });
        if let Some(handle) = self.join_handle.take() {
            handle.join().map_err(|_| DesktopRobotError::Join)?;
        }
        Ok(())
    }

    fn send_command<R: Send + 'static>(
        &self,
        build: impl FnOnce(mpsc::Sender<R>) -> RobotCommand,
    ) -> Result<R, DesktopRobotError> {
        let (tx, rx) = mpsc::channel();
        self.proxy
            .send_event(build(tx))
            .map_err(|_| DesktopRobotError::EventLoopClosed)?;
        rx.recv().map_err(|_| DesktopRobotError::EventLoopClosed)
    }
}

impl Drop for DesktopRobotApp {
    fn drop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            let (tx, _rx) = mpsc::channel();
            let _ = self
                .proxy
                .send_event(RobotCommand::Close { respond_to: tx });
            let _ = handle.join();
        }
    }
}

/// In-memory screenshot of a rendered frame.
#[derive(Clone, Debug)]
pub struct RobotFrameCapture {
    /// Render width in physical pixels.
    pub width: u32,
    /// Render height in physical pixels.
    pub height: u32,
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
}

/// Simplified snapshot of the render scene suitable for assertions.
#[derive(Clone)]
pub struct RobotSceneSnapshot {
    /// Visible texts in the scene.
    pub texts: Vec<TextDraw>,
    /// Drawn shapes in the scene.
    pub shapes: Vec<DrawShape>,
    /// Hit regions without closures.
    pub hits: Vec<RobotHitRegion>,
}

/// Hit region stripped of callbacks for safe cross-thread transfer.
#[derive(Clone)]
pub struct RobotHitRegion {
    /// Rectangle for the hit target.
    pub rect: Rect,
    /// Optional rounded shape for the hit target.
    pub shape: Option<RoundedCornerShape>,
    /// Z-depth used when selecting the deepest hit.
    pub z_index: usize,
    /// Optional clipping rectangle for the hit region.
    pub hit_clip: Option<Rect>,
}

/// Errors surfaced while running a robot-controlled desktop app.
#[derive(Debug, Error)]
pub enum DesktopRobotError {
    /// Creating the winit event loop failed.
    #[error("failed to create event loop: {0}")]
    EventLoop(#[from] EventLoopError),
    /// The offscreen window could not be created.
    #[error("failed to build window")]
    Window,
    /// No compatible GPU adapter is available.
    #[error("no WGPU adapter available for desktop robot")]
    NoAdapter,
    /// Creating the WGPU device failed.
    #[error("failed to create device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    /// The render surface was lost and needs to be recreated.
    #[error("render surface lost")]
    SurfaceLost,
    /// GPU ran out of memory when rendering.
    #[error("render surface out of memory")]
    OutOfMemory,
    /// GPU timed out while producing a frame.
    #[error("render surface timed out")]
    SurfaceTimeout,
    /// Rendering failed within the WGPU renderer.
    #[error("rendering failed: {0}")]
    Render(String),
    /// Mapping the readback buffer failed.
    #[error("readback mapping failed: {0}")]
    Map(wgpu::BufferAsyncError),
    /// The render thread shut down unexpectedly.
    #[error("render thread exited")]
    EventLoopClosed,
    /// The render thread panicked.
    #[error("render thread panicked")]
    Join,
}

enum RobotCommand {
    MovePointer {
        x: f32,
        y: f32,
        respond_to: mpsc::Sender<bool>,
    },
    Press {
        x: f32,
        y: f32,
        respond_to: mpsc::Sender<bool>,
    },
    Release {
        x: f32,
        y: f32,
        respond_to: mpsc::Sender<bool>,
    },
    Click {
        x: f32,
        y: f32,
        respond_to: mpsc::Sender<bool>,
    },
    SetViewport {
        width: f32,
        height: f32,
        respond_to: mpsc::Sender<()>,
    },
    PumpUntilIdle {
        max_iterations: usize,
        respond_to: mpsc::Sender<()>,
    },
    Snapshot(mpsc::Sender<RobotSceneSnapshot>),
    CaptureFrame(mpsc::Sender<Result<RobotFrameCapture, DesktopRobotError>>),
    Close {
        respond_to: mpsc::Sender<()>,
    },
    Wake,
}

/// Launch the real desktop runtime with an embedded robot controller.
pub fn run_with_robot(
    settings: AppSettings,
    content: impl FnMut() + Send + 'static,
) -> Result<DesktopRobotApp, DesktopRobotError> {
    let (ready_tx, ready_rx) = mpsc::channel();

    let join_handle = thread::spawn(move || {
        let result: Result<(), DesktopRobotError> = (|| {
            let event_loop = EventLoopBuilder::<RobotCommand>::with_user_event()
                .build()
                .map_err(DesktopRobotError::EventLoop)?;
            let proxy = event_loop.create_proxy();

            let window = Arc::new(
                WindowBuilder::new()
                    .with_title(settings.window_title)
                    .with_visible(false)
                    .with_inner_size(LogicalSize::new(
                        settings.initial_width as f64,
                        settings.initial_height as f64,
                    ))
                    .build(&event_loop)
                    .map_err(|_| DesktopRobotError::Window)?,
            );

            // Initialize WGPU
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });

            let surface = instance
                .create_surface(window.clone())
                .map_err(|_| DesktopRobotError::Window)?;

            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
                .ok_or(DesktopRobotError::NoAdapter)?;

            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Robot Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            ))?;

            let size = window.inner_size();
            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(surface_caps.formats[0]);

            let mut surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            surface.configure(&device, &surface_config);

            let mut renderer = if let Some(fonts) = settings.fonts {
                WgpuRenderer::new_with_fonts(fonts)
            } else {
                WgpuRenderer::new()
            };
            renderer.init_gpu(Arc::new(device), Arc::new(queue), surface_format);

            let scale_factor = window.scale_factor();
            renderer.set_root_scale(scale_factor as f32);

            let mut app = AppShell::new(renderer, default_root_key(), content);
            let mut platform = DesktopWinitPlatform::default();
            platform.set_scale_factor(scale_factor);

            app.set_frame_waker({
                let proxy = proxy.clone();
                move || {
                    let _ = proxy.send_event(RobotCommand::Wake);
                }
            });

            app.set_buffer_size(size.width, size.height);
            app.set_viewport(
                size.width as f32 / scale_factor as f32,
                size.height as f32 / scale_factor as f32,
            );

            ready_tx
                .send(Ok(proxy.clone()))
                .map_err(|_| DesktopRobotError::EventLoopClosed)?;

            event_loop.run(move |event, elwt| {
                elwt.set_control_flow(ControlFlow::Wait);
                match event {
                    Event::UserEvent(command) => {
                        handle_robot_command(
                            command,
                            &window,
                            &mut app,
                            &surface,
                            &mut surface_config,
                            elwt,
                        );
                    }
                    Event::WindowEvent { window_id, event } if window_id == window.id() => {
                        match event {
                            WindowEvent::CloseRequested => {
                                elwt.exit();
                            }
                            WindowEvent::Resized(new_size) => {
                                resize_surface(
                                    &window,
                                    &mut app,
                                    &surface,
                                    &mut surface_config,
                                    new_size,
                                );
                            }
                            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                                platform.set_scale_factor(scale_factor);
                                app.renderer().set_root_scale(scale_factor as f32);

                                let new_size = window.inner_size();
                                resize_surface(
                                    &window,
                                    &mut app,
                                    &surface,
                                    &mut surface_config,
                                    new_size,
                                );
                            }
                            WindowEvent::CursorMoved { position, .. } => {
                                let logical = platform.pointer_position(position);
                                app.set_cursor(logical.x, logical.y);
                            }
                            WindowEvent::MouseInput {
                                state,
                                button: MouseButton::Left,
                                ..
                            } => match state {
                                ElementState::Pressed => {
                                    app.pointer_pressed();
                                }
                                ElementState::Released => {
                                    app.pointer_released();
                                }
                            },
                            WindowEvent::KeyboardInput { event, .. } => {
                                use winit::keyboard::{KeyCode, PhysicalKey};
                                if event.state == ElementState::Pressed {
                                    if let PhysicalKey::Code(KeyCode::KeyD) = event.physical_key {
                                        app.log_debug_info();
                                    }
                                }
                            }
                            WindowEvent::RedrawRequested => {
                                if let Ok(Some(frame)) = render_to_surface(
                                    &window,
                                    &mut app,
                                    &surface,
                                    &mut surface_config,
                                ) {
                                    frame.present();
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::AboutToWait => {
                        if app.needs_redraw() {
                            window.request_redraw();
                        }

                        if app.has_active_animations() {
                            elwt.set_control_flow(ControlFlow::Poll);
                        }
                    }
                    _ => {}
                }
            })?;

            Ok(())
        })();

        if let Err(err) = result {
            let _ = ready_tx.send(Err(err));
        }
    });

    let proxy = ready_rx
        .recv()
        .map_err(|_| DesktopRobotError::EventLoopClosed)??;

    Ok(DesktopRobotApp {
        proxy,
        join_handle: Some(join_handle),
    })
}

fn handle_robot_command(
    command: RobotCommand,
    window: &winit::window::Window,
    app: &mut AppShell<WgpuRenderer>,
    surface: &wgpu::Surface,
    surface_config: &mut wgpu::SurfaceConfiguration,
    elwt: &winit::event_loop::EventLoopWindowTarget<RobotCommand>,
) {
    match command {
        RobotCommand::MovePointer { x, y, respond_to } => {
            let moved = app.set_cursor(x, y);
            app.update();
            let _ = respond_to.send(moved);
        }
        RobotCommand::Press { x, y, respond_to } => {
            app.set_cursor(x, y);
            let pressed = app.pointer_pressed();
            app.update();
            let _ = respond_to.send(pressed);
        }
        RobotCommand::Release { x, y, respond_to } => {
            app.set_cursor(x, y);
            let released = app.pointer_released();
            app.update();
            let _ = respond_to.send(released);
        }
        RobotCommand::Click { x, y, respond_to } => {
            app.set_cursor(x, y);
            let pressed = app.pointer_pressed();
            let released = app.pointer_released();
            app.update();
            let _ = respond_to.send(pressed || released);
        }
        RobotCommand::SetViewport {
            width,
            height,
            respond_to,
        } => {
            let scale = window.scale_factor() as f32;
            let physical = PhysicalSize::new(
                (width * scale).round() as u32,
                (height * scale).round() as u32,
            );
            let _ = window.request_inner_size(physical);
            resize_surface(window, app, surface, surface_config, physical);
            let _ = respond_to.send(());
        }
        RobotCommand::PumpUntilIdle {
            max_iterations,
            respond_to,
        } => {
            for _ in 0..max_iterations {
                if !app.needs_redraw() {
                    break;
                }
                app.update();
            }
            let _ = respond_to.send(());
        }
        RobotCommand::Snapshot(respond_to) => {
            app.update();
            let snapshot = RobotSceneSnapshot {
                texts: app.scene().texts.clone(),
                shapes: app.scene().shapes.clone(),
                hits: app
                    .scene()
                    .hits
                    .iter()
                    .map(|hit| RobotHitRegion {
                        rect: hit.rect,
                        shape: hit.shape,
                        z_index: hit.z_index,
                        hit_clip: hit.hit_clip,
                    })
                    .collect(),
            };
            let _ = respond_to.send(snapshot);
        }
        RobotCommand::CaptureFrame(respond_to) => {
            let result =
                render_to_surface(window, app, surface, surface_config).and_then(|frame_opt| {
                    match frame_opt {
                        Some(frame) => read_back_frame(app, frame, surface_config),
                        None => Err(DesktopRobotError::SurfaceLost),
                    }
                });
            let _ = respond_to.send(result);
        }
        RobotCommand::Close { respond_to } => {
            elwt.exit();
            let _ = respond_to.send(());
        }
        RobotCommand::Wake => {
            if app.needs_redraw() {
                window.request_redraw();
            }
        }
    }
}

fn render_to_surface(
    window: &winit::window::Window,
    app: &mut AppShell<WgpuRenderer>,
    surface: &wgpu::Surface,
    surface_config: &mut wgpu::SurfaceConfiguration,
) -> Result<Option<wgpu::SurfaceTexture>, DesktopRobotError> {
    app.update();

    let output = match surface.get_current_texture() {
        Ok(output) => output,
        Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
            resize_surface(window, app, surface, surface_config, window.inner_size());
            return Ok(None);
        }
        Err(wgpu::SurfaceError::OutOfMemory) => {
            return Err(DesktopRobotError::OutOfMemory);
        }
        Err(wgpu::SurfaceError::Timeout) => {
            return Err(DesktopRobotError::SurfaceTimeout);
        }
    };

    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    app.renderer()
        .render(&view, surface_config.width, surface_config.height)
        .map_err(|err| DesktopRobotError::Render(format!("{err:?}")))?;

    Ok(Some(output))
}

fn read_back_frame(
    app: &mut AppShell<WgpuRenderer>,
    output: wgpu::SurfaceTexture,
    surface_config: &wgpu::SurfaceConfiguration,
) -> Result<RobotFrameCapture, DesktopRobotError> {
    let texture = &output.texture;
    let renderer = app.renderer();
    let device = renderer.device();
    let queue = renderer.queue();

    let bytes = read_texture_rgba(
        device,
        queue,
        texture,
        surface_config.width,
        surface_config.height,
    )?;

    output.present();

    Ok(RobotFrameCapture {
        width: surface_config.width,
        height: surface_config.height,
        pixels: bytes,
    })
}

fn resize_surface(
    window: &winit::window::Window,
    app: &mut AppShell<WgpuRenderer>,
    surface: &wgpu::Surface,
    surface_config: &mut wgpu::SurfaceConfiguration,
    new_size: PhysicalSize<u32>,
) {
    if new_size.width == 0 || new_size.height == 0 {
        return;
    }

    surface_config.width = new_size.width;
    surface_config.height = new_size.height;
    let device = app.renderer().device();
    surface.configure(device, surface_config);

    let scale_factor = window.scale_factor();
    let logical_width = new_size.width as f32 / scale_factor as f32;
    let logical_height = new_size.height as f32 / scale_factor as f32;

    app.set_buffer_size(new_size.width, new_size.height);
    app.set_viewport(logical_width, logical_height);
}

fn read_texture_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, DesktopRobotError> {
    let bytes_per_pixel = std::mem::size_of::<[u8; 4]>();
    let unpadded_bytes_per_row = width as usize * bytes_per_pixel;
    let padded_bytes_per_row = wgpu::util::align_to(
        unpadded_bytes_per_row,
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize,
    );
    let output_buffer_size = (padded_bytes_per_row * height as usize) as wgpu::BufferAddress;

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("robot-readback"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("robot-copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);

    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => return Err(DesktopRobotError::Map(err)),
        Err(_) => return Err(DesktopRobotError::EventLoopClosed),
    }

    let data = buffer_slice.get_mapped_range();
    let mut pixels = vec![0u8; width as usize * height as usize * bytes_per_pixel];
    for row in 0..height as usize {
        let src_offset = row * padded_bytes_per_row;
        let dst_offset = row * unpadded_bytes_per_row;
        let src = &data[src_offset..src_offset + unpadded_bytes_per_row];
        pixels[dst_offset..dst_offset + unpadded_bytes_per_row].copy_from_slice(src);
    }

    drop(data);
    buffer.unmap();

    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2); // BGRA → RGBA
    }

    Ok(pixels)
}
