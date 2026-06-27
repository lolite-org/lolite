use crate::backend::{BackendType, InputKey, RenderingBackend};
use std::sync::{Arc, Mutex};
use winit::event_loop::EventLoopProxy;

// Re-export types
pub use crate::backend::Params;

#[derive(Clone, Debug)]
pub enum WindowMessage {
    Redraw,
}

pub struct WindowMessageSender(Arc<Mutex<Option<EventLoopProxy<WindowMessage>>>>);

impl Clone for WindowMessageSender {
    fn clone(&self) -> Self {
        WindowMessageSender(Arc::clone(&self.0))
    }
}

impl WindowMessageSender {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    pub(crate) fn set_proxy(&self, proxy: EventLoopProxy<WindowMessage>) {
        *self.0.lock().unwrap() = Some(proxy);
    }

    pub fn send(&self, message: WindowMessage) {
        if let Some(proxy) = self.0.lock().unwrap().as_ref() {
            let _ = proxy.send_event(message);
        }
    }
}

/// Run the windowing system with the default backend for the current platform
pub fn run(
    params: &mut crate::backend::Params,
    message_sender: WindowMessageSender,
) -> anyhow::Result<()> {
    run_with_backend(params, BackendType::default(), message_sender)
}

/// Run the windowing system with a specific backend
pub fn run_with_backend(
    params: &mut crate::backend::Params,
    backend_type: BackendType,
    message_sender: WindowMessageSender,
) -> anyhow::Result<()> {
    println!(
        "Starting windowing system with {} backend",
        backend_type.name()
    );

    match backend_type {
        #[cfg(all(target_os = "windows"))]
        BackendType::D3D12 => {
            run_with_backend_impl::<crate::backend::d3d12::D3D12Backend>(params, message_sender)
        }
        #[cfg(target_os = "macos")]
        BackendType::Metal => {
            run_with_backend_impl::<crate::backend::metal::MetalBackend>(params, message_sender)
        }
        #[cfg(target_os = "linux")]
        BackendType::OpenGL => {
            run_with_backend_impl::<crate::backend::gl::OpenGlBackend>(params, message_sender)
        }
    }
}

/// Generic implementation that works with any backend
fn run_with_backend_impl<'a, B: RenderingBackend>(
    params: &'a mut crate::backend::Params,
    message_sender: WindowMessageSender,
) -> anyhow::Result<()> {
    use winit::{
        application::ApplicationHandler,
        event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
        event_loop::{ActiveEventLoop, EventLoop},
        keyboard::{Key, NamedKey},
        window::WindowId,
    };

    let mut event_loop_builder = EventLoop::<WindowMessage>::with_user_event();
    let event_loop: EventLoop<WindowMessage> = event_loop_builder.build()?;
    // Publish a proxy so non-UI threads (layout/commands) can request redraws.
    message_sender.set_proxy(event_loop.create_proxy());
    let debugging_enabled = std::env::var("LOLITE_DEBUG")
        .ok()
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"));

    if debugging_enabled {
        println!("Debugging enabled. Press F9 to trigger a debug dump.");
    }

    struct Application<'a, B: RenderingBackend> {
        backend: Option<B>,
        params: &'a mut crate::backend::Params,
        scale_factor: f64,
        debug_dump_enabled: bool,
    }

    impl<'a, B: RenderingBackend> ApplicationHandler<WindowMessage> for Application<'a, B> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            assert!(self.backend.is_none());

            self.backend = Some(B::new(event_loop).expect("Failed to create rendering backend"));

            if let Some(ref backend) = self.backend {
                self.scale_factor = backend.scale_factor();

                let physical_size = backend.window_inner_size();
                let logical_size = physical_size.to_logical::<f64>(self.scale_factor);
                (self.params.on_resize)(logical_size.width, logical_size.height);
                backend.request_redraw();
            }
        }

        fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: WindowMessage) {
            match event {
                WindowMessage::Redraw => {
                    if let Some(ref backend) = self.backend {
                        backend.request_redraw();
                    }
                }
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            let backend = self.backend.as_mut().unwrap();

            // First, let the backend handle any backend-specific events
            let backend_handled = backend.handle_window_event(&event);

            // Keep the layout thread's viewport size in sync with the actual window.
            match &event {
                WindowEvent::Resized(new_size) => {
                    let logical_size = new_size.to_logical::<f64>(self.scale_factor);
                    (self.params.on_resize)(logical_size.width, logical_size.height);
                }
                WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                    self.scale_factor = *scale_factor;
                    let physical_size = backend.window_inner_size();
                    let logical_size = physical_size.to_logical::<f64>(self.scale_factor);
                    (self.params.on_resize)(logical_size.width, logical_size.height);
                }
                _ => {
                    if backend_handled {
                        return;
                    }
                }
            }

            // Handle common events
            match event {
                WindowEvent::KeyboardInput { event, .. } => {
                    if event.state != ElementState::Pressed {
                        return;
                    }

                    if let Some(text) = event.text.as_ref() {
                        let committed: String =
                            text.chars().filter(|ch| !ch.is_control()).collect();
                        if !committed.is_empty() {
                            (self.params.on_text_input)(committed);
                        }
                    }

                    match &event.logical_key {
                        Key::Named(NamedKey::ArrowLeft) => {
                            (self.params.on_key)(InputKey::ArrowLeft);
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            (self.params.on_key)(InputKey::ArrowRight);
                        }
                        Key::Named(NamedKey::Backspace) => {
                            (self.params.on_key)(InputKey::Backspace);
                        }
                        Key::Named(NamedKey::ArrowUp) => backend.input_state_mut().y += 10.0,
                        Key::Named(NamedKey::ArrowDown) => backend.input_state_mut().y -= 10.0,
                        Key::Named(NamedKey::F9) => {
                            if self.debug_dump_enabled {
                                (self.params.on_debug_dump)();
                            }
                        }
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        _ => {}
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => {
                    let input_state = backend.input_state();
                    if let Some(cursor_position) = &input_state.cursor_position {
                        let consumed =
                            (self.params.on_mouse_down)(cursor_position.x, cursor_position.y);
                        if !consumed {
                            (self.params.on_click)(cursor_position.x, cursor_position.y);
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: MouseButton::Left,
                    ..
                } => {
                    let input_state = backend.input_state();
                    if let Some(cursor_position) = &input_state.cursor_position {
                        (self.params.on_mouse_up)(cursor_position.x, cursor_position.y);
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x as f64 * 40.0, y as f64 * 40.0),
                        MouseScrollDelta::PixelDelta(pos) => {
                            let logical = pos.to_logical::<f64>(self.scale_factor);
                            (logical.x, logical.y)
                        }
                    };
                    let input_state = backend.input_state();
                    if let Some(cursor_position) = &input_state.cursor_position {
                        (self.params.on_scroll)(cursor_position.x, cursor_position.y, dx, dy);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let logical_position = position.to_logical::<f64>(self.scale_factor);
                    backend.input_state_mut().cursor_position = Some(logical_position);
                    (self.params.on_mouse_move)(logical_position.x, logical_position.y);
                }
                WindowEvent::RedrawRequested => backend.render(self.params),
                WindowEvent::CloseRequested => event_loop.exit(),
                _ => {}
            }
        }
    }

    // unsafe: We avoid lifetime issues by transmuting the params reference.
    // The params always outlife the Application struct
    let mut application = Application::<'a, B> {
        backend: None,
        params,
        scale_factor: 1.0,
        debug_dump_enabled: debugging_enabled,
    };

    event_loop.run_app(&mut application)?;

    Ok(())
}
