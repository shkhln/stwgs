use egui::{pos2, vec2, Align2, Color32, Pos2, Rect, Vec2};
use overlay_ipc::{Color, Knob, Point, ScreenScrapingResult, Shape};

use crate::OverlayState;

pub fn draw_ui(overlay: &OverlayState, ctx: &egui::Context, screen: (u32, u32), scraping_result: Option<ScreenScrapingResult>) -> egui::FullOutput {

  let (screen_width, screen_height) = screen;

  ctx.set_style(egui::Style {
    override_font_id: Some(egui::FontId::new(screen_height as f32 / 20.0, egui::FontFamily::Monospace)),
    visuals: {
      let mut visuals = egui::Visuals::dark();
      visuals.widgets.noninteractive.bg_fill   = egui::Color32::TRANSPARENT;
      visuals.widgets.noninteractive.bg_stroke = egui::Stroke::NONE;
      visuals
    },
    ..Default::default()
  });

  let raw_input = egui::RawInput::default();
  ctx.run(raw_input, |ctx| {

    let style = egui::Style {
      visuals: {
        let mut visuals = egui::Visuals::dark();
        visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
        visuals
      },
      ..Default::default()
    };

    egui::SidePanel::left("whatever")
      .frame(egui::Frame::canvas(&style))
      .show(ctx, |ui| {

        let (_, painter) = ui.allocate_painter(vec2(screen_width as f32, screen_height as f32), egui::Sense::hover());

        for layers in overlay.shapes.values() {
          for (shape_set, mask) in layers {
            for (i, shape) in shape_set.iter().enumerate() {
              if mask & (1 << i) != 0 {

                fn to_pos2(p: &Point, screen_width: u32, screen_height: u32) -> Pos2 {
                  pos2(p.x.to_px(screen_width, screen_height), p.y.to_px(screen_width, screen_height))
                }

                fn to_color32(color: &Color) -> Color32 {
                  Color32::from_rgba_unmultiplied(
                    (color.r * 255.0) as u8,
                    (color.g * 255.0) as u8,
                    (color.b * 255.0) as u8,
                    (color.a * 255.0) as u8)
                }

                match shape {
                  Shape::Circle { center, radius, color, label } => {

                    let center = to_pos2(center, screen_width, screen_height);

                    if color.a > 0.0 {
                      painter.circle_filled(center, radius.to_px(screen_width, screen_height), to_color32(color));
                    }

                    if let Some((text, color)) = label {
                      if !text.is_empty() {
                        let font = egui::FontId::new(screen_height as f32 * 0.05, egui::FontFamily::Monospace);
                        painter.text(center, Align2::CENTER_CENTER, text, font, to_color32(color));
                      }
                    }
                  },
                  Shape::Ring { center, inner_radius, outer_radius, color } => {
                    if color.a > 0.0 {
                      let center       = to_pos2(center, screen_width, screen_height);
                      let inner_radius = inner_radius.to_px(screen_width, screen_height);
                      let outer_radius = outer_radius.to_px(screen_width, screen_height);
                      let ring_width   = outer_radius - inner_radius;
                      let mid_radius   = inner_radius + ring_width / 2.0;
                      painter.circle_stroke(center, mid_radius, egui::Stroke::new(ring_width, to_color32(color)));
                    }
                  },
                  Shape::RingSector { center, direction, width, inner_radius, outer_radius, color, label } => {

                    let center       = to_pos2(center, screen_width, screen_height);
                    let inner_radius = inner_radius.to_px(screen_width, screen_height);
                    let outer_radius = outer_radius.to_px(screen_width, screen_height);
                    let ring_width   = outer_radius - inner_radius;

                    if color.a > 0.0 {
                      let mid_radius = inner_radius + ring_width / 2.0;
                      let offset     = direction.to_rad() - (width.to_rad() / 2.0);
                      let steps      = ((outer_radius / 1080.0).sqrt() * 128.0 * (width.to_rad() / std::f32::consts::PI * 2.0))
                        .clamp(8.0, 128.0) as usize;
                      let step_width = width.to_rad() / steps as f32;

                      let mut points = vec![];
                      for i in 0..=steps {
                        let cos0 = (offset + step_width * i as f32).cos();
                        let sin0 = (offset + step_width * i as f32).sin();
                        points.push(pos2(center[0] + cos0 * mid_radius, center[1] + sin0 * mid_radius));
                      }

                      painter.add(egui::Shape::line(points, egui::Stroke::new(ring_width, to_color32(color))));
                    }

                    if let Some((text, color)) = label {
                      if !text.is_empty() {
                        let r    = inner_radius + ring_width * (3.0 / 4.0);
                        let x    = center[0] + direction.to_rad().cos() * r;
                        let y    = center[1] + direction.to_rad().sin() * r;
                        let font = egui::FontId::new(width.to_rad() * r * 0.6, egui::FontFamily::Monospace);
                        painter.text(pos2(x, y), Align2::CENTER_CENTER, text, font, to_color32(color));
                      }
                    }
                  },
                  Shape::RegularHexagon { center, circumradius, color, label } => {

                    let center       = to_pos2(center, screen_width, screen_height);
                    let circumradius = circumradius.to_px(screen_width, screen_height);

                    if color.a > 0.0 {

                      fn corner(i: usize) -> Vec2 {
                        let angle = std::f32::consts::PI / 3.0 * i as f32; //- std::f32::consts::PI / 6.0;
                        vec2(angle.cos(), angle.sin())
                      }

                      let points = vec![
                        center + corner(0) * circumradius,
                        center + corner(1) * circumradius,
                        center + corner(2) * circumradius,
                        center + corner(3) * circumradius,
                        center + corner(4) * circumradius,
                        center + corner(5) * circumradius
                      ];

                      painter.add(egui::Shape::convex_polygon(points, to_color32(color), egui::Stroke::NONE));
                    }

                    if let Some((text, color)) = label {
                      if !text.is_empty() {
                        let font = egui::FontId::new(circumradius * 0.8, egui::FontFamily::Monospace);
                        painter.text(center, Align2::CENTER_CENTER, text, font, to_color32(color));
                      }
                    }
                  }
                };
              }
            }
          }
        }

        if overlay.hud_is_active {
          let rect = Rect::from_min_size(
            pos2(20.0, screen_height as f32 / 3.0),
            vec2(screen_width as f32, screen_height as f32)
          );
          ui.allocate_ui_at_rect(rect, |ui| {

            if let Some(status_text) = &overlay.status_text {
              ui.add(egui::Label::new(
                egui::RichText::new(status_text)
                  .background_color(Color32::BLACK)
                  .color(Color32::WHITE)));
            }

            for i in 0..overlay.layer_names.len() {
              let name   = &overlay.layer_names[i];
              let active = overlay.mode & (1 << i) != 0;
              ui.add(egui::Label::new(
                egui::RichText::new(if active { format!("+ {}", name) } else { format!("− {}", name) })
                  .background_color(Color32::BLACK)
                  .color(Color32::WHITE)));
            }
          });

          for target in overlay.screen_scraping_targets.iter() {

            let min_x = target.0.bounds.min.x.to_px(screen_width, screen_height);
            let min_y = target.0.bounds.min.y.to_px(screen_width, screen_height);
            let max_x = target.0.bounds.max.x.to_px(screen_width, screen_height);
            let max_y = target.0.bounds.max.y.to_px(screen_width, screen_height);

            let rect = Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y));

            ui.allocate_ui_at_rect(Rect::from_min_size(rect.min, vec2(500.0, 500.0)), |ui| {
              ui.add(egui::Label::new(
                egui::RichText::new(
                  format!("{:.5}\n{:.5}",
                    scraping_result.as_ref().unwrap().pixels_in_range,
                    scraping_result.as_ref().unwrap().uniformity_score))
                  .background_color(Color32::TRANSPARENT)
                  .color(Color32::WHITE)));
            });

            ui.painter().rect(rect, egui::Rounding::ZERO, Color32::TRANSPARENT, egui::Stroke::new(3.0, Color32::GREEN));
          }
        }

        if overlay.knob_menu_visible {
          let rect = Rect::from_min_size(
            pos2(screen_width as f32 / 6.0,       screen_height as f32 * 0.1),
            vec2(screen_width as f32 / 6.0 * 5.0, screen_height as f32));

          ui.allocate_ui_at_rect(rect, |ui| {
            for knob_index in 0..overlay.knobs.len() {
              match &overlay.knobs[knob_index] {
                Knob::Flag { name, value } => {

                  ui.add(egui::Label::new(
                    egui::RichText::new(
                      format!("{} {:<20} {:<20} ", if knob_index == overlay.knob_menu_selected_item { "▶" } else { " " },
                        name, if *value { "Y" } else { "N" }))
                      .background_color(Color32::LIGHT_GRAY)
                      .color(Color32::BLACK)));
                },
                Knob::Enum { name, index, options } => {

                  let at_start = *index == 0;
                  let at_end   = *index == options.len() - 1;

                  let resp = ui.add(egui::Label::new(
                    egui::RichText::new(
                      format!("{} {:<20} {} {:^16} {} ", if knob_index == overlay.knob_menu_selected_item { "▶" } else { " " },
                        name, if at_start { " " } else { "◀" }, options[*index], if at_end { " " } else { "▶" }))
                      .background_color(Color32::LIGHT_GRAY)
                      .color(Color32::BLACK)));

                  let bb    = resp.rect;
                  let range = (bb.min.x + bb.width() / options.len() as f32 * (*index) as f32)..=(bb.min.x + bb.width() / options.len() as f32 * ((*index) + 1) as f32);

                  let stroke_width = bb.height() * 0.15;

                  ui.painter().hline(range, bb.max.y - stroke_width * 0.5, egui::Stroke::new(stroke_width, Color32::BLACK));
                },
                Knob::Number { name, value, min_value, max_value } => {

                  let resp = ui.add(egui::Label::new(
                    egui::RichText::new(
                      format!("{} {:<20} {:<20} ", if knob_index == overlay.knob_menu_selected_item { "▶" } else { " " }, name, value))
                      .background_color(Color32::LIGHT_GRAY)
                      .color(Color32::BLACK)));

                  let bb = resp.rect;
                  let f  = (value - min_value) / (max_value - min_value);

                  let stroke_width = bb.height() * 0.15;

                  ui.painter().hline(bb.min.x..=(bb.min.x + bb.width() * f), bb.max.y - stroke_width * 0.5, egui::Stroke::new(stroke_width, Color32::BLACK));
                }
              }
            }
          });
        }
      });
  })
}
