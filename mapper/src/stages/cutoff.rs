use super::*;

pub fn cutoff(pipeline: PipelineRef<f32>, level: f32) -> PipelineRef<f32> {
  assert!(level > 0.0);
  let fun = Box::new(move |value: f32, _, _, _: &mut Vec<Action>| if value.abs() <= level { value } else { 0.0 });
  let p   = FnStage::from("cutoff", format!("{}", level), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
