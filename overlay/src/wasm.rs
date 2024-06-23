use std::ffi::CStr;
use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::OVERLAY_STATE;

lazy_static! {
  pub static ref WASM: Mutex<Option<WasmState>> = Mutex::new(
    std::env::var("STWGS_OVERLAY_WASM_MODULE").ok().map(|module_path| WasmState::from_file(&module_path)));

  pub static ref REGISTERED_PROBES: Mutex<Vec<Probe>>    = Mutex::new(Vec::new());
  pub static ref ACTIVE_PROBE_IDX:  Mutex<Option<usize>> = Mutex::new(None);
}

pub struct WasmState {
  pub engine:   wasmtime::Engine,
  pub store:    wasmtime::Store<()>,
  pub module:   wasmtime::Module,
  pub instance: wasmtime::Instance
}

pub struct Probe {
  name:   String,
  layers: Vec<String>,
  test:   wasmtime::TypedFunc<(), i32>,
  probe:  wasmtime::TypedFunc<(u32, u32), u64>
}

impl WasmState {

  pub fn from_file(module_path: &str) -> Self {

    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::from_file(&engine, module_path).unwrap();

    let mut linker = wasmtime::Linker::new(&engine);

    fn read_string(memory: &mut wasmtime::Memory, store: &impl wasmtime::AsContext, ptr: i32, len: i32) -> Result<String, ()> {
      let mut buf = Vec::<u8>::with_capacity(len.clone() as usize);
      buf.resize(len as usize, 0); // ?
      memory.read(store, ptr as usize, buf.as_mut_slice()).map_err(|_| ())?;
      Ok(String::from_utf8(buf).map_err(|_| ())?)
    }

    fn read_string_until_nul(memory: &mut wasmtime::Memory, store: &impl wasmtime::AsContext, ptr: i32, max_len: i32) -> Result<String, ()> {
      for i in 0..max_len /* ? */ {
        if memory.data(store)[(ptr + i) as usize] == 0 {
          return read_string(memory, store, ptr as i32, i as i32);
        }
      }
      Err(())
    }

    linker.func_wrap("env", "print", |mut caller: wasmtime::Caller<'_, ()>, ptr: i32, len: i32| {
      if let Some(wasmtime::Extern::Memory(mut memory)) = caller.get_export("memory") {
        let str = read_string(&mut memory, &caller, ptr, len).unwrap();
        print!("{}", str);
      } else {
        panic!();
      }
    }).unwrap();

    linker.func_wrap("env", "_peek_mem32", |address: u32| {

      /*if address % 4 != 0 {
        return 0; // not aligned
      }*/

      /*{
        let vmmap = VM_MAP.lock().unwrap();
        let address = address as u64;
        for entry in vmmap.iter() {
          if entry.start > 0xFFFFFFFF {
            break;
          }
          if entry.start <= address && address < entry.end {
            return if entry.prot & libc::PROT_READ != 0 {
              unsafe { *(address as *const u32) }
            } else {
              0
            };
          }
        }
      }

      //TODO: don't update this more than once per frame
      populate_vmmap();
      return 0;*/

      if address != 0 {
        unsafe { *(address as *const u32) }
      } else {
        0
      }
    }).unwrap();

    linker.func_wrap("env", "_test_screen", |id: u32, x1: f32, y1: f32, x2: f32, y2: f32,
      min_hue: f32, max_hue: f32, min_sat: f32, max_sat: f32, min_val: f32, max_val: f32, threshold1: f32, threshold2: f32| {

      let mut overlay = OVERLAY_STATE.lock().unwrap();

      if let Some((_, result)) = overlay.screen_scraping_targets2.get(&id) {
        return if result.pixels_in_range >= threshold1 || result.uniformity_score >= threshold2 { 1 } else { 0 };
      }

      let area = overlay_ipc::ScreenScrapingArea {
        bounds:  overlay_ipc::Rect {
          min: overlay_ipc::Point { x: overlay_ipc::Length::px(x1), y: overlay_ipc::Length::px(y1) },
          max: overlay_ipc::Point { x: overlay_ipc::Length::px(x2), y: overlay_ipc::Length::px(y2) }
        },
        min_hue,
        max_hue,
        min_sat,
        max_sat,
        min_val,
        max_val
      };

      overlay.screen_scraping_targets2.insert(id, (area, overlay_ipc::ScreenScrapingResult { pixels_in_range: 0.0, uniformity_score: 0.0 }));

      0
    }).unwrap();

    linker.func_wrap("env", "_register_probe", |
      mut caller: wasmtime::Caller<'_, ()>,
      name_ptr:   i32,
      layers_ptr: i32,
      layers_len: i32,
      test_idx:   u32,
      probe_idx:  u32
    | {
      eprintln!("_register_probe: {}, {}, {}, {}, {}", name_ptr, layers_ptr, layers_len, test_idx, probe_idx);

      let mut probes = REGISTERED_PROBES.lock().unwrap();

      if let Some(wasmtime::Extern::Memory(mut memory)) = caller.get_export("memory") {

        let name      = read_string_until_nul(&mut memory, &caller, name_ptr, 1000 /* ? */).unwrap();

        let layers    = &memory.data(&caller)[(layers_ptr as usize)..(layers_ptr as usize + 4 * layers_len as usize)];
        let layers    = bytemuck::cast_slice::<u8, i32>(layers).iter().map(|s|
          read_string_until_nul(&mut memory, &caller, *s, 1000 /* ? */).unwrap()).collect::<Vec<_>>();

        let table     = caller.get_export("__indirect_function_table")
          .unwrap()
          .into_table()
          .unwrap();
        let test_fun  = table.get(&mut caller, test_idx)
          .unwrap()
          .unwrap_func()
          .unwrap()
          .typed::<(), i32>(&caller)
          .unwrap();
        let probe_fun = table.get(&mut caller, probe_idx)
          .unwrap()
          .unwrap_func()
          .unwrap()
          .typed::<(u32, u32), u64>(&caller)
          .unwrap();

        eprintln!("_register_probe: {}, {:?}", name, layers);
        probes.push(Probe { name, layers, test: test_fun, probe: probe_fun });
      } else {
        panic!();
      }
    }).unwrap();

    let mut store = wasmtime::Store::new(&engine, ());

    let instance = linker.instantiate(&mut store, &module).unwrap();

    instance.get_func(&mut store, "init")
      .unwrap()
      .typed::<(), ()>(&mut store)
      .unwrap()
      .call(&mut store, ()).unwrap();

    //TODO: select probe

    let probes = REGISTERED_PROBES.lock().unwrap();

    if let Some(idx) = probes.iter().position(|p| p.test.call(&mut store, ()).unwrap() == 1) {
      eprintln!("Selected probe: {}", probes[idx].name);
      let mut active_idx = ACTIVE_PROBE_IDX.lock().unwrap();
      *active_idx = Some(idx);
    }

    Self {
      engine,
      store,
      module,
      instance
    }
  }

  pub fn run_probe(&mut self, screen_width: u32, screen_height: u32) {
    let probes = REGISTERED_PROBES.lock().unwrap();
    let active_idx = ACTIVE_PROBE_IDX.lock().unwrap();
    if let Some(idx) = *active_idx {
      let res = probes[idx].probe.call(&mut self.store, (screen_width, screen_height)).unwrap();
      println!("probe value: {}", res);
    }
  }
}

#[derive(Clone, Debug)]
struct VmMapEntry {
  pub start: u64,
  pub end:   u64,
  pub prot:  i32,
  pub path:  Option<String>
}

lazy_static! {
  static ref VM_MAP: Mutex<Vec<VmMapEntry>> = Mutex::new(Vec::new());
}

#[cfg(target_os = "freebsd")]
fn populate_vmmap() {

  let mut vmmap = VM_MAP.lock().unwrap();

  unsafe {
    let procstat = libc::procstat_open_sysctl();
    assert!(!procstat.is_null());

    let mut count: u32 = 0;
    let procs = libc::procstat_getprocs(procstat, libc::KERN_PROC_PID, libc::getpid(), &mut count);
    //println!("Found {} processes ({:p})", count, procs);
    assert!(!procs.is_null());
    assert_eq!(count, 1);

    let mut count: u32 = 0;
    let entries = libc::procstat_getvmmap(procstat, procs, &mut count);
    //println!("Found {} entries ({:p})\n", count, entries);
    assert!(!entries.is_null());

    vmmap.clear();

    for i in 0..count {
      let entry = entries.offset(i as isize);
      let path = CStr::from_bytes_until_nul(
        std::slice::from_raw_parts((*entry).kve_path[0].as_ptr() as *const u8, libc::PATH_MAX as usize))
        .ok().filter(|s| !s.is_empty()).map(|s| s.to_string_lossy());
      //println!("{:x}..{:x} -> {:?}", (*entry).kve_start, (*entry).kve_end, path);

      vmmap.push(VmMapEntry {
        start: (*entry).kve_start,
        end:   (*entry).kve_end,
        prot:  (*entry).kve_protection,
        path:  path.map(|s| s.to_string())
      });
    }

    libc::procstat_freevmmap(procstat, entries);
    libc::procstat_freeprocs(procstat, procs);
    libc::procstat_close(procstat);
  };
}
