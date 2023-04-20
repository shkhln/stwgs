use overlay_ipc::{Color, Length, Point, Shape};

use super::*;
use crate::controllers::{HapticFeedbackEffect, HapticFeedbackTarget};

fn distance_from_center(x: f32, y: f32) -> f32 {
  (x.powi(2) + y.powi(2)).sqrt()
}

fn distance_from_point(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
  ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

fn inside_ring(x: f32, y: f32, inner_radius: f32, outer_radius: f32) -> bool {
  let distance_from_center = distance_from_center(x, y);
  distance_from_center >= inner_radius && distance_from_center <= outer_radius
}

fn inside_sector(x: f32, y: f32, sector_width: f32, sector_direction: f32) -> bool {
  let angle = y.atan2(x);
  let mut angle_diff = angle - sector_direction;
  if angle_diff >  std::f32::consts::PI { angle_diff -= std::f32::consts::PI * 2.0; }
  if angle_diff < -std::f32::consts::PI { angle_diff += std::f32::consts::PI * 2.0; }
  angle_diff.abs() <= sector_width / 2.0
}

struct TouchPoint {
  x1: f32,
  y1: f32,
  x2: f32,
  y2: f32,
  x3: f32,
  y3: f32,
  x4: f32,
  y4: f32
}

impl TouchPoint {
  fn new(x: f32, y: f32, margin: f32) -> Self {
    Self {
      x1: x + margin * 0.5, y1: y, // east
      x2: x - margin * 0.5, y2: y, // west
      x3: x, y3: y + margin * 0.5, // north
      x4: x, y4: y - margin * 0.5  // south
    }
  }

  fn inside_ring(&self, inner_radius: f32, outer_radius: f32) -> bool {
    inside_ring(self.x1, self.y1, inner_radius, outer_radius)
      && inside_ring(self.x2, self.y2, inner_radius, outer_radius)
      && inside_ring(self.x3, self.y3, inner_radius, outer_radius)
      && inside_ring(self.x4, self.y4, inner_radius, outer_radius)
  }

  fn inside_sector(&self, sector_width: f32, sector_direction: f32) -> bool {
    inside_sector(self.x1, self.y1, sector_width, sector_direction)
      && inside_sector(self.x2, self.y2, sector_width, sector_direction)
      && inside_sector(self.x3, self.y3, sector_width, sector_direction)
      && inside_sector(self.x4, self.y4, sector_width, sector_direction)
  }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TouchMenuOpts {
  Radial  { inner_radius: f32, outer_radius: f32, margin: f32 },
  HexGrid { margin: f32 }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RadialMenuOpts {
  pub inner_radius: f32,
  pub outer_radius: f32,
  pub margin:       f32
}

struct TouchMenuStage {
  stage_id:        StageId,
  position:        PipelineRef<(f32, f32)>,
  toggle:          PipelineRef<bool>,
  select:          PipelineRef<bool>,
  opts:            TouchMenuOpts,
  items:           Vec<String>,
  selected_option: Option<(u8, Timestamp)>,
  mode:            TouchMenuMode,
  out_value:       Option<Option<u8>>
}

#[derive(Debug)]
enum TouchMenuMode {
  Locked { position: (f32, f32) },
  Unlocked
}

fn hex_grid_positions_on_screen(center: overlay_ipc::Point, circumradius: overlay_ipc::Length, number_of_cells: usize) -> Vec<overlay_ipc::Point> {

  assert!(number_of_cells > 0);

  let mut v      = vec![];
  let mut circle = 1;

  'outer: loop {
    let mut p = Point { x: center.x, y: center.y - circumradius * 3f32.sqrt() * circle as f32 };
    v.push(p);

    if v.len() == number_of_cells {
      break 'outer;
    }

    for _ in 0..circle {
      p = Point { x: p.x + circumradius * 1.5, y: p.y + circumradius * 3f32.sqrt() * 0.5 };
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = Point { x: p.x, y: p.y + circumradius * 3f32.sqrt() };
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = Point { x: p.x - circumradius * 1.5, y: p.y + circumradius * 3f32.sqrt() * 0.5 };
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = Point { x: p.x - circumradius * 1.5, y: p.y - circumradius * 3f32.sqrt() * 0.5 };
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = Point { x: p.x, y: p.y - circumradius * 3f32.sqrt() };
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..(circle - 1) {
      p = Point { x: p.x + circumradius * 1.5, y: p.y - circumradius * 3f32.sqrt() * 0.5 };
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    circle += 1;
  }

  v
}

fn hex_grid_positions_on_touchpad(center: (f32, f32), circumradius: f32, number_of_cells: usize) -> Vec<(f32, f32)> {

  assert!(number_of_cells > 0);

  let mut v      = vec![];
  let mut circle = 1;

  'outer: loop {
    let mut p = (center.0, center.1 + circumradius * 3f32.sqrt() * circle as f32);
    v.push(p);

    if v.len() == number_of_cells {
      break 'outer;
    }

    for _ in 0..circle {
      p = (p.0 + circumradius * 1.5, p.1 - circumradius * 3f32.sqrt() * 0.5);
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = (p.0, p.1 - circumradius * 3f32.sqrt());
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = (p.0 - circumradius * 1.5, p.1 - circumradius * 3f32.sqrt() * 0.5);
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = (p.0 - circumradius * 1.5, p.1 + circumradius * 3f32.sqrt() * 0.5);
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..circle {
      p = (p.0, p.1 + circumradius * 3f32.sqrt());
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    for _ in 0..(circle - 1) {
      p = (p.0 + circumradius * 1.5, p.1 + circumradius * 3f32.sqrt() * 0.5);
      v.push(p);

      if v.len() == number_of_cells {
        break 'outer;
      }
    }

    circle += 1;
  }

  v
}

fn number_of_hex_grid_circles(number_of_cells: usize) -> usize {
  match number_of_cells {
          0 => 0,
     1..= 6 => 1,
     7..=18 => 2,
    19..=36 => 3,
    37..=60 => 4,
    _ => unimplemented!()
  }
}

//TODO: determine haptic feedback target automatically or make it configurable
impl Pipeline<Option<u8>> for TouchMenuStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "touch_menu"
  }

  fn desc(&self) -> String {
    format!("{}(...)", self.name())
  }

  fn inputs(&self) -> Vec<StageId> {
    vec![self.position.borrow().stage_id(), self.toggle.borrow().stage_id(), self.select.borrow().stage_id()]
  }

  fn shapes(&self) -> Vec<Vec<overlay_ipc::Shape>> {

    let center      = Point::vwh(50.0, 50.0);
    let menu_height = Length::vh(50.0);

    match self.opts {
      TouchMenuOpts::Radial { inner_radius, outer_radius, .. } => {

        let sector_width = std::f32::consts::PI * 2.0 / self.items.len() as f32;

        let mut layers = vec![];

        // background
        layers.push(vec![
          Shape::Ring {
            center,
            inner_radius: menu_height * 0.5 * inner_radius,
            outer_radius: menu_height * 0.5 * outer_radius,
            color:        Color::rgba(0.0, 0.2, 0.0, 0.4)
          }
        ]);

        // normal items
        let mut direction = std::f32::consts::PI / 2.0;
        let mut v = Vec::with_capacity(self.items.len());
        for item in &self.items {
          v.push(overlay_ipc::Shape::RingSector {
            center,
            direction:    overlay_ipc::Angle::Rad(-direction),
            width:        overlay_ipc::Angle::Rad(sector_width),
            inner_radius: menu_height * 0.5 * inner_radius,
            outer_radius: menu_height * 0.5 * outer_radius,
            color:        overlay_ipc::Color::rgba(0.0, 0.0, 0.0, 0.0),
            label:        Some((item.clone(), overlay_ipc::Color::rgb(1.0, 1.0, 1.0)))
          });
          direction -= sector_width;
        }
        layers.push(v);

        // selected items
        let mut direction = std::f32::consts::PI / 2.0;
        let mut v = Vec::with_capacity(self.items.len());
        for item in &self.items {
          v.push(overlay_ipc::Shape::RingSector {
            center,
            direction:    overlay_ipc::Angle::Rad(-direction),
            width:        overlay_ipc::Angle::Rad(sector_width),
            inner_radius: menu_height * 0.5 * inner_radius,
            outer_radius: menu_height * 0.5 * outer_radius,
            color:        overlay_ipc::Color::rgba(0.0, 0.8, 0.0, 0.8),
            label:        Some((item.clone(), overlay_ipc::Color::rgb(1.0, 1.0, 1.0)))
          });
          direction -= sector_width;
        }
        layers.push(v);

        // selected and locked items
        let mut direction = std::f32::consts::PI / 2.0;
        let mut v = Vec::with_capacity(self.items.len());
        for item in &self.items {
          v.push(overlay_ipc::Shape::RingSector {
            center,
            direction:    overlay_ipc::Angle::Rad(-direction),
            width:        overlay_ipc::Angle::Rad(sector_width),
            inner_radius: menu_height * 0.5 * inner_radius,
            outer_radius: menu_height * 0.5 * outer_radius,
            color:        overlay_ipc::Color::rgba(0.8, 0.8, 0.0, 0.8),
            label:        Some((item.clone(), overlay_ipc::Color::rgb(1.0, 1.0, 1.0)))
          });
          direction -= sector_width;
        }
        layers.push(v);

        layers
      },
      TouchMenuOpts::HexGrid { .. } => {

        let mut layers = vec![];

        let circumradius = menu_height / (number_of_hex_grid_circles(self.items.len()) * 2 + 1) as f32 / 3f32.sqrt();
        let points       = hex_grid_positions_on_screen(center, circumradius, self.items.len());

        // background
        layers.push(vec![
          Shape::Ring {
            center,
            inner_radius: circumradius,
            outer_radius: menu_height * 0.5,
            color:        Color::rgba(0.0, 0.2, 0.0, 0.4)
          }
        ]);

        // normal items
        let mut v = Vec::with_capacity(self.items.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.items.len() {
          v.push(Shape::RegularHexagon {
            center:       points[i],
            circumradius,
            color:        Color::rgba(0.1, 0.1, 0.1, 0.3),
            label:        Some((self.items[i].clone(), Color::rgb(1.0, 1.0, 1.0)))
          });
        }
        layers.push(v);

        // selected items
        let mut v = Vec::with_capacity(self.items.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.items.len() {
          v.push(Shape::RegularHexagon {
            center:       points[i],
            circumradius,
            color:        Color::rgba(0.0, 0.8, 0.0, 0.8),
            label:        Some((self.items[i].clone(), Color::rgb(1.0, 1.0, 1.0)))
          });
        }
        layers.push(v);

        // selected and locked items
        let mut v = Vec::with_capacity(self.items.len());
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.items.len() {
          v.push(Shape::RegularHexagon {
            center:       points[i],
            circumradius,
            color:        Color::rgba(0.8, 0.8, 0.0, 0.8),
            label:        Some((self.items[i].clone(), Color::rgb(1.0, 1.0, 1.0)))
          });
        }
        layers.push(v);

        layers
      }
    }
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    if insert_stage_description(out, self) {
      self.position.borrow().inspect(out);
      self.toggle  .borrow().inspect(out);
      self.select  .borrow().inspect(out);
    }
  }

  fn apply(&mut self, ctx: &Context, actions: &mut Vec<Action>) -> Option<u8> {

    if self.out_value.is_none() {

      // we effectively process menu selection with a delay of one tick
      // in order to handle touchpad release events
      if self.select.borrow_mut().apply(ctx, actions) {
        self.out_value = Some(match self.mode {
          TouchMenuMode::Locked { .. } => self.selected_option.map(|option| option.0),
          TouchMenuMode::Unlocked      => None
        });
      } else {
        self.out_value = Some(None);
      }

      if self.toggle.borrow_mut().apply(ctx, actions) {

        match self.opts {
          TouchMenuOpts::Radial { inner_radius, outer_radius, margin } => {

            let sector_width = std::f32::consts::PI * 2.0 / self.items.len() as f32;

            let (x, y) = self.position.borrow_mut().apply(ctx, actions);
            let p      = TouchPoint::new(x, y, margin);

            match self.mode {
              TouchMenuMode::Locked { position } => {
                //TODO: better unlocking heuristic?
                if !p.inside_ring(inner_radius, outer_radius * 1.2 /* ? */) || distance_from_point(x, y, position.0, position.1) > margin * 4.0 {
                  self.mode = TouchMenuMode::Unlocked;
                }
              },
              TouchMenuMode::Unlocked => {
                if p.inside_ring(inner_radius, outer_radius * 1.2 /* ? */) {
                  let mut direction = std::f32::consts::PI / 2.0;
                  for i in 0..self.items.len() {
                    if p.inside_sector(sector_width, direction) {

                      if self.selected_option.map(|option| option.0) != Some(i as u8) {
                        //println!("selecting radial menu item {}", i);
                        actions.push(Action::HapticFeedback(
                          HapticFeedbackTarget::LeftSide,
                          HapticFeedbackEffect::SlightBump
                        ));
                        actions.push(Action::HapticFeedback(
                          HapticFeedbackTarget::RightSide,
                          HapticFeedbackEffect::SlightBump
                        ));

                        self.selected_option = Some((i as u8, ctx.time));
                      }

                      break;
                    }
                    direction -= sector_width;
                  }

                  if let Some((_, t)) = self.selected_option {
                    if ctx.time - t >= Duration::from_millis(500) {
                      self.mode = TouchMenuMode::Locked { position: (x, y) };
                      actions.push(Action::HapticFeedback(
                        HapticFeedbackTarget::LeftSide,
                        HapticFeedbackEffect::ModerateBump
                      ));
                      actions.push(Action::HapticFeedback(
                        HapticFeedbackTarget::RightSide,
                        HapticFeedbackEffect::ModerateBump
                      ));
                    }
                  }

                } else {
                  self.selected_option = None;
                }
              }
            }
          },
          TouchMenuOpts::HexGrid { margin } => {

            let circumradius = 2.0 / (number_of_hex_grid_circles(self.items.len()) * 2 + 1) as f32 / 3f32.sqrt();
            let inradius     = 3f32.sqrt() / 2.0 * circumradius;
            assert!(inradius > margin);

            let (x, y) = self.position.borrow_mut().apply(ctx, actions);
            let trigger_distance = inradius - margin;
            //println!("trigger_distance: {}", trigger_distance);

            match self.mode {
              TouchMenuMode::Locked { position } => {
                if distance_from_center(x, y) < trigger_distance || distance_from_point(x, y, position.0, position.1) > inradius + margin {
                  self.mode = TouchMenuMode::Unlocked;
                }
              },
              TouchMenuMode::Unlocked => {
                if distance_from_center(x, y) > trigger_distance {
                  let points = hex_grid_positions_on_touchpad((0.0, 0.0), circumradius, self.items.len()); //TODO: cache this
                  #[allow(clippy::needless_range_loop)]
                  for i in 0..self.items.len() {
                    if distance_from_point(x, y, points[i].0, points[i].1) < trigger_distance {

                      if self.selected_option.map(|option| option.0) != Some(i as u8) {
                        //println!("selecting hex menu item {}", i);
                        actions.push(Action::HapticFeedback(
                          HapticFeedbackTarget::LeftSide,
                          HapticFeedbackEffect::SlightBump
                        ));
                        actions.push(Action::HapticFeedback(
                          HapticFeedbackTarget::RightSide,
                          HapticFeedbackEffect::SlightBump
                        ));

                        self.selected_option = Some((i as u8, ctx.time));
                      }

                      break;
                    }
                  }

                  if let Some((_, t)) = self.selected_option {
                    if ctx.time - t >= Duration::from_millis(500) {
                      self.mode = TouchMenuMode::Locked { position: (x, y) };
                      actions.push(Action::HapticFeedback(
                        HapticFeedbackTarget::LeftSide,
                        HapticFeedbackEffect::ModerateBump
                      ));
                      actions.push(Action::HapticFeedback(
                        HapticFeedbackTarget::RightSide,
                        HapticFeedbackEffect::ModerateBump
                      ));
                    }
                  }

                } else {
                  self.selected_option = None;
                }
              }
            }
          }
        }

        actions.push(Action::ToggleShapes { stage_id: self.stage_id, layer: 0, mask: u64::MAX });

        let mut selected_items = 0;

        if let Some(i) = self.selected_option.map(|option| option.0) {
          selected_items |= 1 << i;
        }

        actions.push(Action::ToggleShapes { stage_id: self.stage_id, layer: 1, mask: u64::MAX & !selected_items });

        match self.mode {
          TouchMenuMode::Locked { .. } => {
            actions.push(Action::ToggleShapes { stage_id: self.stage_id, layer: 3, mask: selected_items });
          },
          TouchMenuMode::Unlocked => {
            actions.push(Action::ToggleShapes { stage_id: self.stage_id, layer: 2, mask: selected_items });
          }
        }

      } else {
        self.selected_option = None;
        self.mode = TouchMenuMode::Unlocked;
      }
    }

    self.out_value.unwrap()
  }

  fn reset(&mut self) {
    if self.out_value.is_some() {
      self.position.borrow_mut().reset();
      self.toggle  .borrow_mut().reset();
      self.select  .borrow_mut().reset();
      self.out_value = None;
    }
  }
}

pub fn touch_menu(position: PipelineRef<(f32, f32)>, toggle: PipelineRef<bool>, select: PipelineRef<bool>, items: Vec<String>, opts: TouchMenuOpts) -> PipelineRef<Option<u8>> {

  assert!(items.len() <= 60);

  match opts {
    TouchMenuOpts::Radial { inner_radius, outer_radius, .. } => {
      assert!(inner_radius > 0.0);
      assert!(outer_radius > inner_radius);
    },
    TouchMenuOpts::HexGrid { .. } => {
      // ?
    }
  }

  std::rc::Rc::new(std::cell::RefCell::new(TouchMenuStage {
    stage_id: generate_stage_id(),
    position,
    toggle,
    select,
    opts,
    items,
    mode: TouchMenuMode::Unlocked,
    selected_option: None,
    out_value: None
  }))
}
