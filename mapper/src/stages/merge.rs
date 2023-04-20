use super::*;

struct MergeStage<T, U> {
  stage_id: StageId,
  pipelines: (PipelineRef<T>, PipelineRef<U>)
}

impl<T: Copy + 'static, U: Copy + 'static> Pipeline<(T, U)> for MergeStage<T, U> {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "merge"
  }

  fn desc(&self) -> String {
    format!("[{} + {}]", self.pipelines.0.borrow().desc(), self.pipelines.1.borrow().desc())
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![
      self.pipelines.0.borrow().stage_id(),
      self.pipelines.1.borrow().stage_id()
    ]
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    if insert_stage_description(out, self) {
      self.pipelines.0.borrow().inspect(out);
      self.pipelines.1.borrow().inspect(out);
    }
  }

  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) -> (T, U) {
    (self.pipelines.0.borrow_mut().apply(ctx, actions), self.pipelines.1.borrow_mut().apply(ctx, actions))
  }

  fn reset(&mut self) {
    self.pipelines.0.borrow_mut().reset();
    self.pipelines.1.borrow_mut().reset();
  }
}

pub fn merge<T: Copy + 'static, U: Copy + 'static>(p1: PipelineRef<T>, p2: PipelineRef<U>) -> PipelineRef<(T, U)> {
  std::rc::Rc::new(std::cell::RefCell::new(MergeStage { stage_id: generate_stage_id(), pipelines: (p1, p2) }))
}
