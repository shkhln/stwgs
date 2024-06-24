use std::sync::{mpsc, Mutex};
use std::thread;

pub use ipc_channel::ipc;
use ipc_channel::ipc::{IpcError, IpcOneShotServer, IpcSender};

use lazy_static::lazy_static;

pub type CommandSender   = IpcSender<OverlayCommand>;
pub type CommandReceiver = mpsc::Receiver<OverlayCommand>;

//TODO: filter by overlay name
pub fn connect_to_overlay() -> Result<Option<CommandSender>, Box<dyn std::error::Error>> {

  use zbus::{proxy, Connection};

  #[proxy(interface = "stwgs.Overlay", default_path = "/overlay")]
  trait SCOverlay {
    fn overlay_name(&self)    -> zbus::fdo::Result<String>;
    fn ipc_server_name(&self) -> zbus::fdo::Result<String>;
  }

  let connection = futures::executor::block_on(Connection::session())?;
  let dbus_proxy = futures::executor::block_on(zbus::fdo::DBusProxy::new(&connection))?;
  let services   = futures::executor::block_on(dbus_proxy.list_names())?;

  for name in services {
    if name.starts_with("stwgs.Overlay") {
      let overlay_proxy =
        futures::executor::block_on(SCOverlayProxy::builder(&connection).destination(&name).unwrap().build())?;
      //println!("{:?}", overlay_proxy);

      let overlay_name = futures::executor::block_on(overlay_proxy.overlay_name())?;
      println!("[client] found overlay: {:?}", overlay_name);
      let server_name  = futures::executor::block_on(overlay_proxy.ipc_server_name())?;
      println!("[client] connecting to ipc server {:?}", server_name);

      let sender = IpcSender::connect(server_name)?;

      return Ok(Some(sender));
    }
  }

  Ok(None)
}

lazy_static! {
  static ref DBUS_CONNECTION: Mutex<Option<zbus::Connection>> = Mutex::new(None);
}

pub fn process_incoming_commands(overlay_name: &str) -> CommandReceiver {
  let (mpsc_sender, mpsc_receiver) = mpsc::channel();
  let _ = mpsc_sender.send(OverlayCommand::SetStatusText(Some("waiting for connection".to_string())));

  fn start_ipc_thread(mpsc_sender: mpsc::Sender<OverlayCommand>) -> (String, thread::JoinHandle<()>) {
    let (server, server_name) = IpcOneShotServer::<OverlayCommand>::new().unwrap();

    let thread_handle = thread::spawn(move || {
      let (receiver, command) = server.accept().unwrap();
      eprintln!("[server] received first: {:?}", command);
      let _ = mpsc_sender.send(command);

      loop {
        match receiver.recv() {
          Ok(command) => {
            eprintln!("[server] received: {:?}", command);
            let _ = mpsc_sender.send(command);
          },
          Err(err) => match err {
            IpcError::Disconnected => {
              eprintln!("[server] client disconnected");
              let _ = mpsc_sender.send(OverlayCommand::ResetOverlay);
              let _ = mpsc_sender.send(OverlayCommand::SetStatusText(Some("waiting for connection".to_string())));
              break;
            },
            _ => panic!("[server] received err: {:?}", err)
          }
        }
      }
    });

    (server_name, thread_handle)
  }

  use zbus::{interface, ConnectionBuilder};

  struct SCOverlay {
    overlay_name: String,
    ipc_server:   Option<(String, thread::JoinHandle<()>)>,
    mpsc_sender:  Mutex<mpsc::Sender<OverlayCommand>>
  }

  #[interface(name = "stwgs.Overlay")]
  impl SCOverlay {
    fn overlay_name(&self) -> &String {
      &self.overlay_name
    }

    fn ipc_server_name(&mut self) -> String {
      if let Some((ipc_server_name, ipc_thread)) = &self.ipc_server {
        if !ipc_thread.is_finished() {
          return ipc_server_name.clone();
        }
      }

      let (ipc_server_name, ipc_thread) = start_ipc_thread(self.mpsc_sender.lock().unwrap().clone());
      self.ipc_server = Some((ipc_server_name.clone(), ipc_thread));
      ipc_server_name
    }
  }

  let mut connection = DBUS_CONNECTION.lock().unwrap();
  assert!(connection.is_none());

  let overlay_dbus_object =
    SCOverlay { overlay_name: overlay_name.to_string(), ipc_server: None, mpsc_sender: Mutex::new(mpsc_sender) };

  *connection = Some(
    futures::executor::block_on(
      ConnectionBuilder::session().unwrap()
        .name(format!("stwgs.Overlay{}", std::process::id())).unwrap()
        .serve_at("/overlay", overlay_dbus_object).unwrap()
        .build())
      .unwrap());

  mpsc_receiver
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Length {
  px: f32,
  vw: f32, // 1% of the viewport's width,  just like in CSS
  vh: f32  // 1% of the viewport's height, just like in CSS
}

impl Length {
  pub fn px(px: f32) -> Self {
    Self { px, vw: 0.0, vh: 0.0 }
  }

  pub fn vw(vw: f32) -> Self {
    Self { px: 0.0, vw, vh: 0.0 }
  }

  pub fn vh(vh: f32) -> Self {
    Self { px: 0.0, vw: 0.0, vh }
  }

  pub fn to_px(&self, screen_width: u32, screen_height: u32) -> f32 {
    self.px + (screen_width as f32 * 0.01 * self.vw) + (screen_height as f32 * 0.01 * self.vh)
  }
}

impl std::ops::Add for Length {
  type Output = Self;

  fn add(self, other: Self) -> Self {
    Self { px: self.px + other.px, vw: self.vw + other.vw, vh: self.vh + other.vh }
  }
}

impl std::ops::Sub for Length {
  type Output = Self;

  fn sub(self, other: Self) -> Self {
    Self { px: self.px - other.px, vw: self.vw - other.vw, vh: self.vh - other.vh }
  }
}

impl std::ops::Mul<f32> for Length {
  type Output = Self;

  fn mul(self, factor: f32) -> Self {
    Self { px: self.px * factor, vw: self.vw * factor, vh: self.vh * factor }
  }
}

impl std::ops::Div<f32> for Length {
  type Output = Self;

  fn div(self, factor: f32) -> Self {
    Self { px: self.px / factor, vw: self.vw / factor, vh: self.vh / factor }
  }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Point {
  pub x: Length,
  pub y: Length
}

impl Point {
  pub fn px(x: f32, y: f32) -> Point {
    Self { x: Length::px(x), y: Length::px(y) }
  }

  pub fn vwh(x: f32, y: f32) -> Point {
    Self { x: Length::vw(x), y: Length::vh(y) }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Color {
  pub r: f32,
  pub g: f32,
  pub b: f32,
  pub a: f32
}

impl Color {
  pub fn rgb(r: f32, g: f32, b: f32) -> Self {
    Self { r, g, b, a: 1.0 }
  }

  pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
    Self { r, g, b, a }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Angle {
  Deg(f32),
  Rad(f32)
}

impl Angle {
  pub fn to_rad(&self) -> f32 {
    match self {
      Angle::Deg(deg) => std::f32::consts::PI / 180.0 * deg,
      Angle::Rad(rad) => *rad
    }
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Shape {
  Circle {
    center: Point,
    radius: Length,
    color:  Color,
    label:  Option<(String, Color)>
  },
  Ring {
    center:       Point,
    inner_radius: Length,
    outer_radius: Length,
    color:        Color
  },
  RingSector {
    center:       Point,
    direction:    Angle,
    width:        Angle,
    inner_radius: Length,
    outer_radius: Length,
    color:        Color,
    label:        Option<(String, Color)>
  },
  RegularHexagon {
    center:       Point,
    circumradius: Length,
    color:        Color,
    label:        Option<(String, Color)>
  }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Knob {
  Flag   { name: String, value: bool },
  Enum   { name: String, index: usize, options: Vec<String> },
  Number { name: String, value: f32,   min_value: f32, max_value: f32 }
}

impl Knob {
  pub fn name(&self) -> String {
    match self {
      Knob::Flag   { name, .. } => name.clone(),
      Knob::Enum   { name, .. } => name.clone(),
      Knob::Number { name, .. } => name.clone()
    }
  }

  pub fn compare_value(&self, other: &Knob) -> bool {
    match (self, other) {
      (Knob::Flag   { value: v1, .. },              Knob::Flag { value: v2, .. })              => *v1 == *v2,
      (Knob::Enum   { index: i1, options: o1, .. }, Knob::Enum { index: i2, options: o2, .. }) => *o1 == *o2 && *i1 == *i2,
      (Knob::Number { value: v1, .. },              Knob::Number { value: v2, .. })            => (*v2 - *v1).abs() < f32::EPSILON,
      _ => false
    }
  }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub enum OverlayMenuCommand {
  OpenKnobsMenu,
  SelectPrevMenuItem,
  SelectNextMenuItem,
  SelectPrevValue,
  SelectNextValue,
  SetValuePercentage, // ?
  CloseKnobsMenu
}

#[derive(Serialize, Deserialize, Debug)]
pub enum OverlayCommand {
  //GetScreenDimensions(IpcSender<(u32, u32)>),
  //ResetOverlay(IpcSender<OverlayEvent>),
  //ResetScreenScraping(IpcSender<ScreenScrapingResult>),
  //SetShapeEffect { stage_id: u64, mask: u64, effect: Effect } // ?
  AddMemoryCheck(u8, u64, Vec<i32>, IpcSender<u64>),
  AddOverlayCheck(String, IpcSender<bool>),
  AddScreenScrapingArea(ScreenScrapingArea, IpcSender<ScreenScrapingResult>),
  GetKnobs(IpcSender<Vec<Knob>>),
  MenuCommand(OverlayMenuCommand),
  RegisterKnobs(Vec<Knob>),
  RegisterShapes { stage_id: u64, shapes: Vec<Vec<Shape>> },
  ResetOverlay,
  SetLayerNames(Vec<String>),
  SetMode(u64),
  SetStatusText(Option<String>),
  ToggleShapes { stage_id: u64, layer: u8, mask: u64 },
  ToggleUI
}

/*#[derive(Serialize, Deserialize, Debug)]
pub enum OverlayEvent {}*/

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScreenScrapingArea {
  pub bounds:  Rect,
  pub min_hue: f32,
  pub max_hue: f32,
  pub min_sat: f32,
  pub max_sat: f32,
  pub min_val: f32,
  pub max_val: f32
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Rect {
  pub min: Point,
  pub max: Point
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScreenScrapingResult {
  pub pixels_in_range:  f32,
  pub uniformity_score: f32 // ?
}
