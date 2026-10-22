@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

// Mipmap generation: each texel of level n is the average of 2x2 texels of level n-1
// (one bilinear sample at their shared corner).
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var src_samp: sampler;

@fragment
fn fs_downsample(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let src_size = vec2<f32>(textureDimensions(src));
    return textureSampleLevel(src, src_samp, (pos.xy * 2.0) / src_size, 0.0);
}
