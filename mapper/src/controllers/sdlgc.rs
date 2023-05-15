use std::sync::{Arc, Mutex};

use crate::controllers::*;

struct SDL2Controller {
  controller: Arc<Mutex<sdl2::controller::GameController>>,
  name:       String,
  guid:       String //TODO: is this actually useful?
}

unsafe fn raw(controller: &sdl2::controller::GameController) -> *mut sdl2::sys::SDL_GameController {

  struct XGameController {
    _subsystem: sdl2::GameControllerSubsystem,
    raw:        *mut sdl2::sys::SDL_GameController
  }

  let controller: &XGameController = std::mem::transmute(controller);
  controller.raw
}

unsafe fn get_guid(joystick_index: u32) -> String {
  let guid = sdl2::sys::SDL_JoystickGetDeviceGUID(joystick_index as i32);
  let mut buffer = [0_u8; 33];
  sdl2::sys::SDL_JoystickGetGUIDString(guid, buffer.as_mut_ptr() as *mut i8, buffer.len() as i32);
  String::from_utf8_lossy(&buffer[0..32]).to_string()
}

unsafe fn get_path(controller: &sdl2::controller::GameController) -> Option<String> {

  extern "C" {
    fn SDL_GameControllerPath(gamecontroller: *mut sdl2::sys::SDL_GameController) -> *const u8;
  }

  let path = SDL_GameControllerPath(raw(controller));
  if !path.is_null() {
    Some(std::ffi::CStr::from_ptr(path as *const _).to_str().unwrap().to_owned())
  } else {
    None
  }
}

unsafe fn get_serial(controller: &sdl2::controller::GameController) -> Option<String> {
  let serial = sdl2::sys::SDL_GameControllerGetSerial(raw(controller));
  if !serial.is_null() {
    Some(std::ffi::CStr::from_ptr(serial as *const _).to_str().unwrap().to_owned())
  } else {
    None
  }
}

//TODO: filter out Steam controllers by usb vendor id?
pub fn available_controllers() -> Result<Vec<Box<dyn Controller>>, String> {
  let mut controllers: Vec<Box<dyn Controller>> = vec![];

  //sdl2::hint::set("SDL_JOYSTICK_HIDAPI_STEAM", "1");

  let context             = sdl2::init()?;
  let game_controller_sys = Arc::new(context.game_controller()?);
  let joy_count           = game_controller_sys.num_joysticks()?;

  for i in 0..joy_count {
    if game_controller_sys.is_game_controller(i) {
      let controller = Arc::new(Mutex::new(game_controller_sys.open(i).map_err(|e| format!("{}", e))?));
      let name       = game_controller_sys.name_for_index(i).map_err(|e| format!("{}", e))?;
      let guid       = unsafe { get_guid(i) };

      #[cfg(target_os = "freebsd")]
      {
        let path = unsafe { get_path(&controller.lock().unwrap()).unwrap() };
        if path.starts_with("/dev/uhid") {
          eprintln!("Ignoring {} at {} (SDL_JOYSTICK_USBHID). Use SDL_JOYSTICK_HIDAPI instead.", name, path);
          continue;
        }
      }

      controllers.push(Box::new(SDL2Controller { controller: Arc::clone(&controller), name, guid }));
    }
  }

  Ok(controllers)
}

impl Controller for SDL2Controller {

  fn name(&self) -> String {
    self.name.clone()
  }

  fn path(&self) -> String {
    format!("//sdl/{}", unsafe { get_path(&self.controller.lock().unwrap()).unwrap() })
  }

  fn serial(&self) -> Option<String> {
    unsafe { get_serial(&self.controller.lock().unwrap()) }
  }

  fn run_polling_loop(&self, sender: Sender<ControllerState>, receiver: Option<Receiver<ControllerCommand>>) -> Result<(), String> {

    let mut controller = self.controller.lock().unwrap();
    assert!(controller.attached());

    eprintln!("sdl controller mapping:    {}", controller.mapping());
    eprintln!("sdl controller has rumble: {}", controller.has_rumble());
    eprintln!("sdl controller has gyro:   {}", controller.has_sensor(sdl2::sensor::SensorType::Gyroscope));

    if controller.has_sensor(sdl2::sensor::SensorType::Accelerometer) {
      controller.sensor_set_enabled(sdl2::sensor::SensorType::Accelerometer, true).unwrap();
    }

    if controller.has_sensor(sdl2::sensor::SensorType::Gyroscope) {
      controller.sensor_set_enabled(sdl2::sensor::SensorType::Gyroscope, true).unwrap();
    }

    let axis_scale_factor = 1f32 / i16::MAX as f32;

    //TODO: handle timestamps
    let instance   = controller.instance_id();
    let mut events = controller.subsystem().sdl().event_pump()?;
    let mut state  = ControllerState::empty();

    let mut prev_gyro_data = [0.0; 3];
    let mut gyro_bias      = [0.0; 3];
    let mut gyro_average   = Average::<500>::new();
    let mut gyro_samples   = 0;

    enum CalibrationState {
      Initial,
      Continuous,
      Disabled
    }

    let mut calibration_state = CalibrationState::Initial;

    loop {
      for event in events.poll_iter() {
        match event {
          sdl2::event::Event::ControllerAxisMotion { timestamp: _, which, axis, value } => {
            //eprintln!("axis: {} {} {:?} {}", timestamp, which, axis, value);
            if which == instance {
              match axis {
                sdl2::controller::Axis::LeftX        => state.axes.ljoy_x = value as f32 *  axis_scale_factor,
                sdl2::controller::Axis::LeftY        => state.axes.ljoy_y = value as f32 * -axis_scale_factor,
                sdl2::controller::Axis::RightX       => state.axes.rjoy_x = value as f32 *  axis_scale_factor,
                sdl2::controller::Axis::RightY       => state.axes.rjoy_y = value as f32 * -axis_scale_factor,
                sdl2::controller::Axis::TriggerLeft  => state.axes.ltrig  = value as f32 *  axis_scale_factor,
                sdl2::controller::Axis::TriggerRight => state.axes.rtrig  = value as f32 *  axis_scale_factor
              }
            }
          },
          sdl2::event::Event::ControllerButtonDown { timestamp: _, which, button } => {
            //eprintln!("button dn: {} {} {:?}", timestamp, which, button);
            if which == instance {
              match button {
                sdl2::controller::Button::A             => state.buttons.a          = true,
                sdl2::controller::Button::B             => state.buttons.b          = true,
                sdl2::controller::Button::X             => state.buttons.x          = true,
                sdl2::controller::Button::Y             => state.buttons.y          = true,
                sdl2::controller::Button::Back          => state.buttons.back       = true,
                sdl2::controller::Button::Guide         => state.buttons.steam      = true,
                sdl2::controller::Button::Start         => state.buttons.start      = true,
                sdl2::controller::Button::LeftStick     => state.buttons.lstick     = true,
                sdl2::controller::Button::RightStick    => state.buttons.rstick     = true,
                sdl2::controller::Button::LeftShoulder  => state.buttons.lbump      = true,
                sdl2::controller::Button::RightShoulder => state.buttons.rbump      = true,
                sdl2::controller::Button::DPadUp        => state.buttons.dpad_up    = true,
                sdl2::controller::Button::DPadDown      => state.buttons.dpad_down  = true,
                sdl2::controller::Button::DPadLeft      => state.buttons.dpad_left  = true,
                sdl2::controller::Button::DPadRight     => state.buttons.dpad_right = true,
                sdl2::controller::Button::Misc1         => (),
                sdl2::controller::Button::Paddle1       => state.buttons.rgrip      = true,
                sdl2::controller::Button::Paddle2       => state.buttons.lgrip      = true,
                sdl2::controller::Button::Paddle3       => (),
                sdl2::controller::Button::Paddle4       => (),
                sdl2::controller::Button::Touchpad      => state.buttons.rpad_press = true
              }
            }
          },
          sdl2::event::Event::ControllerButtonUp   { timestamp: _, which, button } => {
            //eprintln!("button up: {} {} {:?}", timestamp, which, button);
            if which == instance {
              match button {
                sdl2::controller::Button::A             => state.buttons.a          = false,
                sdl2::controller::Button::B             => state.buttons.b          = false,
                sdl2::controller::Button::X             => state.buttons.x          = false,
                sdl2::controller::Button::Y             => state.buttons.y          = false,
                sdl2::controller::Button::Back          => state.buttons.back       = false,
                sdl2::controller::Button::Guide         => state.buttons.steam      = false,
                sdl2::controller::Button::Start         => state.buttons.start      = false,
                sdl2::controller::Button::LeftStick     => state.buttons.lstick     = false,
                sdl2::controller::Button::RightStick    => state.buttons.rstick     = false,
                sdl2::controller::Button::LeftShoulder  => state.buttons.lbump      = false,
                sdl2::controller::Button::RightShoulder => state.buttons.rbump      = false,
                sdl2::controller::Button::DPadUp        => state.buttons.dpad_up    = false,
                sdl2::controller::Button::DPadDown      => state.buttons.dpad_down  = false,
                sdl2::controller::Button::DPadLeft      => state.buttons.dpad_left  = false,
                sdl2::controller::Button::DPadRight     => state.buttons.dpad_right = false,
                sdl2::controller::Button::Misc1         => (),
                sdl2::controller::Button::Paddle1       => state.buttons.rgrip      = false,
                sdl2::controller::Button::Paddle2       => state.buttons.lgrip      = false,
                sdl2::controller::Button::Paddle3       => (),
                sdl2::controller::Button::Paddle4       => (),
                sdl2::controller::Button::Touchpad      => state.buttons.rpad_press = false
              }
            }
          },
          sdl2::event::Event::ControllerSensorUpdated { timestamp: _, which, sensor, data } => {
            //eprintln!("sensor: {} ({:?}) {} {:?} {:?}", timestamp, std::time::Instant::now(), which, sensor, data);
            if which == instance {
              match sensor {
                sdl2::sensor::SensorType::Accelerometer => {
                  state.axes.ax =  data[0]; // left    --> right
                  state.axes.ay = -data[2]; // handles <-- triggers
                  state.axes.az =  data[1]; // back    --> face
                },
                sdl2::sensor::SensorType::Gyroscope => {
                  //eprintln!("gyro: {:?}", data);
                  match calibration_state {
                    CalibrationState::Initial => {

                      gyro_samples += 1;

                      //TODO: check accelerometer as well
                      if is_gyro_steady(data, prev_gyro_data) {

                        gyro_average.push(data);

                        if gyro_average.buffer_is_full() {
                          gyro_bias = gyro_average.average();
                          eprintln!("Calibrated gyro in {} samples", gyro_samples);
                          calibration_state = CalibrationState::Continuous; //TODO: make toggleable
                        }

                      } else {
                        gyro_average.reset();
                      }

                      prev_gyro_data = data;
                    },
                    CalibrationState::Continuous => {

                      //TODO: check accelerometer as well
                      if is_gyro_steady(data, prev_gyro_data) {

                        gyro_average.push(data);

                        if gyro_average.buffer_is_full() {
                          gyro_bias = gyro_average.average();
                        }

                      } else {
                        gyro_average.reset();
                      }

                      state.axes.pitch = data[0] - gyro_bias[0];
                      state.axes.yaw   = data[1] - gyro_bias[1];
                      state.axes.roll  = data[2] - gyro_bias[2];
                      prev_gyro_data   = data;
                    },
                    CalibrationState::Disabled => {
                      state.axes.pitch = data[0] - gyro_bias[0];
                      state.axes.yaw   = data[1] - gyro_bias[1];
                      state.axes.roll  = data[2] - gyro_bias[2];
                      prev_gyro_data   = data;
                    }
                  }
                },
                sdl2::sensor::SensorType::Unknown => unreachable!()
              }
            }
          },
          sdl2::event::Event::ControllerDeviceAdded    { .. } => (),
          sdl2::event::Event::ControllerDeviceRemapped { .. } => (),
          sdl2::event::Event::ControllerDeviceRemoved  { .. } => (),
          sdl2::event::Event::JoyDeviceAdded           { .. } => (),
          sdl2::event::Event::JoyDeviceRemoved         { .. } => (),
          sdl2::event::Event::JoyAxisMotion            { .. } => (),
          sdl2::event::Event::JoyButtonDown            { .. } => (),
          sdl2::event::Event::JoyButtonUp              { .. } => (),
          sdl2::event::Event::JoyHatMotion             { .. } => (),
          sdl2::event::Event::Unknown { type_: 1543, .. } => (), // battery
          e => eprintln!("unhandled event: {:?}", e)
        }
      }

      sender.send(state).map_err(|e| format!("{}", e))?;

      if let Some(receiver) = &receiver {
        if let Ok(command) = receiver.try_recv() {
          match command {
            ControllerCommand::HapticFeedback(target, effect) => {

              let (amplitude, duration) = match effect {
                HapticFeedbackEffect::SlightBump   => (3275, 50),
                HapticFeedbackEffect::ModerateBump => (6550, 50)
              };

              let _ = match target {
                HapticFeedbackTarget::LeftSide     => controller.set_rumble(amplitude, 0, duration),
                HapticFeedbackTarget::RightSide    => controller.set_rumble(0, amplitude, duration),
                HapticFeedbackTarget::LeftTrigger  => controller.set_rumble_triggers(amplitude, 0, duration),
                HapticFeedbackTarget::RightTrigger => controller.set_rumble_triggers(0, amplitude, duration)
              };
            }
          }
        }
      }

      std::thread::sleep(std::time::Duration::from_millis(8)); // what interval should that be?
    } // loop
  }
}

fn is_gyro_steady(v1: [f32; 3], v2: [f32; 3]) -> bool {
  const THRESHOLD: f32 = 0.0174533; // one 1 deg in rads
  (v1[0] - v2[0]).abs() <= THRESHOLD && (v1[1] - v2[1]).abs() <= THRESHOLD && (v1[2] - v2[2]).abs() <= THRESHOLD
}

struct Average<const CAPACITY: usize> {
  buffer: [[f32; 3]; CAPACITY],
  pos:    usize,
  size:   usize,
  sum:    [f32; 3]
}

impl<const CAPACITY: usize> Average<CAPACITY> {

  fn new() -> Self {
    Self {
      buffer: [[0.0; 3]; CAPACITY],
      size:   0,
      pos:    0,
      sum:    [0.0; 3]
    }
  }

  fn push(&mut self, data: [f32; 3]) {

    if self.size == CAPACITY {
      self.sum[0] -= self.buffer[self.pos][0];
      self.sum[1] -= self.buffer[self.pos][1];
      self.sum[2] -= self.buffer[self.pos][2];
    } else {
      self.size += 1;
    }

    self.sum[0] += data[0];
    self.sum[1] += data[1];
    self.sum[2] += data[2];

    self.buffer[self.pos] = data;
    self.pos = (self.pos + 1) % CAPACITY;
  }

  fn reset(&mut self) {
    self.pos  = 0;
    self.size = 0;
    self.sum  = [0.0; 3];
  }

  fn average(&self) -> [f32; 3] {
    [self.sum[0] / self.size as f32, self.sum[1] / self.size as f32, self.sum[2] / self.size as f32]
  }

  fn buffer_is_full(&self) -> bool {
    self.size == CAPACITY
  }
}

#[test]
fn rolling_average_test() {
  let mut avg = Average::<5>::new();
  assert!(avg.average()[0].is_nan());
  avg.push([0.0, 0.0, 0.0]);
  assert_eq!(avg.average()[0], 0.0);
  avg.push([1.0, 0.0, 0.0]);
  assert_eq!(avg.average()[0], 0.5);
  avg.push([2.0, 0.0, 0.0]);
  avg.push([3.0, 0.0, 0.0]);
  avg.push([4.0, 0.0, 0.0]);
  assert!(avg.buffer_is_full());
  assert_eq!(avg.average()[0], 2.0);
  avg.push([5.0, 0.0, 0.0]);
  assert_eq!(avg.average()[0], 3.0);
}
