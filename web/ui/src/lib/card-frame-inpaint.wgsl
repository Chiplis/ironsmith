struct Size { width: u32, height: u32, count: u32, unused: u32 }
@group(0) @binding(0) var<storage, read> source: array<vec2u>;
@group(0) @binding(1) var<storage, read_write> destination: array<vec2u>;
@group(0) @binding(2) var<storage, read> original: array<u32>;
@group(0) @binding(3) var<storage, read> mask: array<u32>;
@group(0) @binding(4) var<uniform> size: Size;
@group(0) @binding(5) var<storage, read> offsets: array<vec2i>;

fn rgb(packed: u32) -> vec3f {
  return vec3f(f32(packed & 255u), f32((packed >> 8u) & 255u), f32((packed >> 16u) & 255u));
}
// Match Uint8ClampedArray's nearest-even conversion, including exact .5 ties.
fn clampedByte(value: f32) -> u32 {
  let v = clamp(value, 0.0, 255.0);
  let lo = u32(floor(v));
  return lo + select(0u, 1u, fract(v) > 0.5 || (fract(v) == 0.5 && (lo & 1u) == 1u));
}
fn pack(value: vec3f) -> u32 {
  return clampedByte(value.x) | (clampedByte(value.y) << 8u) | (clampedByte(value.z) << 16u) | 0xff000000u;
}
fn neighbors(p: u32) -> vec4u {
  let x = p % size.width; let y = p / size.width;
  return vec4u(select(size.count, p - 1u, x > 0u), select(size.count, p + 1u, x + 1u < size.width),
    select(size.count, p - size.width, y > 0u), select(size.count, p + size.width, y + 1u < size.height));
}
fn meanKnown(p: u32) -> vec4f {
  let ns = neighbors(p); var sum = vec3f(0.0); var count = 0.0;
  for (var i = 0u; i < 4u; i++) {
    let n = ns[i];
    if (n < size.count && source[n].y != 0u) {sum += rgb(source[n].x); count += 1.0;}
  }
  return vec4f(sum / max(count, 1.0), count);
}
@compute @workgroup_size(128)
fn propagate(@builtin(global_invocation_id) id: vec3u) {
  let p = id.x; if (p >= size.count) {return;}
  destination[p] = source[p];
  if (mask[p] == 0u || source[p].y != 0u) {return;}
  let mean = meanKnown(p);
  if (mean.w > 0.0) {destination[p] = vec2u(pack(mean.xyz), 1u);}
}
@compute @workgroup_size(128)
fn relax(@builtin(global_invocation_id) id: vec3u) {
  let p = id.x; if (p >= size.count) {return;}
  destination[p] = source[p];
  if (mask[p] == 0u) {return;}
  let mean = meanKnown(p);
  if (mean.w > 0.0) {destination[p].x = pack(mean.xyz);}
}
@compute @workgroup_size(128)
fn grain(@builtin(global_invocation_id) id: vec3u) {
  let p = id.x; if (p >= size.count) {return;}
  destination[p] = source[p]; if (mask[p] == 0u) {return;}
  let xy = vec2i(i32(p % size.width), i32(p / size.width));
  for (var attempt = 0u; attempt < 48u; attempt++) {
    let donor = xy + offsets[(p % 8u) * 48u + attempt];
    if (donor.x <= 0 || donor.y <= 0 || donor.x >= i32(size.width) - 1 || donor.y >= i32(size.height) - 1) {continue;}
    let q = u32(donor.y) * size.width + u32(donor.x);
    let ns = neighbors(q);
    if ((mask[q] | mask[ns.x] | mask[ns.y] | mask[ns.z] | mask[ns.w]) != 0u) {continue;}
    let mean = (rgb(original[ns.x]) + rgb(original[ns.y]) + rgb(original[ns.z]) + rgb(original[ns.w])) / 4.0;
    destination[p].x = pack(rgb(source[p].x) + clamp(rgb(original[q]) - mean, vec3f(-6.0), vec3f(6.0)));
    break;
  }
}
