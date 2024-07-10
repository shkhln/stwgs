use std::ffi::CStr;
use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::OVERLAY_STATE;

lazy_static! {
  pub static ref WASM: Mutex<Option<WasmState>> = Mutex::new(
    std::env::var("STWGS_OVERLAY_WASM_MODULE").ok().map(|module_path| WasmState::from_file(&module_path)));

  pub static ref REGISTERED_PROBES: Mutex<Vec<Probe>>    = Mutex::new(Vec::new());
  pub static ref ACTIVE_PROBE_IDX:  Mutex<Option<usize>> = Mutex::new(None);

  pub static ref WASM_RELOAD_THREAD: Mutex<()> = Mutex::new({
    if let Some(module_path) = std::env::var("STWGS_OVERLAY_WASM_MODULE").ok() {
      let mut old_timestamp = std::fs::metadata(&module_path).unwrap().modified().unwrap();
      std::thread::spawn(move || {
        loop {
          std::thread::sleep(std::time::Duration::from_secs(1));
          let cur_timestamp = std::fs::metadata(&module_path).unwrap().modified().unwrap();
          if cur_timestamp != old_timestamp {
            {
              let mut probes     = REGISTERED_PROBES.lock().unwrap();
              probes.clear();
              let mut active_idx = ACTIVE_PROBE_IDX.lock().unwrap();
              *active_idx = None;
            }

            {
              let mut wasm_state = WASM.lock().unwrap();
              *wasm_state = Some(WasmState::from_file(&module_path));
            }

            let mut overlay = OVERLAY_STATE.lock().unwrap();
            overlay.screen_scraping_targets.clear();
            overlay.probe_initialized = false;
            overlay.probe_flag_names_sent = false;

            old_timestamp = cur_timestamp;
          }
        }
      });
    }
  });
}

pub struct WasmState {
  pub engine:   wasmtime::Engine,
  pub store:    wasmtime::Store<()>,
  pub module:   wasmtime::Module,
  pub instance: wasmtime::Instance
}

pub struct Probe {
  pub name:       String,
  pub executable: String,
  pub flag_names: Vec<String>,
  pub flags:      u64,
  pub init:       wasmtime::TypedFunc<(u32, u32), i32>,
  pub probe:      wasmtime::TypedFunc<(), u64>
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
        eprint!("{}", str);
      } else {
        todo!();
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

    linker.func_wrap("env", "_add_screen_target", |x1: f32, y1: f32, x2: f32, y2: f32| {
      eprintln!("_add_screen_target: {}, {}, {}, {}", x1, y1, x2, y2);

      let mut overlay = OVERLAY_STATE.lock().unwrap();
      let area = overlay_ipc::ScreenScrapingArea {
        bounds:  overlay_ipc::Rect {
          min: overlay_ipc::Point { x: overlay_ipc::Length::px(x1), y: overlay_ipc::Length::px(y1) },
          max: overlay_ipc::Point { x: overlay_ipc::Length::px(x2), y: overlay_ipc::Length::px(y2) }
        },
        min_hue:   0.0,
        max_hue: 360.0,
        min_sat:   0.0,
        max_sat:   1.0,
        min_val:   0.0,
        max_val:   1.0
      };

      overlay.screen_scraping_targets.push(
        (area, overlay_ipc::ScreenScrapingResult { pixels_in_range: 0.0, uniformity_score: 0.0 }));
      (overlay.screen_scraping_targets.len() - 1) as u32
    }).unwrap();

    linker.func_wrap("env", "_set_screen_target_option", |
      mut caller: wasmtime::Caller<'_, ()>, idx: u32, option_ptr: i32, value_ptr: i32
    | {
      if let Some(wasmtime::Extern::Memory(mut memory)) = caller.get_export("memory") {

        let option = read_string_until_nul(&mut memory, &caller, option_ptr, 1000 /* ? */).unwrap();
        let value  = read_string_until_nul(&mut memory, &caller, value_ptr,  1000 /* ? */).unwrap();

        let mut overlay = OVERLAY_STATE.lock().unwrap();
        if let Some(target) = overlay.screen_scraping_targets.get_mut(idx as usize) {
          match option.as_str() {
            "min_hue" => target.0.min_hue = value.parse().unwrap(),
            "max_hue" => target.0.max_hue = value.parse().unwrap(),
            "min_sat" => target.0.min_sat = value.parse().unwrap(),
            "max_sat" => target.0.max_sat = value.parse().unwrap(),
            "min_val" => target.0.min_val = value.parse().unwrap(),
            "max_val" => target.0.max_val = value.parse().unwrap(),
            _ => todo!()
          };
        } else {
          todo!();
        }
      } else {
        todo!();
      }
    }).unwrap();

    linker.func_wrap("env", "_test_screen_target", |idx: u32, threshold1: f32, threshold2: f32| {
      let mut overlay = OVERLAY_STATE.lock().unwrap();
      if let Some((_, result)) = overlay.screen_scraping_targets.get(idx as usize) {
        if result.pixels_in_range >= threshold1 || result.uniformity_score >= threshold2 { 1 } else { 0 }
      } else {
        todo!()
      }
    }).unwrap();

    linker.func_wrap("env", "_register_probe", |
      mut caller: wasmtime::Caller<'_, ()>,
      name_ptr:       i32,
      exe_ptr:        i32,
      flag_names_ptr: i32,
      flag_names_len: i32,
      init_idx:       u32,
      probe_idx:      u32
    | {
      eprintln!("_register_probe: {}, {}, {}, {}, {}, {}", name_ptr, exe_ptr, flag_names_ptr, flag_names_len, init_idx, probe_idx);

      let mut probes = REGISTERED_PROBES.lock().unwrap();

      if let Some(wasmtime::Extern::Memory(mut memory)) = caller.get_export("memory") {

        let name       = read_string_until_nul(&mut memory, &caller, name_ptr, 1000 /* ? */).unwrap();
        let executable = read_string_until_nul(&mut memory, &caller, exe_ptr,  1000 /* ? */).unwrap();

        let flag_names = &memory.data(&caller)[(flag_names_ptr as usize)..(flag_names_ptr as usize + 4 * flag_names_len as usize)];
        let flag_names = bytemuck::cast_slice::<u8, i32>(flag_names).iter().map(|s|
          read_string_until_nul(&mut memory, &caller, *s, 1000 /* ? */).unwrap()).collect::<Vec<_>>();

        let table      = caller.get_export("__indirect_function_table")
          .unwrap()
          .into_table()
          .unwrap();
        let init_fun   = table.get(&mut caller, init_idx)
          .unwrap()
          .unwrap_func()
          .unwrap()
          .typed::<(u32, u32), i32>(&caller)
          .unwrap();
        let probe_fun  = table.get(&mut caller, probe_idx)
          .unwrap()
          .unwrap_func()
          .unwrap()
          .typed::<(), u64>(&caller)
          .unwrap();

        eprintln!("_register_probe: {:?}, {:?}, {:?}", name, executable, flag_names);
        probes.push(Probe { name, executable, flag_names, flags: 0, init: init_fun, probe: probe_fun });
      } else {
        todo!();
      }
    }).unwrap();

    let mut store = wasmtime::Store::new(&engine, ());

    let instance = linker.instantiate(&mut store, &module).unwrap();

    instance.get_func(&mut store, "init")
      .unwrap()
      .typed::<(), ()>(&mut store)
      .unwrap()
      .call(&mut store, ()).unwrap();

    Self {
      engine,
      store,
      module,
      instance
    }
  }

  //TODO: match executable name
  pub fn run_probe_init_fun(&mut self, screen_width: u32, screen_height: u32) {
    let probes = REGISTERED_PROBES.lock().unwrap();
    if let Some(idx) = probes.iter().position(|p| p.init.call(&mut self.store, (screen_width, screen_height)).unwrap() == 1) {
      eprintln!("Selected probe: {}", probes[idx].name);
      let mut active_idx = ACTIVE_PROBE_IDX.lock().unwrap();
      *active_idx = Some(idx);
    }
  }

  pub fn run_probe_fun(&mut self) -> Option<u64> {
    let active_idx = ACTIVE_PROBE_IDX.lock().unwrap();
    if let Some(idx) = *active_idx {
      let mut probes = REGISTERED_PROBES.lock().unwrap();
      let flags = probes[idx].probe.call(&mut self.store, ()).unwrap();
      probes[idx].flags = flags;
      Some(flags)
    } else {
      None
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
