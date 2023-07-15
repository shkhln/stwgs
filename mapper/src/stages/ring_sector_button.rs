use super::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RingSectorButtonOpts {
  pub direction:    f32,
  pub angle:        f32,
  pub inner_radius: f32,
  pub outer_radius: f32,
  pub margin:       f32
}

pub fn ring_sector_button(field: PipelineRef<(f32, f32)>, opts: RingSectorButtonOpts) -> PipelineRef<bool> {

  assert!(opts.angle > 0.0);
  assert!(opts.inner_radius >= 0.0);
  assert!(opts.outer_radius > opts.inner_radius);
  //assert!(opts.margin >= 0.0 && opts.margin < opts.outer_radius - opts.inner_radius);

  fn check(opts: RingSectorButtonOpts, x: f32, y: f32) -> bool {

    let dc = (x.powi(2) + y.powi(2)).sqrt();

    if dc >= opts.inner_radius && dc <= opts.outer_radius {

      let angle = y.atan2(x);

      let mut angle_diff = angle - opts.direction;
      if angle_diff >  std::f32::consts::PI { angle_diff -= std::f32::consts::PI * 2.0; }
      if angle_diff < -std::f32::consts::PI { angle_diff += std::f32::consts::PI * 2.0; }

      angle_diff.abs() <= opts.angle / 2.0
    } else {
      false
    }
  }

  let mut pressed = false;

  let fun = Box::new(move |(x, y), _, _, _: &mut Vec<Action>| {

    let test = check(opts, x + opts.margin / 2.0,  y);
    if test != check(opts, x - opts.margin / 2.0,  y) { return pressed; }
    if test != check(opts, x,  y + opts.margin / 2.0) { return pressed; }
    if test != check(opts, x,  y - opts.margin / 2.0) { return pressed; }

    pressed = test;
    pressed
  });

  let p = FnStage::from("ring_sector_button", format!("{:?}", opts), field, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
