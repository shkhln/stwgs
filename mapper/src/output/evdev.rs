use super::{uinput, KeyboardKey, MapperIO, MouseButton};

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

  fn keyboard_key_down(&mut self, key: KeyboardKey) {
    self.kb.keyboard_key_event(key, uinput::KeyEvent::Down);
  }

  fn keyboard_key_up(&mut self, key: KeyboardKey) {
    self.kb.keyboard_key_event(key, uinput::KeyEvent::Up);
  }

  fn mouse_button_down(&mut self, btn: MouseButton) {
    self.ms.mouse_button_event(btn, uinput::KeyEvent::Down);
  }

  fn mouse_button_up(&mut self, btn: MouseButton) {
    self.ms.mouse_button_event(btn, uinput::KeyEvent::Up);
  }

  fn mouse_cursor_rel_xy(&mut self, x: i32, y: i32) {
    self.ms.mouse_xy_event(x, y);
  }

  fn mouse_wheel_rel(&mut self, value: i32) {
    self.ms.mouse_wheel_event(value);
  }

  fn syn(&mut self) {
    // do nothing
  }
}
