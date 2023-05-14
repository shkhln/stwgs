use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use libusb::{Direction, Recipient, RequestType};

use crate::controllers::*;

struct SteamController {
  bus_number:  u8,
  bus_address: u8,
  serial:      String
}

fn libusb_err_to_string(err: libusb::Error) -> String {
  format!("{}", err)
}

fn is_steam_controller(device: &libusb::Device) -> Result<bool, String> {
  let desc    = device.device_descriptor().map_err(libusb_err_to_string)?;
  let vendor  = desc.vendor_id();
  let product = desc.product_id();
  Ok(vendor == 0x28de && (product == 0x1102 /* wired */ || product == 0x1142 /* wireless */))
}

pub fn available_controllers() -> Result<Vec<Box<dyn Controller>>, String> {
  let mut controllers: Vec<Box<dyn Controller>> = vec![];

  let context = libusb::Context::new().map_err(libusb_err_to_string)?;
  let devices = context.devices().map_err(libusb_err_to_string)?;

  for device in devices.iter() {
    if is_steam_controller(&device)? {
      let handle = device.open().map_err(libusb_err_to_string)?;
      let serial = get_serial_number(&handle)?;
      controllers.push(Box::new(SteamController { bus_number: device.bus_number(), bus_address: device.address(), serial }));
    }
  }

  Ok(controllers)
}

//TODO: verify commands for enabling/disabling gyro
const ENABLE_GYRO: [u8; 32] = [
  0x87,
  0x15,

  0x32,
  0x84,
  0x03, // timeout

  0x18,
  0x00,
  0x00,

  0x31,
  0x02,
  0x00,

  0x08,
  0x07,
  0x00,

  0x07,
  0x07,
  0x00,
  0x30,

  0x14, //0x00,

  0x00,
  0x2e, //0x2f,

  0x01,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00,
  0x00
];

fn disable_lizard_mode(handle: &libusb::DeviceHandle) -> Result<(), String> {
  let request_type = libusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, 2, &[0x81], Duration::new(0, 0)).map_err(libusb_err_to_string)?;
  assert_eq!(transferred, 1);
  Ok(())
}

fn get_serial_number(handle: &libusb::DeviceHandle) -> Result<String, String> {
  let request_type = libusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, 2, &[0xae, 0x15, 0x01], Duration::new(0, 0))
    .map_err(libusb_err_to_string)?;
  assert_eq!(transferred, 3);

  let mut buffer = [0_u8; 64];

  let request_type = libusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
  let transferred  = handle.read_control(request_type, 0x01, 0x0300, 2, &mut buffer, Duration::new(0, 0))
    .map_err(libusb_err_to_string)?;
  assert_eq!(transferred, buffer.len());

  Ok(String::from_utf8_lossy(&buffer[3..=12]).to_string())
}

/*fn enable_lizard_mode(handle: &libusb::DeviceHandle) {

  let transferred = handle.write_control(0x21, 0x09, 0x0300, 2, &[0x85], Duration::new(0, 0)).unwrap();
  assert_eq!(transferred, 1);

  let transferred = handle.write_control(0x21, 0x09, 0x0300, 2, &[0x8e], Duration::new(0, 0)).unwrap();
  assert_eq!(transferred, 1);

  // ???
}*/

fn enable_gyro(handle: &libusb::DeviceHandle) -> Result<(), String> {
  let request_type = libusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, 2, &ENABLE_GYRO, Duration::new(0, 0)).map_err(libusb_err_to_string)?;
  assert_eq!(transferred, ENABLE_GYRO.len());
  Ok(())
}

impl Controller for SteamController {

  fn name(&self) -> String {
    "Steam Controller".to_string()
  }

  fn path(&self) -> String {
    format!("//steam/usb/{}:{}", self.bus_number, self.bus_address)
  }

  fn serial(&self) -> Option<String> {
    Some(self.serial.clone())
  }

  fn run_polling_loop(&self, sender: Sender<ControllerState>, receiver: Option<Receiver<ControllerCommand>>) -> Result<(), String> {
    let context = libusb::Context::new().map_err(libusb_err_to_string)?;
    let devices = context.devices().map_err(libusb_err_to_string)?;

    let device = devices
      .iter()
      .find(|d| d.bus_number() == self.bus_number && d.address() == self.bus_address && is_steam_controller(d).is_ok())
      .ok_or("No controller found.")?;

    let handle = device.open().map_err(libusb_err_to_string)?;
    assert_eq!(get_serial_number(&handle)?, self.serial);

    disable_lizard_mode(&handle)?;
    enable_gyro(&handle)?;

    let mut buffer = [0_u8; 64];
    let mut state  = ControllerState::empty();

    let  pad_scale_factor = 1f32 / i16::MAX as f32;
    let trig_scale_factor = 1f32 /  u8::MAX as f32;

    let lpad_rotation_angle     = -std::f32::consts::PI / 180.0 * 15.0;
    let rpad_rotation_angle     =  std::f32::consts::PI / 180.0 * 15.0;
    let lpad_rotation_angle_cos = lpad_rotation_angle.cos();
    let lpad_rotation_angle_sin = lpad_rotation_angle.sin();
    let rpad_rotation_angle_cos = rpad_rotation_angle.cos();
    let rpad_rotation_angle_sin = rpad_rotation_angle.sin();

    loop {
      let transferred = handle.read_interrupt(0x80 | 0x03, &mut buffer, Duration::new(0, 0)).map_err(libusb_err_to_string)?;
      assert_eq!(transferred, buffer.len());

      if buffer[2] == 0x01 && buffer[3] == 0x3c /* as opposed to 0x0b04 */ {

        let buttons1 = buffer[ 8];
        let buttons2 = buffer[ 9];
        let buttons3 = buffer[10];

        state.buttons.lstick     = (buttons3 & 1 << 6) != 0; // 1000010
        state.buttons.rpad_touch = (buttons3 & 1 << 4) != 0;
        state.buttons.lpad_touch = (buttons3 & 1 << 3) != 0 || (buttons3 & 1 << 7) != 0;
        state.buttons.rpad_press = (buttons3 & 1 << 2) != 0;
        state.buttons.lpad_press = (buttons3 & 1 << 1) != 0 && state.buttons.lpad_touch;
        state.buttons.rgrip      = (buttons3 & 1 << 0) != 0;
        state.buttons.lgrip      = (buttons2 & 1 << 7) != 0;
        state.buttons.start      = (buttons2 & 1 << 6) != 0;
        state.buttons.steam      = (buttons2 & 1 << 5) != 0;
        state.buttons.back       = (buttons2 & 1 << 4) != 0;
        //state.buttons.lpad_down  = (buttons2 & 1 << 3) != 0;
        //state.buttons.lpad_left  = (buttons2 & 1 << 2) != 0;
        //state.buttons.lpad_right = (buttons2 & 1 << 1) != 0;
        //state.buttons.lpad_up    = (buttons2 & 1 << 0) != 0;
        state.buttons.a          = (buttons1 & 1 << 7) != 0;
        state.buttons.x          = (buttons1 & 1 << 6) != 0;
        state.buttons.b          = (buttons1 & 1 << 5) != 0;
        state.buttons.y          = (buttons1 & 1 << 4) != 0;
        state.buttons.lbump      = (buttons1 & 1 << 3) != 0;
        state.buttons.rbump      = (buttons1 & 1 << 2) != 0;
        state.buttons.ltrig      = (buttons1 & 1 << 1) != 0;
        state.buttons.rtrig      = (buttons1 & 1 << 0) != 0;

        state.axes.ltrig = buffer[11] as f32 * trig_scale_factor;
        state.axes.rtrig = buffer[12] as f32 * trig_scale_factor;

        if (buttons3 & 1 << 3) == 0 {
          state.axes.ljoy_x = ((buffer[17] as i16) << 8 | buffer[16] as i16) as f32 * pad_scale_factor;
          state.axes.ljoy_y = ((buffer[19] as i16) << 8 | buffer[18] as i16) as f32 * pad_scale_factor;
        }

        if state.buttons.lpad_touch {

          let lpad_x = ((buffer[59] as i16) << 8 | buffer[58] as i16) as f32 * pad_scale_factor;
          let lpad_y = ((buffer[61] as i16) << 8 | buffer[60] as i16) as f32 * pad_scale_factor;

          state.axes.lpad_x = lpad_x * lpad_rotation_angle_cos - lpad_y * lpad_rotation_angle_sin;
          state.axes.lpad_y = lpad_x * lpad_rotation_angle_sin + lpad_y * lpad_rotation_angle_cos;

        } else {
          state.axes.lpad_x = 0.0;
          state.axes.lpad_y = 0.0;
        }

        let rpad_x = ((buffer[21] as i16) << 8 | buffer[20] as i16) as f32 * pad_scale_factor;
        let rpad_y = ((buffer[23] as i16) << 8 | buffer[22] as i16) as f32 * pad_scale_factor;

        state.axes.rpad_x = rpad_x * rpad_rotation_angle_cos - rpad_y * rpad_rotation_angle_sin;
        state.axes.rpad_y = rpad_x * rpad_rotation_angle_sin + rpad_y * rpad_rotation_angle_cos;

        //TODO: normalize this
        state.axes.ax     = ((buffer[29] as i16) << 8 | buffer[28] as i16) as f32; // left <-> right
        state.axes.ay     = ((buffer[31] as i16) << 8 | buffer[30] as i16) as f32; // back <-> forward
        state.axes.az     = ((buffer[33] as i16) << 8 | buffer[32] as i16) as f32; // down <-> up

        state.axes.pitch  = ((buffer[35] as i16) << 8 | buffer[34] as i16) as f32;
        state.axes.roll   = ((buffer[37] as i16) << 8 | buffer[36] as i16) as f32;
        state.axes.yaw    = ((buffer[39] as i16) << 8 | buffer[38] as i16) as f32;

        state.axes.q0     = (buffer[41] as i16) << 8 | buffer[40] as i16;
        state.axes.q1     = (buffer[43] as i16) << 8 | buffer[42] as i16;
        state.axes.q2     = (buffer[45] as i16) << 8 | buffer[44] as i16;
        state.axes.q3     = (buffer[47] as i16) << 8 | buffer[46] as i16;

        {
          const LENGTH: f32 = i16::MAX as f32;

          let qx = state.axes.q1 as f32 / LENGTH;
          let qy = state.axes.q2 as f32 / LENGTH;
          let qz = state.axes.q3 as f32 / LENGTH;
          let qw = state.axes.q0 as f32 / LENGTH;

          let qxx = qx.powi(2);
          let qyy = qy.powi(2);
          let qzz = qz.powi(2);
          let qww = qw.powi(2);

          let abs_yaw   = (2.0 * (qw * qz - qx * qy)).atan2(qww - qxx + qyy - qzz);
          let abs_pitch = (2.0 * (qy * qz + qw * qx)).asin();
          let abs_roll  = (2.0 * (qw * qy - qx * qz)).atan2(qww - qxx - qyy + qzz);

          state.axes.a_pitch = abs_pitch;
          state.axes.a_roll  = abs_roll;
          state.axes.a_yaw   = abs_yaw;
        }

        sender.send(state).map_err(|e| format!("{}", e))?;
      } else {
        sender.send(state).map_err(|e| format!("{}", e))?; // repeat prev state
      }

      if let Some(receiver) = &receiver {
        if let Ok(command) = receiver.try_recv() {
          match command {
            ControllerCommand::HapticFeedback(target, effect) => {

              let touchpad = match target {
                HapticFeedbackTarget::LeftSide  => Some(1),
                HapticFeedbackTarget::RightSide => Some(0),
                _ => None
              };

              if let Some(touchpad) = touchpad {

                let (amplitude, period, count) = match effect {
                  HapticFeedbackEffect::SlightBump   => (100, 2, 25),
                  HapticFeedbackEffect::ModerateBump => (100, 2, 50),
                };

                //TODO: is this really amplitude, period, count?
                let buffer = [
                  0x8f,
                  0x07,
                  touchpad,
                  (amplitude % 0xff) as u8,
                  (amplitude / 0xff) as u8,
                  (period    % 0xff) as u8,
                  (period    / 0xff) as u8,
                  (count     % 0xff) as u8,
                  (count     / 0xff) as u8
                ];

                let request_type = libusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
                let transferred  = handle.write_control(request_type, 0x09, 0x0300, 2, &buffer, Duration::new(0, 0)).map_err(libusb_err_to_string)?;
                assert_eq!(transferred, buffer.len());
              }
            }
          }
        }
      }
    } // loop
  }
}
