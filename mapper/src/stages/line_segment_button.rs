use super::*;

pub fn line_segment_button(pipeline: PipelineRef<f32>, from: f32, to: f32, margin: f32) -> PipelineRef<bool> {

  assert!(from >= 0.0);
  assert!(to > from);
  assert!(margin >= 0.0 && margin < to - from);

  let mut pressed = false;

  let fun = Box::new(move |x, _, _, _: &mut Vec<Action>| {

    pressed = match () {
      _ if x >= from          && x <= to          => true,
      _ if x <= from - margin || x >= to + margin => false,
      _                                           => pressed
    };

    pressed
  });

  let p = FnStage::from("line_segment_button", format!("from: {}, to: {}, margin: {}", from, to, margin), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
