use super::*;

pub fn distance_from_center(pipeline: PipelineRef<(f32, f32)>) -> PipelineRef<f32> {

  let fun = Box::new(move |(x, y): (f32, f32), _, _, _: &mut Vec<Action>| {
    (x.powi(2) + y.powi(2)).sqrt()
  });

  let p = FnStage::from("distance_from_center", "".to_string(), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
