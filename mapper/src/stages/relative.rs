use super::*;

pub fn relative(axis: PipelineRef<f32>, button: PipelineRef<bool>) -> PipelineRef<f32> {

  let mut bstate = to_button_state();
  let mut prev   = 0.0;

  let fun = Box::new(move |value, button, _, _, _: &mut Vec<Action>| {
    match (bstate)(button) {
      ButtonState::Pressed => {
        prev = value;
        0.0
      },
      ButtonState::Repeat => {
        let diff = value - prev;
        prev = value;
        diff
      },
      _ => 0.0
    }
  });

  let p = BiFnStage::from("relative", "".to_string(), axis, button, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
