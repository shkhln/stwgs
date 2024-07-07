#![no_std]
extern crate std;

use std::ffi::CString;
use std::vec::Vec;

custom_print::define_macros!({ print, println }, concat, extern "C" fn print(_: *const u8, _: usize));
custom_print::define_macros!({ eprint, eprintln, dbg }, concat, extern "C" fn print(_: *const u8, _: usize));
custom_print::define_init_panic_hook!(concat, extern "C" fn print(_: *const u8, _: usize));

extern "C" {
  fn _register_probe(name: *const u8, executable: *const u8, flags: *const *const u8, flags_len: usize,
    init: extern "C" fn(u32, u32) -> i32 /* bool */, probe: extern "C" fn() -> u64);

  fn _add_screen_target(x1: f32, y1: f32, x2: f32, y2: f32) -> u32;
  fn _set_screen_target_option(id: u32, option: *const u8, value: *const u8) -> ();
  fn _test_screen_target(id: u32, threshold1: f32, threshold2: f32) -> i32 /* bool */;

  fn _peek_mem32(address: u32) -> u32;
}

fn register_probe(name: &str, executable: &str, flags: &[&str], test: extern "C" fn(u32, u32) -> i32, probe: extern "C" fn() -> u64) {
  let name  = CString::new(name).unwrap();
  let exe   = CString::new(executable).unwrap();
  let flags = flags.iter()
    .map(|flag_name| CString::new(*flag_name).unwrap().into_raw() as *const u8)
    .collect::<Vec<_>>();
  unsafe {
    _register_probe(name.as_ptr() as *const u8, exe.as_ptr() as *const u8, flags.as_ptr(), flags.len(), test, probe);
  }
}

fn add_screen_target(x1: f32, y1: f32, x2: f32, y2: f32) -> u32 {
  unsafe { _add_screen_target(x1, y1, x2, y2) }
}

fn set_screen_target_option(id: u32, option: &str, value: &str) {
  let option = CString::new(option).unwrap();
  let value  = CString::new(value).unwrap();
  unsafe { _set_screen_target_option(id, option.as_ptr() as *const u8, value.as_ptr() as *const u8); }
}

fn test_screen_target(id: u32, threshold1: f32, threshold2: f32) -> bool {
  unsafe { _test_screen_target(id, threshold1, threshold2) == 1 }
}

fn peek_mem32(address: u32) -> u32 {
  unsafe { _peek_mem32(address) }
}

static GTA_SA_PLANE_IDS: &'static [u16] = &[460, 464, 476, 511, 512, 513, 519, 539, 553, 577, 592, 593];
static GTA_SA_HELI_IDS:  &'static [u16] = &[417, 425, 447, 465, 469, 487, 488, 497, 501, 548, 563];
static GTA_SA_VTOL_ID: u16 = 520;

extern "C" fn gta_sa_init(screen_width: u32, screen_height: u32) -> i32 {

  // should point at the green dollar sign
  let idx = add_screen_target(
    screen_width as f32 * 0.782, screen_height as f32 * 0.185,
    screen_width as f32 * 0.798, screen_height as f32 * 0.212);
  assert_eq!(idx, 0);
  set_screen_target_option(idx, "min_hue", "108.0");
  set_screen_target_option(idx, "max_hue", "110.0");

  true as i32
}

extern "C" fn gta_sa_probe() -> u64 {

  #[derive(Debug)]
  enum VehicleType {
    Heli,
    Plane,
    Other
  }

  //TODO: different proportions
  let hud_is_visible = test_screen_target(0, 0.75, 1.0);
  println!("hud_is_visible: {}", hud_is_visible);

  if hud_is_visible {
    let vehicle = peek_mem32(0x00C0FEE0);
    println!("0x00C0FEE0: {:x}", vehicle);
    if vehicle != 0 {
      let vehicle_id   = (peek_mem32(vehicle + 0x20) >> 16) as u16;
      let vehicle_type = match () {
        _ if vehicle_id == GTA_SA_VTOL_ID => {
          let nozzle_angle = peek_mem32(vehicle + 0x86C) as u16 /* 0 (back) to 5000 (down) */;
          if nozzle_angle < 3000 {
            VehicleType::Plane
          } else {
            VehicleType::Heli
          }
        },
        _ if GTA_SA_PLANE_IDS.iter().any(|id| *id == vehicle_id) => VehicleType::Plane,
        _ if GTA_SA_HELI_IDS .iter().any(|id| *id == vehicle_id) => VehicleType::Heli,
        _ => VehicleType::Other
      };
      println!("vehicle_id: {} [{:?}]", vehicle_id, vehicle_type);
      match vehicle_type {
        VehicleType::Plane => 1 << 4, // plane
        VehicleType::Heli  => 1 << 3, // heli
        VehicleType::Other => 1 << 2  // ride
      }
    } else {
      1 << 1 // walk
    }
  } else {
    1 << 0 // menu
  }
}

#[no_mangle]
pub extern "C" fn init() {
  register_probe("GTA: SA", "gta-sa.exe", &["menu", "walk", "ride", "heli", "plane"], gta_sa_init, gta_sa_probe);
}
