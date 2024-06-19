#![no_std]
extern crate std;

custom_print::define_macros!({ print, println }, concat, extern "C" fn print(_: *const u8, _: usize));
custom_print::define_macros!({ eprint, eprintln, dbg }, concat, extern "C" fn print(_: *const u8, _: usize));
custom_print::define_init_panic_hook!(concat, extern "C" fn print(_: *const u8, _: usize));

extern "C" {
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
  //fn _register_probe(name: *const u8, name_len: usize, probe: extern "C" fn(u32, u32) -> u64);
}

use std::ffi::CString;

/*fn register_probe(name: &str, probe: extern "C" fn(u32, u32) -> u64) {
  let name_cstr = CString::new(name).unwrap();
  unsafe {
    _register_probe(name_cstr.as_ptr() as *const u8, name.len(), probe);
  }
}*/

fn peek_mem32(address: u32) -> u32 {
  unsafe { _peek_mem32(address) }
}

static GTA_SA_PLANE_IDS: &'static [u16] = &[460, 464, 476, 511, 512, 513, 519, 520, 539, 553, 577, 592, 593];
static GTA_SA_HELI_IDS:  &'static [u16] = &[417, 425, 447, 465, 469, 487, 488, 497, 501, 548, 563];

//TODO: should find the actual plane/heli bit for hydra (vtol)
extern "C" fn gta_sa_probe(screen_width: u32, screen_height: u32) -> u64 {
  //TODO: different proportions
  let visible_hud = unsafe {
    _test_screen(0,
      screen_width as f32 * 0.782, screen_height as f32 * 0.185,
      screen_width as f32 * 0.798, screen_height as f32 * 0.212,
      108.0, 110.0,
      0.0, 1.0,
      0.0, 1.0,
      0.8,
      1.0)
  };
  println!("visible_hud: {}", visible_hud);
  if visible_hud {
    let p = peek_mem32(0x00C0FEE0);
    println!("0x00C0FEE0: {:x}", p);
    if p != 0 {
      let vehicle_id   = (peek_mem32(p + 0x20) >> 16) as u16;
      let vehicle_type = match () {
        _ if GTA_SA_PLANE_IDS.iter().any(|id| *id == vehicle_id) => "plane",
        _ if GTA_SA_HELI_IDS .iter().any(|id| *id == vehicle_id) => "heli",
        _ => "car"
      };
      println!("vehicle_id: {} [{}]", vehicle_id, vehicle_type);
      match () {
        _ if GTA_SA_PLANE_IDS.iter().any(|id| *id == vehicle_id) => 4,
        _ if GTA_SA_HELI_IDS .iter().any(|id| *id == vehicle_id) => 3,
        _ => 2
      }
    } else {
      1
    }
  } else {
    0
  }
}

type ProbeFun = extern "C" fn(u32, u32) -> u64;

#[no_mangle]
pub extern "C" fn init() -> *const ProbeFun {

  if true {
    return gta_sa_probe as *const ProbeFun;
  }

  std::ptr::null::<ProbeFun>()
}
