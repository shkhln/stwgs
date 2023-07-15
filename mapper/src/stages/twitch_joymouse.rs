use super::*;

pub fn twitch_joymouse(joystick: PipelineRef<(f32, f32)>) -> PipelineRef<(f32, f32)> {

  let mut prev_x   = 0.0;
  let mut prev_y   = 0.0;
  let mut prev_dfc = 0.0;
  let mut speed    = 0.0;

  let fun = Box::new(move |(x, y): (f32, f32), _, _, _: &mut Vec<Action>| {

    let distance_from_center = (x.powi(2) + y.powi(2)).sqrt();

    let out = if distance_from_center < 0.9 {

      let dfc_diff = distance_from_center - prev_dfc;
      //println!("diff: {}", dfc_diff);

      // http://stackoverflow.com/questions/4026648/how-to-implement-low-pass-filter-using-java/7529271#7529271
      speed += (dfc_diff - speed) / 8.0; //TODO: should take tick time into account

      if dfc_diff > 0.0 {
        (x - prev_x, y - prev_y)
      } else {
        (0.0, 0.0)
      }
    } else {
      let angle = y.atan2(x);
      (angle.cos() * speed, angle.sin() * speed) // preserve speed
    };

    prev_x   = x;
    prev_y   = y;
    prev_dfc = distance_from_center;
    out
  });

  std::rc::Rc::new(std::cell::RefCell::new(FnStage::from("twitch_joymouse", "".to_string(), joystick, fun)))
}
