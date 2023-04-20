mod ast;
mod eval;
mod parser;
mod util;

use std::collections::HashMap;
use std::rc::Rc;

use eval::Constant;
pub use eval::{EvalError, Value};
use overlay_ipc::Knob;

use crate::controllers::{Axis, Button};
use crate::mapper::LayerMask;
use crate::output::{KeyboardKey, MouseAxis, MouseButton};
use crate::stages::*;

pub struct Config {
  pub pipelines: Vec<(LayerMask, Box<dyn Pipeline<()>>)>,
  pub layers:    Vec<String>,
  pub knobs:     Vec<Knob>
}

fn register_defaults(ctx: &mut eval::Context) {

  ctx.register_fun("print", move |args, _| {
    println!("{:?}", args);
    Ok(Value::Nothing)
  });

  ctx.insert_var("LPadX",       Value::Constant(Constant::InputAxis(Axis::LPadX)));
  ctx.insert_var("LPadY",       Value::Constant(Constant::InputAxis(Axis::LPadY)));
  ctx.insert_var("LPadTouch",   Value::Constant(Constant::InputButton(Button::LPadTouch)));
  ctx.insert_var("LPadPress",   Value::Constant(Constant::InputButton(Button::LPad)));
  ctx.insert_var("RPadX",       Value::Constant(Constant::InputAxis(Axis::RPadX)));
  ctx.insert_var("RPadY",       Value::Constant(Constant::InputAxis(Axis::RPadY)));
  ctx.insert_var("RPadTouch",   Value::Constant(Constant::InputButton(Button::RPadTouch)));
  ctx.insert_var("RPadPress",   Value::Constant(Constant::InputButton(Button::RPad)));
  ctx.insert_var("LTrig",       Value::Constant(Constant::InputAxis(Axis::LTrig)));
  ctx.insert_var("RTrig",       Value::Constant(Constant::InputAxis(Axis::RTrig)));
  ctx.insert_var("LTrigPress",  Value::Constant(Constant::InputButton(Button::LTrig)));
  ctx.insert_var("RTrigPress",  Value::Constant(Constant::InputButton(Button::RTrig)));
  ctx.insert_var("JoyX",        Value::Constant(Constant::InputAxis(Axis::LJoyX)));
  ctx.insert_var("JoyY",        Value::Constant(Constant::InputAxis(Axis::LJoyY)));
  ctx.insert_var("LJoyX",       Value::Constant(Constant::InputAxis(Axis::LJoyX)));
  ctx.insert_var("LJoyY",       Value::Constant(Constant::InputAxis(Axis::LJoyY)));
  ctx.insert_var("RJoyX",       Value::Constant(Constant::InputAxis(Axis::RJoyX)));
  ctx.insert_var("RJoyY",       Value::Constant(Constant::InputAxis(Axis::RJoyY)));
  ctx.insert_var("Yaw",         Value::Constant(Constant::InputAxis(Axis::Yaw)));
  ctx.insert_var("Pitch",       Value::Constant(Constant::InputAxis(Axis::Pitch)));
  ctx.insert_var("Roll",        Value::Constant(Constant::InputAxis(Axis::Roll)));
  ctx.insert_var("AbsYaw",      Value::Constant(Constant::InputAxis(Axis::AbsYaw)));
  ctx.insert_var("AbsPitch",    Value::Constant(Constant::InputAxis(Axis::AbsPitch)));
  ctx.insert_var("AbsRoll",     Value::Constant(Constant::InputAxis(Axis::AbsRoll)));
  ctx.insert_var("LBump",       Value::Constant(Constant::InputButton(Button::LBump)));
  ctx.insert_var("RBump",       Value::Constant(Constant::InputButton(Button::RBump)));
  ctx.insert_var("RGrip",       Value::Constant(Constant::InputButton(Button::RGrip)));
  ctx.insert_var("LGrip",       Value::Constant(Constant::InputButton(Button::LGrip)));
  ctx.insert_var("A",           Value::Constant(Constant::InputButton(Button::A)));
  ctx.insert_var("B",           Value::Constant(Constant::InputButton(Button::B)));
  ctx.insert_var("X",           Value::Constant(Constant::InputButton(Button::X)));
  ctx.insert_var("Y",           Value::Constant(Constant::InputButton(Button::Y)));
  ctx.insert_var("Start",       Value::Constant(Constant::InputButton(Button::Start)));
  ctx.insert_var("Back",        Value::Constant(Constant::InputButton(Button::Back)));
  ctx.insert_var("LStickPress", Value::Constant(Constant::InputButton(Button::LStick)));
  ctx.insert_var("RStickPress", Value::Constant(Constant::InputButton(Button::RStick)));
  ctx.insert_var("DPadUp",      Value::Constant(Constant::InputButton(Button::DPadUp)));
  ctx.insert_var("DPadLeft",    Value::Constant(Constant::InputButton(Button::DPadLeft)));
  ctx.insert_var("DPadDown",    Value::Constant(Constant::InputButton(Button::DPadDown)));
  ctx.insert_var("DPadRight",   Value::Constant(Constant::InputButton(Button::DPadRight)));

  let mut ms = HashMap::new();

  ms.insert("X".to_string(),     Value::Constant(Constant::MouseAxis(MouseAxis::X)));
  ms.insert("Y".to_string(),     Value::Constant(Constant::MouseAxis(MouseAxis::Y)));
  ms.insert("Wheel".to_string(), Value::Constant(Constant::MouseAxis(MouseAxis::Wheel)));
  ms.insert("LB".to_string(),    Value::Constant(Constant::MouseButton(MouseButton::Left)));
  ms.insert("RB".to_string(),    Value::Constant(Constant::MouseButton(MouseButton::Right)));
  ms.insert("MB".to_string(),    Value::Constant(Constant::MouseButton(MouseButton::Middle)));

  ctx.insert_var("Ms", Value::Struct(ms));

  let mut kb = HashMap::new();

  use strum::IntoEnumIterator;

  for key in KeyboardKey::iter() {
    kb.insert(key.to_string(), Value::Constant(Constant::KeyboardKey(key)));
  }

  ctx.insert_var("Kb", Value::Struct(kb));

  ctx.register_fun("as_axis", move |args, opts| match args {
    [Value::PipelineB(p)] => {
      if let (Some(Value::Number(value)), Some(Value::Boolean(repeat))) = (opts.get("value"), opts.get("repeat")) {
        Ok(Value::Pipeline1D(as_axis_input(Rc::clone(p), *value, *repeat)))
      } else {
        Err(None)
      }
    },
    _ => Err(None)
  });

  ctx.register_fun("as_line_segment_button", move |args, opts| match args {
    [Value::Pipeline1D(p)] => {
      if let (Some(Value::Number(from)), Some(Value::Number(to)), Some(Value::Number(margin))) =
        (opts.get("from"), opts.get("to"), opts.get("margin"))
      {
        Ok(Value::PipelineB(line_segment_button(Rc::clone(p), *from, *to, *margin)))
      } else {
        Err(None)
      }
    },
    _ => Err(None)
  });

  ctx.register_fun("as_ring_sector_button", move |args, opts| match args {
    [Value::Pipeline2D(p)] => {
      if let (
        Some(Value::Number(direction)),
        Some(Value::Number(angle)),
        Some(Value::Number(inner_radius)),
        Some(Value::Number(outer_radius)),
        Some(Value::Number(margin))
      ) = (
        opts.get("direction"),
        opts.get("angle"),
        opts.get("inner_radius"),
        opts.get("outer_radius"),
        opts.get("margin")
      ) {
        let opts = RingSectorButtonOpts {
          direction:    *direction,
          angle:        *angle,
          inner_radius: *inner_radius,
          outer_radius: *outer_radius,
          margin:       *margin
        };
        Ok(Value::PipelineB(ring_sector_button(Rc::clone(p), opts)))
      } else {
        Err(None)
      }
    },
    _ => Err(None)
  });

  ctx.register_fun("bind", move |args, _| match args {
    [Value::Pipeline1D(p), Value::Constant(Constant::MouseAxis(a))] => {
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(mouse_move(Rc::clone(p), *a))))
    },
    [Value::PipelineB(p), Value::Constant(Constant::MouseButton(b))] => {
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(mouse_button_press(Rc::clone(p), *b))))
    },
    [Value::PipelineB(p), Value::Constant(Constant::KeyboardKey(b))] => {
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(keyboard_key_press(Rc::clone(p), *b))))
    },
    _ => Err(None)
  });

  ctx.register_fun("cartesian", move |args, _| match args {
    [Value::Pipeline2D(p)] => Ok(Value::Pipeline2D(cartesian(Rc::clone(p)))),
    _ => Err(None)
  });

  ctx.register_fun("cutoff", move |args, _| match args {
    [Value::Pipeline1D(p), Value::Number(n)] => Ok(Value::Pipeline1D(cutoff(Rc::clone(p), *n))),
    _ => Err(None)
  });

  ctx.register_fun("cycle_modes", move |args, _| match args {
    [Value::PipelineB(p), Value::List(list)] => {
      let mut masks = vec![];
      for item in list {
        if let Value::LayerMask(mask) = item {
          masks.push(*mask);
        } else {
          return Err(None);
        }
      }
      let p = cycle_modes(Rc::clone(p), masks);
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(p)))
    },
    _ => Err(None)
  });

  ctx.register_fun("deadzone", move |args, _| match args {
    [Value::Pipeline1D(joystick), Value::Number(d)] => {
      Ok(Value::Pipeline1D(deadzone(Rc::clone(joystick), *d)))
    },
    [Value::Pipeline2D(joystick), Value::Number(d)] => {
      Ok(Value::Pipeline2D(cartesian_deadzone(Rc::clone(joystick), *d)))
    },
    _ => Err(None)
  });

  ctx.register_fun("distance_from_center", move |args, _| match args {
    [Value::Pipeline2D(p)] => Ok(Value::Pipeline1D(distance_from_center(Rc::clone(p)))),
    _ => Err(None)
  });

  ctx.register_fun("gate", move |args, _| match args {
    [Value::Pipeline1D(p), Value::PipelineB(m)] => Ok(Value::Pipeline1D(gate_axis(Rc::clone(p), Rc::clone(m)))),
    [Value::PipelineB(p), Value::PipelineB(m)]  => Ok(Value::PipelineB(gate_button(Rc::clone(p), Rc::clone(m)))),
    _ => Err(None)
  });

  ctx.register_fun("hex_grid_menu", move |args, _| {
    let default_menu_opts = TouchMenuOpts::HexGrid {
      margin: 0.015 // ?
    };

    // TODO: doesn't work when items.len() == 1
    let (menu, number_of_items) = match args {
      [Value::Pipeline2D(xy), Value::PipelineB(toggle), Value::List(items)] => {
        if let Some(items) = util::strings(items) {
          let n = items.len();
          (touch_menu(xy.clone(), Rc::clone(toggle), invert(Rc::clone(toggle)), items, default_menu_opts), n)
        } else {
          return Err(Some("items should only contain string values".to_string()));
        }
      },
      [Value::Pipeline2D(xy), Value::PipelineB(toggle), Value::PipelineB(select), Value::List(items)] => {
        if let Some(items) = util::strings(items) {
          let n = items.len();
          (touch_menu(xy.clone(), Rc::clone(toggle), Rc::clone(select), items, default_menu_opts), n)
        } else {
          return Err(Some("items should only contain string values".to_string()));
        }
      },
      _ => return Err(None)
    };

    let mut buttons = vec![];
    for i in 0..number_of_items {
      buttons.push(Value::PipelineB(menu_item(Rc::clone(&menu), i as u8)));
    }

    Ok(Value::List(buttons))
  });

  ctx.register_fun("input", move |args, _| match args {
    [Value::Number(n)]                          => Ok(Value::Pipeline1D(constant_input(*n))),
    [Value::Boolean(b)]                         => Ok(Value::PipelineB(dummy_button_input(*b))),
    [Value::Constant(Constant::InputAxis(a))]   => Ok(Value::Pipeline1D(axis_input(*a))),
    [Value::Constant(Constant::InputButton(b))] => Ok(Value::PipelineB(button_input(*b))),
    _ => Err(None)
  });

  ctx.register_fun("invert", move |args, _| match args {
    [Value::PipelineB(p)] => Ok(Value::PipelineB(invert(Rc::clone(p)))),
    _ => Err(None)
  });

  ctx.register_fun("left_trigger_bump", move |args, _| match args {
    [Value::PipelineB(button)] => {
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(trigger_bump(Rc::clone(button), true))))
    },
    _ => Err(None)
  });

  ctx.register_fun("memory_probe", move |args, _| match args {
    [Value::String(spec)] => Ok(Value::PipelineB(memory_probe(spec).expect("memory_probe"))),
    _ => Err(None)
  });

  ctx.register_fun("merge", move |args, _| match args {
    [Value::Pipeline1D(x), Value::Pipeline1D(y)] => Ok(Value::Pipeline2D(merge(Rc::clone(x), Rc::clone(y)))),
    _ => Err(None)
  });

  ctx.register_fun("offset", move |args, _| match args {
    [Value::Pipeline1D(p), Value::Number(addend)] => {
      Ok(Value::Pipeline1D(offset(Rc::clone(p), *addend)))
    },
    [Value::Pipeline1D(p), Value::Pipeline1D(addend)] => {
      Ok(Value::Pipeline1D(offset_by_axis(Rc::clone(p), Rc::clone(addend))))
    },
    _ => Err(None)
  });

  ctx.register_fun("polar", move |args, _| match args {
    [Value::Pipeline2D(p)] => Ok(Value::Pipeline2D(polar(Rc::clone(p)))),
    _ => Err(None)
  });

  ctx.register_fun("pulse", move |args, _| match args {
    [Value::PipelineB(p), Value::Number(freq), Value::Number(width)] => {
      Ok(Value::PipelineB(pulse(Rc::clone(p), *freq, *width)))
    },
    [Value::PipelineB(p), Value::Number(freq), Value::Pipeline1D(width)] => {
      Ok(Value::PipelineB(pulse_by_axis(Rc::clone(p), Rc::clone(&constant_input(*freq)), Rc::clone(width))))
    },
    [Value::PipelineB(p), Value::Pipeline1D(freq), Value::Number(width)] => {
      Ok(Value::PipelineB(pulse_by_axis(Rc::clone(p), Rc::clone(freq), Rc::clone(&constant_input(*width)))))
    },
    [Value::PipelineB(p), Value::Pipeline1D(freq), Value::Pipeline1D(width)] => {
      Ok(Value::PipelineB(pulse_by_axis(Rc::clone(p), Rc::clone(freq), Rc::clone(width))))
    },
    _ => Err(None)
  });

  ctx.register_fun("radial_menu", move |args, _| {
    let default_menu_opts = TouchMenuOpts::Radial {
      inner_radius: 0.25, // ?
      outer_radius: 1.0,  // ?
      margin:       0.03  // ?
    };

    let (menu, number_of_items) = match args {
      [Value::Pipeline2D(xy), Value::PipelineB(toggle), Value::List(items)] => {
        if let Some(items) = util::strings(items) {
          let n = items.len();
          (touch_menu(xy.clone(), Rc::clone(toggle), invert(Rc::clone(toggle)), items, default_menu_opts), n)
        } else {
          return Err(Some("items should only contain string values".to_string()));
        }
      },
      [Value::Pipeline2D(xy), Value::PipelineB(toggle), Value::PipelineB(select), Value::List(items)] => {
        if let Some(items) = util::strings(items) {
          let n = items.len();
          (touch_menu(xy.clone(), Rc::clone(toggle), Rc::clone(select), items, default_menu_opts), n)
        } else {
          return Err(Some("items should only contain string values".to_string()));
        }
      },
      _ => return Err(None)
    };

    let mut buttons = vec![];
    for i in 0..number_of_items {
      buttons.push(Value::PipelineB(menu_item(Rc::clone(&menu), i as u8)));
    }

    Ok(Value::List(buttons))
  });

  ctx.register_fun("relative", move |args, _| match args {
    [Value::Pipeline1D(axis), Value::PipelineB(button)] => {
      Ok(Value::Pipeline1D(relative(Rc::clone(axis), Rc::clone(button))))
    },
    _ => Err(None)
  });

  ctx.register_fun("right_trigger_bump", move |args, _| match args {
    [Value::PipelineB(button)] => {
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(trigger_bump(Rc::clone(button), false))))
    },
    _ => Err(None)
  });

  ctx.register_fun("rotate", move |args, _| match args {
    [Value::Pipeline2D(p), Value::Number(angle)] => {
      Ok(Value::Pipeline2D(rotate(Rc::clone(p), *angle)))
    },
    [Value::Pipeline2D(p), Value::Pipeline1D(angle)] => {
      Ok(Value::Pipeline2D(rotate_by_axis(Rc::clone(p), Rc::clone(angle))))
    },
    _ => Err(None)
  });

  ctx.register_fun("scale", move |args, _| match args {
    [Value::Pipeline1D(p), Value::Number(factor)] => {
      Ok(Value::Pipeline1D(scale(Rc::clone(p), *factor)))
    },
    [Value::Pipeline1D(p), Value::Pipeline1D(factor)] => {
      Ok(Value::Pipeline1D(scale_by_axis(Rc::clone(p), Rc::clone(factor))))
    },
    _ => Err(None)
  });

  ctx.register_fun("screen_probe", move |args, opts| match args {
    [] => {
      if let (
        Some(Value::Number(x1)),
        Some(Value::Number(y1)),
        Some(Value::Number(x2)),
        Some(Value::Number(y2)),
        Some(Value::Number(min_hue)),
        Some(Value::Number(max_hue)),
        Some(Value::Number(min_sat)),
        Some(Value::Number(max_sat)),
        Some(Value::Number(min_val)),
        Some(Value::Number(max_val)),
        Some(Value::Number(threshold1)),
        Some(Value::Number(threshold2))
      ) = (
        opts.get("x1"),
        opts.get("y1"),
        opts.get("x2"),
        opts.get("y2"),
        opts.get("min_hue")   .or(Some(&Value::Number(  0.0))),
        opts.get("max_hue")   .or(Some(&Value::Number(360.0))),
        opts.get("min_sat")   .or(Some(&Value::Number(  0.0))),
        opts.get("max_sat")   .or(Some(&Value::Number(  1.0))),
        opts.get("min_val")   .or(Some(&Value::Number(  0.0))),
        opts.get("max_val")   .or(Some(&Value::Number(  1.0))),
        opts.get("threshold1").or(Some(&Value::Number(  1.0))),
        opts.get("threshold2").or(Some(&Value::Number(  1.0)))
      ) {
        assert!(x1 < x2);
        assert!(y1 < y2);
        assert!(*min_hue >= 0.0 && *min_hue <  360.0);
        assert!(*max_hue >  0.0 && *max_hue <= 360.0);
        assert!(*min_sat >= 0.0 && *min_sat <  1.0);
        assert!(*max_sat >  0.0 && *max_sat <= 1.0);
        assert!(*min_val >= 0.0 && *min_val <  1.0);
        assert!(*max_val >  0.0 && *max_val <= 1.0);
        assert!(*threshold1 > 0.0);
        assert!(*threshold2 > 0.0);

        let target = overlay_ipc::ScreenScrapingArea {
          bounds:  overlay_ipc::Rect {
            min: overlay_ipc::Point { x: overlay_ipc::Length::px(*x1), y: overlay_ipc::Length::px(*y1) },
            max: overlay_ipc::Point { x: overlay_ipc::Length::px(*x2), y: overlay_ipc::Length::px(*y2) }
          },
          min_hue: *min_hue,
          max_hue: *max_hue,
          min_sat: *min_sat,
          max_sat: *max_sat,
          min_val: *min_val,
          max_val: *max_val
        };

        Ok(Value::PipelineB(screen_probe(target, (*threshold1, *threshold2))))
      } else {
        Err(None)
      }
    },
    _ => Err(None)
  });

  ctx.register_fun("set_mode", move |args, _| match args {
    [Value::PipelineB(p), Value::LayerMask(mask)] => {
      Ok(Value::CompletePipeline(LayerMask::EMPTY, Rc::new(switch_mode(Rc::clone(p), *mask))))
    },
    _ => Err(None)
  });

  ctx.register_fun("smooth", move |args, _| match args {
    [Value::Pipeline1D(p), Value::Number(n)] => Ok(Value::Pipeline1D(smooth(Rc::clone(p), *n))),
    _ => Err(None)
  });

  ctx.register_fun("split", move |args, _| match args {
    [Value::Pipeline2D(p)] => {
      let p0 = select0(Rc::clone(p));
      let p1 = select1(Rc::clone(p));
      Ok(Value::List(vec![Value::Pipeline1D(p0), Value::Pipeline1D(p1)]))
    },
    _ => Err(None)
  });

  ctx.register_fun("twitch_joymouse", move |args, _| match args {
    [Value::Pipeline2D(joystick)] => Ok(Value::Pipeline2D(twitch_joymouse(Rc::clone(joystick)))),
    _ => Err(None)
  });
}

#[cfg(not(test))]
pub fn load_config(script: &str, knob_values: Option<HashMap<String, Value>>) -> Result<Config, String> {
  match parser::parse_config(script) {
    Ok(config) => {
      let mut context = eval::Context::new(knob_values);
      register_defaults(&mut context);

      let config = eval::eval_config(config, &mut context).map_err(|err| match err {
        EvalError(message, Some(location)) => {
          format!("{}\n{} at {}:{}", location.show_in_source(script), message, location.0.0, location.0.1)
        },
        EvalError(message, None) => message
      })?;

      let mut pipelines = vec![];

      for res in util::flatten(config) {
        match res {
          Value::CompletePipeline(mask, p) => {
            let mask = if mask == LayerMask::EMPTY { LayerMask::ALL_USER_BITS } else { mask };
            pipelines.push((mask, std::rc::Rc::try_unwrap(p).unwrap_or_else(|_| panic!("Binding already consumed"))));
          },
          whatever => {
            return Err(format!("Expected closed pipeline, got {:?}", whatever));
          }
        }
      }

      Ok(Config { pipelines, layers: context.layers, knobs: context.knobs })
    },
    Err(err) => Err(err)
  }
}
