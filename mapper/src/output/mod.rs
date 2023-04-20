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
  fn keyboard_key_down(&self, key: KeyboardKey);
  fn keyboard_key_up(&self, key: KeyboardKey);
  fn mouse_button_down(&self, btn: MouseButton);
  fn mouse_button_up(&self, btn: MouseButton);
  fn mouse_cursor_rel_x(&self, value: i32);
  fn mouse_cursor_rel_y(&self, value: i32);
  fn mouse_wheel_rel(&self, value: i32);
  fn syn(&self);
}

pub struct DummyOutput;

impl MapperIO for DummyOutput {
  fn keyboard_key_down(&self, _key: KeyboardKey) {}
  fn keyboard_key_up(&self, _key: KeyboardKey) {}
  fn mouse_button_down(&self, _btn: MouseButton) {}
  fn mouse_button_up(&self, _btn: MouseButton) {}
  fn mouse_cursor_rel_x(&self, _value: i32) {}
  fn mouse_cursor_rel_y(&self, _value: i32) {}
  fn mouse_wheel_rel(&self, _value: i32) {}
  fn syn(&self) {}
}
