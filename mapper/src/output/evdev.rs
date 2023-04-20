use super::{uinput, KeyboardKey, MapperIO, MouseAxis, MouseButton};

pub struct UInputKeyboardAndMouse {
  kb: uinput::UInputDev,
  ms: uinput::UInputDev
}

impl UInputKeyboardAndMouse {

  pub fn new() -> Result<Self, String> {
    Ok(Self {
      kb: uinput::create_device(uinput::OutputDevice::Keyboard)?,
      ms: uinput::create_device(uinput::OutputDevice::Mouse)?
    })
  }
}

impl MapperIO for UInputKeyboardAndMouse {

  fn keyboard_key_down(&self, key: KeyboardKey) {
    self.kb.keyboard_key_event(key, uinput::KeyEvent::Down);
  }

  fn keyboard_key_up(&self, key: KeyboardKey) {
    self.kb.keyboard_key_event(key, uinput::KeyEvent::Up);
  }

  fn mouse_button_down(&self, btn: MouseButton) {
    self.ms.mouse_button_event(btn, uinput::KeyEvent::Down);
  }

  fn mouse_button_up(&self, btn: MouseButton) {
    self.ms.mouse_button_event(btn, uinput::KeyEvent::Up);
  }

  fn mouse_cursor_rel_x(&self, value: i32) {
    if value != 0 {
      self.ms.relative_axis_event(MouseAxis::X, value);
    }
  }

  fn mouse_cursor_rel_y(&self, value: i32) {
    if value != 0 {
      self.ms.relative_axis_event(MouseAxis::Y, value);
    }
  }

  fn mouse_wheel_rel(&self, value: i32) {
    if value != 0 {
      self.ms.relative_axis_event(MouseAxis::Wheel, value);
    }
  }

  fn syn(&self) {
    self.kb.syn();
    self.ms.syn();
  }
}
