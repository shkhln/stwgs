use super::*;

pub fn invert(pipeline: PipelineRef<bool>) -> PipelineRef<bool> {
  let fun = Box::new(move |pressed: bool, _, _: &mut Vec<Action>| !pressed);
  let p   = FnStage::from("invert", "".to_string(), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
