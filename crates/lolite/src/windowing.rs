use crate::backend::{BackendType, RenderingBackend};
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

        #[cfg(all(target_os = "linux", feature = "vulkan"))]
        BackendType::Vulkan => {
            run_with_backend_impl::<crate::backend::vulkan::VulkanBackend>(params, message_sender)
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
        event::{ElementState, MouseButton, WindowEvent},
        event_loop::{ActiveEventLoop, EventLoop},
        keyboard::{Key, NamedKey},
        window::WindowId,
    };

    let mut event_loop_builder = EventLoop::<WindowMessage>::with_user_event();
    let event_loop: EventLoop<WindowMessage> = event_loop_builder.build()?;
    // Publish a proxy so non-UI threads (layout/commands) can request redraws.
    message_sender.set_proxy(event_loop.create_proxy());

    struct Application<'a, B: RenderingBackend> {
        backend: Option<B>,
        params: &'a mut crate::backend::Params,
        startup_error: Option<anyhow::Error>,
    }

    impl<'a, B: RenderingBackend> ApplicationHandler<WindowMessage> for Application<'a, B> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            assert!(self.backend.is_none());

            match B::new(event_loop) {
                Ok(backend) => {
                    self.backend = Some(backend);
                }
                Err(err) => {
                    eprintln!("Failed to create rendering backend: {err:?}");
                    self.startup_error = Some(err);
                    event_loop.exit();
                    return;
                }
            }

            if let Some(ref backend) = self.backend {
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
            if backend.handle_window_event(&event) {
                return; // Event was handled by the backend
            }

            // Handle common events
            match event {
                WindowEvent::KeyboardInput { event, .. } => {
                    let input_state = backend.input_state_mut();
                    match event.logical_key {
                        Key::Named(NamedKey::ArrowLeft) => input_state.x -= 10.0,
                        Key::Named(NamedKey::ArrowRight) => input_state.x += 10.0,
                        Key::Named(NamedKey::ArrowUp) => input_state.y += 10.0,
                        Key::Named(NamedKey::ArrowDown) => input_state.y -= 10.0,
                        Key::Named(NamedKey::Escape) => event_loop.exit(),
                        _ => return,
                    }
                    backend.request_redraw();
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => {
                    let input_state = backend.input_state();
                    if let Some(cursor_position) = &input_state.cursor_position {
                        (self.params.on_click)(cursor_position.x, cursor_position.y);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    backend.input_state_mut().cursor_position = Some(position);
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
        startup_error: None,
    };

    event_loop.run_app(&mut application)?;

    if let Some(err) = application.startup_error {
        return Err(err);
    }

    Ok(())
}
