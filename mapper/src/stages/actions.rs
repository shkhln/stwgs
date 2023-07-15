use super::*;

pub fn keyboard_key_press(pipeline: PipelineRef<bool>, key: KeyboardKey) -> Box<dyn Pipeline<()>> {
  let fun = Box::new(move |pressed, _, _, actions: &mut Vec<Action>| {
    if pressed {
      actions.push(Action::PressKeyboardKey(key));
    }
  });
  Box::new(FnStage::from("key", format!("{:?}", key), pipeline, fun))
}

pub fn mouse_button_press(pipeline: PipelineRef<bool>, btn: MouseButton) -> Box<dyn Pipeline<()>> {
  let fun = Box::new(move |pressed, _, _, actions: &mut Vec<Action>| {
    if pressed {
      actions.push(Action::PressMouseButton(btn));
    }
  });
  Box::new(FnStage::from("button", format!("{:?}", btn), pipeline, fun))
}

pub fn mouse_move(pipeline: PipelineRef<f32>, axis: MouseAxis) -> Box<dyn Pipeline<()>> {
  let fun = Box::new(move |value, _, _, actions: &mut Vec<Action>| {
    actions.push(Action::MoveMouse(axis, value));
  });
  Box::new(FnStage::from("mouse_move", format!("{:?}", axis), pipeline, fun))
}

pub fn switch_mode(pipeline: PipelineRef<bool>, mask: crate::mapper::LayerMask) -> Box<dyn Pipeline<()>> {

  let mut bstate = to_button_state();

  let fun = Box::new(move |pressed, _, _, actions: &mut Vec<Action>| {
    if bstate(pressed) == ButtonState::Pressed {
      actions.push(Action::SetLayerMask(mask));
    }
  });

  Box::new(FnStage::from("switch_mode", format!("{:?}", mask), pipeline, fun))
}

pub fn cycle_modes(pipeline: PipelineRef<bool>, masks: Vec<crate::mapper::LayerMask>) -> Box<dyn Pipeline<()>> {

  let args = format!("{:?}", masks);

  let mut bstate = to_button_state();

  let fun = Box::new(move |pressed, _, mode, actions: &mut Vec<Action>| {
    if bstate(pressed) == ButtonState::Pressed {
      let i = masks.iter().position(|&m| m == mode).unwrap_or(0);
      actions.push(Action::SetLayerMask(masks[(i + 1) % masks.len()]));
    }
  });

  Box::new(FnStage::from("cycle_modes", args, pipeline, fun))
}

struct FlipModeStage {
  stage_id: StageId,
  button:   PipelineRef<bool>,
  target:   LayerMask,
  bstate:   Box<dyn FnMut(bool) -> ButtonState>
}

impl Pipeline<()> for FlipModeStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "flip_mode"
  }

  fn desc(&self) -> String {
    format!("{} -> {}(...)", self.button.borrow().desc(), self.name())
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![self.button.borrow().stage_id()]
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    if insert_stage_description(out, self) {
      self.button.borrow().inspect(out);
    }
  }

  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) {
    let value = self.button.borrow_mut().apply(ctx, actions);
    if (self.bstate)(value) == ButtonState::Pressed && ctx.layers != self.target {
      //eprintln!("flip");
      actions.push(Action::SetLayerMask(self.target));
      self.target = ctx.layers;
    }
  }

  fn reset(&mut self) {
    self.button.borrow_mut().reset();
  }
}

pub fn flip_mode(button: PipelineRef<bool>, target: LayerMask) -> Box<dyn Pipeline<()>> {
  Box::new(FlipModeStage { stage_id: generate_stage_id(), button, target, bstate: to_button_state() })
}

/*pub fn noop(pipeline: PipelineRef<::std::any::Any>) -> Box<Pipeline<Action>> {

  let fun = Box::new(move |pressed, _| { Action::None });

  Box::new(FnStage::from(String::from(format!("noop()")), pipeline, fun))
}*/

pub fn trigger_bump(button: PipelineRef<bool>, left: bool) -> Box<dyn Pipeline<()>> {

  let mut bstate = to_button_state();

  let fun = Box::new(move |pressed, _, _, actions: &mut Vec<Action>| {
    if (bstate)(pressed) == ButtonState::Pressed {
      let target = if left { HapticFeedbackTarget::LeftTrigger } else { HapticFeedbackTarget::RightTrigger };
      actions.push(Action::HapticFeedback(target, HapticFeedbackEffect::SlightBump));
    }
  });

  Box::new(FnStage::from("trigger_bump", "".to_string(), button, fun))
}
