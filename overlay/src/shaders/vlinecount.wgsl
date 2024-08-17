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

fn scan_rect(ul_corner_pix: vec4<f32>, ul_corner_x: i32, ul_corner_y: i32, max_width: i32, max_height: i32) -> vec2<i32> {

  var width  = max_width;
  var height = max_height;

  for (var i = 1; i < max_width; i = i + 1) {
    let pos = vec2<i32>(ul_corner_x + i, ul_corner_y);
    let pix = textureLoad(screen, pos, 0);
    if (pix[0] != ul_corner_pix[0] || pix[1] != ul_corner_pix[1] || pix[2] != ul_corner_pix[2]) {
      width = i;
      break;
    }
  }

  for (var i = 1; i < max_height; i = i + 1) {
    let pos = vec2<i32>(ul_corner_x, ul_corner_y + i);
    let pix = textureLoad(screen, pos, 0);
    if (pix[0] != ul_corner_pix[0] || pix[1] != ul_corner_pix[1] || pix[2] != ul_corner_pix[2]) {
      height = i;
      break;
    }
  }

  return vec2<i32>(width, height);
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  var count = 0;

  for (var x: u32 = params.x1; x < params.x2; x = x + u32(1)) {

    let pos = vec2<i32>(i32(x), i32(params.y1));
    let pix = textureLoad(screen, pos, 0);

    let hsv_ = hsv(pix[0], pix[1], pix[2]);
    let hue  = hsv_[0];
    let sat  = hsv_[1];
    let val  = hsv_[2];
    if (hue >= params.min_hue && hue <= params.max_hue
     && sat >= params.min_sat && sat <= params.max_sat
     && val >= params.min_val && val <= params.max_val)
    {
      let wh = scan_rect(pix, i32(x), i32(params.y1), i32(params.x2 - params.x1), i32(params.y2 - params.y1));
      if (wh[0] > 1 || wh[1] > 1) {
        x = x + u32(wh[0]);
        count = count + 1;
      }
    }
  }

  results[0] = f32(count);
}
