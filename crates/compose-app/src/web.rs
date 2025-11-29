//! Web runtime for Compose applications.
//!
//! Provides a WASM-targeted event loop using `winit` and `wgpu`.

use crate::launcher::AppSettings;
use compose_app_shell::{default_root_key, AppShell};
use compose_platform_desktop_winit::DesktopWinitPlatform;
use compose_render_wgpu::WgpuRenderer;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::platform::web::{EventLoopExtWebSys, WindowExtWebSys};
use winit::window::WindowBuilder;

fn attach_canvas_to_document(window: &winit::window::Window) {
    let canvas = window
        .canvas()
        .expect("winit window should provide a canvas on web");
    let document = web_sys::window()
        .and_then(|w| w.document())
        .expect("expected document");
    let body = document.body().expect("expected body element");

    // Ensure the canvas fills the available viewport by clearing inline sizing first.
    if let Some(style) = canvas.dyn_ref::<web_sys::HtmlElement>() {
        style.style().remove_property("width").ok();
        style.style().remove_property("height").ok();
        style
            .style()
            .set_property("display", "block")
            .expect("failed to update canvas style");
    }

    let _ = body.append_child(&canvas);
}

/// Runs a web Compose application with wgpu rendering.
#[allow(clippy::too_many_lines)]
pub fn run(settings: AppSettings, mut content: impl FnMut() + 'static) -> ! {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("failed to initialize console logging");

    let event_loop = EventLoopBuilder::new().build();

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

    attach_canvas_to_document(&window);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
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
            required_limits:
                wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
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
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 1,
    };

    surface.configure(&device, &surface_config);

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

    let redraw_window = window.clone();
    app.set_frame_waker(move || {
        redraw_window.request_redraw();
    });

    app.set_buffer_size(surface_config.width, surface_config.height);
    let logical_width = surface_config.width as f32 / initial_scale as f32;
    let logical_height = surface_config.height as f32 / initial_scale as f32;
    app.set_viewport(logical_width, logical_height);

    window.request_redraw();

    let spawn_result = event_loop.spawn(move |event_loop| {
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run(move |event, elwt| match event {
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
                            log::warn!("Surface timeout, skipping frame");
                            return;
                        }
                    };

                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    if let Err(err) = app.renderer().render(&view) {
                        log::error!("render failed: {err}");
                    }
                    output.present();
                }
                _ => {}
            },
            Event::AboutToWait => {
                if app.needs_redraw() {
                    window.request_redraw();
                }
            }
            Event::UserEvent(()) => {
                window.request_redraw();
            }
            _ => {}
        });
    });

    match spawn_result {
        Ok(()) => unreachable!("event loop exited unexpectedly"),
        Err(error) => panic!("failed to start web event loop: {error:?}"),
    }
}
