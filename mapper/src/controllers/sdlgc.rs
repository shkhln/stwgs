use std::sync::{Arc, Mutex};

use crate::controllers::*;

struct SDL2Controller {
  controller: Arc<Mutex<sdl2::controller::GameController>>,
  name:       String,
  guid:       String
}

fn get_serial_number(controller: &sdl2::controller::GameController) -> Option<String> {

  struct XGameController {
    _subsystem: sdl2::GameControllerSubsystem,
    raw:        *mut sdl2::sys::SDL_GameController
  }

  unsafe {
    let controller: &XGameController = std::mem::transmute(controller);
    let serial = sdl2::sys::SDL_GameControllerGetSerial(controller.raw);
    if !serial.is_null() {
      Some(std::ffi::CStr::from_ptr(serial as *const _).to_str().unwrap().to_owned())
    } else {
      None
    }
  }
}

fn get_sdl_guid(joystick_index: u32) -> String {
  unsafe {
    let guid = sdl2::sys::SDL_JoystickGetDeviceGUID(joystick_index as i32);
    let mut buffer = [0_u8; 33];
    sdl2::sys::SDL_JoystickGetGUIDString(guid, buffer.as_mut_ptr() as *mut i8, buffer.len() as i32);
    String::from_utf8_lossy(&buffer[0..32]).to_string()
  }
}

//TODO: filter out Steam controllers by usb vendor id?
pub fn available_controllers() -> Result<Vec<Box<dyn Controller>>, String> {
  let mut controllers: Vec<Box<dyn Controller>> = vec![];

  let context             = sdl2::init()?;
  let game_controller_sys = Arc::new(context.game_controller()?);
  let joy_count           = game_controller_sys.num_joysticks()?;

  for i in 0..joy_count {
    if game_controller_sys.is_game_controller(i) {
      let controller = Arc::new(Mutex::new(game_controller_sys.open(i).map_err(|e| format!("{}", e))?));
      let name       = game_controller_sys.name_for_index(i).map_err(|e| format!("{}", e))?;
      let guid       = get_sdl_guid(i);
      controllers.push(Box::new(SDL2Controller { controller: Arc::clone(&controller), name, guid }));
    }
  }

  Ok(controllers)
}

impl Controller for SDL2Controller {

  fn name(&self) -> String {
    self.name.clone()
  }

  fn path(&self) -> Option<String> {
    Some(format!("//sdl/{}", self.guid))
  }

  fn serial(&self) -> Option<String> {
    get_serial_number(&self.controller.lock().unwrap())
  }

  fn run_polling_loop(&self, sender: Sender<ControllerState>, receiver: Option<Receiver<ControllerCommand>>) -> Result<(), String> {

    let mut controller = self.controller.lock().unwrap();
    assert!(controller.attached());

    eprintln!("sdl controller mapping:  {}", controller.mapping());

    let axis_scale_factor = 1f32 / i16::MAX as f32;

    //TODO: handle timestamps
    let instance   = controller.instance_id();
    let mut events = controller.subsystem().sdl().event_pump()?;
    let mut state  = ControllerState::empty();
    loop {
      for event in events.poll_iter() {
        match event {
          sdl2::event::Event::ControllerAxisMotion { timestamp, which, axis, value } => {
            //eprintln!("axis: {} {} {:?} {}", timestamp, which, axis, value);
            let _ = timestamp;
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
          sdl2::event::Event::ControllerButtonDown { timestamp, which, button } => {
            //eprintln!("button dn: {} {} {:?}", timestamp, which, button);
            let _ = timestamp;
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
          sdl2::event::Event::ControllerButtonUp   { timestamp, which, button } => {
            //eprintln!("button up: {} {} {:?}", timestamp, which, button);
            let _ = timestamp;
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
          sdl2::event::Event::ControllerDeviceAdded    { .. } => (),
          sdl2::event::Event::ControllerDeviceRemapped { .. } => (),
          sdl2::event::Event::ControllerDeviceRemoved  { .. } => (),
          sdl2::event::Event::JoyDeviceAdded           { .. } => (),
          sdl2::event::Event::JoyDeviceRemoved         { .. } => (),
          sdl2::event::Event::JoyAxisMotion            { .. } => (),
          sdl2::event::Event::JoyButtonDown            { .. } => (),
          sdl2::event::Event::JoyButtonUp              { .. } => (),
          sdl2::event::Event::JoyHatMotion             { .. } => (),
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
