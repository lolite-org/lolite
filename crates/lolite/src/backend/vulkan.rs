use super::{InputState, Params, RenderingBackend};
use anyhow::Result;
use ash::vk::Handle;
use ash::{vk, Entry};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use skia_safe::{
    gpu::{backend_render_targets, direct_contexts, surfaces},
    ColorType, Surface,
};
use std::{ffi::CString, ptr};
use winit::{
    dpi::{LogicalSize, Size},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes},
};

const BUFFER_COUNT: usize = 2;

pub struct VulkanBackend {
    window: Window,

    #[allow(dead_code)]
    entry: Entry,
    instance: ash::Instance,

    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,

    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,

    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    image_layouts: Vec<vk::ImageLayout>,
    swapchain_format: vk::Format,
    extent: vk::Extent2D,

    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    direct_context: skia_safe::gpu::DirectContext,
    surfaces: Vec<Option<(Surface, skia_safe::gpu::BackendRenderTarget)>>,

    input_state: InputState,
}

impl RenderingBackend for VulkanBackend {
    fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        use anyhow::Context;

        let mut window_attributes = WindowAttributes::default();
        window_attributes.inner_size = Some(Size::new(LogicalSize::new(800, 800)));
        window_attributes.title = "Lolite CSS - Vulkan".into();

        let window = event_loop
            .create_window(window_attributes)
            .expect("Failed to create window");

        let entry = unsafe {
            Entry::load().context(
                "Failed to load Vulkan loader (libvulkan). On Linux you typically need the 'vulkan-icd-loader' package, plus a Vulkan ICD (driver).",
            )?
        };

        let display_handle = window.display_handle()?.as_raw();

        // Instance extensions required for X11/Wayland surface creation.
        let required_exts = ash_window::enumerate_required_extensions(display_handle)?;
        let extension_names = required_exts.to_vec();

        let app_name = CString::new("lolite")?;
        let engine_name = CString::new("lolite")?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_1);

        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .context(
                    "vkCreateInstance failed. If this is WSL, you may not have any Vulkan ICD installed. On Arch, installing 'vulkan-swrast' (lavapipe) provides a software ICD and creates /usr/share/vulkan/icd.d/*.json.",
                )?
        };

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                display_handle,
                window.window_handle()?.as_raw(),
                None,
            )?
        };

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let (physical_device, queue_family_index) =
            pick_physical_device_and_queue_family(&instance, &surface_loader, surface)?;

        let device_extensions = [ash::khr::swapchain::NAME.as_ptr()];
        let queue_priorities = [1.0f32];
        let queue_create_info = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities)];

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_info)
            .enabled_extension_names(&device_extensions);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);

        let (swapchain, swapchain_images, swapchain_format, extent) = create_swapchain(
            &instance,
            physical_device,
            &surface_loader,
            surface,
            &swapchain_loader,
            &device,
            queue_family_index,
            window.inner_size().width.max(1),
            window.inner_size().height.max(1),
        )?;

        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&command_pool_info, None)? };

        let command_buffer = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )?[0]
        };

        let direct_context = create_skia_direct_context(
            &entry,
            &instance,
            &device,
            physical_device,
            queue,
            queue_family_index,
        )?;

        let mut backend = Self {
            window,
            entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            queue,
            queue_family_index,
            swapchain_loader,
            swapchain,
            swapchain_images,
            image_layouts: vec![],
            swapchain_format,
            extent,
            command_pool,
            command_buffer,
            direct_context,
            surfaces: vec![],
            input_state: InputState::default(),
        };

        backend.recreate_surfaces()?;
        Ok(backend)
    }

    fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    if let Err(err) = self.resize(new_size.width, new_size.height) {
                        eprintln!("Resize failed: {err:?}");
                    }
                    self.request_redraw();
                }
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, params: &mut Params) {
        if self.swapchain_images.is_empty() {
            return;
        }

        // Acquire image (blocking). This is not the highest-performance approach, but it keeps
        // synchronization simple and correct without needing Skia semaphore integration.
        let (image_index, suboptimal) = unsafe {
            match self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                vk::Semaphore::null(),
                vk::Fence::null(),
            ) {
                Ok(r) => r,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    let _ = self.resize(self.extent.width, self.extent.height);
                    return;
                }
                Err(e) => {
                    eprintln!("acquire_next_image failed: {e:?}");
                    return;
                }
            }
        };

        if suboptimal {
            let _ = self.resize(self.extent.width, self.extent.height);
        }

        // Ensure we have a surface for this image.
        let idx = image_index as usize;
        if idx >= self.surfaces.len() || self.surfaces[idx].is_none() {
            let _ = self.recreate_surfaces();
        }

        let image = match self.swapchain_images.get(idx).copied() {
            Some(img) => img,
            None => return,
        };

        if idx >= self.image_layouts.len() {
            // Shouldn't happen, but keep layout bookkeeping resilient.
            self.image_layouts
                .resize(self.swapchain_images.len(), vk::ImageLayout::UNDEFINED);
        }

        let old_layout = self.image_layouts[idx];

        // Transition PRESENT -> COLOR_ATTACHMENT_OPTIMAL
        if let Err(err) = self.transition_image_layout(
            image,
            old_layout,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ) {
            eprintln!("layout transition failed: {err:?}");
            return;
        }

        self.image_layouts[idx] = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;

        // Draw.
        {
            let Some((surface, _render_target)) =
                self.surfaces.get_mut(idx).and_then(|s| s.as_mut())
            else {
                return;
            };
            (params.on_draw)(surface.canvas());
        }

        // Flush Skia work and wait for completion.
        self.direct_context.flush_and_submit();
        unsafe {
            let _ = self.device.device_wait_idle();
        }

        // Transition COLOR_ATTACHMENT_OPTIMAL -> PRESENT
        let _ = self.transition_image_layout(
            image,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        self.image_layouts[idx] = vk::ImageLayout::PRESENT_SRC_KHR;

        // Present.
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(std::slice::from_ref(&image_index));

        unsafe {
            match self
                .swapchain_loader
                .queue_present(self.queue, &present_info)
            {
                Ok(_suboptimal) => {}
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    let _ = self.resize(self.extent.width, self.extent.height);
                }
                Err(e) => eprintln!("queue_present failed: {e:?}"),
            }
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

impl VulkanBackend {
    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        unsafe {
            let _ = self.device.device_wait_idle();
        }

        let (swapchain, swapchain_images, swapchain_format, extent) = create_swapchain(
            &self.instance,
            self.physical_device,
            &self.surface_loader,
            self.surface,
            &self.swapchain_loader,
            &self.device,
            self.queue_family_index,
            width,
            height,
        )?;

        // Destroy old swapchain after new is created.
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.image_layouts = vec![vk::ImageLayout::UNDEFINED; self.swapchain_images.len()];
        self.swapchain_format = swapchain_format;
        self.extent = extent;

        self.recreate_surfaces()?;
        Ok(())
    }

    fn recreate_surfaces(&mut self) -> Result<()> {
        self.direct_context.flush_and_submit();

        self.surfaces.clear();
        self.surfaces
            .resize_with(self.swapchain_images.len(), || None);
        self.image_layouts
            .resize(self.swapchain_images.len(), vk::ImageLayout::UNDEFINED);

        let images = self.swapchain_images.clone();
        for (i, image) in images.into_iter().enumerate() {
            let (surface, rt) = self.make_surface_for_image(image)?;
            self.surfaces[i] = Some((surface, rt));
        }

        Ok(())
    }

    fn make_surface_for_image(
        &mut self,
        image: vk::Image,
    ) -> Result<(Surface, skia_safe::gpu::BackendRenderTarget)> {
        let (vk_format, color_type) = match self.swapchain_format {
            vk::Format::B8G8R8A8_UNORM => (
                skia_safe::gpu::vk::Format::B8G8R8A8_UNORM,
                ColorType::BGRA8888,
            ),
            vk::Format::R8G8B8A8_UNORM => (
                skia_safe::gpu::vk::Format::R8G8B8A8_UNORM,
                ColorType::RGBA8888,
            ),
            other => {
                anyhow::bail!("Unsupported swapchain format: {other:?}");
            }
        };

        let alloc = skia_safe::gpu::vk::Alloc::default();

        // The swapchain images are optimal-tiling and are used as color attachments.
        let image_info = unsafe {
            skia_safe::gpu::vk::ImageInfo::new(
                image.as_raw() as _,
                alloc,
                skia_safe::gpu::vk::ImageTiling::OPTIMAL,
                skia_safe::gpu::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                vk_format,
                1,
                Some(self.queue_family_index),
                None,
                None,
                None,
            )
        };

        let render_target = backend_render_targets::make_vk(
            (self.extent.width as i32, self.extent.height as i32),
            &image_info,
        );

        let surface = surfaces::wrap_backend_render_target(
            &mut self.direct_context,
            &render_target,
            skia_safe::gpu::SurfaceOrigin::TopLeft,
            color_type,
            None,
            None,
        )
        .ok_or_else(|| anyhow::anyhow!("Failed to wrap Vulkan render target"))?;

        Ok((surface, render_target))
    }

    fn transition_image_layout(
        &self,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<()> {
        let (src_access_mask, dst_access_mask, src_stage, dst_stage) =
            match (old_layout, new_layout) {
                (vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ),
                (vk::ImageLayout::PRESENT_SRC_KHR, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ),
                (vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR) => (
                    vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    vk::AccessFlags::empty(),
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                ),
                _ => (
                    vk::AccessFlags::empty(),
                    vk::AccessFlags::empty(),
                    vk::PipelineStageFlags::ALL_COMMANDS,
                    vk::PipelineStageFlags::ALL_COMMANDS,
                ),
            };

        unsafe {
            self.device
                .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())?;

            self.device.begin_command_buffer(
                self.command_buffer,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;

            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_access_mask(src_access_mask)
                .dst_access_mask(dst_access_mask)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(
                    vk::ImageSubresourceRange::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1),
                );

            self.device.cmd_pipeline_barrier(
                self.command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                std::slice::from_ref(&barrier),
            );

            self.device.end_command_buffer(self.command_buffer)?;

            self.device.queue_submit(
                self.queue,
                std::slice::from_ref(
                    &vk::SubmitInfo::default()
                        .command_buffers(std::slice::from_ref(&self.command_buffer)),
                ),
                vk::Fence::null(),
            )?;

            self.device.queue_wait_idle(self.queue)?;
        }

        Ok(())
    }
}

impl Drop for VulkanBackend {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
        }

        self.direct_context.abandon();

        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);

            self.surface_loader.destroy_surface(self.surface, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

fn pick_physical_device_and_queue_family(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, u32)> {
    let physical_devices = unsafe { instance.enumerate_physical_devices()? };

    for physical_device in physical_devices {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        for (index, qf) in queue_families.iter().enumerate() {
            let supports_graphics = qf.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            if !supports_graphics {
                continue;
            }

            let supports_present = unsafe {
                surface_loader.get_physical_device_surface_support(
                    physical_device,
                    index as u32,
                    surface,
                )?
            };

            if supports_present {
                return Ok((physical_device, index as u32));
            }
        }
    }

    anyhow::bail!("No suitable Vulkan physical device / queue family found")
}

fn create_swapchain(
    _instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    swapchain_loader: &ash::khr::swapchain::Device,
    device: &ash::Device,
    queue_family_index: u32,
    width: u32,
    height: u32,
) -> Result<(vk::SwapchainKHR, Vec<vk::Image>, vk::Format, vk::Extent2D)> {
    let surface_caps = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
    };

    let surface_formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface)? };
    let surface_format = surface_formats
        .first()
        .ok_or_else(|| anyhow::anyhow!("No Vulkan surface formats"))?;

    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
    };

    let present_mode = present_modes
        .iter()
        .copied()
        .find(|m| *m == vk::PresentModeKHR::FIFO)
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let extent = match surface_caps.current_extent.width {
        u32::MAX => vk::Extent2D {
            width: width.clamp(
                surface_caps.min_image_extent.width,
                surface_caps.max_image_extent.width,
            ),
            height: height.clamp(
                surface_caps.min_image_extent.height,
                surface_caps.max_image_extent.height,
            ),
        },
        _ => surface_caps.current_extent,
    };

    let image_count = surface_caps.min_image_count.max(BUFFER_COUNT as u32);

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(std::slice::from_ref(&queue_family_index))
        .pre_transform(surface_caps.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null());

    let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };
    let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

    // Make sure the images are usable as color attachments.
    let format = surface_format.format;

    // Touch device to avoid unused warnings in some cfgs.
    let _ = device;

    Ok((swapchain, images, format, extent))
}

fn create_skia_direct_context(
    entry: &Entry,
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    queue: vk::Queue,
    queue_family_index: u32,
) -> Result<skia_safe::gpu::DirectContext> {
    // Provide Skia with function pointer resolution.
    let get_proc = |gpo: skia_safe::gpu::vk::GetProcOf| unsafe {
        match gpo {
            skia_safe::gpu::vk::GetProcOf::Instance(vk_instance, name) => entry
                .get_instance_proc_addr(ash::vk::Instance::from_raw(vk_instance as _), name)
                .map(|f| f as *const std::ffi::c_void)
                .unwrap_or(ptr::null()),
            skia_safe::gpu::vk::GetProcOf::Device(vk_device, name) => instance
                .get_device_proc_addr(ash::vk::Device::from_raw(vk_device as _), name)
                .map(|f| f as *const std::ffi::c_void)
                .unwrap_or(ptr::null()),
        }
    };

    let backend_context = unsafe {
        skia_safe::gpu::vk::BackendContext::new(
            instance.handle().as_raw() as _,
            physical_device.as_raw() as _,
            device.handle().as_raw() as _,
            (queue.as_raw() as _, queue_family_index as usize),
            &get_proc,
        )
    };

    let direct_context = direct_contexts::make_vulkan(&backend_context, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to create Skia Vulkan DirectContext"))?;

    Ok(direct_context)
}
