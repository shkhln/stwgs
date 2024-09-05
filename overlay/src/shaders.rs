#[derive(Clone, Copy, Debug)]
pub enum BuiltinShader {
  PixelCount,
  Clusters
}

pub trait ScreenScraper {
  fn bbox(&self) -> egui::Rect;
  fn shader(&self) -> BuiltinShader;
  fn set_parameter(&mut self, name: &str, value: &str) -> Result<(), String>;
  fn write_params(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer);
  fn max_results(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct PixelCount {
  min_x:   f32,
  min_y:   f32,
  max_x:   f32,
  max_y:   f32,
  min_hue: f32,
  max_hue: f32,
  min_sat: f32,
  max_sat: f32,
  min_val: f32,
  max_val: f32
}

impl std::default::Default for PixelCount {

  fn default() -> Self {
    Self  {
      min_x:     0.0,
      min_y:     0.0,
      max_x:     0.0,
      max_y:     0.0,
      min_hue:   0.0,
      max_hue: 360.0,
      min_sat:   0.0,
      max_sat:   1.0,
      min_val:   0.0,
      max_val:   1.0
    }
  }
}

impl ScreenScraper for PixelCount {

  fn bbox(&self) -> egui::Rect {
    let min_x = self.min_x as f32;
    let min_y = self.min_y as f32;
    let max_x = self.max_x as f32;
    let max_y = self.max_y as f32;
    egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
  }

  fn shader(&self) -> BuiltinShader {
    BuiltinShader::PixelCount
  }

  fn set_parameter(&mut self, name: &str, value: &str) -> Result<(), String> {
    match name {
      "min_x"   => self.min_x   = value.parse().map_err(|err| format!("{}", err))?,
      "min_y"   => self.min_y   = value.parse().map_err(|err| format!("{}", err))?,
      "max_x"   => self.max_x   = value.parse().map_err(|err| format!("{}", err))?,
      "max_y"   => self.max_y   = value.parse().map_err(|err| format!("{}", err))?,
      "min_hue" => self.min_hue = value.parse().map_err(|err| format!("{}", err))?,
      "max_hue" => self.max_hue = value.parse().map_err(|err| format!("{}", err))?,
      "min_sat" => self.min_sat = value.parse().map_err(|err| format!("{}", err))?,
      "max_sat" => self.max_sat = value.parse().map_err(|err| format!("{}", err))?,
      "min_val" => self.min_val = value.parse().map_err(|err| format!("{}", err))?,
      "max_val" => self.max_val = value.parse().map_err(|err| format!("{}", err))?,
      _ => return Err(format!("Unknown param: {}", name))
    };
    Ok(())
  }

  fn write_params(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
    let mut offset = 0;

    let values = &[
      self.min_x as u32,
      self.min_y as u32,
      self.max_x as u32,
      self.max_y as u32
    ];
    let bytes = bytemuck::cast_slice::<u32, u8>(values);
    queue.write_buffer(buffer, offset as u64, bytes);
    offset += bytes.len();

    let values = &[
      self.min_hue,
      self.max_hue,
      self.min_sat,
      self.max_sat,
      self.min_val,
      self.max_val,
    ];
    let bytes = bytemuck::cast_slice::<f32, u8>(values);
    queue.write_buffer(buffer, offset as u64, bytes);
  }

  fn max_results(&self) -> usize {
    2
  }
}


#[derive(Clone, Debug)]
pub struct Clusters {
  min_x:   f32,
  min_y:   f32,
  max_x:   f32,
  max_y:   f32,
  min_hue: f32,
  max_hue: f32,
  min_sat: f32,
  max_sat: f32,
  min_val: f32,
  max_val: f32
}

impl std::default::Default for Clusters {

  fn default() -> Self {
    Self  {
      min_x:     0.0,
      min_y:     0.0,
      max_x:     0.0,
      max_y:     0.0,
      min_hue:   0.0,
      max_hue: 360.0,
      min_sat:   0.0,
      max_sat:   1.0,
      min_val:   0.0,
      max_val:   1.0
    }
  }
}

impl ScreenScraper for Clusters {

  fn bbox(&self) -> egui::Rect {
    let min_x = self.min_x as f32;
    let min_y = self.min_y as f32;
    let max_x = self.max_x as f32;
    let max_y = self.max_y as f32;
    egui::Rect::from_min_max(egui::pos2(min_x, min_y), egui::pos2(max_x, max_y))
  }

  fn shader(&self) -> BuiltinShader {
    BuiltinShader::Clusters
  }

  fn set_parameter(&mut self, name: &str, value: &str) -> Result<(), String> {
    match name {
      "min_x"   => self.min_x   = value.parse().map_err(|err| format!("{}", err))?,
      "min_y"   => self.min_y   = value.parse().map_err(|err| format!("{}", err))?,
      "max_x"   => self.max_x   = value.parse().map_err(|err| format!("{}", err))?,
      "max_y"   => self.max_y   = value.parse().map_err(|err| format!("{}", err))?,
      "min_hue" => self.min_hue = value.parse().map_err(|err| format!("{}", err))?,
      "max_hue" => self.max_hue = value.parse().map_err(|err| format!("{}", err))?,
      "min_sat" => self.min_sat = value.parse().map_err(|err| format!("{}", err))?,
      "max_sat" => self.max_sat = value.parse().map_err(|err| format!("{}", err))?,
      "min_val" => self.min_val = value.parse().map_err(|err| format!("{}", err))?,
      "max_val" => self.max_val = value.parse().map_err(|err| format!("{}", err))?,
      _ => return Err(format!("Unknown param: {}", name))
    };
    Ok(())
  }

  fn write_params(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
    let mut offset = 0;

    let values = &[
      self.min_x as u32,
      self.min_y as u32,
      self.max_x as u32,
      self.max_y as u32
    ];
    let bytes = bytemuck::cast_slice::<u32, u8>(values);
    queue.write_buffer(buffer, offset as u64, bytes);
    offset += bytes.len();

    let values = &[
      self.min_hue,
      self.max_hue,
      self.min_sat,
      self.max_sat,
      self.min_val,
      self.max_val,
    ];
    let bytes = bytemuck::cast_slice::<f32, u8>(values);
    queue.write_buffer(buffer, offset as u64, bytes);
  }

  fn max_results(&self) -> usize {
    1
  }
}
