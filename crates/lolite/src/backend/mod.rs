use anyhow::Result;
use skia_safe::Canvas;
use winit::{event::WindowEvent, event_loop::ActiveEventLoop};

#[cfg(target_os = "windows")]
pub mod d3d12;
#[cfg(target_os = "linux")]
pub mod gl;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(all(target_os = "linux", feature = "vulkan"))]
pub mod vulkan;

/// Common parameters shared across all rendering backends
pub struct Params {
    pub on_draw: Box<dyn FnMut(&Canvas)>,
    pub on_click: Box<dyn FnMut(f64, f64)>, // x, y coordinates
}

/// State shared across all backends for input handling
pub struct InputState {
    pub x: f32,
    pub y: f32,
    pub cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            x: 100.0,
            y: 100.0,
            cursor_position: None,
        }
    }
}

/// Trait that all rendering backends must implement
pub trait RenderingBackend {
    /// Create a new backend instance
    fn new(event_loop: &ActiveEventLoop) -> Result<Self>
    where
        Self: Sized;

    /// Handle window events specific to this backend
    fn handle_window_event(&mut self, event: &WindowEvent) -> bool;

    /// Render a frame
    fn render(&mut self, params: &mut Params);

    /// Get mutable reference to input state
    fn input_state_mut(&mut self) -> &mut InputState;

    /// Get reference to input state
    fn input_state(&self) -> &InputState;

    /// Request a redraw
    fn request_redraw(&self);
}

/// Available backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    #[cfg(all(target_os = "windows"))]
    D3D12,
    #[cfg(target_os = "macos")]
    Metal,
    #[cfg(target_os = "linux")]
    OpenGL,
    #[cfg(all(target_os = "linux", feature = "vulkan"))]
    Vulkan,
}

impl BackendType {
    /// Get the default backend for the current platform
    pub fn default() -> Self {
        #[cfg(all(target_os = "windows"))]
        return BackendType::D3D12;

        #[cfg(target_os = "macos")]
        return BackendType::Metal;

        #[cfg(target_os = "linux")]
        return BackendType::OpenGL;

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        compile_error!("No default backend available for this platform");
    }

    /// Get a human-readable name for the backend
    pub fn name(&self) -> &'static str {
        match self {
            #[cfg(all(target_os = "windows"))]
            BackendType::D3D12 => "Direct3D 12",
            #[cfg(target_os = "macos")]
            BackendType::Metal => "Metal",
            #[cfg(target_os = "linux")]
            BackendType::OpenGL => "OpenGL",
            #[cfg(all(target_os = "linux", feature = "vulkan"))]
            BackendType::Vulkan => "Vulkan",
        }
    }
}
