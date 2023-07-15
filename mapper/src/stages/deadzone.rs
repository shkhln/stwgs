use super::*;

pub fn deadzone(pipeline: PipelineRef<f32>, level: f32) -> PipelineRef<f32> {

  assert!(level > 0.0);

  let fun = Box::new(move |value: f32, _, _, _: &mut Vec<Action>| {
    if value.abs() > level {
      if value > 0.0 { value - level } else { value + level }
    } else {
      0.0
    }
  });

  std::rc::Rc::new(std::cell::RefCell::new(FnStage::from("deadzone", format!("{}", level), pipeline, fun)))
}

pub fn cartesian_deadzone(pipeline: PipelineRef<(f32, f32)>, level: f32) -> PipelineRef<(f32, f32)> {

  assert!(level > 0.0);

  let fun = Box::new(move |(x, y): (f32, f32), _, _, _: &mut Vec<Action>| {

    let distance_from_center = (x.powi(2) + y.powi(2)).sqrt();
    let effective_distance   = distance_from_center - level;

    if effective_distance > 0.0 {
      let angle = y.atan2(x);
      (effective_distance * angle.cos(), effective_distance * angle.sin())
    } else {
      (0.0, 0.0)
    }
  });

  std::rc::Rc::new(std::cell::RefCell::new(FnStage::from("deadzone", format!("{}", level), pipeline, fun)))
}

#[cfg(test)]
mod tests {

  use super::*;

  #[test]
  fn deadzone_test() {
    let mut state   = crate::controllers::ControllerState::empty();
    let mut actions = vec![];

    let joy = cartesian_deadzone(merge(axis_input(Axis::LJoyX), axis_input(Axis::LJoyY)), 50.0);

    let values: &[((f32, f32), (f32, f32))] = &[
      ((  0.0,   0.0), (   0.0,    0.0)),
      (( 25.0,  25.0), (   0.0,    0.0)),
      (( 50.0,  50.0), (  15.0,   15.0)),
      (( 25.0,  50.0), (   3.0,    5.0)),
      ((-50.0, -50.0), ( -15.0,  -15.0))
    ];

    for ((joy_x, joy_y), (expected_x, expected_y)) in values {
      joy.borrow_mut().reset();
      state.axes.ljoy_x = *joy_x;
      state.axes.ljoy_y = *joy_y;
      let ctx = Context { state: &state, time: Timestamp(0), layers: LayerMask::EMPTY, probe_values: &HashMap::new() };
      let (x, y) = joy.borrow_mut().apply(&ctx, &mut actions);
      assert_eq!(x.round(), *expected_x);
      assert_eq!(y.round(), *expected_y);
    }
  }
}
