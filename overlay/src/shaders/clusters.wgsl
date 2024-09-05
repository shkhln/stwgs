struct Params {
  x1:      u32,
  y1:      u32,
  x2:      u32,
  y2:      u32,
  min_hue: f32,
  max_hue: f32,
  min_sat: f32,
  max_sat: f32,
  min_val: f32,
  max_val: f32
}

@group(0) @binding(0)
var screen: texture_2d<f32>;

@group(0) @binding(1)
var<storage, read> params: Params;

@group(0) @binding(2)
var<storage, read_write> results: array<f32>;

fn is_color_in_range(pixel: vec4<f32>) -> bool {
  let hsv_ = hsv(pixel[0], pixel[1], pixel[2]);
  let hue  = hsv_[0];
  let sat  = hsv_[1];
  let val  = hsv_[2];
  return
    hue >= params.min_hue && hue <= params.max_hue &&
    sat >= params.min_sat && sat <= params.max_sat &&
    val >= params.min_val && val <= params.max_val;
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  var count = 0;
  var prev: bool = false;

  // vertical line
  if (params.x1 == params.x2) {
    for (var y: u32 = params.y1; y < params.y2; y = y + u32(1)) {
      let pos = vec2<i32>(i32(params.x1), i32(y));
      let pix = textureLoad(screen, pos, 0);
      let cur = is_color_in_range(pix);
      if (cur && !prev) {
        count = count + 1;
      }
      prev = cur;
    }
    results[0] = f32(count);
    return;
  }

  // horizontal line
  if (params.y1 == params.y2) {
    for (var x: u32 = params.x1; x < params.x2; x = x + u32(1)) {
      let pos = vec2<i32>(i32(x), i32(params.y1));
      let pix = textureLoad(screen, pos, 0);
      let cur = is_color_in_range(pix);
      if (cur && !prev) {
        count = count + 1;
      }
      prev = cur;
    }
    results[0] = f32(count);
    return;
  }

  //TODO: diagonal lines
  results[0] = f32(-1);
  return;
}
