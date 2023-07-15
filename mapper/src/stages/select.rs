use super::*;

pub fn select0(pipeline: PipelineRef<(f32, f32)>) -> PipelineRef<f32> {
  let fun = Box::new(move |value: (f32, f32), _, _, _: &mut Vec<Action>| value.0);
  let p   = FnStage::from("select0", "".to_string(), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

pub fn select1(pipeline: PipelineRef<(f32, f32)>) -> PipelineRef<f32> {
  let fun = Box::new(move |value: (f32, f32), _, _, _: &mut Vec<Action>| value.1);
  let p   = FnStage::from("select1", "".to_string(), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
