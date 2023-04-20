use super::*;

pub fn pulse(pipeline: PipelineRef<bool>, frequency: f32, width: f32) -> PipelineRef<bool> {

  assert!(width > 0.0 && width < 1.0);

  let cycle_time = std::time::Duration::from_millis((1000.0 / frequency) as u64);
  let pulse_time = cycle_time.mul_f32(width);
  let wait_time  = cycle_time.mul_f32(1.0 - width);

  let mut bstate    = to_button_state();
  let mut last_flip = None;
  let mut value     = false;

  let fun = Box::new(move |b, now, _: &mut Vec<Action>| {

    match bstate(b) {
      ButtonState::Pressed => {
        value     = true;
        last_flip = Some(now);
      },
      ButtonState::Repeat => {
        if now - last_flip.unwrap() >= (if value { pulse_time } else { wait_time }) {
          value     = !value;
          last_flip = Some(now);
        }
      },
      ButtonState::Released => {
        value     = false;
        last_flip = None;
      },
      ButtonState::NoInput => ()
    }

    value
  });

  let p = FnStage::from("pulse", format!("{}", frequency), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

struct AxisPulseStage {
  stage_id:  StageId,
  pipelines: (PipelineRef<bool>, PipelineRef<f32>, PipelineRef<f32>),
  bstate:    Box<dyn FnMut(bool) -> ButtonState>,
  last_flip: Option<Timestamp>,
  value:     bool,
  out_value: Option<bool>
}

impl Pipeline<bool> for AxisPulseStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "pulse"
  }

  fn desc(&self) -> String {
    format!("{} -> {}({}, {})",
      self.pipelines.0.borrow().desc(), self.name(), self.pipelines.1.borrow().desc(), self.pipelines.2.borrow().desc())
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![
      self.pipelines.0.borrow().stage_id(),
      self.pipelines.1.borrow().stage_id(),
      self.pipelines.2.borrow().stage_id()
    ]
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    if insert_stage_description(out, self) {
      self.pipelines.0.borrow().inspect(out);
      self.pipelines.1.borrow().inspect(out);
      self.pipelines.2.borrow().inspect(out);
    }
  }

  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) -> bool {

    if self.out_value.is_none() {

      let button    = self.pipelines.0.borrow_mut().apply(ctx, actions);
      let frequency = self.pipelines.1.borrow_mut().apply(ctx, actions);
      let width     = self.pipelines.2.borrow_mut().apply(ctx, actions);

      // pulse width should never be outside of 0..=1 range
      let width = width.clamp(0.0, 1.0);

      let cycle_time = std::time::Duration::from_millis((1000.0 / frequency) as u64);
      let pulse_time = cycle_time.mul_f32(width);
      let wait_time  = cycle_time.mul_f32(1.0 - width);
      //println!("ct: {:?}, pt: {:?}, wt: {:?} ({})", cycle_time, pulse_time, wait_time, width);

      match (self.bstate)(button) {
        ButtonState::Pressed => {
          self.value     = true;
          self.last_flip = Some(ctx.time);
        },
        ButtonState::Repeat => {
          //println!("{} {} -> {:?} {:?}", frequency, width, pulse_time, wait_time);
          if ctx.time - self.last_flip.unwrap() >= (if self.value { pulse_time } else { wait_time }) {
            self.value     = !self.value;
            self.last_flip = Some(ctx.time);
          }
        },
        ButtonState::Released => {
          self.value     = false;
          self.last_flip = None;
        },
        ButtonState::NoInput => ()
      }

      self.out_value = Some(self.value);
    }

    self.out_value.unwrap()
  }

  fn reset(&mut self) {
    if self.out_value.is_some() {
      self.pipelines.0.borrow_mut().reset();
      self.pipelines.1.borrow_mut().reset();
      self.pipelines.2.borrow_mut().reset();
      self.out_value = None;
    }
  }
}

pub fn pulse_by_axis(button: PipelineRef<bool>, frequency: PipelineRef<f32>, width: PipelineRef<f32>) -> PipelineRef<bool> {
  let p = AxisPulseStage {
    stage_id:  generate_stage_id(),
    pipelines: (button, frequency, width),
    bstate:    to_button_state(),
    last_flip: None,
    value:     false,
    out_value: None
  };
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
