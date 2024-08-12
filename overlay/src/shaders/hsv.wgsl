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
