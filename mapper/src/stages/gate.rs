use super::*;

pub fn gate_axis(p1: PipelineRef<f32>, p2: PipelineRef<bool>) -> PipelineRef<f32> {
  let fun = Box::new(move |value, pressed, _, _: &mut Vec<Action>| { if pressed { value } else { 0.0 } });
  let p   = BiFnStage::from("gate", "".to_string(), p1, p2, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

pub fn gate_button(p1: PipelineRef<bool>, p2: PipelineRef<bool>) -> PipelineRef<bool> {
  let fun = Box::new(move |value, pressed, _, _: &mut Vec<Action>| { if pressed { value } else { false } });
  let p   = BiFnStage::from("gate", "".to_string(), p1, p2, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
