use super::{InputState, Params, RenderingBackend};
use anyhow::Result;
use winit::{
    dpi::{LogicalSize, Size},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

use core_graphics_types::geometry::CGSize;
use metal::{
    foreign_types::{ForeignType, ForeignTypeRef},
    Device, MetalLayer,
};
use objc::runtime::YES;
use raw_window_handle::HasWindowHandle;
use skia_safe::{
    gpu::{
        backend_render_targets::make_mtl,
        direct_contexts::make_metal,
        mtl::{BackendContext, TextureInfo},
        surfaces, BackendRenderTarget, DirectContext, SurfaceOrigin,
    },
    ColorType, Surface,
};

const BUFFER_COUNT: usize = 3;

/// Metal rendering backend implementation for macOS
pub struct MetalBackend {
    window: Window,
    device: Device,
    layer: MetalLayer,
    direct_context: DirectContext,
    surfaces: [Option<(Surface, BackendRenderTarget)>; BUFFER_COUNT],
    input_state: InputState,
    current_width: u32,
    current_height: u32,
}

impl RenderingBackend for MetalBackend {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        let mut window_attributes = WindowAttributes::default();
        window_attributes.inner_size = Some(Size::new(LogicalSize::new(800, 800)));
        window_attributes.title = "Lolite CSS - Metal".into();

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");

        let (width, height) = window.inner_size().into();

        // Create Metal device
        let device = Device::system_default()
            .ok_or_else(|| anyhow::anyhow!("Failed to create Metal device"))?;

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
        let direct_context = unsafe { make_metal(&backend_context, None) }
            .ok_or_else(|| anyhow::anyhow!("Failed to create Metal DirectContext"))?;

        let mut backend = Self {
            window,
            device,
            layer,
            direct_context,
            surfaces: [None, None, None],
            input_state: InputState::default(),
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

    fn render(&mut self, params: &mut Params) {
        // Get next drawable from layer
        let drawable = match self.layer.next_drawable() {
            Some(drawable) => drawable,
            None => {
                eprintln!("Failed to get next drawable");
                return;
            }
        };

        let texture_info =
            unsafe { TextureInfo::new(drawable.texture().as_ptr() as *mut std::ffi::c_void) };

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
            (params.on_draw)(canvas);

            // Flush and present
            self.direct_context
                .flush_and_submit_surface(&mut surface, None);

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

    fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

impl MetalBackend {
    fn recreate_surfaces(&mut self, width: u32, height: u32) -> Result<()> {
        // Update layer drawable size
        self.layer
            .set_drawable_size(CGSize::new(width as f64, height as f64));

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

impl Drop for MetalBackend {
    fn drop(&mut self) {
        // Ensure all GPU work is finished
        self.direct_context.flush_and_submit();

        // Clear surfaces
        for surface in &mut self.surfaces {
            *surface = None;
        }
    }
}
