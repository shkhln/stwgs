/*use std::time::Duration;
use super::*;

pub fn timed(pipeline: PipelineRef<bool>) -> PipelineRef<(Option<Timestamp>, bool)> {

  let mut pressed_at: Option<Timestamp> = None;
  let mut bstate = to_button_state();

  let fun = Box::new(move |value, now, _: &mut Vec<Action>| {
    match bstate(value) {
      ButtonState::Pressed  => pressed_at = Some(now),
      ButtonState::Repeat   => (),
      ButtonState::Released => (), //pressed_at = None,
      ButtonState::NoInput  => ()
    };
    (pressed_at, value)
  });

  let p = FnStage::from("timed", "".to_string(), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ButtonState2 {
  Pressed(Timestamp),
  Active,
  Released,
  Inactive
}

pub fn to_button_state2() -> Box<dyn FnMut(bool, Timestamp) -> ButtonState2> {

  let mut bstate = ButtonState2::Inactive;

  Box::new(move |value, now| {
    bstate = match bstate {
      ButtonState2::Pressed(_) => if value { ButtonState2::Active       } else { ButtonState2::Released },
      ButtonState2::Active     => if value { ButtonState2::Active       } else { ButtonState2::Released },
      ButtonState2::Released   => if value { ButtonState2::Pressed(now) } else { ButtonState2::Inactive },
      ButtonState2::Inactive   => if value { ButtonState2::Pressed(now) } else { ButtonState2::Inactive }
    };

    bstate
  })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TapOpts {
  pub max_taps: usize,
  pub max_duration: Duration,
  pub succession_threshold: Duration,
}

fn count_taps(opts: TapOpts) -> Box<dyn FnMut(ButtonState2, Timestamp) -> Option<usize>> {

  use ButtonState2::*;

  let mut pressed_at  = None;
  let mut released_at = None;

  let mut taps = 0;

  Box::new(move |bstate, now| {

    let mut result = None;

    match bstate {
      Pressed(t) => pressed_at = Some(t),
      Active     => (),
      Released   => {
        released_at = Some(now);
        if now - pressed_at.unwrap() <= opts.max_duration {
          taps += 1;
        } else {
          taps = 0;
        }
      },
      Inactive => {
        if taps > 0 {
          if now - released_at.unwrap() > opts.succession_threshold {
            result = Some(if taps > opts.max_taps { opts.max_taps } else { taps });
            taps   = 0;
          }
        }
      }
    }

    result
  })
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Effect {
  Tap(usize), Press(Duration)
}

#[derive(Clone, Debug)]
pub struct EffectStageOpts {
  pub taps: Option<TapOpts>,
  pub presses: Vec<Duration>
}

pub fn detect(pipeline: PipelineRef<(Option<Timestamp>, bool)>, opts: EffectStageOpts) -> PipelineRef<Option<Effect>> {

  let mut _opts = opts.clone();

  let mut presses = opts.presses.clone();
  presses.sort_by(|a, b| b.cmp(a));

  let mut count_taps = if let Some(tap_opts) = opts.taps {
    count_taps(tap_opts)
  } else {
    Box::new(|_, _| { None })
  };

  let mut bstate2 = to_button_state2();

  let fun = Box::new(move |(pressed_at, value), now, _: &mut Vec<Action>| {

    if let Some(t) = pressed_at {

      if let Some(tap_opts) = opts.taps {

        if let Some(count) = count_taps(bstate2(value, t), now) {
          return Some(Effect::Tap(count));
        }

        if now - t <= tap_opts.max_duration {
          return None;
        }
      }

      if value {

        let press_time = now - t;

        for i in 0..presses.len() {
          let threshold = presses[i];
          if press_time > threshold {
            return Some(Effect::Press(threshold));
          }
        }

        return Some(Effect::Press(Duration::from_millis(0)));
      }
    }

    None
  });

  let p = FnStage::from("detect", format!("{:?}", _opts), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}

pub fn select(pipeline: PipelineRef<Option<Effect>>, effect: Effect) -> PipelineRef<bool> {

  let fun = Box::new(move |value, _, _: &mut Vec<Action>| { value == Some(effect) });

  let p = FnStage::from("select", format!("({:?})", effect), pipeline, fun);
  std::rc::Rc::new(std::cell::RefCell::new(p))
}
*/
