#[cfg(feature = "evdev")]
pub mod evdev;
#[cfg(feature = "evdev")]
mod uinput;
#[cfg(feature = "x11")]
pub mod xcb;

use strum_macros::{EnumCount, EnumIter};

#[derive(Copy, Clone, Debug, PartialEq, EnumCount, EnumIter, strum_macros::Display)]
pub enum MouseAxis {
  X,
  Y,
  Wheel
}

#[derive(Copy, Clone, Debug, PartialEq, EnumCount, EnumIter, strum_macros::Display)]
pub enum MouseButton {
  Left,
  Right,
  Middle
}

#[derive(Copy, Clone, Debug, PartialEq, EnumCount, EnumIter, strum_macros::Display)]
pub enum KeyboardKey {
  A,
  B,
  C,
  D,
  E,
  F,
  G,
  H,
  I,
  J,
  K,
  L,
  M,
  N,
  O,
  P,
  Q,
  R,
  S,
  T,
  U,
  V,
  W,
  X,
  Y,
  Z,
  Esc,
  Enter,
  Space,
  Shift,
  Ctrl,
  Tab,
  Alt,
  _1,
  _2,
  _3,
  _4,
  _5,
  _6,
  _7,
  _8,
  _9,
  _0,
  KP1,
  KP2,
  KP3,
  KP4,
  KP5,
  KP6,
  KP7,
  KP8,
  KP9,
  KP0,
  Up,
  Left,
  Down,
  Right,
  PageDown,
  PageUp,
  Backslash
}

pub trait MapperIO {
  fn keyboard_key_down(&mut self, key: KeyboardKey);
  fn keyboard_key_up(&mut self, key: KeyboardKey);
  fn mouse_button_down(&mut self, btn: MouseButton);
  fn mouse_button_up(&mut self, btn: MouseButton);
  fn mouse_cursor_rel_xy(&mut self, x: i32, y: i32);
  fn mouse_wheel_rel(&mut self, value: i32);
  fn syn(&mut self);
}

pub struct DummyOutput;

impl MapperIO for DummyOutput {
  fn keyboard_key_down(&mut self, _key: KeyboardKey) {}
  fn keyboard_key_up(&mut self, _key: KeyboardKey) {}
  fn mouse_button_down(&mut self, _btn: MouseButton) {}
  fn mouse_button_up(&mut self, _btn: MouseButton) {}
  fn mouse_cursor_rel_xy(&mut self, _: i32, _: i32) {}
  fn mouse_wheel_rel(&mut self, _value: i32) {}
  fn syn(&mut self) {}
}
