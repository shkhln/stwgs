#[cfg(feature = "sdl")]
mod sdlgc;
mod steam;

use std::sync::mpsc::{Receiver, Sender};
use strum_macros::EnumIter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, EnumIter)]
pub enum Button {
  LStick,
  RStick,
  RPadTouch,
  LPadTouch,
  RPad,
  LPad,
  RGrip,
  LGrip,
  Start,
  Steam,
  Back,
  DPadDown,
  DPadLeft,
  DPadRight,
  DPadUp,
  A,
  X,
  B,
  Y,
  LBump,
  RBump,
  LTrig,
  RTrig
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, EnumIter)]
pub enum Axis {
  LTrig,
  RTrig,
  LJoyX,
  LJoyY,
  RJoyX,
  RJoyY,
  LPadX,
  LPadY,
  RPadX,
  RPadY,
  AX,
  AY,
  AZ,
  Pitch,
  Roll,
  Yaw,
  Q0,
  Q1,
  Q2,
  Q3,
  AbsPitch,
  AbsRoll,
  AbsYaw
}

#[derive(Default, Copy, Clone)]
pub struct ControllerButtons {
  pub lstick:     bool,
  pub rstick:     bool,
  pub rpad_touch: bool,
  pub lpad_touch: bool,
  pub rpad_press: bool,
  pub lpad_press: bool,
  pub rgrip:      bool,
  pub lgrip:      bool,
  pub start:      bool,
  pub steam:      bool,
  pub back:       bool,
  pub dpad_down:  bool,
  pub dpad_left:  bool,
  pub dpad_right: bool,
  pub dpad_up:    bool,
  pub a:          bool,
  pub x:          bool,
  pub b:          bool,
  pub y:          bool,
  pub lbump:      bool,
  pub rbump:      bool,
  pub ltrig:      bool,
  pub rtrig:      bool
}

#[derive(Default, Copy, Clone)]
pub struct ControllerAxes {
  pub ltrig:   f32,
  pub rtrig:   f32,
  pub ljoy_x:  f32,
  pub ljoy_y:  f32,
  pub rjoy_x:  f32,
  pub rjoy_y:  f32,
  pub lpad_x:  f32,
  pub lpad_y:  f32,
  pub rpad_x:  f32,
  pub rpad_y:  f32,
  pub ax:      f32,
  pub ay:      f32,
  pub az:      f32,
  pub pitch:   f32,
  pub roll:    f32,
  pub yaw:     f32,
  pub q0:      i16,
  pub q1:      i16,
  pub q2:      i16,
  pub q3:      i16,
  pub a_pitch: f32,
  pub a_roll:  f32,
  pub a_yaw:   f32
}

#[derive(Default, Copy, Clone)]
pub struct ControllerState {
  pub buttons: ControllerButtons,
  pub axes:    ControllerAxes
}

impl ControllerState {

  pub fn empty() -> Self {
    Self {
      buttons: ControllerButtons {
        lstick:     false,
        rstick:     false,
        rpad_touch: false,
        lpad_touch: false,
        rpad_press: false,
        lpad_press: false,
        rgrip:      false,
        lgrip:      false,
        start:      false,
        steam:      false,
        back:       false,
        dpad_down:  false,
        dpad_left:  false,
        dpad_right: false,
        dpad_up:    false,
        a:          false,
        x:          false,
        b:          false,
        y:          false,
        lbump:      false,
        rbump:      false,
        ltrig:      false,
        rtrig:      false
      },
      axes:    ControllerAxes {
        ltrig:   0.0,
        rtrig:   0.0,
        ljoy_x:  0.0,
        ljoy_y:  0.0,
        rjoy_x:  0.0,
        rjoy_y:  0.0,
        lpad_x:  0.0,
        lpad_y:  0.0,
        rpad_x:  0.0,
        rpad_y:  0.0,
        ax:      0.0,
        ay:      0.0,
        az:      0.0,
        pitch:   0.0,
        roll:    0.0,
        yaw:     0.0,
        q0:      0,
        q1:      0,
        q2:      0,
        q3:      0,
        a_pitch: 0.0,
        a_roll:  0.0,
        a_yaw:   0.0
      }
    }
  }

  pub fn read_button(&self, button: Button) -> bool {
    match button {
      Button::LStick    => self.buttons.lstick,
      Button::RStick    => self.buttons.rstick,
      Button::RPadTouch => self.buttons.rpad_touch,
      Button::LPadTouch => self.buttons.lpad_touch,
      Button::RPad      => self.buttons.rpad_press,
      Button::LPad      => self.buttons.lpad_press,
      Button::RGrip     => self.buttons.rgrip,
      Button::LGrip     => self.buttons.lgrip,
      Button::Start     => self.buttons.start,
      Button::Steam     => self.buttons.steam,
      Button::Back      => self.buttons.back,
      Button::DPadDown  => self.buttons.dpad_down,
      Button::DPadLeft  => self.buttons.dpad_left,
      Button::DPadRight => self.buttons.dpad_right,
      Button::DPadUp    => self.buttons.dpad_up,
      Button::A         => self.buttons.a,
      Button::X         => self.buttons.x,
      Button::B         => self.buttons.b,
      Button::Y         => self.buttons.y,
      Button::LBump     => self.buttons.lbump,
      Button::RBump     => self.buttons.rbump,
      Button::LTrig     => self.buttons.ltrig,
      Button::RTrig     => self.buttons.rtrig
    }
  }

  pub fn read_axis(&self, axis: Axis) -> f32 {
    match axis {
      Axis::LTrig    => self.axes.ltrig,
      Axis::RTrig    => self.axes.rtrig,
      Axis::LJoyX    => self.axes.ljoy_x,
      Axis::LJoyY    => self.axes.ljoy_y,
      Axis::RJoyX    => self.axes.rjoy_x,
      Axis::RJoyY    => self.axes.rjoy_y,
      Axis::LPadX    => self.axes.lpad_x,
      Axis::LPadY    => self.axes.lpad_y,
      Axis::RPadX    => self.axes.rpad_x,
      Axis::RPadY    => self.axes.rpad_y,
      Axis::AX       => self.axes.ax,
      Axis::AY       => self.axes.ay,
      Axis::AZ       => self.axes.az,
      Axis::Pitch    => self.axes.pitch,
      Axis::Roll     => self.axes.roll,
      Axis::Yaw      => self.axes.yaw,
      Axis::Q0       => self.axes.q0 as f32,
      Axis::Q1       => self.axes.q1 as f32,
      Axis::Q2       => self.axes.q2 as f32,
      Axis::Q3       => self.axes.q3 as f32,
      Axis::AbsPitch => self.axes.a_pitch,
      Axis::AbsRoll  => self.axes.a_roll,
      Axis::AbsYaw   => self.axes.a_yaw
    }
  }

  pub fn random<R: ::rand::Rng>(rng: &mut R) -> Self {
    Self {
      buttons: ControllerButtons {
        lstick:     rng.gen(),
        rstick:     rng.gen(),
        rpad_touch: rng.gen(),
        lpad_touch: rng.gen(),
        rpad_press: rng.gen(),
        lpad_press: rng.gen(),
        rgrip:      rng.gen(),
        lgrip:      rng.gen(),
        start:      rng.gen(),
        steam:      rng.gen(),
        back:       rng.gen(),
        dpad_down:  rng.gen(),
        dpad_left:  rng.gen(),
        dpad_right: rng.gen(),
        dpad_up:    rng.gen(),
        a:          rng.gen(),
        x:          rng.gen(),
        b:          rng.gen(),
        y:          rng.gen(),
        lbump:      rng.gen(),
        rbump:      rng.gen(),
        ltrig:      rng.gen(),
        rtrig:      rng.gen()
      },
      axes:    ControllerAxes {
        ltrig:   rng.gen_range( 0.0..=1.0),
        rtrig:   rng.gen_range( 0.0..=1.0),
        ljoy_x:  rng.gen_range(-1.0..=1.0),
        ljoy_y:  rng.gen_range(-1.0..=1.0),
        rjoy_x:  rng.gen_range(-1.0..=1.0),
        rjoy_y:  rng.gen_range(-1.0..=1.0),
        lpad_x:  rng.gen_range(-1.0..=1.0),
        lpad_y:  rng.gen_range(-1.0..=1.0),
        rpad_x:  rng.gen_range(-1.0..=1.0),
        rpad_y:  rng.gen_range(-1.0..=1.0),
        ax:      rng.gen_range(-1.0..=1.0), // ?
        ay:      rng.gen_range(-1.0..=1.0), // ?
        az:      rng.gen_range(-1.0..=1.0), // ?
        pitch:   rng.gen_range(-1.0..=1.0), // ?
        roll:    rng.gen_range(-1.0..=1.0), // ?
        yaw:     rng.gen_range(-1.0..=1.0), // ?
        q0:      rng.gen_range(i16::MIN..=i16::MAX),
        q1:      rng.gen_range(i16::MIN..=i16::MAX),
        q2:      rng.gen_range(i16::MIN..=i16::MAX),
        q3:      rng.gen_range(i16::MIN..=i16::MAX),
        a_pitch: rng.gen_range(-std::f32::consts::PI..=std::f32::consts::PI),
        a_roll:  rng.gen_range(-std::f32::consts::PI..=std::f32::consts::PI),
        a_yaw:   rng.gen_range(-std::f32::consts::PI..=std::f32::consts::PI)
      }
    }
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HapticFeedbackTarget {
  LeftSide,
  RightSide,
  LeftTrigger,
  RightTrigger
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HapticFeedbackEffect {
  SlightBump,
  ModerateBump
}

#[derive(Copy, Clone)]
pub enum ControllerCommand {
  HapticFeedback(HapticFeedbackTarget, HapticFeedbackEffect)
}

pub trait Controller {
  fn name(&self)   -> String;
  fn path(&self)   -> String;
  fn serial(&self) -> Option<String>;
  fn run_polling_loop(&self, sender: Sender<ControllerState>, receiver: Option<Receiver<ControllerCommand>>) -> Result<(), String>;
}

pub fn available_controllers() -> Result<Vec<Box<dyn Controller>>, String> {
  let mut controllers: Vec<Box<dyn Controller>> = vec![];
  controllers.extend(steam::available_controllers()?);
  #[cfg(feature = "sdl")]
  controllers.extend(sdlgc::available_controllers()?);
  Ok(controllers)
}

pub fn find_controller(serial_or_partial_path: Option<String>) -> Result<Option<Box<dyn Controller>>, String> {
  let controllers = available_controllers()?;

  if let Some(serial_or_partial_path) = serial_or_partial_path.map(|s| s.to_lowercase()) {

    for controller in controllers {

      if let Some(serial) = controller.serial() {
        if serial.to_lowercase() == serial_or_partial_path {
          return Ok(Some(controller));
        }
      }

      if controller.path().to_lowercase().contains(&serial_or_partial_path) {
        return Ok(Some(controller));
      }

      if controller.name().to_lowercase().contains(&serial_or_partial_path) {
        return Ok(Some(controller));
      }
    }

    Ok(None)
  } else {
    Ok(controllers.into_iter().next())
  }
}
