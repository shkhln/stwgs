use super::*;

//TODO: consider replacing this with:
// input(amount: f32).gate((button: PipelineB).pulse(...))
// input(amount: f32).gate((button: PipelineB).once())
pub fn as_axis_input(pipeline: PipelineRef<bool>, amount: f32, repeat: bool) -> PipelineRef<f32> {

  let mut bstate = to_button_state();

  let fun = Box::new(move |pressed, _, _: &mut Vec<Action>| {
    match bstate(pressed) {
      ButtonState::Pressed  => amount,
      ButtonState::Repeat   => if repeat { amount } else { 0.0 },
      ButtonState::Released => 0.0,
      ButtonState::NoInput  => 0.0
    }
  });

  let p = FnStage::from("as_axis_input", format!("{}, repeat: {}", amount, repeat), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
