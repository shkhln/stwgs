use std::collections::HashMap;
use std::time::Duration;

pub use overlay_ipc::OverlayMenuCommand;

use crate::controllers::{Axis, Button, HapticFeedbackEffect, HapticFeedbackTarget};
use crate::mapper::LayerMask;
use crate::output::{KeyboardKey, MouseAxis, MouseButton};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Action {
  PressKeyboardKey(KeyboardKey),
  PressMouseButton(MouseButton),
  MoveMouse(MouseAxis, f32),
  //EnableLayers(LayerMask),
  //DisableLayers(LayerMask),
  SetLayerMask(LayerMask),
  ToggleShapes { stage_id: StageId, layer: u8, mask: u64 },
  ToggleOverlayUI,
  HapticFeedback(HapticFeedbackTarget, HapticFeedbackEffect),
  SendOverlayMenuCommand(OverlayMenuCommand)
}

#[cfg(not(test))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub std::time::Instant);

#[cfg(test)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub u64);

impl std::ops::Sub<Timestamp> for Timestamp {

  type Output = Duration;

  #[cfg(not(test))]
  fn sub(self, other: Timestamp) -> Duration {
    let Timestamp(instant1) = self;
    let Timestamp(instant2) = other;
    instant1 - instant2
  }

  #[cfg(test)]
  fn sub(self, other: Timestamp) -> Duration {
    let Timestamp(t1) = self;
    let Timestamp(t2) = other;
    Duration::from_millis(t1 - t2)
  }
}

pub type ControllerState<'a> = &'a crate::controllers::ControllerState;

pub type PipelineRef<R> = std::rc::Rc<std::cell::RefCell<dyn Pipeline<R>>>;

/*#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Direction {
  Up, Left, Down, Right
}*/

pub type StageId = usize;

use lazy_static::lazy_static;

lazy_static! {
  static ref STAGE_ID_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
}

pub fn generate_stage_id() -> StageId {
  STAGE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

#[repr(C)]
pub union ProbeValue {
  pub u64:  u64,
  pub f64:  f64,
  pub ff32: (f32, f32)
}

pub struct Context<'a> {
  pub state:        ControllerState<'a>,
  pub time:         Timestamp, //TODO: replace with tick duration?
  pub layers:       LayerMask,
  pub probe_values: &'a HashMap<StageId, ProbeValue>
}

pub trait Pipeline<R: Copy> {
  fn stage_id(&self) -> StageId;
  fn name(&self)     -> &'static str;
  fn desc(&self)     -> String;

  fn opts(&self) -> String {
    "".to_string()
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![]
  }

  fn probe(&self) -> Option<Probe> {
    None
  }

  fn shapes(&self) -> Vec<Vec<overlay_ipc::Shape>> {
    vec![]
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>);
  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) -> R;
  fn reset(&mut self); //TODO: rename to clear_memoization?
}

#[derive(Clone, Debug)]
pub enum Probe {
  Screen { target: overlay_ipc::ScreenScrapingArea },
  Memory { usize: u8, address: u64, offsets: Vec<i32> }
}

#[derive(Clone, Debug)]
pub struct PipelineStageDescription {
  pub id:     StageId,
  pub name:   &'static str,
  pub opts:   String,
  pub inputs: Vec<StageId>, //TODO: named inputs?
  pub probe:  Option<Probe>,
  pub shapes: Vec<Vec<overlay_ipc::Shape>>
}

fn insert_stage_description<T: Copy + 'static>(out: &mut HashMap<StageId, PipelineStageDescription>, p: &dyn Pipeline<T>) -> bool {
  if let std::collections::hash_map::Entry::Vacant(e) = out.entry(p.stage_id()) {
    e.insert(PipelineStageDescription {
      id:     p.stage_id(),
      name:   p.name(),
      opts:   p.opts(),
      inputs: p.inputs(),
      probe:  p.probe(),
      shapes: p.shapes()
    });
    true
  } else {
    false
  }
}

pub struct FnStage<T: 'static, R: 'static> {
  stage_id: StageId,
  name:     &'static str,
  args:     String,
  pipeline: PipelineRef<T>,
  fun:      Box<dyn FnMut(T, Timestamp, LayerMask, &mut Vec<Action>) -> R>,
  out:      Option<R>
}

impl<T: Copy + 'static, R: Copy + 'static> FnStage<T, R> {

  pub fn from(name: &'static str, args: String, pipeline: PipelineRef<T>, fun: Box<dyn FnMut(T, Timestamp, LayerMask, &mut Vec<Action>) -> R>) -> Self {
    FnStage { stage_id: generate_stage_id(), name, args, pipeline, fun, out: None }
  }
}

impl<T: Copy + 'static, R: Copy + 'static> Pipeline<R> for FnStage<T, R> {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    self.name
  }

  fn desc(&self) -> String {
    format!("{} -> {}({})", self.pipeline.borrow().desc(), self.name, self.args)
  }

  fn opts(&self) -> String {
    self.args.clone()
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![self.pipeline.borrow().stage_id()]
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    if insert_stage_description(out, self) {
      self.pipeline.borrow().inspect(out);
    }
  }

  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) -> R {
    if self.out.is_none() {
      self.out = Some((self.fun)(self.pipeline.borrow_mut().apply(ctx, actions), ctx.time, ctx.layers, actions));
    }
    self.out.unwrap()
  }

  fn reset(&mut self) {
    if self.out.is_some() {
      self.pipeline.borrow_mut().reset();
      self.out = None;
    }
  }
}

/*impl<R: Copy + 'static, RR: Copy + 'static> std::fmt::Display for FnStage<R, RR> {

  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{} -> {}", self.pipeline.borrow(), self.desc)
  }
}*/

pub struct BiFnStage<T: 'static, U: 'static, R: 'static> {
  stage_id:  StageId,
  name:      &'static str,
  args:      String,
  pipeline1: PipelineRef<T>,
  pipeline2: PipelineRef<U>,
  fun:       Box<dyn FnMut(T, U, Timestamp, LayerMask, &mut Vec<Action>) -> R>,
  out:       Option<R>
}

impl<T: Copy + 'static, U: Copy + 'static, R: Copy + 'static> BiFnStage<T, U, R> {

  pub fn from(name: &'static str, args: String, pipeline1: PipelineRef<T>, pipeline2: PipelineRef<U>, fun: Box<dyn FnMut(T, U, Timestamp, LayerMask, &mut Vec<Action>) -> R>) -> Self {
    BiFnStage { stage_id: generate_stage_id(), name, args, pipeline1, pipeline2, fun, out: None }
  }
}

impl<T: Copy + 'static, U: Copy + 'static, R: Copy + 'static> Pipeline<R> for BiFnStage<T, U, R> {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    self.name
  }

  fn desc(&self) -> String {
    if !self.args.is_empty() {
      format!("{} -> {}({}, {})", self.pipeline1.borrow().desc(), self.name, self.pipeline2.borrow().desc(), self.args)
    } else {
      format!("{} -> {}({})", self.pipeline1.borrow().desc(), self.name, self.pipeline2.borrow().desc())
    }
  }

  fn opts(&self) -> String {
    self.args.clone()
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![self.pipeline1.borrow().stage_id(), self.pipeline2.borrow().stage_id()]
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    if insert_stage_description(out, self) {
      self.pipeline1.borrow().inspect(out);
      self.pipeline2.borrow().inspect(out);
    }
  }

  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) -> R {
    if self.out.is_none() {
      let v1 = self.pipeline1.borrow_mut().apply(ctx, actions);
      let v2 = self.pipeline2.borrow_mut().apply(ctx, actions);
      self.out = Some((self.fun)(v1, v2, ctx.time, ctx.layers, actions));
    }
    self.out.unwrap()
  }

  fn reset(&mut self) {
    if self.out.is_some() {
      self.pipeline1.borrow_mut().reset();
      self.pipeline2.borrow_mut().reset();
      self.out = None;
    }
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ButtonState {
  Pressed,
  Repeat,
  Released,
  NoInput
}

pub fn to_button_state() -> Box<dyn FnMut(bool) -> ButtonState> {
  let mut bstate = ButtonState::NoInput;

  Box::new(move |pressed| {
    bstate = match bstate {
      ButtonState::Pressed  => if pressed { ButtonState::Repeat  } else { ButtonState::Released },
      ButtonState::Repeat   => if pressed { ButtonState::Repeat  } else { ButtonState::Released },
      ButtonState::Released => if pressed { ButtonState::Pressed } else { ButtonState::NoInput  },
      ButtonState::NoInput  => if pressed { ButtonState::Pressed } else { ButtonState::NoInput  }
    };

    bstate
  })
}

/*pub fn extend(pipeline: PipelineRef<bool>, duration: Duration) -> PipelineRef<bool> {

  let mut pressed_at = None;
  let mut bstate = to_button_state();

  let fun = Box::new(move |pressed, now, _: &mut Vec<Action>| {
    match bstate(pressed) {
      ButtonState::Pressed  => {pressed_at = Some(now); true},
      ButtonState::Repeat   => true,
      ButtonState::Released | ButtonState::NoInput => {
        if let Some(t) = pressed_at {
          if now - t < duration {
            true
          } else {
            pressed_at = None;
            false
          }
        } else {
          false
        }
      }
    }
  });

  let p = FnStage::from("extend", format!("{:?}", duration), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}*/

mod actions;
pub use self::actions::*;

mod as_axis_input;
pub use self::as_axis_input::*;

mod cartesian;
pub use self::cartesian::*;

mod cutoff;
pub use self::cutoff::*;

mod deadzone;
pub use self::deadzone::*;

mod distance_from_center;
pub use self::distance_from_center::*;

mod gate;
pub use self::gate::*;

mod input;
pub use self::input::*;

mod invert;
pub use self::invert::*;

mod line_segment_button;
pub use self::line_segment_button::*;

mod menu_item;
pub use self::menu_item::*;

mod merge;
pub use self::merge::*;

mod mode_is;
pub use self::mode_is::*;

mod offset;
pub use self::offset::*;

mod polar;
pub use self::polar::*;

mod press_types;
pub use self::press_types::*;

mod pulse;
pub use self::pulse::*;

mod relative;
pub use self::relative::*;

mod ring_sector_button;
pub use self::ring_sector_button::*;

mod rotate;
pub use self::rotate::*;

mod select;
pub use self::select::*;

mod scale;
pub use self::scale::*;

mod smooth;
pub use self::smooth::*;

mod touch_menu;
pub use self::touch_menu::*;

mod twitch_joymouse;
pub use self::twitch_joymouse::*;
