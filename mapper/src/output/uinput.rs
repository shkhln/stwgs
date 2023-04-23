use evdev::uinput::VirtualDeviceBuilder;
use evdev::{AttributeSet, EventType, InputEvent, Key, RelativeAxisType};

use super::{KeyboardKey, MouseButton};

fn keyboard_key_to_evdev_type(key: KeyboardKey) -> Key {
  match key {
    KeyboardKey::A         => Key::KEY_A,
    KeyboardKey::B         => Key::KEY_B,
    KeyboardKey::C         => Key::KEY_C,
    KeyboardKey::D         => Key::KEY_D,
    KeyboardKey::E         => Key::KEY_E,
    KeyboardKey::F         => Key::KEY_F,
    KeyboardKey::G         => Key::KEY_G,
    KeyboardKey::H         => Key::KEY_H,
    KeyboardKey::I         => Key::KEY_I,
    KeyboardKey::J         => Key::KEY_J,
    KeyboardKey::K         => Key::KEY_K,
    KeyboardKey::L         => Key::KEY_L,
    KeyboardKey::M         => Key::KEY_M,
    KeyboardKey::N         => Key::KEY_N,
    KeyboardKey::O         => Key::KEY_O,
    KeyboardKey::P         => Key::KEY_P,
    KeyboardKey::Q         => Key::KEY_Q,
    KeyboardKey::R         => Key::KEY_R,
    KeyboardKey::S         => Key::KEY_S,
    KeyboardKey::T         => Key::KEY_T,
    KeyboardKey::U         => Key::KEY_U,
    KeyboardKey::V         => Key::KEY_V,
    KeyboardKey::W         => Key::KEY_W,
    KeyboardKey::X         => Key::KEY_X,
    KeyboardKey::Y         => Key::KEY_Y,
    KeyboardKey::Z         => Key::KEY_Z,
    KeyboardKey::Esc       => Key::KEY_ESC,
    KeyboardKey::Enter     => Key::KEY_ENTER,
    KeyboardKey::Space     => Key::KEY_SPACE,
    KeyboardKey::Ctrl      => Key::KEY_LEFTCTRL,
    KeyboardKey::Shift     => Key::KEY_LEFTSHIFT,
    KeyboardKey::Tab       => Key::KEY_TAB,
    KeyboardKey::Alt       => Key::KEY_LEFTALT,
    KeyboardKey::_1        => Key::KEY_1,
    KeyboardKey::_2        => Key::KEY_2,
    KeyboardKey::_3        => Key::KEY_3,
    KeyboardKey::_4        => Key::KEY_4,
    KeyboardKey::_5        => Key::KEY_5,
    KeyboardKey::_6        => Key::KEY_6,
    KeyboardKey::_7        => Key::KEY_7,
    KeyboardKey::_8        => Key::KEY_8,
    KeyboardKey::_9        => Key::KEY_9,
    KeyboardKey::_0        => Key::KEY_0,
    KeyboardKey::KP1       => Key::KEY_KP1,
    KeyboardKey::KP2       => Key::KEY_KP2,
    KeyboardKey::KP3       => Key::KEY_KP3,
    KeyboardKey::KP4       => Key::KEY_KP4,
    KeyboardKey::KP5       => Key::KEY_KP5,
    KeyboardKey::KP6       => Key::KEY_KP6,
    KeyboardKey::KP7       => Key::KEY_KP7,
    KeyboardKey::KP8       => Key::KEY_KP8,
    KeyboardKey::KP9       => Key::KEY_KP9,
    KeyboardKey::KP0       => Key::KEY_KP0,
    KeyboardKey::Up        => Key::KEY_UP ,
    KeyboardKey::Left      => Key::KEY_LEFT ,
    KeyboardKey::Down      => Key::KEY_DOWN ,
    KeyboardKey::Right     => Key::KEY_RIGHT,
    KeyboardKey::PageDown  => Key::KEY_PAGEDOWN,
    KeyboardKey::PageUp    => Key::KEY_PAGEUP,
    KeyboardKey::Backslash => Key::KEY_BACKSLASH
  }
}

fn mouse_button_to_evdev_type(button: MouseButton) -> Key {
  match button {
    MouseButton::Left   => Key::BTN_LEFT,
    MouseButton::Right  => Key::BTN_RIGHT,
    MouseButton::Middle => Key::BTN_MIDDLE
  }
}

pub struct UInputDev {
  vdev: evdev::uinput::VirtualDevice
}

#[allow(dead_code)]
pub enum KeyEvent {
  Up     = 0,
  Down   = 1,
  Repeat = 2
}

impl UInputDev {

  pub fn keyboard_key_event(&mut self, key: KeyboardKey, value: KeyEvent) {
    self.vdev.emit(&[InputEvent::new(EventType::KEY, keyboard_key_to_evdev_type(key).code(), value as i32)]).unwrap();
  }

  pub fn mouse_button_event(&mut self, button: MouseButton, value: KeyEvent) {
    self.vdev.emit(&[InputEvent::new(EventType::KEY, mouse_button_to_evdev_type(button).code(), value as i32)]).unwrap();
  }

  pub fn mouse_xy_event(&mut self, x: i32, y: i32) {
    self.vdev.emit(&[
      InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_X.0, x),
      InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_Y.0, y)
    ]).unwrap();
  }

  pub fn mouse_wheel_event(&mut self, value: i32) {
    self.vdev.emit(&[InputEvent::new(EventType::RELATIVE, RelativeAxisType::REL_WHEEL.0, value)]).unwrap();
  }
}

#[allow(dead_code)]
pub enum OutputDevice {
  Keyboard,
  Mouse,
  Gamepad
}

pub fn create_device(device_type: OutputDevice) -> Result<UInputDev, String> {

  match device_type {
    OutputDevice::Keyboard => {

      let mut keys = AttributeSet::<Key>::new();

      //TODO: enumerate all keys
      for key in [
        KeyboardKey::A,
        KeyboardKey::S,
        KeyboardKey::D,
        KeyboardKey::Q,
        KeyboardKey::W,
        KeyboardKey::E,
        KeyboardKey::R,
        KeyboardKey::T,
        KeyboardKey::Y,
        KeyboardKey::I,
        KeyboardKey::M,
        KeyboardKey::C,
        KeyboardKey::V,
        KeyboardKey::G,
        KeyboardKey::F,
        KeyboardKey::L,
        KeyboardKey::J,
        KeyboardKey::H,
        KeyboardKey::Z,
        KeyboardKey::X,
        KeyboardKey::Esc,
        KeyboardKey::Enter,
        KeyboardKey::Space,
        KeyboardKey::Ctrl,
        KeyboardKey::Shift,
        KeyboardKey::Tab,
        KeyboardKey::Alt,
        KeyboardKey::_1,
        KeyboardKey::_2,
        KeyboardKey::_3,
        KeyboardKey::_4,
        KeyboardKey::_5,
        KeyboardKey::_6,
        KeyboardKey::_7,
        KeyboardKey::_8,
        KeyboardKey::_9,
        KeyboardKey::_0,
        KeyboardKey::KP1,
        KeyboardKey::KP2,
        KeyboardKey::KP3,
        KeyboardKey::KP4,
        KeyboardKey::KP5,
        KeyboardKey::KP6,
        KeyboardKey::KP7,
        KeyboardKey::KP8,
        KeyboardKey::KP9,
        KeyboardKey::KP0,
        KeyboardKey::Up,
        KeyboardKey::Left,
        KeyboardKey::Down,
        KeyboardKey::Right,
        KeyboardKey::PageDown,
        KeyboardKey::PageUp
      ] {
        keys.insert(keyboard_key_to_evdev_type(key));
      }

      let device = VirtualDeviceBuilder::new()
        .map_err(|e| format!("{}", e))?
        .name("stwgs keyboard")
        .with_keys(&keys)
        .map_err(|e| format!("{}", e))?
        .build()
        .map_err(|e| format!("{}", e))?;

      Ok(UInputDev { vdev: device })
    },

    OutputDevice::Mouse => {

      let mut keys = AttributeSet::<Key>::new();
      keys.insert(Key::BTN_LEFT);
      keys.insert(Key::BTN_RIGHT);
      keys.insert(Key::BTN_MIDDLE);

      let mut axes = AttributeSet::<RelativeAxisType>::new();
      axes.insert(RelativeAxisType::REL_X);
      axes.insert(RelativeAxisType::REL_Y);
      axes.insert(RelativeAxisType::REL_WHEEL);

      let device = VirtualDeviceBuilder::new()
        .map_err(|e| format!("{}", e))?
        .name("stwgs mouse")
        .with_keys(&keys)
        .map_err(|e| format!("{}", e))?
        .with_relative_axes(&axes)
        .map_err(|e| format!("{}", e))?
        .build()
        .map_err(|e| format!("{}", e))?;

      Ok(UInputDev { vdev: device })
    },

    OutputDevice::Gamepad => {
      unimplemented!()
    }
  }
}
