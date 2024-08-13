use std::ffi::CStr;
use std::mem::transmute;
use std::os::raw::c_char;
use std::sync::Mutex;

use ash::vk::Handle;
use lazy_static::lazy_static;

use crate::REGISTRY;

#[derive(Default)]
struct DummyState {
  pub instance:        ash::vk::Instance,
  //pub physical_device: ash::vk::PhysicalDevice,
  pub device:          ash::vk::Device,
  pub surface:         ash::vk::SurfaceKHR,
  pub swapchain:       ash::vk::SwapchainKHR
}

lazy_static! {
  static ref STATE: Mutex<DummyState> = Mutex::new(DummyState::default());
}

//TODO: vkEnumeratePhysicalDevices

unsafe extern "C" fn wgpu_create_instance(
  _create_info: *const ash::vk::InstanceCreateInfo,
  _allocator:   *const ash::vk::AllocationCallbacks,
  instance:     *mut   ash::vk::Instance
)
  -> ash::vk::Result
{
  *instance = STATE.lock().unwrap().instance;
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_destroy_instance() {}

unsafe extern "C" fn wgpu_create_device(
  _physical_device: ash::vk::PhysicalDevice,
  _create_info:     *const ash::vk::DeviceCreateInfo,
  _allocator:       *const ash::vk::AllocationCallbacks,
  device:           *mut ash::vk::Device
)
  -> ash::vk::Result
{
  *device = STATE.lock().unwrap().device;
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_destroy_device() {}

unsafe extern "C" fn wgpu_enumerate_instance_extension_properties(
  _layer_name:     *const c_char,
  property_count:  *mut u32,
  _properties:     *mut ash::vk::ExtensionProperties
)
  -> ash::vk::Result
{
  *property_count = 0;
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_enumerate_instance_layer_properties(
  property_count:  *mut u32,
  _properties:     *mut ash::vk::LayerProperties
)
  -> ash::vk::Result
{
  *property_count = 0;
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_enumerate_instance_version(api_version: *mut u32) -> ash::vk::Result {
  *api_version = ash::vk::API_VERSION_1_0; // ?
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_create_xcb_surface_khr(
  _instance:    ash::vk::Instance,
  _create_info: *const ash::vk::XcbSurfaceCreateInfoKHR,
  _allocator:   *const ash::vk::AllocationCallbacks,
  surface:      *mut ash::vk::SurfaceKHR
)
  -> ash::vk::Result
{
  *surface = STATE.lock().unwrap().surface;
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_destroy_surface_khr() {}

unsafe extern "C" fn wgpu_create_swapchain_khr(
  _device:      ash::vk::Device,
  _create_info: *const ash::vk::SwapchainCreateInfoKHR,
  _allocator:   *const ash::vk::AllocationCallbacks,
  swapchain:    *mut ash::vk::SwapchainKHR
)
  -> ash::vk::Result
{
  *swapchain = STATE.lock().unwrap().swapchain;
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_destroy_swapchain_khr() {}

use std::cell::Cell;

thread_local! {
  static CURRENT_SWAPCHAIN_IMAGE_INDEX: Cell<u32> = Cell::new(0);
  static NOPE_FENCE: Cell<ash::vk::Fence> = Cell::new(ash::vk::Fence::null());
}

unsafe extern "C" fn wgpu_acquire_next_image_khr(
  _device:     ash::vk::Device,
  _swapchain:  ash::vk::SwapchainKHR,
  _timeout:    u64,
  semaphore:   ash::vk::Semaphore,
  fence:       ash::vk::Fence,
  image_index: *mut u32
)
  -> ash::vk::Result
{
  //println!("wgpu_acquire_next_image_khr({:?}, {:?}, {:?}, {:?}, {:?}, {:?})", _device, _swapchain, _timeout, semaphore, fence, image_index);
  assert_eq!(semaphore, ash::vk::Semaphore::null());
  assert_ne!(fence,     ash::vk::Fence::null());
  NOPE_FENCE.with(|value| value.set(fence));
  *image_index = CURRENT_SWAPCHAIN_IMAGE_INDEX.with(|value| value.get());
  ash::vk::Result::SUCCESS
}

unsafe extern "C" fn wgpu_wait_for_fences(
  device:      ash::vk::Device,
  fence_count: u32,
  fences:      *const ash::vk::Fence,
  wait_all:    ash::vk::Bool32,
  timeout:     u64
) -> ash::vk::Result
{
  //println!("wgpu_wait_for_fences({:?}, {:?}, {:?}, {:?}, {:?})", device, fence_count, fences, wait_all, timeout);
  //println!("fence: {:?}", *fences.offset(0));

  if NOPE_FENCE.with(|value| value.get() == *fences.offset(0)) {
    //println!("wgpu_wait_for_fences: nope!");
    return ash::vk::Result::SUCCESS;
  }

  let registry = REGISTRY.lock().unwrap();
  let wait_for_fences = registry.device(device).fp_v1_0().wait_for_fences;
  let err = wait_for_fences(device, fence_count, fences, wait_all, timeout);
  //println!("wgpu_wait_for_fences: exit");
  err
}

unsafe extern "C" fn wgpu_create_render_pass(
  device:      ash::vk::Device,
  create_info: *const ash::vk::RenderPassCreateInfo,
  allocator:   *const ash::vk::AllocationCallbacks,
  render_pass: *mut ash::vk::RenderPass
)
  -> ash::vk::Result
{
  /*for i in 0..(*create_info).attachment_count {
    println!("wgpu_create_render_pass: {:?}", (*(*create_info).p_attachments.offset(i as isize)));
  }*/

  let registry = REGISTRY.lock().unwrap();
  let create_render_pass = registry.device(device).fp_v1_0().create_render_pass;

  // "(wgpu internal) clear_texture clear pass"
  if (*create_info).attachment_count == 1 && (*(*create_info).p_attachments.offset(0)).load_op == ash::vk::AttachmentLoadOp::CLEAR {

    let mut create_info = *create_info;
    let mut attachments = *create_info.p_attachments;
    create_info.p_attachments = &attachments;
    attachments.load_op = ash::vk::AttachmentLoadOp::LOAD;

    create_render_pass(device, &create_info, allocator, render_pass)
  } else {
    create_render_pass(device, create_info, allocator, render_pass)
  }
}

// https://github.com/KhronosGroup/Vulkan-ValidationLayers/issues/1345
unsafe extern "C" fn wgpu_allocate_command_buffers(
  device:          ash::vk::Device,
  allocate_info:   *const ash::vk::CommandBufferAllocateInfo,
  command_buffers: *mut ash::vk::CommandBuffer
)
  -> ash::vk::Result
{
  let registry = REGISTRY.lock().unwrap();
  let allocate_command_buffers = registry.device(device).fp_v1_0().allocate_command_buffers;

  let err = allocate_command_buffers(device, allocate_info, command_buffers);
  if err == ash::vk::Result::SUCCESS {
    for i in 0..(*allocate_info).command_buffer_count {
      let command_buffer = *command_buffers.offset(i as isize);
      *(command_buffer.as_raw() as *mut usize) = *(device.as_raw() as *mut usize);
    }
  }
  err
}

// ?
unsafe extern "C" fn wgpu_queue_present_khr(_queue: ash::vk::Queue, _present_info: *const ash::vk::PresentInfoKHR) -> ash::vk::Result {
  //println!("wgpu_queue_present_khr({:p}, {:p})", _queue, _present_info);
  ash::vk::Result::SUCCESS
}

unsafe fn get_wgpu_overlay_function(name: *const c_char) -> Option<*const()> {
  match CStr::from_ptr(name).to_str().unwrap() {
    "vkGetDeviceProcAddr"                      => Some(wgpu_get_device_proc_addr                    as *const()),
    "vkGetInstanceProcAddr"                    => Some(wgpu_get_instance_proc_addr                  as *const()),
    "vkAcquireNextImageKHR"                    => Some(wgpu_acquire_next_image_khr                  as *const()),
    "vkAllocateCommandBuffers"                 => Some(wgpu_allocate_command_buffers                as *const()),
    "vkCreateDevice"                           => Some(wgpu_create_device                           as *const()),
    "vkCreateInstance"                         => Some(wgpu_create_instance                         as *const()),
    "vkCreateRenderPass"                       => Some(wgpu_create_render_pass                      as *const()),
    "vkCreateSwapchainKHR"                     => Some(wgpu_create_swapchain_khr                    as *const()),
    "vkCreateXcbSurfaceKHR"                    => Some(wgpu_create_xcb_surface_khr                  as *const()),
    "vkDestroyDevice"                          => Some(wgpu_destroy_device                          as *const()),
    "vkDestroyInstance"                        => Some(wgpu_destroy_instance                        as *const()),
    "vkDestroySurfaceKHR"                      => Some(wgpu_destroy_surface_khr                     as *const()),
    "vkDestroySwapchainKHR"                    => Some(wgpu_destroy_swapchain_khr                   as *const()),
    "vkEnumerateInstanceExtensionProperties"   => Some(wgpu_enumerate_instance_extension_properties as *const()),
    "vkEnumerateInstanceLayerProperties"       => Some(wgpu_enumerate_instance_layer_properties     as *const()),
    "vkEnumerateInstanceVersion"               => Some(wgpu_enumerate_instance_version              as *const()),
    "vkQueuePresentKHR"                        => Some(wgpu_queue_present_khr                       as *const()),
    "vkWaitForFences"                          => Some(wgpu_wait_for_fences                         as *const()),
    _ => None
  }
}

unsafe extern "C" fn wgpu_get_device_proc_addr(device: ash::vk::Device, name: *const c_char) -> *const() {
  if let Some(fun) = get_wgpu_overlay_function(name) {
    fun
  } else {
    //println!("wgpu_get_device_proc_addr: {:?} {}", device, CStr::from_ptr(name).to_str().unwrap());
    let registry = REGISTRY.lock().unwrap();
    transmute(registry.get_device_proc_addr(device, name))
  }
}

unsafe extern "C" fn wgpu_get_instance_proc_addr(instance: ash::vk::Instance, name: *const c_char) -> *const() {
  if let Some(fun) = get_wgpu_overlay_function(name) {
    fun
  } else {
    //println!("wgpu_get_instance_proc_addr: {:?} {}", instance, CStr::from_ptr(name).to_str().unwrap());
    let registry = REGISTRY.lock().unwrap();
    transmute(registry.get_instance_proc_addr(instance, name))
  }
}

pub unsafe fn create_wgpu_instance(instance: ash::vk::Instance, device: ash::vk::Device, swapchain: ash::vk::SwapchainKHR) -> wgpu::Instance {

  #[allow(mutable_transmutes)]
  unsafe fn replace_get_device_proc_addr(instance: &ash::Instance, get_device_proc_addr: ash::vk::PFN_vkGetDeviceProcAddr) {

    struct XInstance {
      pub _handle:          ash::vk::Instance,
      pub instance_fn_1_0:  ash::vk::InstanceFnV1_0,
      pub _instance_fn_1_1: ash::vk::InstanceFnV1_1,
      pub _instance_fn_1_2: ash::vk::InstanceFnV1_2,
    }

    let instance: &mut XInstance = transmute(instance);
    instance.instance_fn_1_0.get_device_proc_addr = get_device_proc_addr;
  }

  let get_instance_proc_addr = transmute(wgpu_get_instance_proc_addr as *const ());
  let get_device_proc_addr   = transmute(wgpu_get_device_proc_addr   as *const ());

  let ash_entry    = ash::Entry::from_static_fn(ash::vk::StaticFn { get_instance_proc_addr });
  let ash_instance = ash::Instance::load(&ash::vk::StaticFn { get_instance_proc_addr }, instance);
  replace_get_device_proc_addr(&ash_instance, get_device_proc_addr);

  {
    let mut dstate = STATE.lock().unwrap();
    dstate.instance  = instance;
    dstate.device    = device;
    dstate.swapchain = swapchain;
  }

  let wgpu_hal_instance = <wgpu_hal::api::Vulkan as wgpu_hal::Api>::Instance::from_raw(
    ash_entry,
    ash_instance,
    ash::vk::API_VERSION_1_0 /* create_info.p_application_info.api_version */,
    0, // android_sdk_version
    None, // debug_utils_create_info
    vec![ash::extensions::khr::XcbSurface::name()],
    wgpu::InstanceFlags::empty(),
    false, // has_nv_optimus
    None // drop_guard
  ).unwrap();

  wgpu::Instance::from_hal::<wgpu_hal::api::Vulkan>(wgpu_hal_instance)
}

pub unsafe fn create_surface<'window>(
  wgpu_instance: &wgpu::Instance,
  wgpu_device:   &wgpu::Device,
  create_info:   *const ash::vk::SwapchainCreateInfoKHR
)
  -> Result<wgpu::Surface<'window>, wgpu::CreateSurfaceError>
{
  {
    let mut dstate = STATE.lock().unwrap();
    dstate.surface = (*create_info).surface;
  }

  // we don't really care what is being passed here, wgpu_create_xcb_surface_khr will return the proper surface
  let wgpu_surface = wgpu_instance.create_surface_unsafe(
    wgpu::SurfaceTargetUnsafe::RawHandle {
      raw_window_handle:  raw_window_handle::RawWindowHandle::Xcb(transmute(42 as u64)),
      raw_display_handle: raw_window_handle::RawDisplayHandle::Xcb(transmute(42 as u128))
    }
  ).unwrap();

  wgpu_surface.configure(wgpu_device, &wgpu::SurfaceConfiguration {
    usage: {

      let mut flags = wgpu::TextureUsages::empty();

      if (*create_info).image_usage.contains(ash::vk::ImageUsageFlags::TRANSFER_SRC) {
        flags |= wgpu::TextureUsages::COPY_SRC;
      }

      if (*create_info).image_usage.contains(ash::vk::ImageUsageFlags::TRANSFER_DST) {
        flags |= wgpu::TextureUsages::COPY_DST;
      }

      /*if (*create_info).image_usage.contains(ash::vk::ImageUsageFlags::SAMPLED) {
        flags |= wgpu::TextureUsages::RESOURCE;
      }*/

      /*if (*create_info).image_usage.contains(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT) {
        flags |= wgpu::TextureUsages::COLOR_TARGET;
      }*/

      /*if (*create_info).image_usage.contains(ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT) {
        flags |= wgpu::TextureUsages::DEPTH_STENCIL_READ;
        flags |= wgpu::TextureUsages::DEPTH_STENCIL_WRITE;
      }*/

      //TODO: ash::vk::ImageUsageFlags::STORAGE
      //TODO: ash::vk::ImageUsageFlags::TRANSIENT_ATTACHMENT
      //TODO: ash::vk::ImageUsageFlags::INPUT_ATTACHMENT

      flags |= wgpu::TextureUsages::RENDER_ATTACHMENT; // ?
      flags |= wgpu::TextureUsages::TEXTURE_BINDING; // ?

      //flags |= wgpu::TextureUsages::all(); // ?

      flags
    },
    format:
      match (*create_info).image_format {
        ash::vk::Format::B8G8R8A8_SRGB  => wgpu::TextureFormat::Bgra8UnormSrgb,
        ash::vk::Format::B8G8R8A8_UNORM => wgpu::TextureFormat::Bgra8Unorm,
        format => unimplemented!("Unknown image format: {:?}", format)
      },
    width:  (*create_info).image_extent.width,  // ?
    height: (*create_info).image_extent.height, // ?
    present_mode:
      match (*create_info).present_mode {
        ash::vk::PresentModeKHR::IMMEDIATE => wgpu::PresentMode::Immediate,
        ash::vk::PresentModeKHR::FIFO      => wgpu::PresentMode::Fifo,
        ash::vk::PresentModeKHR::MAILBOX   => wgpu::PresentMode::Mailbox,
        mode => unimplemented!("Unknown presentation mode: {:?}", mode)
      },
    desired_maximum_frame_latency: 2, // ?
    alpha_mode: wgpu::CompositeAlphaMode::Auto,
    view_formats: vec![]
  });

  Ok(wgpu_surface)
}

pub fn get_frame(surface: &wgpu::Surface, image_index: u32) -> wgpu::SurfaceTexture {
  CURRENT_SWAPCHAIN_IMAGE_INDEX.with(|value| value.set(image_index));
  surface.get_current_texture().unwrap()
}

pub fn prepare_compute_pipeline(source: &'static str, device: &wgpu::Device) -> wgpu::ComputePipeline {

  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source))
  });

  let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
          sample_type:    wgpu::TextureSampleType::Float { filterable: false }, // ?
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled:   false
        },
        count: None
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: true },
          has_dynamic_offset: false,
          min_binding_size: None
        },
        count: None
      },
      wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: false,
          min_binding_size: None
        },
        count: None
      }
    ]
  });

  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts:   &[&bind_group_layout],
    push_constant_ranges: &[]
  });

  device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: None,
    layout: Some(&pipeline_layout),
    module: &shader,
    entry_point: "main"
  })
}

pub fn compute(
  frame:           &wgpu::SurfaceTexture,
  device:          &wgpu::Device,
  queue:           &wgpu::Queue,
  targets_buffer:  &wgpu::Buffer,
  results_buffer:  &wgpu::Buffer,
  results_buffer2: &wgpu::Buffer,
  pipeline:        &wgpu::ComputePipeline,
  targets:         &Vec<overlay_ipc::ScreenScrapingArea>
)
  -> overlay_ipc::ScreenScrapingResult
{
  let view = frame
    .texture
    .create_view(&wgpu::TextureViewDescriptor {
      label: None,
      format: None, //Some(wgpu::TextureFormat::Bgra8Unorm), //surface.get_preferred_format(&adapter),
      //..Default::default()
      dimension:         Some(wgpu::TextureViewDimension::D2),
      aspect:            wgpu::TextureAspect::All, // ?
      base_mip_level:    0,
      mip_level_count:   Some(1), // ?
      base_array_layer:  0,
      array_layer_count: Some(1) // ?
    });

  let screen_width  = frame.texture.width();
  let screen_height = frame.texture.height();

  let mut offset = 0;
  for target in targets {
    let values = &[
      target.bounds.min.x.to_px(screen_width, screen_height) as u32,
      target.bounds.min.y.to_px(screen_width, screen_height) as u32,
      target.bounds.max.x.to_px(screen_width, screen_height) as u32,
      target.bounds.max.y.to_px(screen_width, screen_height) as u32
    ];
    let bytes = bytemuck::cast_slice::<u32, u8>(values);
    queue.write_buffer(targets_buffer, offset as u64, bytes);
    offset += bytes.len();

    let values = &[
      target.min_hue,
      target.max_hue,
      target.min_sat,
      target.max_sat,
      target.min_val,
      target.max_val,
    ];
    let bytes = bytemuck::cast_slice::<f32, u8>(values);
    queue.write_buffer(targets_buffer, offset as u64, bytes);
    offset += bytes.len();
  }

  let bind_group_layout = pipeline.get_bind_group_layout(0);
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &bind_group_layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(&view)
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: targets_buffer.as_entire_binding()
      },
      wgpu::BindGroupEntry {
        binding: 2,
        resource: results_buffer.as_entire_binding()
      }
    ]
  });

  let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

  {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
    pass.set_bind_group(0, &bind_group, &[]);
    pass.set_pipeline(pipeline);
    pass.dispatch_workgroups(1, 1, 1); // ?
  }

  encoder.copy_buffer_to_buffer(results_buffer, 0, results_buffer2, 0, 100);

  queue.submit(Some(encoder.finish()));

  let buffer_slice = results_buffer2.slice(..);
  buffer_slice.map_async(wgpu::MapMode::Read, |_| ());

  device.poll(wgpu::Maintain::Wait);

  let result = bytemuck::cast_slice::<u8, f32>(&buffer_slice.get_mapped_range()).to_owned();
  results_buffer2.unmap();

  overlay_ipc::ScreenScrapingResult { pixels_in_range: result[0], uniformity_score: result[1] }
}
