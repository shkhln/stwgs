use super::*;

struct AxisInputStage {
  stage_id: StageId,
  axis: Axis
}

impl Pipeline<f32> for AxisInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.axis)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> f32 {
    ctx.state.read_axis(self.axis)
  }

  fn reset(&mut self) {}
}

pub fn axis_input(axis: Axis) -> PipelineRef<f32> {
  std::rc::Rc::new(std::cell::RefCell::new(AxisInputStage { stage_id: generate_stage_id(), axis }))
}

struct ButtonInputStage {
  stage_id: StageId,
  button: Button
}

impl Pipeline<bool> for ButtonInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.button)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> bool {
    ctx.state.read_button(self.button)
  }

  fn reset(&mut self) {}
}

pub fn button_input(button: Button) -> PipelineRef<bool> {
  std::rc::Rc::new(std::cell::RefCell::new(ButtonInputStage { stage_id: generate_stage_id(), button }))
}

struct DummyButtonInput {
  stage_id: StageId,
  value: bool
}

impl Pipeline<bool> for DummyButtonInput {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.value)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, _: &Context, _: &mut Vec<Action>) -> bool {
    self.value
  }

  fn reset(&mut self) {}
}

pub fn dummy_button_input(value: bool) -> PipelineRef<bool> {
  std::rc::Rc::new(std::cell::RefCell::new(DummyButtonInput { stage_id: generate_stage_id(), value }))
}

struct ConstantInputStage {
  stage_id: StageId,
  value: f32
}

impl Pipeline<f32> for ConstantInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.value)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, _: &Context, _: &mut Vec<Action>) -> f32 {
    self.value
  }

  fn reset(&mut self) {}
}

pub fn constant_input(value: f32) -> PipelineRef<f32> {
  std::rc::Rc::new(std::cell::RefCell::new(ConstantInputStage { stage_id: generate_stage_id(), value }))
}

struct ExternalProbeStage {
  stage_id: StageId,
  name:     String
}

impl Pipeline<bool> for ExternalProbeStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "external_probe"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn probe(&self) -> Option<Probe> {
    Some(Probe::External { name: self.name.clone() })
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> bool {
    ctx.probe_values[&self.stage_id]
  }

  fn reset(&mut self) {}
}

pub fn external_probe(name: &str) -> PipelineRef<bool> {

  let stage = ExternalProbeStage {
    stage_id: generate_stage_id(),
    name:     name.to_string()
  };

  std::rc::Rc::new(std::cell::RefCell::new(stage))
}
