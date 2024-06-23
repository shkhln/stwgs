#![no_std]
extern crate std;

use std::ffi::CString;
use std::vec::Vec;

custom_print::define_macros!({ print, println }, concat, extern "C" fn print(_: *const u8, _: usize));
custom_print::define_macros!({ eprint, eprintln, dbg }, concat, extern "C" fn print(_: *const u8, _: usize));
custom_print::define_init_panic_hook!(concat, extern "C" fn print(_: *const u8, _: usize));

extern "C" {
  fn _register_probe(name: *const u8, layers: *const *const u8, layers_len: usize,
    test: extern "C" fn() -> i32 /* bool */, /*init: extern "C" fn() -> (),*/ probe: extern "C" fn(u32, u32) -> u64);

  fn _peek_mem32(address: u32) -> u32;
  fn _test_screen(id: u32,
    x1: f32, y1: f32,
    x2: f32, y2: f32,
    min_hue: f32, max_hue: f32,
    min_sat: f32, max_sat: f32,
    min_val: f32, max_val: f32,
    threshold1: f32,
    threshold2: f32
  ) -> bool;
}

fn register_probe(name: &str, layers: &[&str], test: extern "C" fn() -> i32, probe: extern "C" fn(u32, u32) -> u64) {
  let name   = CString::new(name).unwrap();
  let layers = layers.iter()
    .map(|layer| CString::new(*layer).unwrap().into_raw() as *const u8)
    .collect::<Vec<_>>();
  unsafe {
    _register_probe(name.as_ptr() as *const u8, layers.as_ptr(), layers.len(), test, probe);
  }
}

fn peek_mem32(address: u32) -> u32 {
  unsafe { _peek_mem32(address) }
}

static GTA_SA_PLANE_IDS: &'static [u16] = &[460, 464, 476, 511, 512, 513, 519, 539, 553, 577, 592, 593];
static GTA_SA_HELI_IDS:  &'static [u16] = &[417, 425, 447, 465, 469, 487, 488, 497, 501, 548, 563];
static GTA_SA_VTOL_ID: u16 = 520;

extern "C" fn gta_sa_test() -> i32 {
  true as i32
}

extern "C" fn gta_sa_probe(screen_width: u32, screen_height: u32) -> u64 {

  #[derive(Debug)]
  enum VehicleType {
    Heli,
    Plane,
    Other
  }

  //TODO: different proportions
  let visible_hud = unsafe {
    // should point at the green dollar sign
    _test_screen(0,
      screen_width as f32 * 0.782, screen_height as f32 * 0.185,
      screen_width as f32 * 0.798, screen_height as f32 * 0.212,
      108.0, 110.0,
      0.0, 1.0,
      0.0, 1.0,
      0.75,
      1.0)
  };
  println!("visible_hud: {}", visible_hud);

  if visible_hud {
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
  register_probe("GTA: SA", &["menu", "walk", "ride", "heli", "plane"], gta_sa_test, gta_sa_probe);
}
