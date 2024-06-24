use std::collections::HashMap;
use std::ffi::CStr;
use std::mem::transmute;
use std::os::raw::c_char;
use std::sync::Mutex;

use lazy_static::lazy_static;

mod definitions;
mod gui;
mod wasm;
mod wgpu_util;

use definitions::*;
use overlay_ipc::{Knob, OverlayCommand, OverlayMenuCommand, Shape};

pub struct WGPUSwapchainProps<'window> {
  pub width:            u32,
  pub height:           u32,
  pub instance:         wgpu::Instance,
  pub adapter:          wgpu::Adapter,
  pub device:           wgpu::Device,
  pub queue:            wgpu::Queue,
  pub surface:          wgpu::Surface<'window>,
  //pub pipeline:         wgpu::RenderPipeline,
  pub compute_pipeline: wgpu::ComputePipeline,
  pub egui_renderer:    egui_wgpu::Renderer,
  pub egui_ctx:         egui::Context
}

pub struct OverlayState {
  pub hud_is_active: bool,
  pub screen_scraping_targets: Vec<(overlay_ipc::ScreenScrapingArea, overlay_ipc::ipc::IpcSender<overlay_ipc::ScreenScrapingResult>)>,
  pub screen_scraping_targets2: HashMap<u32, (overlay_ipc::ScreenScrapingArea, overlay_ipc::ScreenScrapingResult)>,
  pub memory_targets: Vec<(u64, Vec<i32>, overlay_ipc::ipc::IpcSender<u64>)>,
  pub layer_checks: Vec<(usize, overlay_ipc::ipc::IpcSender<bool>)>,
  pub layer_names: Vec<String>,
  pub mode: u64,
  pub status_text: Option<String>,
  pub shapes: HashMap<u64 /* stage id */, Vec<(Vec<Shape>, u64 /* visibility mask */)>>,
  pub knobs: Vec<Knob>,
  pub knob_menu_visible: bool,
  pub knob_menu_selected_item: usize
}

impl OverlayState {

  pub fn new() -> Self {
    Self {
      hud_is_active: true,
      screen_scraping_targets: vec![],
      screen_scraping_targets2: HashMap::new(),
      memory_targets: vec![],
      layer_checks: vec![],
      layer_names: vec![],
      mode: 0,
      status_text: None,
      shapes: HashMap::new(),
      knobs: vec![],
      knob_menu_visible: false,
      knob_menu_selected_item: 0
    }
  }

  pub fn reset(&mut self) {
    self.hud_is_active = true;
    self.screen_scraping_targets.clear();
    self.screen_scraping_targets2.clear();
    self.memory_targets.clear();
    self.layer_checks.clear();
    self.layer_names.clear();
    self.mode = 0;
    self.status_text = None;
    self.shapes.clear();
    self.knobs.clear();
  }
}

impl Default for OverlayState {
  fn default() -> Self {
    Self::new()
  }
}

lazy_static! {

  static ref WGPU_SWAPCHAIN_PROPS: Mutex<HashMap<ash::vk::SwapchainKHR, WGPUSwapchainProps<'static>>> = Mutex::new(HashMap::new());

  static ref OVERLAY_STATE: Mutex<OverlayState> = Mutex::new(OverlayState::new());

  static ref OVERLAY_COMMAND_THREAD: Mutex<std::thread::JoinHandle<()>> = Mutex::new({

    let overlay_name = std::env::var("STWGS_OVERLAY_NAME").unwrap_or_else(|_|
      std::env::current_exe().unwrap().file_name().unwrap().to_string_lossy().into_owned());

    let receiver = overlay_ipc::process_incoming_commands(&overlay_name);
    std::thread::spawn(move || {
      loop {
        match receiver.recv() {
          Ok(command) => match command {
            OverlayCommand::ToggleUI => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.hud_is_active = !overlay.hud_is_active;
            },
            OverlayCommand::AddScreenScrapingArea(area, sender) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.screen_scraping_targets.push((area, sender));
            },
            OverlayCommand::AddMemoryCheck(pointer_size, address, offsets, sender) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              if core::mem::size_of::<usize>() * 8 == pointer_size as usize {
                overlay.memory_targets.push((address, offsets, sender));
              } else {
                eprintln!("pointer size mismatch: {:?}", overlay_ipc::OverlayCommand::AddMemoryCheck(pointer_size, address, offsets, sender));
              }
            },
            OverlayCommand::AddOverlayCheck(name, sender) => {
              let active_idx = wasm::ACTIVE_PROBE_IDX.lock().unwrap();
              if let Some(idx) = *active_idx {
                let probes = wasm::REGISTERED_PROBES.lock().unwrap();
                if let Some(overlay_layer_idx) = probes[idx].layers.iter().position(|overlay_layer| *overlay_layer == name) {
                  let mut overlay = OVERLAY_STATE.lock().unwrap();
                  overlay.layer_checks.push((overlay_layer_idx, sender));
                } else {
                  // nope
                }
              } else {
                // nope
              }
            },
            OverlayCommand::ResetOverlay => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.reset();
            },
            OverlayCommand::SetLayerNames(names) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.layer_names = names;
            },
            OverlayCommand::SetMode(mode) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.mode = mode;
            },
            OverlayCommand::SetStatusText(str) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.status_text = str;
            },
            OverlayCommand::RegisterShapes { stage_id, shapes } => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.shapes.insert(stage_id, shapes.iter().map(|s| (s.clone(), 0)).collect());
            },
            OverlayCommand::ToggleShapes { stage_id, layer, mask } => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              if let Some(layers) = overlay.shapes.get_mut(&stage_id) {
                if let Some((_, m)) = layers.get_mut(layer as usize) {
                  *m = mask;
                } else {
                  eprintln!("Layer {} for stage {} doesn't exist", layer, stage_id);
                }
              } else {
                eprintln!("Found no layers for stage {}", stage_id);
              }
            },
            OverlayCommand::RegisterKnobs(knobs) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              overlay.knobs = knobs;
            },
            OverlayCommand::MenuCommand(command) => {
              let mut overlay = OVERLAY_STATE.lock().unwrap();
              match command {
                OverlayMenuCommand::OpenKnobsMenu => {
                  overlay.knob_menu_visible       = true;
                  overlay.knob_menu_selected_item = 0;
                },
                OverlayMenuCommand::SelectPrevMenuItem => {
                  if overlay.knob_menu_selected_item > 0 {
                    overlay.knob_menu_selected_item -= 1;
                  } else {
                    overlay.knob_menu_selected_item = overlay.knobs.len() - 1;
                  }
                },
                OverlayMenuCommand::SelectNextMenuItem => {
                  overlay.knob_menu_selected_item = (overlay.knob_menu_selected_item + 1) % overlay.knobs.len();
                },
                OverlayMenuCommand::SelectPrevValue => {
                  let knob_menu_selected_item = overlay.knob_menu_selected_item;
                  let knob = &mut overlay.knobs[knob_menu_selected_item];
                  match knob {
                    Knob::Flag { value, .. } => {
                      *value = !*value;
                    },
                    Knob::Enum { index, .. } => {
                      if *index > 0 {
                        *index -= 1;
                      }
                    },
                    Knob::Number { value, min_value, max_value, .. } => {
                      assert!(*max_value > *min_value);
                      *value -= (*max_value - *min_value) * 0.05;
                      if *value < *min_value {
                        *value = *min_value;
                      }
                    }
                  }
                },
                OverlayMenuCommand::SelectNextValue => {
                  let knob_menu_selected_item = overlay.knob_menu_selected_item;
                  let knob = &mut overlay.knobs[knob_menu_selected_item];
                  match knob {
                    Knob::Flag { value, .. } => {
                      *value = !*value;
                    },
                    Knob::Enum { index, options, .. } => {
                      if *index < options.len() - 1 {
                        *index += 1;
                      }
                    },
                    Knob::Number { value, min_value, max_value, .. } => {
                      assert!(*max_value > *min_value);
                      *value += (*max_value - *min_value) * 0.05;
                      if *value > *max_value {
                        *value = *max_value;
                      }
                    }
                  }
                },
                OverlayMenuCommand::SetValuePercentage => todo!(),
                OverlayMenuCommand::CloseKnobsMenu => {
                  overlay.knob_menu_visible = false;
                }
              }
            },
            OverlayCommand::GetKnobs(sender) => {
              let overlay = OVERLAY_STATE.lock().unwrap();
              let _ = sender.send(overlay.knobs.clone());
            }
          },
          Err(_) => {
            panic!(); //TODO: this line prevents 100% CPU usage in gta-sa and it's not even being executed, wtf?
          }
        }
      }
    })
  });
}

use std::ops::Deref;
use std::sync::Arc;

struct Registry {
  instances:       HashMap<ash::vk::Instance,       (Arc<ash::Instance>, ash::vk::PFN_vkGetInstanceProcAddr)>,
  phys_dev_2_inst: HashMap<ash::vk::PhysicalDevice, Arc<ash::Instance>>,
  devices:         HashMap<ash::vk::Device,         (Arc<ash::Instance>, Arc<ash::Device>, ash::extensions::khr::Swapchain, ash::vk::PhysicalDevice)>,
  queue_2_dev:     HashMap<ash::vk::Queue, Arc<ash::Device>>
}

impl Registry {

  pub fn new() -> Self {
    Self {
      instances:       HashMap::new(),
      phys_dev_2_inst: HashMap::new(),
      devices:         HashMap::new(),
      queue_2_dev:     HashMap::new()
    }
  }

  pub unsafe fn register_instance(&mut self, instance: ash::vk::Instance, get_instance_proc_addr: ash::vk::PFN_vkGetInstanceProcAddr) {

    let ash_instance = Arc::new(ash::Instance::load(&ash::vk::StaticFn { get_instance_proc_addr }, instance));
    self.instances.insert(instance, (Arc::clone(&ash_instance), get_instance_proc_addr));

    for phys_device in ash_instance.enumerate_physical_devices().unwrap() {
      self.phys_dev_2_inst.insert(phys_device, Arc::clone(&ash_instance));
    }
  }

  pub fn remove_instance(&mut self, instance: ash::vk::Instance) {
    self.instances.remove(&instance);
  }

  pub fn instance(&self, instance: ash::vk::Instance) -> &ash::Instance {
    &self.instances[&instance].0
  }

  pub fn instance_by_dev(&self, device: ash::vk::Device) -> &ash::Instance {
    &self.devices[&device].0
  }

  pub unsafe fn get_instance_proc_addr(&self, instance: ash::vk::Instance, name: *const c_char) -> ash::vk::PFN_vkVoidFunction {
    (self.instances[&instance].1)(instance, name)
  }

  //TODO: do we actually need to replace get_device_proc_addr here?
  pub unsafe fn register_device(&mut self, physical_device: ash::vk::PhysicalDevice, device: ash::vk::Device, get_device_proc_addr: ash::vk::PFN_vkGetDeviceProcAddr) {

    #[allow(mutable_transmutes)]
    unsafe fn replace_get_device_proc_addr(instance: &ash::Instance, get_device_proc_addr: ash::vk::PFN_vkGetDeviceProcAddr) {

      struct XInstance {
        pub _handle:          ash::vk::Instance,
        pub instance_fn_1_0:  ash::vk::InstanceFnV1_0,
        pub _instance_fn_1_1: ash::vk::InstanceFnV1_1,
        pub _instance_fn_1_2: ash::vk::InstanceFnV1_2
      }

      let instance: &mut XInstance = transmute(instance);
      instance.instance_fn_1_0.get_device_proc_addr = get_device_proc_addr;
    }

    let ash_instance  = &self.phys_dev_2_inst[&physical_device];
    replace_get_device_proc_addr(ash_instance, get_device_proc_addr);
    let ash_device    = Arc::new(ash::Device::load(ash_instance.fp_v1_0(), device));
    let ash_swapchain = ash::extensions::khr::Swapchain::new(ash_instance.deref(), ash_device.deref());
    self.devices.insert(device, (Arc::clone(ash_instance), ash_device, ash_swapchain, physical_device));
  }

  pub fn remove_device(&mut self, device: ash::vk::Device) {
    self.devices.remove(&device);
  }

  pub fn device(&self, device: ash::vk::Device) -> &ash::Device {
    &self.devices[&device].1
  }

  /*pub fn device_with_swapchain(&self, device: ash::vk::Device) -> (&ash::Device, &ash::extensions::khr::Swapchain) {
    (&self.devices[&device].1, &self.devices[&device].2)
  }*/

  pub fn device_by_queue(&self, queue: ash::vk::Queue) -> &ash::Device {
    &self.queue_2_dev[&queue]
  }

  pub unsafe fn get_device_proc_addr(&self, device: ash::vk::Device, name: *const c_char) -> ash::vk::PFN_vkVoidFunction {
    self.devices[&device].0.get_device_proc_addr(device, name)
  }

  pub fn swapchain(&self, device: ash::vk::Device) -> &ash::extensions::khr::Swapchain {
    &self.devices[&device].2
  }

  /*pub fn physical_device_by_dev(&self, device: ash::vk::Device) -> ash::vk::PhysicalDevice {
    self.devices[&device].3
  }*/

  pub fn register_queue(&mut self, queue: ash::vk::Queue, device: ash::vk::Device) {
    self.queue_2_dev.insert(queue, Arc::clone(&self.devices[&device].1));
  }
}

lazy_static! {
  static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}

unsafe extern "C" fn overlay_vk_create_instance(
  create_info: *const ash::vk::InstanceCreateInfo,
  allocator:   *const ash::vk::AllocationCallbacks,
  instance:    *mut   ash::vk::Instance
)
  -> ash::vk::Result
{
  #[cfg(debug_assertions)]
  {
    eprintln!("overlay_vk_create_instance(#{:p}, #{:p}, #{:p})", create_info, allocator, instance);

    assert!(!create_info.is_null());
    let application_info = (*create_info).p_application_info;
    if !application_info.is_null() {
      let api_version = (*application_info).api_version;
      assert_eq!(ash::vk::api_version_variant(api_version), 0);
      eprintln!("api version: {}.{}.{}",
        ash::vk::api_version_major(api_version),
        ash::vk::api_version_minor(api_version),
        ash::vk::api_version_patch(api_version));
    }

    for i in 0..(*create_info).enabled_extension_count {
      eprintln!("instance extension: {}", CStr::from_ptr(*(*create_info).pp_enabled_extension_names.offset(i as isize)).to_string_lossy());
    }

    for i in 0..(*create_info).enabled_layer_count {
      eprintln!("instance layer: {}",     CStr::from_ptr(*(*create_info).pp_enabled_layer_names.offset(i as isize)).to_string_lossy());
    }
  }

  let mut layer_create_info = (*create_info).p_next as *mut VkLayerInstanceCreateInfo;

  while !layer_create_info.is_null() && (
    (*layer_create_info).s_type   != ash::vk::StructureType::LOADER_INSTANCE_CREATE_INFO ||
    (*layer_create_info).function != VK_LAYER_LINK_INFO)
  {
    layer_create_info = (*layer_create_info).p_next as *mut VkLayerInstanceCreateInfo;
  }

  if layer_create_info.is_null() {
    return ash::vk::Result::ERROR_INITIALIZATION_FAILED;
  }

  let gpa = (*(*layer_create_info).u.layer_info).next_get_instance_proc_addr;

  (*layer_create_info).u.layer_info = (*(*layer_create_info).u.layer_info).p_next as *const VkLayerInstanceLink;

  let create_instance: ash::vk::PFN_vkCreateInstance = transmute(gpa(ash::vk::Instance::null(),
    CStr::from_bytes_with_nul_unchecked(b"vkCreateInstance\0").as_ptr()).unwrap());

  let err = create_instance(create_info, allocator, instance);

  if err == ash::vk::Result::SUCCESS {
    let mut registry = REGISTRY.lock().unwrap();
    registry.register_instance(*instance, gpa);
  }

  err
}

unsafe extern "C" fn overlay_vk_destroy_instance(instance: ash::vk::Instance, allocator: *const ash::vk::AllocationCallbacks) {
  let mut registry = REGISTRY.lock().unwrap();
  (registry.instance(instance).fp_v1_0().destroy_instance)(instance, allocator);
  registry.remove_instance(instance);
}

unsafe extern "C" fn overlay_vk_create_device(
  physical_device: ash::vk::PhysicalDevice,
  create_info:     *const ash::vk::DeviceCreateInfo,
  allocator:       *const ash::vk::AllocationCallbacks,
  device:          *mut ash::vk::Device
)
  -> ash::vk::Result
{
  #[cfg(debug_assertions)]
  {
    eprintln!("overlay_vk_create_device");

    for i in 0..(*create_info).enabled_extension_count {
      eprintln!("device extension: {}", CStr::from_ptr(*(*create_info).pp_enabled_extension_names.offset(i as isize)).to_string_lossy());
    }
  }

  let revised_create_info = {
    let mut create_info = *create_info;
    let mut extensions  = vec![];
    extensions.extend_from_slice(std::slice::from_raw_parts(create_info.pp_enabled_extension_names, create_info.enabled_extension_count as usize));
    extensions.push(b"VK_KHR_maintenance1\0".as_ptr() as *const c_char); // wgpu uses negative viewport heights
    extensions.push(b"VK_KHR_maintenance2\0".as_ptr() as *const c_char); // wgpu needs this
    create_info.pp_enabled_extension_names = extensions.as_slice().as_ptr();
    create_info.enabled_extension_count    = extensions.len() as u32;
    std::mem::forget(extensions); // ?
    create_info
  };

  let create_info = &revised_create_info;

  let mut layer_create_info = create_info.p_next as *mut VkLayerDeviceCreateInfo;

  while !layer_create_info.is_null() && (
    (*layer_create_info).s_type   != ash::vk::StructureType::LOADER_DEVICE_CREATE_INFO ||
    (*layer_create_info).function != VK_LAYER_LINK_INFO)
  {
    layer_create_info = (*layer_create_info).p_next as *mut VkLayerDeviceCreateInfo;
  }

  if layer_create_info.is_null() {
    return ash::vk::Result::ERROR_INITIALIZATION_FAILED;
  }

  let gipa = (*(*layer_create_info).u.layer_info).next_get_instance_proc_addr;
  let gdpa = (*(*layer_create_info).u.layer_info).next_get_device_proc_addr;

  (*layer_create_info).u.layer_info = (*(*layer_create_info).u.layer_info).p_next as *const VkLayerDeviceLink;

  let create_device: ash::vk::PFN_vkCreateDevice = transmute(gipa(ash::vk::Instance::null(),
    CStr::from_bytes_with_nul_unchecked(b"vkCreateDevice\0").as_ptr()).unwrap());

  let err = create_device(physical_device, create_info, allocator, device);
  if err == ash::vk::Result::SUCCESS {
    let mut registry = REGISTRY.lock().unwrap();
    registry.register_device(physical_device, *device, gdpa);
  }
  err
}

unsafe extern "C" fn overlay_vk_destroy_device(device: ash::vk::Device, allocator: *const ash::vk::AllocationCallbacks) {
  let mut registry = REGISTRY.lock().unwrap();
  let destroy_device = registry.device(device).fp_v1_0().destroy_device;
  destroy_device(device, allocator);
  registry.remove_device(device);
}

//TODO: vkGetDeviceQueue2?
unsafe extern "C" fn overlay_vk_get_device_queue(device: ash::vk::Device, queue_family_index: u32, queue_index: u32, queue: *mut ash::vk::Queue) {
  let mut registry = REGISTRY.lock().unwrap();
  *queue = registry.device(device).get_device_queue(queue_family_index, queue_index);
  registry.register_queue(*queue, device);
}

unsafe extern "C" fn overlay_vk_create_swapchain_khr(
  device:      ash::vk::Device,
  create_info: *const ash::vk::SwapchainCreateInfoKHR,
  allocator:   *const ash::vk::AllocationCallbacks,
  swapchain:   *mut ash::vk::SwapchainKHR
)
  -> ash::vk::Result
{
  //eprintln!("[overlay_vk_create_swapchain_khr]");
  let err = {
    let registry = REGISTRY.lock().unwrap();
    let create_swapchain_khr = registry.swapchain(device).fp().create_swapchain_khr;
    create_swapchain_khr(device, create_info, allocator, swapchain)
  };

  if err == ash::vk::Result::SUCCESS {

    // wgpu
    {
      let instance = {
        let registry = REGISTRY.lock().unwrap();
        registry.instance_by_dev(device).clone()
      };

      let wgpu_instance = wgpu_util::create_wgpu_instance(instance.handle(), device, *swapchain);

      let adapter =
        futures::executor::block_on(
          wgpu_instance
            .request_adapter(&wgpu::RequestAdapterOptions {
              power_preference:       wgpu::PowerPreference::default(),
              force_fallback_adapter: false,
              compatible_surface:     None //Some(&wgpu_surface)
            })
        ).expect("Failed to find an appropriate adapter");

      let (wgpu_device, wgpu_queue) = futures::executor::block_on(
        adapter
          .request_device(
            &wgpu::DeviceDescriptor {
              label:             None,
              required_features: wgpu::Features::empty(),
              required_limits:   wgpu::Limits::default()
                .using_resolution(adapter.limits()),
            },
            None)
      ).expect("Failed to create device");

      let wgpu_surface = wgpu_util::create_surface(&wgpu_instance, &wgpu_device, create_info).unwrap();
      //println!("wgpu surface: {:?}", wgpu_surface);

      //let pipeline = wgpu_util::prepare(&adapter, &wgpu_device, &wgpu_surface);
      let compute_pipeline = wgpu_util::prepare_compute_pipeline(&wgpu_device);

      let egui_ctx      = egui::Context::default();
      let egui_renderer = egui_wgpu::Renderer::new(&wgpu_device,
        wgpu::TextureFormat::Bgra8Unorm /*wgpu_surface.get_preferred_format(&adapter).unwrap()*/, None, 1);

      let mut overlay = OVERLAY_STATE.lock().unwrap();
      overlay.screen_scraping_targets2.clear();

      WGPU_SWAPCHAIN_PROPS.lock().unwrap().insert(*swapchain, WGPUSwapchainProps {
        width:    (*create_info).image_extent.width,
        height:   (*create_info).image_extent.height,
        instance: wgpu_instance,
        adapter,
        device:   wgpu_device,
        queue:    wgpu_queue,
        surface:  wgpu_surface,
        //pipeline,
        compute_pipeline,
        egui_renderer,
        egui_ctx
      });
    }

    // ipc init
    drop(OVERLAY_COMMAND_THREAD.lock().unwrap());
  }
  err
}

unsafe extern "C" fn overlay_vk_destroy_swapchain_khr(
  device:    ash::vk::Device,
  swapchain: ash::vk::SwapchainKHR,
  allocator: *const ash::vk::AllocationCallbacks
) {
  WGPU_SWAPCHAIN_PROPS.lock().unwrap().remove(&swapchain);
  let registry = REGISTRY.lock().unwrap();
  let destroy_swapchain_khr = registry.swapchain(device).fp().destroy_swapchain_khr;
  destroy_swapchain_khr(device, swapchain, allocator);
}

pub unsafe fn follow_pointer_chain(address: u64, offsets: &Vec<i32>) -> usize {
  assert_ne!(address, 0);
  let mut p: *const u8 = address as *const u8;
  for offset in offsets {
    p = *(p as *const *const u8);
    if p.is_null() {
      return 0;
    }
    p = p.offset(*offset as isize);
  }
  *(p as *const usize)
}

unsafe extern "C" fn overlay_vk_queue_present_khr(queue: ash::vk::Queue, present_info: *const ash::vk::PresentInfoKHR) -> ash::vk::Result {

  //println!("overlay_vk_queue_present_khr({:p}, {:p})", queue, present_info);

  let device            = {
    let registry = REGISTRY.lock().unwrap();
    registry.device_by_queue(queue).clone()
  };

  /*let instance          = {
    let registry = REGISTRY.lock().unwrap();
    registry.instance_by_dev(device.handle())
  };*/

  let queue_present_khr = {
    let registry = REGISTRY.lock().unwrap();
    registry.swapchain(device.handle()).fp().queue_present_khr
  };

  let present_info = *present_info;
  assert_eq!(present_info.swapchain_count, 1);

  let mut result = ash::vk::Result::SUCCESS;

  let mut pi = present_info;
  for i in 0..(present_info.swapchain_count) {

    pi.swapchain_count = 1;
    pi.p_image_indices = present_info.p_image_indices.offset(i as isize);
    pi.p_swapchains    = present_info.p_swapchains   .offset(i as isize);

    let mut wgpu_props = WGPU_SWAPCHAIN_PROPS.lock().unwrap();
    let wgpu_props     = wgpu_props.get_mut(&*present_info.p_swapchains).unwrap();
    let screen_width   = wgpu_props.width;
    let screen_height  = wgpu_props.height;

    let frame = wgpu_util::get_frame(&wgpu_props.surface, *pi.p_image_indices);

    if let Some(ref mut wasm) = wasm::WASM.lock().unwrap().as_mut() {
      wasm.run_probe(screen_width, screen_height);
    }

    let mut overlay = OVERLAY_STATE.lock().unwrap();

    let scraping_result = {
      /*if !overlay.screen_scraping_targets.is_empty() {
        let targets = overlay.screen_scraping_targets.iter().map(|t| t.0.clone()).collect();
        let scraping_result = wgpu_util::compute(
          &frame, &wgpu_props.device, &wgpu_props.queue, &wgpu_props.compute_pipeline, &targets, screen_width, screen_height);
        let _ = overlay.screen_scraping_targets[0].1.send(scraping_result.clone());
        Some(scraping_result)
      } else {
        None
      }*/

      if !overlay.screen_scraping_targets2.is_empty() {
        for (_, v) in overlay.screen_scraping_targets2.iter_mut() {
          let mut targets = vec![v.0.clone()];
          let scraping_result = wgpu_util::compute(
            &frame, &wgpu_props.device, &wgpu_props.queue, &wgpu_props.compute_pipeline, &targets, screen_width, screen_height);
          *v = (targets.remove(0), scraping_result);
        }
        overlay.screen_scraping_targets2.values().nth(0).map(|x| x.1.clone())
      } else {
        None
      }
    };

    if !overlay.memory_targets.is_empty() {
      for target in &overlay.memory_targets {
        let value = follow_pointer_chain(target.0, &target.1);
        let _ = target.2.send(value as u64);
      }
    }

    let egui_output        = gui::draw_ui(&overlay, &wgpu_props.egui_ctx, (screen_width, screen_height), scraping_result);
    let clipped_primitives = wgpu_props.egui_ctx.tessellate(egui_output.shapes, wgpu_props.egui_ctx.pixels_per_point());

    let mut encoder = wgpu_props.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("egui encoder") });

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
      size_in_pixels:  [wgpu_props.width, wgpu_props.height],
      pixels_per_point: 1.0 // ?
    };

    let egui_renderer = &mut wgpu_props.egui_renderer;

    for (id, delta) in egui_output.textures_delta.set {
      egui_renderer.update_texture(&wgpu_props.device, &wgpu_props.queue, id, &delta);
    }

    egui_renderer.update_buffers(&wgpu_props.device, &wgpu_props.queue, &mut encoder, &clipped_primitives, &screen_descriptor);

    let view = frame
      .texture
      .create_view(&wgpu::TextureViewDescriptor {
        label:             None,
        format:            Some(wgpu::TextureFormat::Bgra8Unorm),
        dimension:         Some(wgpu::TextureViewDimension::D2),
        aspect:            wgpu::TextureAspect::All, // ?
        base_mip_level:    0,
        mip_level_count:   Some(1), // ?
        base_array_layer:  0,
        array_layer_count: None //Some(1) // ?
      });

    {
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("egui"),
        color_attachments: &[
          Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None, // ?
            ops: wgpu::Operations {
              load:  wgpu::LoadOp::Load,
              store: wgpu::StoreOp::Store
            },
          })
        ],
        depth_stencil_attachment: None,
        timestamp_writes:         None,
        occlusion_query_set:      None
      });

      egui_renderer.render(&mut render_pass, &clipped_primitives, &screen_descriptor);
    }

    wgpu_props.queue.submit(Some(encoder.finish()));

    //pi.pWaitSemaphores = &swapchain_data->submission_semaphore[pPresentInfo->pImageIndices[i]];
    //pi.waitSemaphoreCount = 1;

    let res = queue_present_khr(queue, &pi);
    if res != ash::vk::Result::SUCCESS {
      result = res;
    }

    //frame.present(); // noop
    device.queue_wait_idle(queue).unwrap(); // validation error in wgpu 0.12
    //frame.texture.destroy();

    // TODO: are we supposed to free textures here?
    for id in egui_output.textures_delta.free {
      wgpu_props.egui_renderer.free_texture(&id);
    }

    //std::thread::sleep(std::time::Duration::from_millis(10000));

    if !present_info.p_results.is_null() {
      *present_info.p_results.offset(i as isize) = res;
    }
  }

  result
}

unsafe fn get_overlay_function(name: *const c_char) -> Option<*const()> {
  match CStr::from_ptr(name).to_str().unwrap() {
    "vkGetDeviceProcAddr"    => Some(overlay_vk_get_device_proc_addr as *const()),
    "vkGetInstanceProcAddr"  => Some(overlay_vk_get_instance_proc_addr as *const()),
    "vkCreateDevice"         => Some(overlay_vk_create_device as *const()),
    "vkCreateInstance"       => Some(overlay_vk_create_instance as *const()),
    "vkCreateSwapchainKHR"   => Some(overlay_vk_create_swapchain_khr as *const()),
    "vkDestroyDevice"        => Some(overlay_vk_destroy_device as *const()),
    "vkDestroyInstance"      => Some(overlay_vk_destroy_instance as *const()),
    "vkDestroySwapchainKHR"  => Some(overlay_vk_destroy_swapchain_khr as *const()),
    "vkGetDeviceQueue"       => Some(overlay_vk_get_device_queue as *const()),
    "vkQueuePresentKHR"      => Some(overlay_vk_queue_present_khr as *const()),
    _ => None
  }
}

#[no_mangle]
pub unsafe extern "C" fn overlay_vk_get_device_proc_addr(device: ash::vk::Device, name: *const c_char) -> *const() {
  if let Some(fun) = get_overlay_function(name) {
    fun
  } else {
    eprintln!("overlay_vk_get_device_proc_addr: {:?} {}", device, CStr::from_ptr(name).to_str().unwrap());
    let registry = REGISTRY.lock().unwrap();
    transmute(registry.get_device_proc_addr(device, name))
  }
}

#[no_mangle]
pub unsafe extern "C" fn overlay_vk_get_instance_proc_addr(instance: ash::vk::Instance, name: *const c_char) -> *const() {
  if let Some(fun) = get_overlay_function(name) {
    fun
  } else {
    eprintln!("overlay_vk_get_instance_proc_addr: {:?} {}", instance, CStr::from_ptr(name).to_str().unwrap());
    let registry = REGISTRY.lock().unwrap();
    transmute(registry.get_instance_proc_addr(instance, name))
  }
}
