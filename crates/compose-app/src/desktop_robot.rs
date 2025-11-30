//! Desktop runtime with robot control support
//!
//! This module extends the desktop runtime to support programmatic robot control
//! for testing purposes, while reusing all the same rendering and event handling
//! logic as the normal desktop app.

use crate::launcher::AppSettings;
use compose_app_shell::{default_root_key, AppShell};
use compose_platform_desktop_winit::DesktopWinitPlatform;
use compose_render_wgpu::WgpuRenderer;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowBuilder;

/// Robot commands that can be sent to control the app
#[derive(Debug, Clone)]
pub enum RobotCommand {
    /// Click at coordinates (x, y)
    Click {
        /// X coordinate
        x: f32,
        /// Y coordinate
        y: f32,
    },
    /// Move cursor to coordinates (x, y)
    Move {
        /// X coordinate
        x: f32,
        /// Y coordinate
        y: f32,
    },
    /// Press at coordinates (x, y)
    Press {
        /// X coordinate
        x: f32,
        /// Y coordinate
        y: f32,
    },
    /// Release at coordinates (x, y)
    Release {
        /// X coordinate
        x: f32,
        /// Y coordinate
        y: f32,
    },
    /// Drag from one position to another
    Drag {
        /// Starting X coordinate
        from_x: f32,
        /// Starting Y coordinate
        from_y: f32,
        /// Ending X coordinate
        to_x: f32,
        /// Ending Y coordinate
        to_y: f32,
    },
    /// Take a screenshot
    Screenshot {
        /// Path to save screenshot
        path: String,
    },
    /// Shutdown the app
    Shutdown,
}

/// Handle to control a running robot app
pub struct RobotAppHandle {
    command_sender: EventLoopProxy<RobotCommand>,
}

impl RobotAppHandle {
    /// Click at the given coordinates
    pub fn click(&self, x: f32, y: f32) -> Result<(), String> {
        self.send_command(RobotCommand::Click { x, y })
    }

    /// Move cursor to the given coordinates
    pub fn move_to(&self, x: f32, y: f32) -> Result<(), String> {
        self.send_command(RobotCommand::Move { x, y })
    }

    /// Drag from one position to another
    pub fn drag(&self, from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> Result<(), String> {
        self.send_command(RobotCommand::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
        })
    }

    /// Take a screenshot and save to the given path
    pub fn screenshot(&self, path: &str) -> Result<(), String> {
        self.send_command(RobotCommand::Screenshot {
            path: path.to_string(),
        })
    }

    /// Shutdown the app
    pub fn shutdown(&self) -> Result<(), String> {
        self.send_command(RobotCommand::Shutdown)
    }

    fn send_command(&self, command: RobotCommand) -> Result<(), String> {
        self.command_sender
            .send_event(command)
            .map_err(|e| format!("Failed to send command: {:?}", e))
    }
}

/// Run desktop app with robot control enabled
///
/// This launches the app in a background thread and returns a handle
/// that can be used to send robot commands.
///
/// This reuses ALL the same code as the normal desktop app - same event loop,
/// same rendering, same input handling - but adds robot command support on top.
pub fn run_with_robot(
    settings: AppSettings,
    content: impl FnMut() + 'static + Send,
) -> RobotAppHandle {
    let (handle_sender, handle_receiver) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        // Use EventLoop with custom user events for robot commands
        let event_loop = EventLoop::<RobotCommand>::with_user_event()
            .expect("failed to create event loop");

        let proxy = event_loop.create_proxy();

        // Send the proxy back to the caller so they can control the app
        handle_sender.send(proxy).unwrap();

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

        // === EXACT SAME WGPU INITIALIZATION AS DESKTOP APP ===
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
                let _ = proxy.send_event(RobotCommand::Shutdown); // Dummy event for waking
            }
        });

        app.set_buffer_size(size.width, size.height);
        let logical_width = size.width as f32 / initial_scale as f32;
        let logical_height = size.height as f32 / initial_scale as f32;
        app.set_viewport(logical_width, logical_height);

        // === EVENT LOOP - SAME AS DESKTOP APP BUT WITH ROBOT COMMANDS ===
        let _ = event_loop.run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);
            match event {
                // Handle robot commands
                Event::UserEvent(cmd) => match cmd {
                    RobotCommand::Click { x, y } => {
                        use winit::dpi::PhysicalPosition;
                        let logical = platform.pointer_position(PhysicalPosition::new(x as f64, y as f64));
                        app.set_cursor(logical.x, logical.y);
                        app.pointer_pressed();
                        app.pointer_released();
                        window.request_redraw();
                    }
                    RobotCommand::Move { x, y } => {
                        use winit::dpi::PhysicalPosition;
                        let logical = platform.pointer_position(PhysicalPosition::new(x as f64, y as f64));
                        app.set_cursor(logical.x, logical.y);
                        window.request_redraw();
                    }
                    RobotCommand::Press { x, y } => {
                        use winit::dpi::PhysicalPosition;
                        let logical = platform.pointer_position(PhysicalPosition::new(x as f64, y as f64));
                        app.set_cursor(logical.x, logical.y);
                        app.pointer_pressed();
                        window.request_redraw();
                    }
                    RobotCommand::Release { x, y } => {
                        use winit::dpi::PhysicalPosition;
                        let logical = platform.pointer_position(PhysicalPosition::new(x as f64, y as f64));
                        app.set_cursor(logical.x, logical.y);
                        app.pointer_released();
                        window.request_redraw();
                    }
                    RobotCommand::Drag {
                        from_x,
                        from_y,
                        to_x,
                        to_y,
                    } => {
                        use winit::dpi::PhysicalPosition;
                        // Simulate drag as press, move, release
                        let from = platform.pointer_position(PhysicalPosition::new(from_x as f64, from_y as f64));
                        app.set_cursor(from.x, from.y);
                        app.pointer_pressed();

                        let to = platform.pointer_position(PhysicalPosition::new(to_x as f64, to_y as f64));
                        app.set_cursor(to.x, to.y);
                        app.pointer_released();
                        window.request_redraw();
                    }
                    RobotCommand::Screenshot { path } => {
                        // TODO: Implement screenshot capture
                        log::info!("Screenshot requested: {}", path);
                    }
                    RobotCommand::Shutdown => {
                        elwt.exit();
                    }
                },

                // === REST IS IDENTICAL TO DESKTOP APP ===
                Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(new_size) => {
                        let scale_factor = window.scale_factor();
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
                Event::AboutToWait => {
                    if app.should_render() || app.needs_redraw() {
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        });
    });

    let proxy = handle_receiver.recv().expect("Failed to receive robot handle");
    RobotAppHandle {
        command_sender: proxy,
    }
}
