use crate::backend::{BackendType, RenderingBackend};
use std::cell::RefCell;

// Re-export types
pub use crate::backend::Params;

/// Run the windowing system with the default backend for the current platform
pub fn run(params: &RefCell<crate::backend::Params>) -> anyhow::Result<()> {
    run_with_backend(params, BackendType::default())
}

/// Run the windowing system with a specific backend
pub fn run_with_backend(
    params: &RefCell<crate::backend::Params>,
    backend_type: BackendType,
) -> anyhow::Result<()> {
    println!(
        "Starting windowing system with {} backend",
        backend_type.name()
    );

    match backend_type {
        #[cfg(all(target_os = "windows"))]
        BackendType::D3D12 => run_with_backend_impl::<crate::backend::d3d12::D3D12Backend>(params),
        #[cfg(target_os = "macos")]
        BackendType::Metal => run_with_backend_impl::<crate::backend::metal::MetalBackend>(params),
    }
}

/// Generic implementation that works with any backend
fn run_with_backend_impl<'a, B: RenderingBackend<'a>>(
    params: &'a RefCell<crate::backend::Params>,
) -> anyhow::Result<()> {
    use winit::{
        application::ApplicationHandler,
        event::{ElementState, MouseButton, WindowEvent},
        event_loop::{ActiveEventLoop, EventLoop},
        keyboard::{Key, NamedKey},
        window::WindowId,
    };

    let event_loop = EventLoop::new()?;

    struct Application<'a, B: RenderingBackend<'a>> {
        backend: Option<B>,
        params: &'a RefCell<crate::backend::Params>,
    }

    impl<'a, B: RenderingBackend<'a>> ApplicationHandler for Application<'a, B> {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            assert!(self.backend.is_none());
            self.backend =
                Some(B::new(event_loop, self.params).expect("Failed to create rendering backend"));
            if let Some(ref backend) = self.backend {
                backend.request_redraw();
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
                        if let Some(ref mut on_click) = backend.params().borrow_mut().on_click {
                            on_click(cursor_position.x, cursor_position.y);
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    backend.input_state_mut().cursor_position = Some(position);
                }
                WindowEvent::RedrawRequested => backend.render(),
                WindowEvent::CloseRequested => event_loop.exit(),
                _ => {}
            }
        }
    }

    let mut application = Application::<B> {
        backend: None,
        params,
    };

    event_loop
        .run_app(&mut application)
        .expect("Failed to run event loop");

    Ok(())
}
