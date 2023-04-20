use super::*;

struct AxisInputStage {
  stage_id: StageId,
  axis: Axis
}

impl Pipeline<f32> for AxisInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.axis)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> f32 {
    ctx.state.read_axis(self.axis)
  }

  fn reset(&mut self) {}
}

pub fn axis_input(axis: Axis) -> PipelineRef<f32> {
  std::rc::Rc::new(std::cell::RefCell::new(AxisInputStage { stage_id: generate_stage_id(), axis }))
}

struct ButtonInputStage {
  stage_id: StageId,
  button: Button
}

impl Pipeline<bool> for ButtonInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.button)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> bool {
    ctx.state.read_button(self.button)
  }

  fn reset(&mut self) {}
}

pub fn button_input(button: Button) -> PipelineRef<bool> {
  std::rc::Rc::new(std::cell::RefCell::new(ButtonInputStage { stage_id: generate_stage_id(), button }))
}

struct DummyButtonInput {
  stage_id: StageId,
  value: bool
}

impl Pipeline<bool> for DummyButtonInput {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.value)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, _: &Context, _: &mut Vec<Action>) -> bool {
    self.value
  }

  fn reset(&mut self) {}
}

pub fn dummy_button_input(value: bool) -> PipelineRef<bool> {
  std::rc::Rc::new(std::cell::RefCell::new(DummyButtonInput { stage_id: generate_stage_id(), value }))
}

struct ConstantInputStage {
  stage_id: StageId,
  value: f32
}

impl Pipeline<f32> for ConstantInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "input"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}", self.value)
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, _: &Context, _: &mut Vec<Action>) -> f32 {
    self.value
  }

  fn reset(&mut self) {}
}

pub fn constant_input(value: f32) -> PipelineRef<f32> {
  std::rc::Rc::new(std::cell::RefCell::new(ConstantInputStage { stage_id: generate_stage_id(), value }))
}

struct ScreenScrapingInputStage {
  stage_id:  StageId,
  target:    overlay_ipc::ScreenScrapingArea,
  threshold: (f32, f32)
}

impl Pipeline<bool> for ScreenScrapingInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "screen_probe"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{:?}, thresholds = {:?}", self.target, self.threshold)
  }

  fn probe(&self) -> Option<Probe> {
    Some(Probe::Screen { target: self.target.clone() })
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> bool {
    let (pixels_in_range, uniformity_score): (f32, f32) = unsafe { ctx.probe_values[&self.stage_id].ff32 };
    pixels_in_range >= self.threshold.0 || uniformity_score >= self.threshold.1
  }

  fn reset(&mut self) {}
}

pub fn screen_probe(target: overlay_ipc::ScreenScrapingArea, threshold: (f32, f32)) -> PipelineRef<bool> {
  std::rc::Rc::new(std::cell::RefCell::new(ScreenScrapingInputStage { stage_id: generate_stage_id(), target, threshold }))
}

struct MemoryInputStage {
  stage_id: StageId,
  usize:    u8,
  address:  u64,
  offsets:  Vec<i32>,
  relation: Relation,
  var_type: VarType,
  values:   Vec<u64>
}

impl Pipeline<bool> for MemoryInputStage {

  fn stage_id(&self) -> StageId {
    self.stage_id
  }

  fn name(&self) -> &'static str {
    "memory_probe"
  }

  fn desc(&self) -> String {
    format!("{}({})", self.name(), self.opts())
  }

  fn opts(&self) -> String {
    format!("{}, 0x{:x}, {:?}, {:?}, {:?}, {:?}",
      self.usize, self.address, self.offsets, self.relation, self.var_type, self.values)
  }

  fn probe(&self) -> Option<Probe> {
    Some(Probe::Memory { usize: self.usize, address: self.address, offsets: self.offsets.clone() })
  }

  fn inspect(&self, out: &mut HashMap<StageId, PipelineStageDescription>) {
    insert_stage_description(out, self);
  }

  fn apply(&mut self, ctx: &Context, _: &mut Vec<Action>) -> bool {

    fn compare_u<T>(rel: &Relation, var: T, values: &Vec<u64>) -> bool where u64: From<T> {
      match rel {
        Relation::Eq =>  values.contains(&u64::from(var)),
        Relation::Ne => !values.contains(&u64::from(var)),
        Relation::Gt => {
          assert_eq!(values.len(), 1);
          values[0] > u64::from(var)
        },
        Relation::Lt => {
          assert_eq!(values.len(), 1);
          values[0] < u64::from(var)
        },
        Relation::Ge => {
          assert_eq!(values.len(), 1);
          values[0] >= u64::from(var)
        },
        Relation::Le => {
          assert_eq!(values.len(), 1);
          values[0] <= u64::from(var)
        }
      }
    }

    fn compare_s<T: Copy>(rel: &Relation, var: T, values: &Vec<u64>) -> bool where i64: From<T> {
      match rel {
        Relation::Eq =>  values.iter().any(|val| *val as i64 == i64::from(var)),
        Relation::Ne => !values.iter().all(|val| *val as i64 != i64::from(var)),
        Relation::Gt => {
          assert_eq!(values.len(), 1);
          (values[0] as i64) > i64::from(var)
        },
        Relation::Lt => {
          assert_eq!(values.len(), 1);
          (values[0] as i64) < i64::from(var)
        },
        Relation::Ge => {
          assert_eq!(values.len(), 1);
          (values[0] as i64) >= i64::from(var)
        },
        Relation::Le => {
          assert_eq!(values.len(), 1);
          (values[0] as i64) <= i64::from(var)
        }
      }
    }

    let probe_value = unsafe { ctx.probe_values[&self.stage_id].u64 };

    match self.var_type {
      VarType::I8  => compare_s(&self.relation, probe_value as  i8, &self.values),
      VarType::U8  => compare_u(&self.relation, probe_value as  u8, &self.values),
      VarType::I16 => compare_s(&self.relation, probe_value as i16, &self.values),
      VarType::U16 => compare_u(&self.relation, probe_value as u16, &self.values),
      VarType::I32 => compare_s(&self.relation, probe_value as i32, &self.values),
      VarType::U32 => compare_u(&self.relation, probe_value as u32, &self.values),
      VarType::I64 => compare_s(&self.relation, probe_value as i64, &self.values),
      VarType::U64 => compare_u(&self.relation, probe_value,        &self.values)
    }
  }

  fn reset(&mut self) {}
}

#[derive(Debug)]
enum VarType {
  I8, U8, I16, U16, I32, U32, I64, U64
}

#[derive(Debug)]
enum Relation {
  Eq, Ne, Gt, Lt, Ge, Le
}

pub fn memory_probe(spec: &str) -> Result<PipelineRef<bool>, String> {

  fn to_number(s: &str, pos: usize) -> Result<u64, String> {
    (if let Some(hex) = s.strip_prefix("0x") { u64::from_str_radix(hex, 16) } else { s.parse::<u64>() })
      .map_err(|e| format!("unexpected input at pos {}: {}", pos, e))
  }

  enum Phase {
    PointerSize,
    EntryPointer,
    Offsets,
    Relation,
    VarType,
    Value,
    End
  }

  let mut stage = MemoryInputStage {
    stage_id: generate_stage_id(),
    usize:    0,
    address:  0,
    offsets:  vec![],
    relation: Relation::Eq,
    var_type: VarType::U64,
    values:   vec![]
  };

  let mut phase = Phase::PointerSize;

  let parts = spec.split(';').collect::<Vec<_>>();
  for i in 0..parts.len() {
    let str = parts[i];
    match phase {
      Phase::PointerSize => {
        match str {
          "32" => stage.usize = 32,
          "64" => stage.usize = 64,
          x => return Err(format!("expected either 32 or 64 at pos {}, got {}", i + 1, x))
        };
        phase = Phase::EntryPointer;
      },
      Phase::EntryPointer => {
        stage.address = to_number(str, i + 1)?;
        phase = Phase::Offsets;
      },
      Phase::Offsets => {
        match str.split_at(1) {
          ("+", s) => stage.offsets.push(  to_number(s, i + 1)? as i32),
          ("-", s) => stage.offsets.push(-(to_number(s, i + 1)? as i32)),
          (x, _) => return Err(format!("expected +|- at pos {}, got {}", i + 1, x))
        };

        if i + 1 == parts.len() {
          return Err(format!("unexpected end of input at pos {}", i + 2));
        }

        if !(parts[i + 1].starts_with('+') || parts[i + 1].starts_with('-')) {
          phase = Phase::Relation;
        }
      },
      Phase::Relation => {
        match str {
          "eq" | "==" => stage.relation = Relation::Eq,
          "ne" | "!=" => stage.relation = Relation::Ne,
          "gt" | ">"  => stage.relation = Relation::Gt,
          "lt" | "<"  => stage.relation = Relation::Lt,
          "ge" | ">=" => stage.relation = Relation::Ge,
          "le" | "<=" => stage.relation = Relation::Le,
          x => return Err(format!("expected eq|ne|gt|lt|ge|le at pos {}, got {}", i + 1, x))
        }
        phase = Phase::VarType;
      },
      Phase::VarType => {
        match str {
          "i8"  => stage.var_type = VarType::I8,
          "u8"  => stage.var_type = VarType::U8,
          "i16" => stage.var_type = VarType::I16,
          "u16" => stage.var_type = VarType::U16,
          "i32" => stage.var_type = VarType::I32,
          "u32" => stage.var_type = VarType::U32,
          "i64" => stage.var_type = VarType::I64,
          "u64" => stage.var_type = VarType::U64,
          x => return Err(format!("expected i8|u8|i16|u16|i32|u32|i64|u64 at pos {}, got {}", i + 1, x))
        }
        phase = Phase::Value;
      },
      Phase::Value => {
        for s in str.split(',') {
          stage.values.push(to_number(s, i + 1)?);
        }
        phase = Phase::End;
      },
      Phase::End => {
        return Err(format!("unexpected input at pos {}: {}", i + 1, str));
      }
    }
  }

  Ok(std::rc::Rc::new(std::cell::RefCell::new(stage)))
}
