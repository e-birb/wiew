// render the texture as a quad

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var t_sampler: sampler;

var<private> v_positions: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOut {
    var out: VertexOut;

    out.position = vec4<f32>(v_positions[v_idx], 0.0, 1.0);
    out.tex_coords = v_positions[v_idx] * 0.5 + 0.5;
    out.tex_coords.y = 1.0 - out.tex_coords.y;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color = textureSample(texture, t_sampler, in.tex_coords);
    return color;

    // for now, always red:
    //return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}

//let w = textureDimensions(texture).x;
//let h = textureDimensions(texture).y;
//let t_coords = vec2<f32>(in.position.x * 0.5 + 0.5, in.position.y * -0.5 + 0.5);
//let color = textureSample(texture, t_sampler, t_coords);
//return vec4<f32>(color.r, color.g, color.b, 1.0);