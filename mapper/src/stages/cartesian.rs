use super::*;

pub fn cartesian(pipeline: PipelineRef<(f32, f32)>) -> PipelineRef<(f32, f32)> {

  let fun = Box::new(move |(distance, angle): (f32, f32), _, _: &mut Vec<Action>| {
    let x = distance * angle.cos();
    let y = distance * angle.sin();
    (x, y)
  });

  std::rc::Rc::new(std::cell::RefCell::new(FnStage::from("cartesian", "".to_string(), pipeline, fun)))
}
