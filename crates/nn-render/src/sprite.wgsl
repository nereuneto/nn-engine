// Shader de sprites com instancing: cada instância é um sprite e os seis
// vértices do quad (dois triângulos) são gerados aqui, sem vertex buffer.

struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct Instance {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) origin: vec2<f32>,
    @location(3) rotation: f32,
    @location(4) region: vec4<f32>, // x, y, largura, altura (UV, origem em cima)
    @location(5) color: vec4<f32>,  // linear
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, inst: Instance) -> VertexOut {
    // Cantos do quad, com (0,0) no canto inferior esquerdo.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];

    let local = (corner - inst.origin) * inst.size;
    let s = sin(inst.rotation);
    let c = cos(inst.rotation);
    let rotated = vec2<f32>(local.x * c - local.y * s, local.x * s + local.y * c);
    let world = inst.position + rotated;

    var out: VertexOut;
    out.clip_position = camera.view_proj * vec4<f32>(world, 0.0, 1.0);
    // A imagem tem V crescendo para baixo; o sprite tem Y para cima.
    out.uv = inst.region.xy + vec2<f32>(corner.x, 1.0 - corner.y) * inst.region.zw;
    out.color = inst.color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(sprite_texture, sprite_sampler, in.uv) * in.color;
}
