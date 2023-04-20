use super::*;

struct ModeCheckStage {
  stage_id: StageId,
  target:   LayerMask
}

impl Pipeline<bool> for ModeCheckStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "mode_is"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{}", self.target)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> bool {
    ctx.layers == self.target
  }

  fn reset(&mut self) {}
}

pub fn mode_is(target: LayerMask) -> PipelineRef<bool> {
  std::rc::Rc::new(std::cell::RefCell::new(ModeCheckStage { stage_id: generate_stage_id(), target }))
}
