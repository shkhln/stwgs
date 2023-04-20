use super::*;

pub fn polar(pipeline: PipelineRef<(f32, f32)>) -> PipelineRef<(f32, f32)> {

  let fun = Box::new(move |(x, y): (f32, f32), _, _: &mut Vec<Action>| {
    let distance = (x.powi(2) + y.powi(2)).sqrt();
    let angle    = y.atan2(x);
    (distance, angle)
  });

  std::rc::Rc::new(std::cell::RefCell::new(FnStage::from("polar", "".to_string(), pipeline, fun)))
}
