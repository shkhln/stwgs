use std::collections::HashMap;

use lazy_static::lazy_static;
use strum::{EnumCount, IntoEnumIterator};

use overlay_ipc::Knob;

use crate::config::Config;
use crate::controllers::{ControllerCommand, ControllerState};
use crate::output::{KeyboardKey, MapperIO, MouseAxis, MouseButton};
use crate::stages::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct LayerMask(u32);

impl std::fmt::Display for LayerMask {

  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.0, f)
  }
}

impl std::fmt::LowerHex for LayerMask {

  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::LowerHex::fmt(&self.0, f)
  }
}

impl std::ops::BitOr for LayerMask {

  type Output = LayerMask;

  fn bitor(self, rhs: LayerMask) -> Self::Output {
    LayerMask(self.0 | rhs.0)
  }
}

impl std::ops::BitAnd for LayerMask {

  type Output = LayerMask;

  fn bitand(self, rhs: LayerMask) -> Self::Output {
    LayerMask(self.0 & rhs.0)
  }
}

impl std::ops::Not for LayerMask {

  type Output = LayerMask;

  fn not(self) -> Self::Output {
    LayerMask(!self.0)
  }
}

impl LayerMask {

  pub const MAX_LAYERS:          usize = 32;
  pub const MAX_USER_LAYERS:     usize = 24;
  pub const MAX_INTERNAL_LAYERS: usize = Self::MAX_LAYERS - Self::MAX_USER_LAYERS;
  pub const EMPTY:               Self  = Self(0);
  pub const ALL_USER_BITS:       Self  = Self((1 << Self::MAX_USER_LAYERS) - 1);
  pub const ALL_INTERNAL_BITS:   Self  = Self(u32::MAX & !((1 << Self::MAX_USER_LAYERS) - 1));
  pub const ALL:                 Self  = Self(u32::MAX);

  pub fn user_layer(n: usize) -> Result<Self, ()> {
    if n <= Self::MAX_USER_LAYERS {
      Ok(Self(1 << n))
    } else {
      Err(())
    }
  }

  pub fn internal_layer(n: usize) -> Result<Self, ()> {
    if n <= Self::MAX_INTERNAL_LAYERS {
      Ok(Self(1 << (Self::MAX_USER_LAYERS + n)))
    } else {
      Err(())
    }
  }
}

lazy_static! {
  static ref EMPTY_STATE: ControllerState = ControllerState::empty();
}

pub struct Layer {
  name: String
}

pub struct Mapper<'m> {

  controller: Option<&'m std::sync::mpsc::Sender<ControllerCommand>>,
  overlay:    Option<&'m overlay_ipc::CommandSender>,
  output:     &'m mut dyn MapperIO,

  curr_layer_mask: LayerMask,
  next_layer_mask: Option<LayerMask>,

  layers:    Vec<Layer>,
  pipelines: Vec<(LayerMask, Box<dyn Pipeline<()>>, bool)>,

  actions:           Vec<Action>,
  discarded_actions: Vec<Action>,

  probes:        HashMap<StageId, Probe>,
  probe_values:  HashMap<StageId, ProbeValue>,
  //TODO: we should probably use a single receiver for all probes or at least refactor them to use a single return type
  probe_ssr_rcv:  HashMap<StageId, overlay_ipc::ipc::IpcReceiver<overlay_ipc::ScreenScrapingResult>>,
  probe_u64_rcv:  HashMap<StageId, overlay_ipc::ipc::IpcReceiver<u64>>,
  probe_bool_rcv: HashMap<StageId, overlay_ipc::ipc::IpcReceiver<bool>>,

  shapes:           HashMap<StageId, Vec<Vec<overlay_ipc::Shape>>>,
  curr_shape_state: HashMap<StageId, Vec<u64>>,
  next_shape_state: HashMap<StageId, Vec<u64>>,

  curr_keyboard_key_state: [bool; KeyboardKey::COUNT],
  next_keyboard_key_state: [bool; KeyboardKey::COUNT],
  curr_mouse_button_state: [bool; MouseButton::COUNT],
  next_mouse_button_state: [bool; MouseButton::COUNT],

  rel_mouse_x: f32,
  rel_mouse_y: f32,

  log_level: u8,

  knobs: Vec<Knob>,
  knobs_changed: bool
}

pub enum ExitReason {
  KnobsChanged(Vec<Knob>),
  OverlayRequired
}

impl<'m> Mapper<'m> {

  pub fn new(controller: Option<&'m std::sync::mpsc::Sender<ControllerCommand>>,
             overlay:    Option<&'m overlay_ipc::CommandSender>,
             config: Config,
             output:     &'m mut dyn MapperIO,
             log_level:  u8
  ) -> Self {

    let mut m = Self {
      controller,
      overlay,
      output,

      curr_layer_mask: LayerMask::user_layer(0).unwrap(),
      next_layer_mask: None,

      layers:    Vec::new(),
      pipelines: Vec::new(),

      actions:           Vec::new(),
      discarded_actions: Vec::new(),

      //TODO: extract probes into a separate object?
      probes:         HashMap::new(),
      probe_values:   HashMap::new(),
      probe_ssr_rcv:  HashMap::new(),
      probe_u64_rcv:  HashMap::new(),
      probe_bool_rcv: HashMap::new(),

      shapes:           HashMap::new(),
      curr_shape_state: HashMap::new(),
      next_shape_state: HashMap::new(),

      curr_keyboard_key_state: [false; KeyboardKey::COUNT],
      next_keyboard_key_state: [false; KeyboardKey::COUNT],
      curr_mouse_button_state: [false; MouseButton::COUNT],
      next_mouse_button_state: [false; MouseButton::COUNT],

      rel_mouse_x: 0.0,
      rel_mouse_y: 0.0,

      log_level,

      knobs: config.knobs,
      knobs_changed: false
    };

    //TODO: we should probably accept the mask number there as well
    fn register_layer(mapper: &mut Mapper, name: Option<String>) -> LayerMask {
      let i = mapper.layers.len();
      assert!(i < std::mem::size_of::<LayerMask>() * 8);

      mapper.layers.push(Layer { name: name.unwrap_or_else(|| "???".to_string()) });
      LayerMask::user_layer(i).unwrap()
    }

    for name in config.layers {
      let mask = register_layer(&mut m, Some(name.clone()));
      if log_level > 0 {
        eprintln!("layer {:?}: {}", name, mask);
      }
    }

    let mut meta = HashMap::new();

    for (mask, pipeline) in config.pipelines {
      pipeline.inspect(&mut meta);
      m.pipelines.push((mask, pipeline, false));
    }

    for (stage_id, stage_description) in meta {

      if let Some(probe) = &stage_description.probe {
        m.probes.insert(stage_id, probe.clone());
        m.probe_values.insert(stage_id, ProbeValue { u64: 0 }); // ?
      }

      let layer_count = stage_description.shapes.len();
      if layer_count > 0 {
        assert!(layer_count <= 256);
        let mut v = vec![];
        for i in 0..layer_count {
          let shape_count = stage_description.shapes[i].len();
          if shape_count <= 64 {
            v.push(stage_description.shapes[i].clone());
          } else {
            eprintln!("too many shapes ({}) in stage {} (id: {}) layer {}, truncated",
              shape_count, stage_description.name, stage_id, i);
            v.push(stage_description.shapes[i][0..=63].to_vec());
          }
        }
        m.shapes.insert(stage_id, v);
      }
    }

    m
  }

  fn init_probes(&mut self) -> bool {

    //TODO: rename init_probes to init or move overlay layer registration to some other place
    if let Some(overlay) = &self.overlay {
      overlay.send(overlay_ipc::OverlayCommand::ResetOverlay).unwrap();
    }

    for (id, probe) in &self.probes {
      match probe {

        Probe::Screen { target } => {

          let (sender, receiver) = overlay_ipc::ipc::channel().unwrap();

          if let Some(overlay) = &self.overlay {
            overlay.send(overlay_ipc::OverlayCommand::AddScreenScrapingArea(target.clone(), sender)).unwrap();
            self.probe_ssr_rcv.insert(*id, receiver);
          } else {
            eprintln!("Probe {:?} requires overlay to be present", probe);
            return false;
          }
        },

        Probe::Memory {usize, address, offsets } => {

          let (sender, receiver) = overlay_ipc::ipc::channel().unwrap();

          if let Some(overlay) = &self.overlay {
            overlay.send(overlay_ipc::OverlayCommand::AddMemoryCheck(*usize, *address, offsets.clone(), sender)).unwrap();
            self.probe_u64_rcv.insert(*id, receiver);
          } else {
            eprintln!("Probe {:?} requires overlay to be present", probe);
            return false;
          }
        },

        Probe::Overlay { name } => {

          let (sender, receiver) = overlay_ipc::ipc::channel().unwrap();

          if let Some(overlay) = &self.overlay {
            overlay.send(overlay_ipc::OverlayCommand::AddOverlayCheck(name.clone(), sender)).unwrap();
            self.probe_bool_rcv.insert(*id, receiver);
          } else {
            eprintln!("Probe {:?} requires overlay to be present", probe);
            return false;
          }
        }
      }
    }

    //TODO: rename init_probes to init or move shape registration to some other place
    if !self.shapes.is_empty() {
      if let Some(overlay) = &self.overlay {
        for (id, shapes) in &self.shapes {
          overlay.send(overlay_ipc::OverlayCommand::RegisterShapes { stage_id: *id as u64, shapes: shapes.clone() }).unwrap();
          self.curr_shape_state.insert(*id, vec![0; shapes.len()]);
          self.next_shape_state.insert(*id, vec![0; shapes.len()]);
        }
      } else {
        eprintln!("Menus require overlay to be present");
        return false;
      }
    }

    if let Some(overlay) = &self.overlay {
      overlay.send(overlay_ipc::OverlayCommand::SetLayerNames(self.layers.iter().map(|layer| layer.name.clone()).collect())).unwrap();
    }

    if let Some(overlay) = &self.overlay {
      overlay.send(overlay_ipc::OverlayCommand::RegisterKnobs(self.knobs.clone())).unwrap();
    }

    true
  }

  fn poll_probes(&mut self) {
    for (id, probe) in &self.probes {
      match probe {
        Probe::Screen { .. } => {
          if let Ok(result) = self.probe_ssr_rcv[id].try_recv() {
            self.probe_values.insert(*id, ProbeValue { ff32: (result.pixels_in_range, result.uniformity_score) });
          }
        },
        Probe::Memory { .. } => {
          if let Ok(result) = self.probe_u64_rcv[id].try_recv() {
            self.probe_values.insert(*id, ProbeValue { u64: result});
          }
        },
        Probe::Overlay { .. } => {
          if let Ok(result) = self.probe_bool_rcv[id].try_recv() {
            self.probe_values.insert(*id, ProbeValue { bool: result});
          }
        }
      }
    }
  }

  //TODO: we need to fuzz probe's on/off states instead of raw input data
  fn randomize_probe_values<R: ::rand::Rng>(&mut self, rng: &mut R) {
    for id in self.probes.keys() {
      self.probe_values.insert(*id, ProbeValue { u64: rng.gen() });
    }
  }

  fn apply_action(&mut self, i: usize) {

    match self.actions[i] {

      Action::PressKeyboardKey(key) => {
        self.next_keyboard_key_state[key as usize] = true;
      },

      Action::PressMouseButton(btn) => {
        self.next_mouse_button_state[btn as usize] = true;
      },

      Action::MoveMouse(axis, value) => {
        match axis {
          MouseAxis::X     => self.rel_mouse_x += value,
          MouseAxis::Y     => self.rel_mouse_y += value,
          MouseAxis::Wheel => if value != 0.0 {
            self.output.mouse_wheel_rel(value as i32);
          }
        };
      },

      /*Action::EnableLayers(mask) => {
        if let Some(m) = self.next_layer_mask {
          self.next_layer_mask = Some(m | mask);
        } else {
          self.next_layer_mask = Some(self.curr_layer_mask | mask);
        }
      },*/

      /*Action::DisableLayers(mask) => {
        if let Some(m) = self.next_layer_mask {
          self.next_layer_mask = Some(m & !mask);
        } else {
          self.next_layer_mask = Some(self.curr_layer_mask & !mask);
        }
      },*/

      Action::SetLayerMask(mask) => {
        self.next_layer_mask = Some(mask);
      },

      Action::ToggleShapes { stage_id, layer, mask } => {
        self.next_shape_state.get_mut(&stage_id).unwrap()[layer as usize] = mask;
      },

      Action::ToggleOverlayUI => {
        if let Some(overlay) = self.overlay {
          overlay.send(overlay_ipc::OverlayCommand::ToggleUI).unwrap();
        }
      },

      Action::HapticFeedback(target, effect) => {
        if let Some(controller) = self.controller {
          controller.send(ControllerCommand::HapticFeedback(target, effect)).unwrap();
        }
      },

      Action::SendOverlayMenuCommand(command) => {
        if let Some(overlay) = self.overlay {
          overlay.send(overlay_ipc::OverlayCommand::MenuCommand(command)).unwrap();
          if command == OverlayMenuCommand::CloseKnobsMenu {

            let (sender, receiver) = overlay_ipc::ipc::channel().unwrap();
            overlay.send(overlay_ipc::OverlayCommand::GetKnobs(sender)).unwrap();

            let knobs = receiver.recv().unwrap(); // it's ok to block here
            assert_eq!(self.knobs.len(), knobs.len());

            #[allow(clippy::needless_range_loop)]
            for i in 0..self.knobs.len() {
              if !self.knobs[i].compare_value(&knobs[i]) {
                self.knobs_changed = true;
                break;
              }
            }

            if self.knobs_changed {
              self.knobs = knobs;
            }
          }
        }
      }
    }
  }

  fn apply_actions(&mut self, state: &ControllerState, now: Timestamp) {

    self.actions.clear();
    self.discarded_actions.clear();

    for key in KeyboardKey::iter() {
      self.next_keyboard_key_state[key as usize] = false;
    }

    for btn in MouseButton::iter() {
      self.next_mouse_button_state[btn as usize] = false;
    }

    for (_, masks) in self.next_shape_state.iter_mut() {
      for mask in masks {
        *mask = 0;
      }
    }

    for &mut (mask, ref mut pipeline, ref mut should_apply_empty_state) in &mut self.pipelines {
      if *should_apply_empty_state {
        assert_eq!(mask & self.curr_layer_mask, LayerMask::EMPTY);
        pipeline.reset();
      };
    }

    for &mut (mask, ref mut pipeline, ref mut should_apply_empty_state) in &mut self.pipelines {
      if *should_apply_empty_state {
        assert_eq!(mask & self.curr_layer_mask, LayerMask::EMPTY);
        let ctx = Context { state: &EMPTY_STATE, time: now, layers: self.curr_layer_mask, probe_values: &self.probe_values };
        pipeline.apply(&ctx, &mut self.discarded_actions);
        *should_apply_empty_state = false;
      }
    }

    /*if self.discarded_actions.len() > 0 {
      eprintln!("discarded actions: {:?}", self.discarded_actions);
    }*/

    for &mut (mask, ref mut pipeline, _) in &mut self.pipelines {
      if mask & self.curr_layer_mask != LayerMask::EMPTY {
        pipeline.reset();
      }
    }

    for &mut (mask, ref mut pipeline, _) in &mut self.pipelines {
      if mask & self.curr_layer_mask != LayerMask::EMPTY {
        let ctx = Context { state, time: now, layers: self.curr_layer_mask, probe_values: &self.probe_values };
        pipeline.apply(&ctx, &mut self.actions);
      }
    }

    for i in 0..self.actions.len() {
      self.apply_action(i);
    }

    if let Some(next_mask) = self.next_layer_mask {

      if let Some(overlay) = self.overlay {
        overlay.send(overlay_ipc::OverlayCommand::SetMode(next_mask.0 as u64)).unwrap();
      }

      if self.log_level > 0 {
        eprintln!("switch to mode: {}", next_mask);
      }

      //TODO: should probably think of something more intelligent for resetting double press timers, etc.
      for &mut (mask, _, ref mut should_apply_empty_state) in &mut self.pipelines {
        if mask & self.curr_layer_mask != LayerMask::EMPTY && mask & next_mask == LayerMask::EMPTY {
          *should_apply_empty_state = true;
        }
      }

      self.curr_layer_mask = next_mask;
      self.next_layer_mask = None;
    }

    for key in KeyboardKey::iter() {
      match (self.curr_keyboard_key_state[key as usize], self.next_keyboard_key_state[key as usize]) {
        (false, true) => self.output.keyboard_key_down(key),
        (true, false) => self.output.keyboard_key_up(key),
        _ => ()
      }
      self.curr_keyboard_key_state[key as usize] = self.next_keyboard_key_state[key as usize];
    }

    for btn in MouseButton::iter() {
      match (self.curr_mouse_button_state[btn as usize], self.next_mouse_button_state[btn as usize]) {
        (false, true) => self.output.mouse_button_down(btn),
        (true, false) => self.output.mouse_button_up(btn),
        _ => ()
      }
      self.curr_mouse_button_state[btn as usize] = self.next_mouse_button_state[btn as usize];
    }

    let x = self.rel_mouse_x.trunc();
    let y = self.rel_mouse_y.trunc();
    if x != 0.0 || y != 0.0 {
      self.output.mouse_cursor_rel_xy(x as i32, y as i32);
      self.rel_mouse_x -= x;
      self.rel_mouse_y -= y;
    }

    self.output.syn();

    if let Some(overlay) = self.overlay {
      for (stage_id, masks) in self.next_shape_state.iter() {
        #[allow(clippy::needless_range_loop)]
        for i in 0..masks.len() {
          if masks[i] != self.curr_shape_state[stage_id][i] {
            overlay.send(overlay_ipc::OverlayCommand::ToggleShapes { stage_id: *stage_id as u64, layer: i as u8, mask: masks[i] }).unwrap();
            self.curr_shape_state.get_mut(stage_id).unwrap()[i] = masks[i];
          }
        }
      }
    }
  }

  // TODO: Sender<ControllerCommand> vs Receiver<ControllerState> set up
  #[cfg(not(test))]
  pub fn run(&mut self, controller_state_receiver: &'m std::sync::mpsc::Receiver<ControllerState>) -> Result<ExitReason, String> {

    if !self.init_probes() {
      return Ok(ExitReason::OverlayRequired);
    }

    // make sure LOGO (GUIDE) button is not pressed
    loop {
      let state = controller_state_receiver.recv().map_err(|e| format!("{}", e))?;
      if !state.buttons.steam {
        break;
      }
    }

    loop {
      let state = controller_state_receiver.recv().map_err(|e| format!("{}", e))?;
      let now   = std::time::Instant::now(); //TODO: put timestamp into ControllerState
      self.apply_actions(&state, Timestamp(now));
      self.poll_probes();

      if self.knobs_changed {
        return Ok(ExitReason::KnobsChanged(self.knobs.clone()));
      }
    }
  }

  #[cfg(not(test))]
  pub fn fuzz(&mut self, max_iterations: usize) {
    let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed([42; 32]);
    for i in 1..max_iterations {
      let state = ControllerState::random(&mut rng);
      self.randomize_probe_values(&mut rng);
      self.apply_actions(&state, Timestamp(std::time::Instant::now()));
      if i % 10_000 == 0 {
        eprint!("*"); // ?
      }
    }
    eprintln!();
  }
}

#[cfg(test)]
mod tests {

  use std::cell::Cell;

  use crate::controllers::Button;
  use crate::output::KeyboardKey;
  use crate::stages::*;
  use super::*;

  //TODO: rename to OutputRecorder or so
  pub struct DummyOutput2 {
    keys: Cell<Vec<(bool, KeyboardKey)>>
  }

  impl MapperIO for DummyOutput2 {

    fn keyboard_key_down(&mut self, key: KeyboardKey) {
      //println!("down {:?}", key);
      let mut keys = self.keys.take();
      keys.push((true, key));
      self.keys.replace(keys);
    }

    fn keyboard_key_up(&mut self, key: KeyboardKey) {
      //println!("up {:?}", key);
      let mut keys = self.keys.take();
      keys.push((false, key));
      self.keys.replace(keys);
    }

    fn mouse_button_down(&mut self, _btn: MouseButton) {}
    fn mouse_button_up(&mut self, _btn: MouseButton) {}
    fn mouse_cursor_rel_xy(&mut self, _: i32, _: i32) {}
    fn mouse_wheel_rel(&mut self, _: i32) {}
    fn syn(&mut self) {}
  }

  fn config(pipelines: Vec<(LayerMask, Box<dyn Pipeline<()>>)>) -> Config {
    Config { pipelines, layers: vec![], knobs: vec![] }
  }

  /*#[test]
  fn layer_switching_test() {

    let config = config(vec![
      (LayerMask(0b01), switch_mode(button_input(Button::X), LayerMask(0b10))),
      (LayerMask(0b10), switch_mode(button_input(Button::X), LayerMask(0b01)))
    ]);

    let mut mapper = Mapper::new(None, None, config, &crate::output::DummyOutput, 0);
    let mut state  = crate::controllers::ControllerState::empty();

    assert_eq!(mapper.curr_layer_mask, LayerMask(0b01));

    state.buttons.x = true;
    mapper.apply_actions(&state, Timestamp(1));
    assert_eq!(mapper.curr_layer_mask, LayerMask(0b10));

    mapper.apply_actions(&state, Timestamp(2));
    assert_eq!(mapper.curr_layer_mask, LayerMask(0b10));

    state.buttons.x = false;
    mapper.apply_actions(&state, Timestamp(3));
    assert_eq!(mapper.curr_layer_mask, LayerMask(0b10));

    mapper.apply_actions(&state, Timestamp(4));
    assert_eq!(mapper.curr_layer_mask, LayerMask(0b10));

    state.buttons.x = true;
    mapper.apply_actions(&state, Timestamp(5));
    assert_eq!(mapper.curr_layer_mask, LayerMask(0b01));

    mapper.apply_actions(&state, Timestamp(6));
    assert_eq!(mapper.curr_layer_mask, LayerMask(0b01));
  }*/

  #[test]
  fn layer_disengagement_test() {

    let config = config(vec![
      (LayerMask(0b01), keyboard_key_press(button_input(Button::A), KeyboardKey::A)),
      (LayerMask(0b10), keyboard_key_press(button_input(Button::B), KeyboardKey::B)),
      (LayerMask(0b11), switch_mode(button_input(Button::X), LayerMask(0b10)))
    ]);

    let mut output = DummyOutput2 { keys: Cell::new(vec![]) };
    let mut mapper = Mapper::new(None, None, config, &mut output, 0);
    let mut state  = crate::controllers::ControllerState::empty();

    state.buttons.a = true;
    mapper.apply_actions(&state, Timestamp(0));

    state.buttons.x = true;
    mapper.apply_actions(&state, Timestamp(1));

    state.buttons.b = true;
    mapper.apply_actions(&state, Timestamp(2));

    assert_eq!(output.keys.take(), vec![(true, KeyboardKey::A), (false, KeyboardKey::A), (true, KeyboardKey::B)])
  }

  #[test]
  fn layer_disengagement_test_2() {

    let config = config(vec![
      (LayerMask(0b01), keyboard_key_press(dummy_button_input(true), KeyboardKey::A)),
      (LayerMask(0b10), keyboard_key_press(button_input(Button::B),  KeyboardKey::B)),
      (LayerMask(0b11), switch_mode(button_input(Button::X), LayerMask(0b10)))
    ]);

    let mut output = DummyOutput2 { keys: Cell::new(vec![]) };
    let mut mapper = Mapper::new(None, None, config, &mut output, 0);
    let mut state  = crate::controllers::ControllerState::empty();

    mapper.apply_actions(&state, Timestamp(0));

    state.buttons.x = true;
    mapper.apply_actions(&state, Timestamp(1));

    state.buttons.b = true;
    mapper.apply_actions(&state, Timestamp(2));

    assert_eq!(output.keys.take(), vec![(true, KeyboardKey::A), (false, KeyboardKey::A), (true, KeyboardKey::B)])
  }
}
