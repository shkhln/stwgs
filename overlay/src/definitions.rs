use std::os::raw::c_void;

pub type VkLayerFunction = u32; // ?

pub const VK_LAYER_LINK_INFO:      VkLayerFunction = 0;
//pub const VK_LOADER_DATA_CALLBACK: VkLayerFunction = 1;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkLayerInstanceLink {
  pub p_next:                        *const c_void,
  pub next_get_instance_proc_addr:   ash::vk::PFN_vkGetInstanceProcAddr,
  pub get_physical_device_proc_addr: *const c_void // PFN_GetPhysicalDeviceProcAddr
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkLayerInstanceCreateInfo {
  pub s_type: ash::vk::StructureType,
  pub p_next: *const c_void,
  pub function: VkLayerFunction,
  pub u: VkLayerInstanceCreateInfoU
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union VkLayerInstanceCreateInfoU {
  pub layer_info:               *const VkLayerInstanceLink,
  pub set_instance_loader_data: *const c_void,
  /*struct {
      PFN_vkLayerCreateDevice pfnLayerCreateDevice;
      PFN_vkLayerDestroyDevice pfnLayerDestroyDevice;
  } layerDevice;
  VkLoaderFeatureFlags loaderFeatures;*/
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkLayerInstanceInfo {
  pub instance_info:               *const c_void,
  pub next_get_instance_proc_addr: ash::vk::PFN_vkGetInstanceProcAddr
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkLayerDeviceLink {
  pub p_next: *const c_void,
  pub next_get_instance_proc_addr: ash::vk::PFN_vkGetInstanceProcAddr,
  pub next_get_device_proc_addr:   ash::vk::PFN_vkGetDeviceProcAddr
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct VkLayerDeviceCreateInfo {
  pub s_type: ash::vk::StructureType,
  pub p_next: *const c_void,
  pub function: VkLayerFunction,
  pub u: VkLayerDeviceCreateInfoU
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union VkLayerDeviceCreateInfoU {
  pub layer_info: *const VkLayerDeviceLink,
  pub set_device_loader_data: *const c_void // PFN_vkSetDeviceLoaderData
}
