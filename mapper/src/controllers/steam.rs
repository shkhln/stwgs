use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use rusb::{Direction, Recipient, RequestType, UsbContext};

use crate::controllers::*;

struct SteamController {
  bus_number:  u8,
  bus_address: u8,
  interface:   u8,
  endpoint:    u8,
  kind:        ControllerType,
  serial:      Option<String>
}

fn libusb_err_to_string(err: rusb::Error) -> String {
  err.to_string()
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum ControllerType {
  Wired,
  Wireless
}

fn get_controller_type<T: UsbContext>(device: &rusb::Device<T>) -> Result<Option<ControllerType>, String> {
  let desc = device.device_descriptor().map_err(libusb_err_to_string)?;
  match (desc.vendor_id(), desc.product_id()) {
    (0x28de, 0x1102) => Ok(Some(ControllerType::Wired)),
    (0x28de, 0x1142) => Ok(Some(ControllerType::Wireless)),
    _ => Ok(None)
  }
}

fn is_controller<T: UsbContext>(device: &rusb::Device<T>) -> bool {
  matches!(get_controller_type(device), Ok(Some(_)))
}

pub fn available_controllers() -> Result<Vec<Box<dyn Controller>>, String> {
  let mut controllers: Vec<SteamController> = vec![];

  let context = rusb::Context::new().map_err(libusb_err_to_string)?;
  let devices = context.devices().map_err(libusb_err_to_string)?;

  for device in devices.iter() {
    if let Some(controller_type) = get_controller_type(&device)? {

      let config_desc = device.active_config_descriptor().map_err(libusb_err_to_string)?;
      for interface in config_desc.interfaces() {
        for interface_desc in interface.descriptors() {

          assert_eq!(interface_desc.num_endpoints(), 1);

          if interface_desc.class_code() == 3 && interface_desc.sub_class_code() == 0 && interface_desc.protocol_code() == 0 {

            let serial = if controller_type == ControllerType::Wired {
              let handle = device.open().map_err(libusb_err_to_string)?;
              get_serial_number(&handle, interface_desc.interface_number() as u16)?
            } else {
              None
            };

            controllers.push(SteamController {
              bus_number:  device.bus_number(),
              bus_address: device.address(),
              interface:   interface_desc.interface_number(),
              endpoint:    interface_desc.interface_number() + 1,
              kind:        controller_type,
              serial
            });
          }
        }
      }

    }
  }

  controllers.sort_by_key(|controller| controller.kind);

  Ok(controllers.into_iter().map(|controller| Box::new(controller) as Box<dyn Controller>).collect())
}

fn disable_lizard_mode<T: UsbContext>(handle: &rusb::DeviceHandle<T>, index: u16) -> Result<(), String> {
  let request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, index, &[0x81], Duration::new(0, 0)).map_err(libusb_err_to_string)?;
  assert_eq!(transferred, 1);
  let request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, index, &[0x88], Duration::new(0, 0)).map_err(libusb_err_to_string)?;
  assert_eq!(transferred, 1);
  Ok(())
}

/*fn enable_lizard_mode(handle: &rusb::DeviceHandle, index: u16) -> Result<(), String> {

  let transferred = handle.write_control(0x21, 0x09, 0x0300, index, &[0x85], Duration::new(0, 0)).unwrap();
  assert_eq!(transferred, 1);

  let transferred = handle.write_control(0x21, 0x09, 0x0300, index, &[0x8e], Duration::new(0, 0)).unwrap();
  assert_eq!(transferred, 1);

  Ok(())
}*/

fn get_serial_number<T: UsbContext>(handle: &rusb::DeviceHandle<T>, index: u16) -> Result<Option<String>, String> {
  let request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, index, &[0xae, 0x1, 0x01], Duration::new(0, 0))
    .map_err(libusb_err_to_string)?;
  assert_eq!(transferred, 3);

  let mut buffer = [0_u8; 64];

  let request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
  let transferred  = handle.read_control(request_type, 0x01, 0x0300, index, &mut buffer, Duration::new(0, 0))
    .map_err(libusb_err_to_string)?;
  assert_eq!(transferred, buffer.len());

  if buffer[0] != 0xae {
    return Err("unexpected packet type".to_string());
  }

  if buffer[1] == 0 {
    return Ok(None)
  }

  let payload = &buffer[3..=(3 + buffer[1] as usize)];
  if let Some(i) = payload.iter().position(|&char| char == 0) {
    Ok(Some(String::from_utf8_lossy(&payload[..i]).to_string()))
  } else {
    Err("malformed serial".to_string())
  }
}

//TODO: what's the default inactivity timeout setting?
const DISABLE_MOUSE_SMOOTH:      [u8; 3] = [0x18, 0x00, 0x00];
const ENABLE_GYRO:               [u8; 3] = [0x30, 0x18, 0x00];
const SET_WIRELESS_PACKET_VER_2: [u8; 3] = [0x31, 0x02, 0x00];
const UNSET_LPAD_MODE:           [u8; 3] = [0x07, 0x07, 0x00];
const UNSET_RPAD_MODE:           [u8; 3] = [0x08, 0x07, 0x00];

fn update_settings<T: UsbContext>(handle: &rusb::DeviceHandle<T>, index: u16, settings: &[[u8; 3]]) -> Result<(), String> {
  let mut buffer = [0_u8; 64];
  assert!(settings.len() <= (buffer.len() - 2) / 3);
  buffer[0] = 0x87;
  buffer[1] = settings.len() as u8 * 3;
  for (i, settings) in settings.iter().enumerate() {
    buffer[2 + i * 3]     = settings[0];
    buffer[2 + i * 3 + 1] = settings[1];
    buffer[2 + i * 3 + 2] = settings[2];
  }
  let request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
  let transferred  = handle.write_control(request_type, 0x09, 0x0300, index, &buffer, Duration::new(0, 0)).map_err(libusb_err_to_string)?;
  assert_eq!(transferred, buffer.len());
  Ok(())
}

fn prepare_controller<T: UsbContext>(handle: &rusb::DeviceHandle<T>, index: u16) -> Result<(), String> {
  disable_lizard_mode(handle, index)?;
  update_settings(handle, index, &[
    SET_WIRELESS_PACKET_VER_2,
    UNSET_LPAD_MODE,
    UNSET_RPAD_MODE,
    DISABLE_MOUSE_SMOOTH,
    ENABLE_GYRO
  ])?;
  Ok(())
}

impl Controller for SteamController {

  fn name(&self) -> String {
    format!("{} Steam Controller", if self.kind == ControllerType::Wired { "Wired" } else { "Wireless" })
  }

  fn path(&self) -> String {
    format!("//steam/usb/{}:{}/{}", self.bus_number, self.bus_address, self.endpoint)
  }

  fn serial(&self) -> Option<String> {
    self.serial.clone()
  }

  fn run_polling_loop(&self, sender: Sender<ControllerState>, receiver: Option<Receiver<ControllerCommand>>) -> Result<(), String> {
    let context = rusb::Context::new().map_err(libusb_err_to_string)?;
    let devices = context.devices().map_err(libusb_err_to_string)?;

    let device = devices
      .iter()
      .find(|d| d.bus_number() == self.bus_number && d.address() == self.bus_address && is_controller(d))
      .ok_or("No controller found.")?;

    let mut handle = device.open().map_err(libusb_err_to_string)?;
    handle.claim_interface(self.interface).map_err(libusb_err_to_string)?;

    if self.kind == ControllerType::Wired {
      assert_eq!(get_serial_number(&handle, self.interface as u16)?, self.serial);
      prepare_controller(&handle, self.interface as u16)?;
    }

    let mut buffer = [0_u8; 64];
    let mut state  = ControllerState::empty();

    let accel_scale_factor = 1f32 / 32768.0 * 2.0 * 9.80665;
    let  gyro_scale_factor = 1f32 / 32768.0 * (2000.0 * std::f32::consts::PI / 180.0);
    let   pad_scale_factor = 1f32 / 32768.0;
    let  trig_scale_factor = 1f32 / u8::MAX as f32;

    let lpad_rotation_angle     = -std::f32::consts::PI / 180.0 * 15.0;
    let rpad_rotation_angle     =  std::f32::consts::PI / 180.0 * 15.0;
    let lpad_rotation_angle_cos = lpad_rotation_angle.cos();
    let lpad_rotation_angle_sin = lpad_rotation_angle.sin();
    let rpad_rotation_angle_cos = rpad_rotation_angle.cos();
    let rpad_rotation_angle_sin = rpad_rotation_angle.sin();

    loop {
      let transferred = handle.read_interrupt(0x80 | self.endpoint, &mut buffer, Duration::new(0, 0)).map_err(libusb_err_to_string)?;
      assert_eq!(transferred, buffer.len());

      if buffer[2] == 0x01 /* state */ && buffer[3] == 0x3c {

        let buttons1 = buffer[ 8];
        let buttons2 = buffer[ 9];
        let buttons3 = buffer[10];

        state.buttons.lstick     = (buttons3 & 1 << 6) != 0; // 1000010
        state.buttons.rpad_touch = (buttons3 & 1 << 4) != 0;
        state.buttons.lpad_touch = (buttons3 & 1 << 3) != 0 || (buttons3 & 1 << 7) != 0;
        state.buttons.rpad_press = (buttons3 & 1 << 2) != 0;
        state.buttons.lpad_press = (buttons3 & 1 << 1) != 0;
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

          if (buttons3 & 1 << 7) == 0 {
            state.axes.lpad_x = 0.0;
            state.axes.lpad_y = 0.0;
          }
        } else {
          let lpad_x = ((buffer[17] as i16) << 8 | buffer[16] as i16) as f32 * pad_scale_factor;
          let lpad_y = ((buffer[19] as i16) << 8 | buffer[18] as i16) as f32 * pad_scale_factor;

          state.axes.lpad_x = lpad_x * lpad_rotation_angle_cos - lpad_y * lpad_rotation_angle_sin;
          state.axes.lpad_y = lpad_x * lpad_rotation_angle_sin + lpad_y * lpad_rotation_angle_cos;

          if (buttons3 & 1 << 7) == 0 {
            state.axes.ljoy_x = 0.0;
            state.axes.ljoy_y = 0.0;
          }
        }

        let rpad_x = ((buffer[21] as i16) << 8 | buffer[20] as i16) as f32 * pad_scale_factor;
        let rpad_y = ((buffer[23] as i16) << 8 | buffer[22] as i16) as f32 * pad_scale_factor;

        state.axes.rpad_x = rpad_x * rpad_rotation_angle_cos - rpad_y * rpad_rotation_angle_sin;
        state.axes.rpad_y = rpad_x * rpad_rotation_angle_sin + rpad_y * rpad_rotation_angle_cos;

        state.axes.ax     = ((buffer[29] as i16) << 8 | buffer[28] as i16) as f32 * accel_scale_factor; // left    --> right
        state.axes.ay     = ((buffer[31] as i16) << 8 | buffer[30] as i16) as f32 * accel_scale_factor; // handles --> triggers
        state.axes.az     = ((buffer[33] as i16) << 8 | buffer[32] as i16) as f32 * accel_scale_factor; // back    --> face

        state.axes.pitch  = ((buffer[35] as i16) << 8 | buffer[34] as i16) as f32 * gyro_scale_factor;
        state.axes.roll   = ((buffer[37] as i16) << 8 | buffer[36] as i16) as f32 * gyro_scale_factor;
        state.axes.yaw    = ((buffer[39] as i16) << 8 | buffer[38] as i16) as f32 * gyro_scale_factor;

        state.axes.q0     = (buffer[41] as i16) << 8 | buffer[40] as i16;
        state.axes.q1     = (buffer[43] as i16) << 8 | buffer[42] as i16;
        state.axes.q2     = (buffer[45] as i16) << 8 | buffer[44] as i16;
        state.axes.q3     = (buffer[47] as i16) << 8 | buffer[46] as i16;

        let qx = state.axes.q1 as f32 / 32768.0;
        let qy = state.axes.q2 as f32 / 32768.0;
        let qz = state.axes.q3 as f32 / 32768.0;
        let qw = state.axes.q0 as f32 / 32768.0;

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

        sender.send(state).map_err(|e| e.to_string())?;
      } else {
        sender.send(state).map_err(|e| e.to_string())?; // repeat prev state
      }

      if buffer[2] == 0x03 /* wireless packet */ && buffer[4] == 0x02 /* connected */ {
        prepare_controller(&handle, self.interface as u16)?;
      }

      if buffer[2] == 0x04 /* status */ {
        // ?
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

                let request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
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
