use super::*;

pub fn offset(pipeline: PipelineRef<f32>, addend: f32) -> PipelineRef<f32> {
  let fun = Box::new(move |value, _, _: &mut Vec<Action>| value + addend);
  let p   = FnStage::from("offset", format!("{}", addend), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

pub fn offset_by_axis(p1: PipelineRef<f32>, p2: PipelineRef<f32>) -> PipelineRef<f32> {
  let fun = Box::new(move |v1, v2, _, _: &mut Vec<Action>| v1 + v2);
  let p   = BiFnStage::from("offset", "".to_string(), p1, p2, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
