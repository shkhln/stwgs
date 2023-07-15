use super::*;

// http://stackoverflow.com/questions/4026648/how-to-implement-low-pass-filter-using-java/7529271#7529271
pub fn smooth(pipeline: PipelineRef<f32>, factor: f32) -> PipelineRef<f32> {

  let mut smoothed_value = 0.0;

  let fun = Box::new(move |value, _, _, _: &mut Vec<Action>| {
    smoothed_value += (value - smoothed_value) / factor;
    smoothed_value
  });

  let p = FnStage::from("smooth", format!("{}", factor), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
