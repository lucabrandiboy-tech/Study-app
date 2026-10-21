// Recedo tablet test shaders.
//
// Colours are 8-bit sRGB-encoded and blended as stored (no linearisation), the way Krita
// blends 8-bit sRGB documents.

struct Globals {
    size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

struct DabIn {
    @location(0) center: vec2<f32>,
    @location(1) radii: vec2<f32>,
    // x: angle (radians), y: hardness
    @location(2) params: vec4<f32>,
    // premultiplied RGBA
    @location(3) color: vec4<f32>,
};

struct DabOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) hardness: f32,
};

@vertex
fn vs_dab(@builtin(vertex_index) vi: u32, d: DabIn) -> DabOut {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let c = corners[vi];
    // One extra pixel around the ellipse for the anti-aliased edge.
    let local = c * (d.radii + vec2<f32>(1.0, 1.0));
    let ca = cos(d.params.x);
    let sa = sin(d.params.x);
    let p = d.center + vec2<f32>(local.x * ca - local.y * sa, local.x * sa + local.y * ca);
    var o: DabOut;
    o.pos = vec4<f32>(p.x / g.size.x * 2.0 - 1.0, 1.0 - p.y / g.size.y * 2.0, 0.0, 1.0);
    o.uv = local / d.radii;
    o.color = d.color;
    o.hardness = d.params.y;
    return o;
}

@fragment
fn fs_dab(i: DabOut) -> @location(0) vec4<f32> {
    let d = length(i.uv);
    // About one pixel of anti-aliasing at the ellipse edge, in screen space.
    let aa = max(fwidth(d), 1e-4);
    let edge = clamp((1.0 - d) / aa + 0.5, 0.0, 1.0);
    var soft = 1.0;
    if (i.hardness < 0.999) {
        soft = 1.0 - smoothstep(i.hardness, 1.0, d);
    }
    let a = edge * soft;
    return i.color * a;
}

struct Composite {
    show_dots: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var canvas_tex: texture_2d<f32>;
@group(0) @binding(1) var dots_tex: texture_2d<f32>;
@group(0) @binding(2) var<uniform> comp: Composite;

@vertex
fn vs_full(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4<f32> {
    // One triangle covering the screen: (-1,-1), (3,-1), (-1,3).
    let x = f32((vi << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(vi & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_composite(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(pos.xy));
    let dims = vec2<i32>(textureDimensions(canvas_tex));
    if (p.x >= dims.x || p.y >= dims.y) {
        return vec4<f32>(0.12, 0.12, 0.13, 1.0);
    }
    // Checkerboard shows where the canvas is transparent (erased).
    let checker = ((p.x / 8) + (p.y / 8)) % 2;
    let base = select(vec3<f32>(0.80, 0.80, 0.80), vec3<f32>(0.68, 0.68, 0.68), checker == 1);
    let c = textureLoad(canvas_tex, p, 0);
    var col = c.rgb + base * (1.0 - c.a);
    if (comp.show_dots != 0u) {
        let d = textureLoad(dots_tex, p, 0);
        col = d.rgb + col * (1.0 - d.a);
    }
    return vec4<f32>(col, 1.0);
}
