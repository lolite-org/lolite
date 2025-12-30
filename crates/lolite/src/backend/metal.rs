use super::{InputState, Params, RenderingBackend};
use anyhow::Result;
use std::cell::RefCell;
use winit::{
    dpi::{LogicalSize, Size},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

#[cfg(target_os = "macos")]
use skia_safe::{
    gpu::{
        mtl::{BackendContext, TextureInfo},
        surfaces, BackendRenderTarget, DirectContext, SurfaceOrigin,
        direct_contexts::make_metal,
        backend_render_targets::make_mtl,
    },
    ColorType, Surface,
};

#[cfg(target_os = "macos")]
use metal::{Device, MetalLayer, foreign_types::{ForeignType, ForeignTypeRef}};

#[cfg(target_os = "macos")]
use objc::runtime::YES;

#[cfg(target_os = "macos")]
use raw_window_handle::HasWindowHandle;

#[cfg(target_os = "macos")]
use core_graphics_types::geometry::CGSize;

#[cfg(target_os = "macos")]
const BUFFER_COUNT: usize = 3;

/// Metal rendering backend implementation for macOS
#[cfg(target_os = "macos")]
pub struct MetalBackend<'a> {
    window: Window,
    device: Device,
    layer: MetalLayer,
    direct_context: DirectContext,
    surfaces: [Option<(Surface, BackendRenderTarget)>; BUFFER_COUNT],
    input_state: InputState,
    params: &'a RefCell<Params>,
    current_width: u32,
    current_height: u32,
}

#[cfg(target_os = "macos")]
impl<'a> RenderingBackend<'a> for MetalBackend<'a> {
    fn new(event_loop: &ActiveEventLoop, params: &'a RefCell<Params>) -> Result<Self> {
        let mut window_attributes = WindowAttributes::default();
        window_attributes.inner_size = Some(Size::new(LogicalSize::new(800, 800)));
        window_attributes.title = "Lolite CSS - Metal".into();

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");

        let (width, height) = window.inner_size().into();

        // Create Metal device
        let device = Device::system_default().ok_or_else(|| {
            anyhow::anyhow!("Failed to create Metal device")
        })?;

        // Create Metal layer
        let layer = MetalLayer::new();
        layer.set_device(&device);
        layer.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
        layer.set_presents_with_transaction(false);
        layer.set_drawable_size(CGSize::new(width as f64, height as f64));

        // Set up the layer with the window
        unsafe {
            use objc::{msg_send, sel, sel_impl};

            if let Ok(window_handle) = window.window_handle() {
                match window_handle.as_raw() {
                    raw_window_handle::RawWindowHandle::AppKit(handle) => {
                        let ns_view = handle.ns_view;
                        
                        // Set layer on the view
                        let _: () = msg_send![ns_view.as_ptr() as *mut objc::runtime::Object, setLayer: layer.as_ref()];
                        let _: () = msg_send![ns_view.as_ptr() as *mut objc::runtime::Object, setWantsLayer: YES];
                    }
                    _ => {}
                }
            }
        }

        // Create Skia Metal BackendContext
        let backend_context = unsafe {
            BackendContext::new(
                device.as_ptr() as *mut std::ffi::c_void,
                device.new_command_queue().as_ptr() as *mut std::ffi::c_void,
            )
        };

        // Create Skia Metal DirectContext
        let direct_context = unsafe {
            make_metal(&backend_context, None)
        }.ok_or_else(|| anyhow::anyhow!("Failed to create Metal DirectContext"))?;

        let mut backend = Self {
            window,
            device,
            layer,
            direct_context,
            surfaces: [None, None, None],
            input_state: InputState::default(),
            params,
            current_width: width,
            current_height: height,
        };

        backend.recreate_surfaces(width, height)?;

        println!("Metal backend initialized with {} surfaces.", BUFFER_COUNT);
        Ok(backend)
    }

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    // Perform safe resize
                    if let Err(err) = self.resize(new_size.width, new_size.height) {
                        eprintln!("Resize failed: {:?}", err);
                    }
                    self.request_redraw();
                }
                true
            }
            _ => false,
        }
    }

    fn render(&mut self) {
        // Get next drawable from layer
        let drawable = match self.layer.next_drawable() {
            Some(drawable) => drawable,
            None => {
                eprintln!("Failed to get next drawable");
                return;
            }
        };

        let texture_info = unsafe {
            TextureInfo::new(drawable.texture().as_ptr() as *mut std::ffi::c_void)
        };

        let backend_render_target = make_mtl(
            (self.current_width as i32, self.current_height as i32),
            &texture_info,
        );

        let surface = surfaces::wrap_backend_render_target(
            &mut self.direct_context,
            &backend_render_target,
            SurfaceOrigin::TopLeft,
            ColorType::BGRA8888,
            None,
            None,
        );

        if let Some(mut surface) = surface {
            let canvas = surface.canvas();
            
            // Call the draw callback
            (self.params.borrow_mut().on_draw)(canvas);
            
            // Flush and present
            self.direct_context.flush_and_submit_surface(&mut surface, None);
            
            // Present the drawable
            drawable.present();
        }
    }

    fn input_state_mut(&mut self) -> &mut InputState {
        &mut self.input_state
    }

    fn input_state(&self) -> &InputState {
        &self.input_state
    }

    fn params(&self) -> &'a RefCell<Params> {
        self.params
    }

    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

#[cfg(target_os = "macos")]
impl<'a> MetalBackend<'a> {
    fn recreate_surfaces(&mut self, width: u32, height: u32) -> Result<()> {
        // Update layer drawable size
        self.layer.set_drawable_size(CGSize::new(width as f64, height as f64));
        
        // Clear existing surfaces
        for surface in &mut self.surfaces {
            *surface = None;
        }
        
        self.current_width = width;
        self.current_height = height;
        Ok(())
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        // Flush any pending work
        self.direct_context.flush_and_submit();
        
        // Recreate surfaces with new dimensions
        self.recreate_surfaces(width, height)?;
        
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl<'a> Drop for MetalBackend<'a> {
    fn drop(&mut self) {
        // Ensure all GPU work is finished
        self.direct_context.flush_and_submit();
        
        // Clear surfaces
        for surface in &mut self.surfaces {
            *surface = None;
        }
    }
}

// Stub implementation for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub struct MetalBackend<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

#[cfg(not(target_os = "macos"))]
impl<'a> RenderingBackend<'a> for MetalBackend<'a> {
    fn new(_event_loop: &ActiveEventLoop, _params: &'a RefCell<Params>) -> Result<Self> {
        anyhow::bail!("Metal backend is not available on this platform");
    }

    fn handle_window_event(&mut self, _event: &WindowEvent) -> bool {
        false
    }

    fn render(&mut self) {}

    fn input_state_mut(&mut self) -> &mut InputState {
        unreachable!("Metal backend is not available on this platform")
    }

    fn input_state(&self) -> &InputState {
        unreachable!("Metal backend is not available on this platform")
    }

    fn params(&self) -> &'a RefCell<Params> {
        unreachable!("Metal backend is not available on this platform")
    }

    fn request_redraw(&self) {
        unreachable!("Metal backend is not available on this platform")
    }
}

