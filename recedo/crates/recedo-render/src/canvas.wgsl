// Recedo canvas display. One draw per canvas chunk texture: every screen pixel is mapped
// back to canvas coordinates; pixels in this chunk show the canvas (premultiplied) over a
// checkerboard, the rest are discarded (the target was cleared to the background colour).

struct View {
    // canvas = (dot(inv0.xyz, (sx, sy, 1)), dot(inv1.xyz, (sx, sy, 1)))
    inv0: vec4<f32>,
    inv1: vec4<f32>,
    // chunk origin x, y and size w, h in canvas pixels
    chunk: vec4<f32>,
    // x: mip level to sample, y: 1 = nearest pixels, z: checker cell size, w: 1 = sRGB target
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: View;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_canvas(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let s = vec3<f32>(pos.xy, 1.0);
    let c = vec2<f32>(dot(u.inv0.xyz, s), dot(u.inv1.xyz, s)) - u.chunk.xy;
    if (c.x < 0.0 || c.y < 0.0 || c.x >= u.chunk.z || c.y >= u.chunk.w) {
        discard;
    }
    var px: vec4<f32>;
    if (u.params.y > 0.5) {
        let size = vec2<i32>(u.chunk.zw);
        px = textureLoad(tex, clamp(vec2<i32>(floor(c)), vec2<i32>(0), size - 1), 0);
    } else {
        px = textureSampleLevel(tex, samp, c / u.chunk.zw, u.params.x);
    }
    let cell = vec2<i32>(floor(pos.xy / u.params.z));
    let checker = select(0.6, 0.75, ((cell.x + cell.y) & 1) == 0);
    var out = px.rgb + vec3<f32>(checker) * (1.0 - px.a);
    if (u.params.w > 0.5) {
        out = srgb_to_linear(out);
    }
    return vec4<f32>(out, 1.0);
}
