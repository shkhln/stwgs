use std::ffi::CStr;
use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::OVERLAY_STATE;

lazy_static! {
  pub static ref WASM: Mutex<Option<WasmState>> = Mutex::new(
    std::env::var("STWGS_OVERLAY_WASM_MODULE").ok().map(|module_path| WasmState::from_file(&module_path)));
}

pub struct WasmState {
  pub engine:   wasmtime::Engine,
  pub store:    wasmtime::Store<()>,
  pub module:   wasmtime::Module,
  pub instance: wasmtime::Instance,
  pub probe:    Option<wasmtime::TypedFunc<(u32, u32), u64>>
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

    linker.func_wrap("env", "print", |mut caller: wasmtime::Caller<'_, ()>, ptr: i32, len: i32| {
      if let Some(wasmtime::Extern::Memory(mut memory)) = caller.get_export("memory") {
        let str = read_string(&mut memory, &caller, ptr, len).unwrap();
        println!("{}", str);
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

    /*linker.func_wrap("env", "_register_probe", |mut caller: wasmtime::Caller<'_, ()>, name_ptr: i32, name_len: i32, probe_idx: i32| {
      if let Some(wasmtime::Extern::Memory(mut memory)) = caller.get_export("memory") {
        let mut probes = PROBES.lock().unwrap();
        let name = read_string(&mut memory, &caller, name_ptr, name_len).unwrap();

        println!("probe: {:?}", probe_idx);

        let table = caller.get_export("__indirect_function_table")
          .unwrap()
          .into_table()
          .unwrap();
        let func = table.get(&mut caller, probe_idx as u32).unwrap()
          .unwrap_func()
          .unwrap()
          .typed::<(u32, u32), u64>(&caller)
          .unwrap();

        probes.insert(name, func);

      } else {
        panic!();
      }
    }).unwrap();*/

    let mut store = wasmtime::Store::new(&engine, ());

    let instance = linker.instantiate(&mut store, &module).unwrap();

    let init = instance.get_func(&mut store, "init")
      .unwrap()
      .typed::<(), i32>(&mut store)
      .unwrap();
    let probe_idx = init.call(&mut store, ()).unwrap();

    let probe = if probe_idx != 0 {
      let table = instance.get_export(&mut store, "__indirect_function_table")
        .unwrap()
        .into_table()
        .unwrap();
      let func = table.get(&mut store, probe_idx as u32).unwrap()
        .unwrap_func()
        .unwrap()
        .typed::<(u32, u32), u64>(&mut store)
        .unwrap();

      Some(func)
    } else {
      None
    };

    Self {
      engine,
      store,
      module,
      instance,
      probe
    }
  }

  pub fn run_probe(&mut self, screen_width: u32, screen_height: u32) {
    if let Some(probe) = &self.probe {
      let res = probe.call(&mut self.store, (screen_width, screen_height)).unwrap();
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
