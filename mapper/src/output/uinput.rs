#![allow(dead_code)]

use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

use evdev::raw::{input_event, input_id};
use evdev::Key;
use nix::libc;
use nix::libc::{c_schar, c_void};

use super::{KeyboardKey, MouseAxis, MouseButton};

const BUS_USB: u16 = 0x03;

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;

const ABS_MAX: usize = 0x3f;
const ABS_CNT: usize = ABS_MAX + 1;

const REL_X: u16     = 0x00;
const REL_Y: u16     = 0x01;
const REL_WHEEL: u16 = 0x08;

const UINPUT_MAX_NAME_SIZE: usize = 80;

#[allow(non_camel_case_types)]
#[repr(C)]
struct uinput_user_dev {
  name:           [c_schar; UINPUT_MAX_NAME_SIZE],
  id:             input_id,
  ff_effects_max: u32,
  absmax:         [i32; ABS_CNT],
  absmin:         [i32; ABS_CNT],
  absfuzz:        [i32; ABS_CNT],
  absflat:        [i32; ABS_CNT]
}

impl std::default::Default for uinput_user_dev {

  fn default() -> Self {
    unsafe { std::mem::zeroed() }
  }
}

#[cfg(target_os = "freebsd")]
mod ioctl {

  use nix::libc::{c_int, c_ulong};
  use nix::{convert_ioctl_res, io, ioc};

  // based on ioctl! macro from nix @ src/sys/ioctl/mod.rs
  macro_rules! bsd_ioctl {
    (write_int $name:ident with $ioty:expr, $nr:expr) => (
      pub unsafe fn $name(fd: c_int, val: c_int) -> nix::Result<c_int> {
        nix::convert_ioctl_res!(
          nix::sys::ioctl::ioctl(fd, iowint!($ioty, $nr, std::mem::size_of::<c_int>()) as c_ulong, val))
      }
    );
  }

  macro_rules! iowint {
    ($g:expr, $n:expr, $len:expr) => (nix::ioc!(nix::sys::ioctl::VOID, $g, $n, $len))
  }

  const UINPUT_IOCTL_BASE: u8 = b'U';

  nix::ioctl!(none ui_dev_create  with UINPUT_IOCTL_BASE, 1);
  nix::ioctl!(none ui_dev_destroy with UINPUT_IOCTL_BASE, 2);

  bsd_ioctl!(write_int ui_set_evbit  with UINPUT_IOCTL_BASE, 100);
  bsd_ioctl!(write_int ui_set_keybit with UINPUT_IOCTL_BASE, 101);
  bsd_ioctl!(write_int ui_set_relbit with UINPUT_IOCTL_BASE, 102);
  bsd_ioctl!(write_int ui_set_absbit with UINPUT_IOCTL_BASE, 103);
}

pub enum KeyEvent {
  Up     = 0,
  Down   = 1,
  Repeat = 2
}

pub struct UInputDev {
  uinput: File
}

trait EvdevCode {
  fn evdev_code(&self) -> u16;
}

impl EvdevCode for KeyboardKey {

  fn evdev_code(&self) -> u16 {
    match *self {
      KeyboardKey::A         => Key::KEY_A         as u16,
      KeyboardKey::B         => Key::KEY_B         as u16,
      KeyboardKey::C         => Key::KEY_C         as u16,
      KeyboardKey::D         => Key::KEY_D         as u16,
      KeyboardKey::E         => Key::KEY_E         as u16,
      KeyboardKey::F         => Key::KEY_F         as u16,
      KeyboardKey::G         => Key::KEY_G         as u16,
      KeyboardKey::H         => Key::KEY_H         as u16,
      KeyboardKey::I         => Key::KEY_I         as u16,
      KeyboardKey::J         => Key::KEY_J         as u16,
      KeyboardKey::K         => Key::KEY_K         as u16,
      KeyboardKey::L         => Key::KEY_L         as u16,
      KeyboardKey::M         => Key::KEY_M         as u16,
      KeyboardKey::N         => Key::KEY_N         as u16,
      KeyboardKey::O         => Key::KEY_O         as u16,
      KeyboardKey::P         => Key::KEY_P         as u16,
      KeyboardKey::Q         => Key::KEY_Q         as u16,
      KeyboardKey::R         => Key::KEY_R         as u16,
      KeyboardKey::S         => Key::KEY_S         as u16,
      KeyboardKey::T         => Key::KEY_T         as u16,
      KeyboardKey::U         => Key::KEY_U         as u16,
      KeyboardKey::V         => Key::KEY_V         as u16,
      KeyboardKey::W         => Key::KEY_W         as u16,
      KeyboardKey::X         => Key::KEY_X         as u16,
      KeyboardKey::Y         => Key::KEY_Y         as u16,
      KeyboardKey::Z         => Key::KEY_Z         as u16,
      KeyboardKey::Esc       => Key::KEY_ESC       as u16,
      KeyboardKey::Enter     => Key::KEY_ENTER     as u16,
      KeyboardKey::Space     => Key::KEY_SPACE     as u16,
      KeyboardKey::Ctrl      => Key::KEY_LEFTCTRL  as u16,
      KeyboardKey::Shift     => Key::KEY_LEFTSHIFT as u16,
      KeyboardKey::Tab       => Key::KEY_TAB       as u16,
      KeyboardKey::Alt       => Key::KEY_LEFTALT   as u16,
      KeyboardKey::_1        => Key::KEY_1         as u16,
      KeyboardKey::_2        => Key::KEY_2         as u16,
      KeyboardKey::_3        => Key::KEY_3         as u16,
      KeyboardKey::_4        => Key::KEY_4         as u16,
      KeyboardKey::_5        => Key::KEY_5         as u16,
      KeyboardKey::_6        => Key::KEY_6         as u16,
      KeyboardKey::_7        => Key::KEY_7         as u16,
      KeyboardKey::_8        => Key::KEY_8         as u16,
      KeyboardKey::_9        => Key::KEY_9         as u16,
      KeyboardKey::_0        => Key::KEY_0         as u16,
      KeyboardKey::KP1       => Key::KEY_KP1       as u16,
      KeyboardKey::KP2       => Key::KEY_KP2       as u16,
      KeyboardKey::KP3       => Key::KEY_KP3       as u16,
      KeyboardKey::KP4       => Key::KEY_KP4       as u16,
      KeyboardKey::KP5       => Key::KEY_KP5       as u16,
      KeyboardKey::KP6       => Key::KEY_KP6       as u16,
      KeyboardKey::KP7       => Key::KEY_KP7       as u16,
      KeyboardKey::KP8       => Key::KEY_KP8       as u16,
      KeyboardKey::KP9       => Key::KEY_KP9       as u16,
      KeyboardKey::KP0       => Key::KEY_KP0       as u16,
      KeyboardKey::Up        => Key::KEY_UP        as u16,
      KeyboardKey::Left      => Key::KEY_LEFT      as u16,
      KeyboardKey::Down      => Key::KEY_DOWN      as u16,
      KeyboardKey::Right     => Key::KEY_RIGHT     as u16,
      KeyboardKey::PageDown  => Key::KEY_PAGEDOWN  as u16,
      KeyboardKey::PageUp    => Key::KEY_PAGEUP    as u16,
      KeyboardKey::Backslash => Key::KEY_BACKSLASH as u16,
    }
  }
}

impl EvdevCode for MouseButton {

  fn evdev_code(&self) -> u16 {
    match *self {
      MouseButton::Left   => Key::BTN_LEFT   as u16,
      MouseButton::Right  => Key::BTN_RIGHT  as u16,
      MouseButton::Middle => Key::BTN_MIDDLE as u16
    }
  }
}

impl EvdevCode for MouseAxis {

  fn evdev_code(&self) -> u16 {
    match *self {
      MouseAxis::X     => REL_X,
      MouseAxis::Y     => REL_Y,
      MouseAxis::Wheel => REL_WHEEL
    }
  }
}

impl UInputDev {

  fn write_event(&self, _type: u16, code: u16, value: i32) {
    let mut ev = input_event { _type, code, value, ..Default::default() };
    let ev_ptr = &mut ev as *mut _ as *mut c_void;

    let ret = unsafe { libc::write(self.uinput.as_raw_fd(), ev_ptr, std::mem::size_of::<input_event>()) };
    assert!(ret >= 0);
  }

  pub fn keyboard_key_event(&self, key: KeyboardKey, value: KeyEvent) {
    self.write_event(EV_KEY, key.evdev_code(), value as i32);
  }

  pub fn mouse_button_event(&self, key: MouseButton, value: KeyEvent) {
    self.write_event(EV_KEY, key.evdev_code(), value as i32);
  }

  pub fn relative_axis_event(&self, axis: MouseAxis, value: i32) {
    self.write_event(EV_REL, axis.evdev_code(), value);
  }

  pub fn syn(&self) {
    self.write_event(EV_SYN, 0, 0);
  }
}

impl Drop for UInputDev {

  fn drop(&mut self) {
    let ret = unsafe { ioctl::ui_dev_destroy(self.uinput.as_raw_fd()) };
    assert_eq!(ret, Ok(0));
  }
}

#[cfg(target_os = "freebsd")]
const UINPUT_PATH: &str = "/dev/uinput";

#[cfg(target_os = "linux")]
const UINPUT_PATH: &str = "/dev/input/uinput";

fn enable_event_type(uinput: &File, _type: i32) {
  let ret = unsafe { ioctl::ui_set_evbit(uinput.as_raw_fd(), _type) };
  assert_eq!(ret, Ok(0));
}

fn enable_key_code(uinput: &File, code: i32) {
  let ret = unsafe { ioctl::ui_set_keybit(uinput.as_raw_fd(), code) };
  assert_eq!(ret, Ok(0));
}

fn enable_rel_code(uinput: &File, code: i32) {
  let ret = unsafe { ioctl::ui_set_relbit(uinput.as_raw_fd(), code) };
  assert_eq!(ret, Ok(0));
}

fn set_name(uidev: &mut uinput_user_dev, name: &'static str) {
  assert!(name.len() <= uidev.name.len());
  for (a, c) in uidev.name.iter_mut().zip(format!("{}\0", name).bytes()) {
    *a = c as i8;
  }
}

pub enum OutputDevice {
  Keyboard,
  Mouse,
  Gamepad
}

pub fn create_device(device_type: OutputDevice) -> Result<UInputDev, String> {
  let uinput = OpenOptions::new().create(false).read(false).write(true)
    .open(UINPUT_PATH).map_err(|e| format!("{}", e))?;

  let mut uidev: uinput_user_dev = Default::default();

  //TODO: check what libevdev does
  uidev.id.bustype = BUS_USB;
  uidev.id.vendor  = 0x1234;
  uidev.id.product = 0xfedc;
  uidev.id.version = 1;

  enable_event_type(&uinput, EV_SYN as i32);

  match device_type {
    OutputDevice::Keyboard => {
      set_name(&mut uidev, "SC keyboard");

      enable_event_type(&uinput, EV_KEY as i32);

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
        enable_key_code(&uinput, key.evdev_code() as i32);
      }
    },

    OutputDevice::Mouse => {
      set_name(&mut uidev, "SC mouse");

      enable_event_type(&uinput, EV_KEY as i32);
      enable_event_type(&uinput, EV_REL as i32);

      enable_rel_code(&uinput, MouseAxis::X.evdev_code()     as i32);
      enable_rel_code(&uinput, MouseAxis::Y.evdev_code()     as i32);
      enable_rel_code(&uinput, MouseAxis::Wheel.evdev_code() as i32);

      enable_key_code(&uinput, MouseButton::Left.evdev_code()   as i32);
      enable_key_code(&uinput, MouseButton::Right.evdev_code()  as i32);
      enable_key_code(&uinput, MouseButton::Middle.evdev_code() as i32);
    },

    OutputDevice::Gamepad => {
      unimplemented!()
    }
  }

  let uidev_ptr = &mut uidev as *mut _ as *mut c_void;

  let ret = unsafe { libc::write(uinput.as_raw_fd(), uidev_ptr, std::mem::size_of::<uinput_user_dev>()) };
  assert!(ret >= 0);

  let ret = unsafe { ioctl::ui_dev_create(uinput.as_raw_fd()) };
  assert_eq!(ret, Ok(0));

  Ok(UInputDev { uinput })
}
