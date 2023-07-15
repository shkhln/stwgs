use super::*;

pub fn rotate(field: PipelineRef<(f32, f32)>, angle: f32) -> PipelineRef<(f32, f32)> {

  let fun = Box::new(move |(x, y), _, _, _: &mut Vec<Action>| {
    let cs = angle.cos();
    let sn = angle.sin();
    (x * cs - y * sn, x * sn + y * cs)
  });

  let p = FnStage::from("rotate", format!("{}", angle), field, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

pub fn rotate_by_axis(field: PipelineRef<(f32, f32)>, angle: PipelineRef<f32>) -> PipelineRef<(f32, f32)> {

  let fun = Box::new(move |(x, y), angle: f32, _, _, _: &mut Vec<Action>| {
    let cs = angle.cos();
    let sn = angle.sin();
    (x * cs - y * sn, x * sn + y * cs)
  });

  let p = BiFnStage::from("rotate", "".to_string(), field, angle, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
