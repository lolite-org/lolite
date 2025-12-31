use super::{InputState, Params, RenderingBackend};
use anyhow::Result;
use skia_safe::{
    gpu::{
        d3d::{BackendContext, TextureResourceInfo},
        surfaces, BackendRenderTarget, DirectContext, Protected, SurfaceOrigin,
    },
    ColorType, Surface,
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{
                D3D12CreateDevice, D3D12GetDebugInterface, ID3D12Debug, ID3D12Device, ID3D12Fence,
                D3D12_FENCE_FLAG_NONE, D3D12_RESOURCE_STATE_PRESENT,
            },
            Dxgi::{
                Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
                CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory4, IDXGISwapChain3,
                DXGI_ADAPTER_FLAG, DXGI_ADAPTER_FLAG_NONE, DXGI_ADAPTER_FLAG_SOFTWARE,
                DXGI_PRESENT, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1,
                DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
            },
        },
        System::Threading::{CreateEventW, WaitForSingleObject},
    },
};
use winit::{
    dpi::{LogicalSize, Size},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};
const BUFFER_COUNT: usize = 2;

/// Direct3D 12 rendering backend implementation
pub struct D3D12Backend {
    window: Window,
    #[allow(unused)]
    factory: IDXGIFactory4,
    // Device/queue container declared BEFORE dependents so it drops LAST
    backend_context: BackendContext,
    // Swap chain declared before DirectContext so it drops AFTER Skia context
    swap_chain: IDXGISwapChain3,
    // Skia context declared before backend_context so it drops BEFORE device/queue
    direct_context: DirectContext,
    // Surfaces declared after above so they drop FIRST
    surfaces: [Option<(Surface, BackendRenderTarget)>; BUFFER_COUNT],
    input_state: InputState,
    current_width: u32,
    current_height: u32,
}

impl RenderingBackend for D3D12Backend {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        // Enable D3D12 debug layer (best effort)
        #[cfg(debug_assertions)]
        unsafe {
            let mut dbg: Option<ID3D12Debug> = None;
            if D3D12GetDebugInterface(&mut dbg).is_ok() {
                if let Some(debug) = dbg {
                    debug.EnableDebugLayer();
                }
            }
        }
        let mut window_attributes = WindowAttributes::default();
        window_attributes.inner_size = Some(Size::new(LogicalSize::new(800, 800)));
        window_attributes.title = "Lolite CSS - Direct3D 12".into();

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");

        let hwnd = HWND(u64::from(window.id()) as *mut _);
        let (width, height) = window.inner_size().into();

        let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1() }?;
        let (adapter, device) = get_hardware_adapter_and_device(&factory)?;
        let queue = unsafe { device.CreateCommandQueue(&Default::default()) }?;

        let backend_context = BackendContext {
            adapter,
            device,
            queue,
            memory_allocator: None,
            protected_context: Protected::No,
        };
        let direct_context = unsafe { DirectContext::new_d3d(&backend_context, None) }.unwrap();

        let swap_chain: IDXGISwapChain3 = unsafe {
            factory.CreateSwapChainForHwnd(
                &backend_context.queue,
                hwnd,
                &DXGI_SWAP_CHAIN_DESC1 {
                    Width: width,
                    Height: height,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                    BufferCount: BUFFER_COUNT as _,
                    SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                    Scaling: DXGI_SCALING_NONE,
                    SampleDesc: DXGI_SAMPLE_DESC {
                        Count: 1,
                        Quality: 0,
                    },
                    ..Default::default()
                },
                None,
                None,
            )
        }?
        .cast()?;

        let mut backend = Self {
            window,
            factory,
            backend_context,
            swap_chain,
            direct_context,
            surfaces: [None, None],
            input_state: InputState::default(),
            current_width: width,
            current_height: height,
        };

        backend.recreate_surfaces(width, height)?;

        println!("D3D12 backend initialized with {} surfaces.", BUFFER_COUNT);
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
        let index = unsafe { self.swap_chain.GetCurrentBackBufferIndex() };
        if self.surfaces[index as usize].is_none() {
            // Attempt to restore valid surfaces to avoid panic
            let _ = self.recreate_surfaces(self.current_width, self.current_height);
        }
        let Some((surface, _)) = self.surfaces[index as usize].as_mut() else {
            // Give up this frame rather than panic
            return;
        };
        let canvas = surface.canvas();

        (params.on_draw)(canvas);
        self.direct_context.flush_and_submit_surface(surface, None);
        // Extra flush to ensure state transitions back to PRESENT/COMMON before Present
        self.direct_context.flush_and_submit();
        unsafe { self.swap_chain.Present(1, DXGI_PRESENT::default()) }.unwrap();
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

impl D3D12Backend {
    fn recreate_surfaces(&mut self, width: u32, height: u32) -> Result<()> {
        for i in 0..BUFFER_COUNT {
            let resource = unsafe { self.swap_chain.GetBuffer(i as u32).unwrap() };
            let backend_render_target = BackendRenderTarget::new_d3d(
                (width as i32, height as i32),
                &TextureResourceInfo {
                    resource,
                    alloc: None,
                    resource_state: D3D12_RESOURCE_STATE_PRESENT,
                    format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    sample_count: 1,
                    level_count: 1,
                    // For single-sample backbuffers, quality should be 0
                    sample_quality_pattern: 0,
                    protected: Protected::No,
                },
            );
            let surface = surfaces::wrap_backend_render_target(
                &mut self.direct_context,
                &backend_render_target,
                SurfaceOrigin::TopLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .unwrap();
            self.surfaces[i] = Some((surface, backend_render_target));
        }
        self.current_width = width;
        self.current_height = height;
        Ok(())
    }

    fn drop_surfaces(&mut self) {
        for i in 0..BUFFER_COUNT {
            if let Some((surface, backend)) = self.surfaces[i].take() {
                drop(surface);
                drop(backend);
            }
        }
        self.direct_context.flush_and_submit();
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        // Ensure GPU is idle and release Skia refs
        self.direct_context.flush_and_submit();
        self.drop_surfaces();
        self.direct_context.flush_and_submit();
        // Fully abandon Skia context to drop any cached refs to old swapchain buffers
        self.direct_context.abandon();

        // Ensure GPU has finished all work on the old backbuffers
        self.wait_for_gpu_idle()?;

        // Resize swap chain buffers
        let resize_result = unsafe {
            self.swap_chain.ResizeBuffers(
                BUFFER_COUNT as u32,
                width,
                height,
                DXGI_FORMAT_R8G8B8A8_UNORM,
                Default::default(),
            )
        };

        // Recreate DirectContext AFTER resize to ensure fresh Skia state without stale refs
        self.direct_context =
            unsafe { DirectContext::new_d3d(&self.backend_context, None) }.unwrap();

        match resize_result {
            Ok(()) => {
                self.recreate_surfaces(width, height)?;
            }
            Err(e) => {
                eprintln!("Resize failed: {:?}", e);
                let _ = self.recreate_surfaces(self.current_width, self.current_height);
            }
        }
        Ok(())
    }

    fn wait_for_gpu_idle(&self) -> Result<()> {
        // Create fence
        let fence: ID3D12Fence = unsafe {
            self.backend_context
                .device
                .CreateFence(0, D3D12_FENCE_FLAG_NONE)?
        };
        let value: u64 = 1;
        // Signal queue
        unsafe {
            self.backend_context.queue.Signal(&fence, value)?;
        }
        // Event and wait
        let event: HANDLE = unsafe { CreateEventW(None, false, false, None)? };
        if event.is_invalid() {
            anyhow::bail!("Failed to create event for GPU sync");
        }
        unsafe {
            fence.SetEventOnCompletion(value, event)?;
        }
        unsafe {
            WaitForSingleObject(event, u32::MAX);
        }
        unsafe {
            let _ = CloseHandle(event);
        }
        Ok(())
    }
}

impl Drop for D3D12Backend {
    fn drop(&mut self) {
        // Ensure Skia finishes and releases all refs before Device/SwapChain are dropped
        self.direct_context.flush_and_submit();
        self.drop_surfaces();
        self.direct_context.flush_and_submit();
        // Wait for GPU idle to ensure swapchain/device have no pending work
        let _ = self.wait_for_gpu_idle();
        // Best-effort: make Skia forget any cached GPU refs
        self.direct_context.abandon();
    }
}

fn get_hardware_adapter_and_device(
    factory: &IDXGIFactory4,
) -> windows::core::Result<(IDXGIAdapter1, ID3D12Device)> {
    for i in 0.. {
        let adapter = unsafe { factory.EnumAdapters1(i) }?;
        let adapter_desc = unsafe { adapter.GetDesc1() }?;
        if (DXGI_ADAPTER_FLAG(adapter_desc.Flags as _) & DXGI_ADAPTER_FLAG_SOFTWARE)
            != DXGI_ADAPTER_FLAG_NONE
        {
            continue;
        }
        let mut device = None;
        if unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }.is_ok() {
            return Ok((adapter, device.unwrap()));
        }
    }
    unreachable!()
}
