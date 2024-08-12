
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

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {

  //TODO: we don't really need multiple targets
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
