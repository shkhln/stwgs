mod config;
mod controllers;
mod mapper;
mod output;
mod stages;

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc;
use std::{io, thread};

use clap::{Parser, Subcommand, ValueEnum};

use controllers::{available_controllers, find_controller, Axis, Button};
use mapper::LayerMask;
use overlay_ipc::Knob;
use stages::{PipelineStageDescription, StageId};

#[derive(Parser)]
struct Cli {
  #[command(subcommand)]
  command: Option<Command>
}

#[derive(Copy, Clone, ValueEnum)]
enum OutputBackend {
  #[cfg(feature = "x11")]
  X11,
  #[cfg(feature = "evdev")]
  Evdev
}

#[derive(Subcommand)]
enum Command {
  Check {
    script: String
  },
  Load {
    script:     String,
    knobs:      Option<String>,
    #[arg(short, long)]
    controller: Option<String>,
    #[arg(short, long)]
    output:     Option<OutputBackend>
  },
  Test {
    script: String
  },
  Dot {
    script: String
  },
  Knobs {
    script: String,
    knobs:  Option<String>
  },
  List {
    #[arg(short, long)]
    controllers: Option<bool>,
    #[arg(short, long)]
    overlays:    Option<bool>
  }
}

#[cfg(not(test))]
fn main() {

  fn load_config(script: &str, knob_values: Option<HashMap<String, config::Value>>) -> config::Config {
    match config::load_config(script, knob_values) {
      Ok(config) => config,
      Err(err) => {
        eprintln!("Can't load config:\n{}", err);
        std::process::exit(1);
      }
    }
  }

  fn load_config_from_file(path: &str, knob_values: Option<HashMap<String, config::Value>>) -> config::Config {
    load_config(&std::fs::read_to_string(path).unwrap(), knob_values)
  }

  fn load_knobs_from_file(path: &str) -> HashMap<String, config::Value> {

    use serde_json::Value;

    let settings = std::fs::read_to_string(path).unwrap();

    if let Value::Object(settings) = serde_json::from_str(&settings).unwrap() {
      let mut map = HashMap::new();

      for (key, value) in settings {
        match value {
          Value::Bool(v)   => map.insert(key.clone(), config::Value::Boolean(v)),
          Value::Number(v) => map.insert(key.clone(), config::Value::Number(v.as_f64().unwrap() as f32)),
          Value::String(v) => map.insert(key.clone(), config::Value::String(v)),
          _ => panic!()
        };
      }

      map
    } else {
      panic!();
    }
  }

  fn serialize_knobs(knobs: &Vec<Knob>) -> String {

    use serde_json::{json, Value};

    let mut h = serde_json::map::Map::with_capacity(knobs.len());
    for knob in knobs {
      h.insert(
        knob.name().clone(),
        match knob {
          Knob::Enum   { index, options, .. } => json!(options[*index]),
          Knob::Flag   { value, .. }          => json!(value),
          Knob::Number { value, .. }          => json!(value)
        }
      );
    }

    serde_json::to_string_pretty(&Value::Object(h)).unwrap()
  }

  let cli = Cli::parse();

  match cli.command {
    Some(Command::Check { script }) => {
      let config = load_config_from_file(&script, None);
      for (mask, p) in config.pipelines {
        println!("{:?} -> {}", mask, p.desc());
      }
    },
    Some(Command::Load { script, knobs: knobs_path, controller: serial_or_partial_path, output }) => {

      let (controller_state_sender,   controller_state_receiver)   = mpsc::channel();
      let (controller_command_sender, controller_command_receiver) = mpsc::channel();

      //TODO: consider getting rid of thread + channel here
      //TODO: wait for the controller appearance if it's not connected?
      thread::spawn(move || {
        if let Some(controller) = find_controller(serial_or_partial_path).unwrap() {
          controller.run_polling_loop(controller_state_sender, Some(controller_command_receiver)).unwrap();
        } else {
          eprintln!("No controllers found.");
          std::process::exit(1);
        }
      });

      thread::spawn(move || {

        let available_backends = vec![
          #[cfg(feature = "x11")]
          OutputBackend::X11,
          #[cfg(feature = "evdev")]
          OutputBackend::Evdev
        ];

        assert_ne!(available_backends.len(), 0);

        let output: Box<dyn output::MapperIO> = match output.unwrap_or(available_backends[0]) {
          #[cfg(feature = "x11")]
          OutputBackend::X11 => match output::xcb::XcbKeyboardAndMouse::new() {
            Ok(xtest_out) => Box::new(xtest_out),
            Err(e) => {
              eprintln!("Can't initialize xtest keyboard and mouse: {}", e);
              std::process::exit(1);
            }
          },
          #[cfg(feature = "evdev")]
          OutputBackend::Evdev => match output::evdev::UInputKeyboardAndMouse::new() {
            Ok(uinput_out) => Box::new(uinput_out),
            Err(e) => {
              eprintln!("Can't initialize uinput keyboard and mouse: {}", e);
              std::process::exit(1);
            }
          }
        };

        let mut overlay = overlay_ipc::connect_to_overlay().unwrap();
        let mut overlay_required = false;

        let mut knob_values = knobs_path
          .clone()
          .and_then(|path| {
            if Path::new(&path).exists() {
              Some(load_knobs_from_file(&path))
            } else {
              None
            }
          })
          .unwrap_or_default();

        loop {
          if overlay_required && overlay.is_none() {
            eprintln!("Waiting for overlay...");
            loop {
              overlay = overlay_ipc::connect_to_overlay().unwrap();
              if overlay.is_some() {
                break;
              }
              thread::sleep(std::time::Duration::from_secs(1));
              eprint!("*");
            }
          }

          let mut config = load_config_from_file(&script, Some(knob_values.clone()));
          for (mask, p) in &config.pipelines {
            println!("{:?} -> {}", mask, p.desc());
          }

          if overlay.is_some() {

            use stages::*;

            fn toggle_overlay_ui(pipeline: PipelineRef<bool>) -> Box<dyn Pipeline<()>> {

              let mut bstate = to_button_state();

              let fun = Box::new(move |pressed, _, actions: &mut Vec<Action>| {
                if bstate(pressed) == ButtonState::Pressed {
                  actions.push(Action::ToggleOverlayUI);
                }
              });

              Box::new(FnStage::from("toggle_overlay_ui", "".to_string(), pipeline, fun))
            }

            config.pipelines.push((LayerMask::ALL_INTERNAL_BITS, toggle_overlay_ui(button_input(Button::X))));

            fn overlay_menu_command(pipeline: PipelineRef<bool>, command: OverlayMenuCommand) -> Box<dyn Pipeline<()>> {

              let mut bstate = to_button_state();

              let fun = Box::new(move |pressed, _, actions: &mut Vec<Action>| {
                if bstate(pressed) == ButtonState::Pressed {
                  eprintln!("command: {:?}", command);
                  actions.push(Action::SendOverlayMenuCommand(command));
                }
              });

              Box::new(FnStage::from("overlay_menu_command", format!("{:?}", command), pipeline, fun))
            }

            let knobs_menu_layer = LayerMask::internal_layer(0).unwrap();

            //TODO: switch to the last active overlay on Steam button press
            config.pipelines.push((LayerMask::ALL, flip_mode(button_input(Button::Steam), knobs_menu_layer)));
            config.pipelines.push((LayerMask::ALL, overlay_menu_command(       mode_is(knobs_menu_layer),  OverlayMenuCommand::OpenKnobsMenu)));
            config.pipelines.push((LayerMask::ALL, overlay_menu_command(invert(mode_is(knobs_menu_layer)), OverlayMenuCommand::CloseKnobsMenu)));

            fn deg_to_rad(deg: f32) -> f32 {
              deg * std::f32::consts::PI / 180.0
            }

            let left_pad  = merge(axis_input(Axis::LPadX), axis_input(Axis::LPadY));
            let right_pad = merge(axis_input(Axis::RPadX), axis_input(Axis::RPadY));

            let mut opts = RingSectorButtonOpts {
              direction:    0.0,
              angle:        deg_to_rad(120.0),
              inner_radius: 0.25,
              outer_radius: 1.2,
              margin:       0.1
            };

            opts.direction = deg_to_rad( 90.0);
            let up    = ring_sector_button(left_pad .clone(), opts);
            opts.direction = deg_to_rad(-90.0);
            let down  = ring_sector_button(left_pad .clone(), opts);
            opts.direction = deg_to_rad(180.0);
            let left  = ring_sector_button(right_pad.clone(), opts);
            opts.direction = deg_to_rad(  0.0);
            let right = ring_sector_button(right_pad.clone(), opts);

            config.pipelines.push((knobs_menu_layer, overlay_menu_command(up,    OverlayMenuCommand::SelectPrevMenuItem)));
            config.pipelines.push((knobs_menu_layer, overlay_menu_command(down,  OverlayMenuCommand::SelectNextMenuItem)));
            config.pipelines.push((knobs_menu_layer, overlay_menu_command(left,  OverlayMenuCommand::SelectPrevValue)));
            config.pipelines.push((knobs_menu_layer, overlay_menu_command(right, OverlayMenuCommand::SelectNextValue)));

            config.pipelines.push((knobs_menu_layer, overlay_menu_command(button_input(Button::DPadUp),    OverlayMenuCommand::SelectPrevMenuItem)));
            config.pipelines.push((knobs_menu_layer, overlay_menu_command(button_input(Button::DPadDown),  OverlayMenuCommand::SelectNextMenuItem)));
            config.pipelines.push((knobs_menu_layer, overlay_menu_command(button_input(Button::DPadLeft),  OverlayMenuCommand::SelectPrevValue)));
            config.pipelines.push((knobs_menu_layer, overlay_menu_command(button_input(Button::DPadRight), OverlayMenuCommand::SelectNextValue)));
          }

          let mut mapper = mapper::Mapper::new(Some(&controller_command_sender), overlay.as_ref(), config, &*output, 1);

          match mapper.run(&controller_state_receiver) {

            Ok(mapper::ExitReason::KnobsChanged(knobs)) => {
              eprintln!("reconfiguring with knobs {:?}", knobs);
              knob_values.clear();

              for knob in &knobs {
                match knob {
                  Knob::Enum { name, index, options } => {
                    knob_values.insert(name.clone(), config::Value::String(options[*index].clone()));
                  },
                  Knob::Flag { name, value } => {
                    knob_values.insert(name.clone(), config::Value::Boolean(*value));
                  },
                  Knob::Number { name, value, .. } => {
                    knob_values.insert(name.clone(), config::Value::Number(*value));
                  }
                }
              }

              if let Some(knobs_path) = &knobs_path {
                if let Err(e) = std::fs::write(knobs_path, serialize_knobs(&knobs)) {
                  eprintln!("Unable to save knob values: {}", e);
                }
              }
            },

            Ok(mapper::ExitReason::OverlayRequired) => {
              overlay_required = true;
            },

            Err(e) => {
              eprintln!("exiting on error: {}", e);
              //TODO: enable lizard mode
              std::process::exit(1);
            }
          }
        }
      });

      let _ = io::stdin().read_line(&mut String::new());
    },
    Some(Command::Test { script }) => {

      let output = output::DummyOutput {};

      let config = load_config_from_file(&script, None);
      let mut mapper = mapper::Mapper::new(None, None, config, &output, 0);

      let iterations = 1_000_000;
      let start      = std::time::Instant::now();

      mapper.fuzz(iterations);
      println!("{} iterations in {} ms: {} per ms", iterations, start.elapsed().as_millis(), iterations as u128 / start.elapsed().as_millis());
    },
    Some(Command::Dot { script }) => {
      let config::Config { pipelines, layers, .. } = load_config_from_file(&script, None);

      let mut meta = HashMap::new();

      for (_, pipeline) in &pipelines {
        pipeline.inspect(&mut meta);
      }

      let masks = {
        let mut masks = HashMap::new();

        fn scan(
          masks: &mut HashMap<StageId, LayerMask>,
          mask:  LayerMask,
          stage: &PipelineStageDescription,
          meta:  &HashMap<StageId, PipelineStageDescription>
        ) {
          match masks.entry(stage.id) {
            std::collections::hash_map::Entry::Vacant(e) => {
              e.insert(mask);
            },
            std::collections::hash_map::Entry::Occupied(mut e) => {
              *e.get_mut() = *e.get() | mask;
            }
          }

          for input_stage_id in &stage.inputs {
            scan(masks, mask, &meta[input_stage_id], meta);
          }
        }

        for (mask, pipeline) in pipelines {
          scan(&mut masks, mask, &meta[&pipeline.stage_id()], &meta);
        }

        masks
      };

      let edges = {
        let mut edges = vec![];
        for stage in meta.values() {
          for input_stage_id in &stage.inputs {
            edges.push((input_stage_id, stage.id));
          }
        }
        edges.sort();
        edges.dedup();
        edges
      };

      let nodes_by_layer = {
        let mut nodes_by_layer: HashMap<LayerMask, Vec<PipelineStageDescription>> = HashMap::new();

        for stage in meta.values() {
          let mask = masks[&stage.id];
          nodes_by_layer.entry(mask).or_insert_with(Vec::new);
          nodes_by_layer.get_mut(&mask).unwrap().push(stage.clone());
        }

        for nodes in nodes_by_layer.values_mut() {
          nodes.sort_by_key(|stage| stage.id);
        }

        let mut nodes_by_layer = nodes_by_layer.iter()
          .map(|(mask, stages)| (*mask, stages.clone()))
          .collect::<Vec<(LayerMask, Vec<PipelineStageDescription>)>>();
        nodes_by_layer.sort_by_key(|(mask, _)| *mask);
        nodes_by_layer
      };

      println!("digraph {{");
      println!("  rankdir=LR;");
      println!("  node [shape=box];");

      for (mask, nodes) in nodes_by_layer {
        println!("  subgraph cluster_{} {{", mask);

        let mut v = vec![];
        #[allow(clippy::needless_range_loop)]
        for i in 0..layers.len() {
          if mask & LayerMask::user_layer(i).unwrap() != LayerMask::EMPTY {
            v.push(layers[i].clone());
          }
        }
        println!("    label = \"{}\";", v.join(" | "));

        for node in nodes {
          let label = if node.opts.is_empty() {
            node.name.to_string()
          } else {
            format!("{}({})", node.name, node.opts)
          };

          print!("    {:4} [label=\"", node.id);
          if label.len() > 25 {
            print!("{}...", &label[..25]);
          } else {
            print!("{}", label);
          }
          println!("\", tooltip=\"{}: {}\"];", node.id, label);
        }

        println!("  }};");
      }

      for (source, target) in edges {
        println!("  {:4} -> {:4};", source, target);
      }

      println!("}}");
    },
    Some(Command::Knobs { script, knobs }) => {
      let knob_values = knobs.map(|path| load_knobs_from_file(&path));
      let config::Config { knobs, .. } = load_config_from_file(&script, knob_values);
      println!("{}", serialize_knobs(&knobs));
    },
    Some(Command::List { controllers: show_controllers, overlays: _ }) => {
      if show_controllers.unwrap_or(true) {
        let controllers = available_controllers().unwrap();
        if !controllers.is_empty() {
          println!("Found: ");
          for controller in controllers {
            println!("  {}{}{}",
              controller.name(),
              controller.serial().map(|p| format!(" [{}]", p)).unwrap_or_else(|| "".to_string()),
              controller.path()  .map(|p| format!(" @ {}", p)).unwrap_or_else(|| "".to_string()));
          }
        } else {
          eprintln!("No controllers found.");
        }
      }
      //TODO: list overlays
    },
    None => {}
  }
}
