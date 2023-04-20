
struct Target {
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

struct Targets {
  values: array<Target>
}

struct Result {
  pixels_in_range:  f32,
  uniformity_score: f32
}

struct Results {
  values: array<Result>
}

@group(0) @binding(0)
var screen: texture_2d<f32>;

@group(0) @binding(1)
var<storage, read> targets: Targets;

@group(0) @binding(2)
var<storage, read_write> results: Results;

fn hsv_mod(a: f32, b: f32) -> f32 {
  return a - floor(a / b) * b;
}

fn hsv(r: f32, g: f32, b: f32) -> vec3<f32> {
  let M = max(r, max(g, b));
  let m = min(r, min(g, b));
  if (M == 0.0) {
    return vec3<f32>(0.0, 0.0, 0.0);
  }
  let chroma = M - m;
  if (chroma != 0.0) {
    if (M == r) {
      return vec3<f32>(60.0 * hsv_mod((g - b) / chroma, 6.0), chroma / M, M);
    }
    if (M == g) {
      return vec3<f32>(60.0 * ((b - r) / chroma + 2.0),       chroma / M, M);
    }
    if (M == b) {
      return vec3<f32>(60.0 * ((r - g) / chroma + 4.0),       chroma / M, M);
    }
  }
  return vec3<f32>(0.0, 0.0, M);
}

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  //TODO: iterate over multiple targets
  let t = targets.values[0];

  var pixels_in_range_count: u32 = u32(0);
  var seq_matches_count:     u32 = u32(0);
  var uniformity_score           = 0.0;

  for (var x: u32 = t.x1; x < t.x2; x = x + u32(1)) {
    for (var y: u32 = t.y1; y < t.y2; y = y + u32(1)) {

      let pos = vec2<i32>(i32(x), i32(y));
      let pix = textureLoad(screen, pos, 0);

      let hsv_ = hsv(pix[0], pix[1], pix[2]);
      let hue  = hsv_[0];
      let sat  = hsv_[1];
      let val  = hsv_[2];
      if (hue >= t.min_hue && hue <= t.max_hue
       && sat >= t.min_sat && sat <= t.max_sat
       && val >= t.min_val && val <= t.max_val)
      {
        pixels_in_range_count = pixels_in_range_count + u32(1);
        seq_matches_count     = seq_matches_count     + u32(1);
      } else {
        if (seq_matches_count > u32(0)) {
          uniformity_score  = uniformity_score + pow(f32(seq_matches_count), 2.0);
          seq_matches_count = u32(0);
        }
      }
    }
  }

  if (seq_matches_count > u32(0)) {
    uniformity_score = uniformity_score + pow(f32(seq_matches_count), 2.0);
  }

  let pixel_count = f32((t.x2 - t.x1) * (t.y2 - t.y1));

  results.values[0].pixels_in_range  = f32(pixels_in_range_count) / pixel_count;
  results.values[0].uniformity_score = f32(uniformity_score)      / pow(pixel_count, 2.0);
}

//fn vec3_eq(a: vec3<f32>, b: vec3<f32>) -> bool {
//  return a[0] == b[0] && a[1] == b[1] && a[2] == b[2];
//}

//fn check(condition: bool) -> i32 {
//  if (condition) {
//    return 1000;
//  } else {
//    return 1;
//  }
//}

//fn test_hsv() -> i32 {
//  var count = 0;
//  count = count + check(vec3_eq(hsv(0.0,  0.0,  0.0 ), vec3<f32>(  0.0, 0.0, 0.0 ))); // black
//  count = count + check(vec3_eq(hsv(1.0,  1.0,  1.0 ), vec3<f32>(  0.0, 0.0, 1.0 ))); // white
//  count = count + check(vec3_eq(hsv(1.0,  0.0,  0.0 ), vec3<f32>(  0.0, 1.0, 1.0 ))); // red
//  count = count + check(vec3_eq(hsv(0.0,  1.0,  0.0 ), vec3<f32>(120.0, 1.0, 1.0 ))); // lime
//  count = count + check(vec3_eq(hsv(0.0,  0.0,  1.0 ), vec3<f32>(240.0, 1.0, 1.0 ))); // blue
//  count = count + check(vec3_eq(hsv(1.0,  1.0,  0.0 ), vec3<f32>( 60.0, 1.0, 1.0 ))); // yellow
//  count = count + check(vec3_eq(hsv(0.0,  1.0,  1.0 ), vec3<f32>(180.0, 1.0, 1.0 ))); // cyan
//  count = count + check(vec3_eq(hsv(1.0,  0.0,  1.0 ), vec3<f32>(300.0, 1.0, 1.0 ))); // magenta
//  count = count + check(vec3_eq(hsv(0.75, 0.75, 0.75), vec3<f32>(  0.0, 0.0, 0.75))); // silver
//  count = count + check(vec3_eq(hsv(0.5,  0.5,  0.5 ), vec3<f32>(  0.0, 0.0, 0.5 ))); // gray
//  count = count + check(vec3_eq(hsv(0.5,  0.0,  0.0 ), vec3<f32>(  0.0, 1.0, 0.5 ))); // maroon
//  count = count + check(vec3_eq(hsv(0.5,  0.5,  0.0 ), vec3<f32>( 60.0, 1.0, 0.5 ))); // olive
//  count = count + check(vec3_eq(hsv(0.0,  0.5,  0.0 ), vec3<f32>(120.0, 1.0, 0.5 ))); // green
//  count = count + check(vec3_eq(hsv(0.5,  0.0,  0.5 ), vec3<f32>(300.0, 1.0, 0.5 ))); // purple
//  count = count + check(vec3_eq(hsv(0.0,  0.5,  0.5 ), vec3<f32>(180.0, 1.0, 0.5 ))); // teal
//  count = count + check(vec3_eq(hsv(0.0,  0.0,  0.5 ), vec3<f32>(240.0, 1.0, 0.5 ))); // navy
//  return count;
//}
