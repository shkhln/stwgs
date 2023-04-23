use xcb::x::{GetKeyboardMapping, Window};
use xcb::xtest::FakeInput;
use xcb::Xid;

use super::{KeyboardKey, MapperIO, MouseButton};

fn get_keyboard_key_index(key: KeyboardKey) -> usize {
  match key {
    KeyboardKey::A         =>  0,
    KeyboardKey::B         =>  1,
    KeyboardKey::C         =>  2,
    KeyboardKey::D         =>  3,
    KeyboardKey::E         =>  4,
    KeyboardKey::F         =>  5,
    KeyboardKey::G         =>  6,
    KeyboardKey::H         =>  7,
    KeyboardKey::I         =>  8,
    KeyboardKey::J         =>  9,
    KeyboardKey::K         => 10,
    KeyboardKey::L         => 11,
    KeyboardKey::M         => 12,
    KeyboardKey::N         => 13,
    KeyboardKey::O         => 14,
    KeyboardKey::P         => 15,
    KeyboardKey::Q         => 16,
    KeyboardKey::R         => 17,
    KeyboardKey::S         => 18,
    KeyboardKey::T         => 19,
    KeyboardKey::U         => 20,
    KeyboardKey::V         => 21,
    KeyboardKey::W         => 22,
    KeyboardKey::X         => 23,
    KeyboardKey::Y         => 24,
    KeyboardKey::Z         => 25,
    KeyboardKey::Esc       => 26,
    KeyboardKey::Enter     => 27,
    KeyboardKey::Space     => 28,
    KeyboardKey::Ctrl      => 29,
    KeyboardKey::Shift     => 30,
    KeyboardKey::Tab       => 31,
    KeyboardKey::Alt       => 32,
    KeyboardKey::_1        => 33,
    KeyboardKey::_2        => 34,
    KeyboardKey::_3        => 35,
    KeyboardKey::_4        => 36,
    KeyboardKey::_5        => 37,
    KeyboardKey::_6        => 38,
    KeyboardKey::_7        => 39,
    KeyboardKey::_8        => 40,
    KeyboardKey::_9        => 41,
    KeyboardKey::_0        => 42,
    KeyboardKey::KP1       => 43,
    KeyboardKey::KP2       => 44,
    KeyboardKey::KP3       => 45,
    KeyboardKey::KP4       => 46,
    KeyboardKey::KP5       => 47,
    KeyboardKey::KP6       => 48,
    KeyboardKey::KP7       => 49,
    KeyboardKey::KP8       => 50,
    KeyboardKey::KP9       => 51,
    KeyboardKey::KP0       => 52,
    KeyboardKey::Up        => 53,
    KeyboardKey::Left      => 54,
    KeyboardKey::Down      => 55,
    KeyboardKey::Right     => 56,
    KeyboardKey::PageDown  => 57,
    KeyboardKey::PageUp    => 58,
    KeyboardKey::Backslash => 59
  }
}

fn keysym_to_keyboard_key(keysym: std::os::raw::c_uint) -> Option<KeyboardKey> {
  match keysym {
    x11::keysym::XK_A         => Some(KeyboardKey::A),
    x11::keysym::XK_B         => Some(KeyboardKey::B),
    x11::keysym::XK_C         => Some(KeyboardKey::C),
    x11::keysym::XK_D         => Some(KeyboardKey::D),
    x11::keysym::XK_E         => Some(KeyboardKey::E),
    x11::keysym::XK_F         => Some(KeyboardKey::F),
    x11::keysym::XK_G         => Some(KeyboardKey::G),
    x11::keysym::XK_H         => Some(KeyboardKey::H),
    x11::keysym::XK_I         => Some(KeyboardKey::I),
    x11::keysym::XK_J         => Some(KeyboardKey::J),
    x11::keysym::XK_K         => Some(KeyboardKey::K),
    x11::keysym::XK_L         => Some(KeyboardKey::L),
    x11::keysym::XK_M         => Some(KeyboardKey::M),
    x11::keysym::XK_N         => Some(KeyboardKey::N),
    x11::keysym::XK_O         => Some(KeyboardKey::O),
    x11::keysym::XK_P         => Some(KeyboardKey::P),
    x11::keysym::XK_Q         => Some(KeyboardKey::Q),
    x11::keysym::XK_R         => Some(KeyboardKey::R),
    x11::keysym::XK_S         => Some(KeyboardKey::S),
    x11::keysym::XK_T         => Some(KeyboardKey::T),
    x11::keysym::XK_U         => Some(KeyboardKey::U),
    x11::keysym::XK_V         => Some(KeyboardKey::V),
    x11::keysym::XK_W         => Some(KeyboardKey::W),
    x11::keysym::XK_X         => Some(KeyboardKey::X),
    x11::keysym::XK_Y         => Some(KeyboardKey::Y),
    x11::keysym::XK_Z         => Some(KeyboardKey::Z),
    x11::keysym::XK_Escape    => Some(KeyboardKey::Esc),
    x11::keysym::XK_Return    => Some(KeyboardKey::Enter),
    x11::keysym::XK_space     => Some(KeyboardKey::Space),
    x11::keysym::XK_Control_L => Some(KeyboardKey::Ctrl),
    x11::keysym::XK_Shift_L   => Some(KeyboardKey::Shift),
    x11::keysym::XK_Tab       => Some(KeyboardKey::Tab),
    x11::keysym::XK_Alt_L     => Some(KeyboardKey::Alt),
    x11::keysym::XK_1         => Some(KeyboardKey::_1),
    x11::keysym::XK_2         => Some(KeyboardKey::_2),
    x11::keysym::XK_3         => Some(KeyboardKey::_3),
    x11::keysym::XK_4         => Some(KeyboardKey::_4),
    x11::keysym::XK_5         => Some(KeyboardKey::_5),
    x11::keysym::XK_6         => Some(KeyboardKey::_6),
    x11::keysym::XK_7         => Some(KeyboardKey::_7),
    x11::keysym::XK_8         => Some(KeyboardKey::_8),
    x11::keysym::XK_9         => Some(KeyboardKey::_9),
    x11::keysym::XK_0         => Some(KeyboardKey::_0),
    x11::keysym::XK_KP_1      => Some(KeyboardKey::KP1),
    x11::keysym::XK_KP_2      => Some(KeyboardKey::KP2),
    x11::keysym::XK_KP_3      => Some(KeyboardKey::KP3),
    x11::keysym::XK_KP_4      => Some(KeyboardKey::KP4),
    x11::keysym::XK_KP_5      => Some(KeyboardKey::KP5),
    x11::keysym::XK_KP_6      => Some(KeyboardKey::KP6),
    x11::keysym::XK_KP_7      => Some(KeyboardKey::KP7),
    x11::keysym::XK_KP_8      => Some(KeyboardKey::KP8),
    x11::keysym::XK_KP_9      => Some(KeyboardKey::KP9),
    x11::keysym::XK_KP_0      => Some(KeyboardKey::KP0),
    x11::keysym::XK_Up        => Some(KeyboardKey::Up),
    x11::keysym::XK_Left      => Some(KeyboardKey::Left),
    x11::keysym::XK_Down      => Some(KeyboardKey::Down),
    x11::keysym::XK_Right     => Some(KeyboardKey::Right),
    x11::keysym::XK_Page_Down => Some(KeyboardKey::PageDown),
    x11::keysym::XK_Page_Up   => Some(KeyboardKey::PageUp),
    x11::keysym::XK_backslash => Some(KeyboardKey::Backslash),
    _ => None
  }
}

fn get_mouse_button_code(btn: MouseButton) -> u8 {
  match btn {
    MouseButton::Left   => 1,
    MouseButton::Middle => 2,
    MouseButton::Right  => 3
  }
}

pub struct XcbKeyboardAndMouse {
  connection: xcb::Connection,
  rel_x:      i32,
  rel_y:      i32,
  keycodes:   [u8; 101]
}

impl XcbKeyboardAndMouse {

  pub fn new() -> Result<Self, String> {
    let (connection, _) = xcb::Connection::connect(None).map_err(|e| format!("{}", e))?;

    let (min_keycode, max_keycode) = {
      let setup = connection.get_setup();
      (setup.min_keycode(), setup.max_keycode())
    };

    let mut keycodes: [u8; 101] = [0; 101];

    for keycode in min_keycode..=max_keycode {
      let cookie = connection.send_request(&GetKeyboardMapping { first_keycode: keycode, count: 1 });
      if let Ok(reply) = connection.wait_for_reply(cookie) {
        for keysym in reply.keysyms() {
          if let Some(key) = keysym_to_keyboard_key(*keysym) {
            let i = get_keyboard_key_index(key);
            if keycodes[i] == 0 {
              keycodes[i] = keycode;
            }
            break;
          }
        }
      } else {
        panic!()
      }
    }

    Ok(Self { connection, rel_x: 0, rel_y: 0, keycodes })
  }
}

use x11::xinput2::{
  XI_ButtonPress   as X11_BUTTON_PRESS,
  XI_ButtonRelease as X11_BUTTON_RELEASE,
  XI_KeyPress      as X11_KEY_PRESS,
  XI_KeyRelease    as X11_KEY_RELEASE,
  XI_Motion        as X11_MOTION
};

fn fake_input_request(event: i32, code: u8, x: i16, y: i16) -> FakeInput {
  FakeInput {
    r#type:   event as u8,
    detail:   code,
    time:     0,
    root:     Window::none(),
    root_x:   x,
    root_y:   y,
    deviceid: 0
  }
}

//TODO: poll for errors
impl MapperIO for XcbKeyboardAndMouse {

  fn keyboard_key_down(&mut self, key: KeyboardKey) {
    let keycode = self.keycodes[get_keyboard_key_index(key)];
    self.connection.send_request(&fake_input_request(X11_KEY_PRESS, keycode, 0, 0));
  }

  fn keyboard_key_up(&mut self, key: KeyboardKey) {
    let keycode = self.keycodes[get_keyboard_key_index(key)];
    self.connection.send_request(&fake_input_request(X11_KEY_RELEASE, keycode, 0, 0));
  }

  fn mouse_button_down(&mut self, btn: MouseButton) {
    let code = get_mouse_button_code(btn);
    self.connection.send_request(&fake_input_request(X11_BUTTON_PRESS, code, 0, 0));
  }

  fn mouse_button_up(&mut self, btn: MouseButton) {
    let code = get_mouse_button_code(btn);
    self.connection.send_request(&fake_input_request(X11_BUTTON_RELEASE, code, 0, 0));
  }

  fn mouse_cursor_rel_xy(&mut self, x: i32, y: i32) {
    self.rel_x += x;
    self.rel_y += y;
  }

  fn mouse_wheel_rel(&mut self, value: i32) {
    let code = if value > 0 { 4 } else { 5 };
    for _ in 0..value.abs() {
      self.connection.send_request(&fake_input_request(X11_BUTTON_PRESS,   code, 0, 0));
      self.connection.send_request(&fake_input_request(X11_BUTTON_RELEASE, code, 0, 0));
    }
  }

  fn syn(&mut self) {
    let x = self.rel_x as i16;
    let y = self.rel_y as i16;
    if x != 0 || y != 0 {
      self.connection.send_request(&fake_input_request(X11_MOTION, 1, x, y));
      self.rel_x = 0;
      self.rel_y = 0;
    }
    self.connection.flush().unwrap();
  }
}
