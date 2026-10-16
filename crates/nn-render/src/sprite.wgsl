// Shader de sprites com instancing: cada instância é um sprite e os seis
// vértices do quad (dois triângulos) são gerados aqui, sem vertex buffer.

struct View {
    view_proj: mat4x4<f32>,
    // 1.0 no espaço da tela (Y para baixo), 0.0 no mundo (Y para cima).
    y_down: f32,
};

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct Instance {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) origin: vec2<f32>,
    @location(3) rotation: f32,
    @location(4) region: vec4<f32>, // x, y, largura, altura (UV, origem em cima)
    @location(5) color: vec4<f32>,  // linear
    @location(6) repeat: vec2<f32>, // repetições lado a lado (1, 1 = esticar)
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    // Coordenada nas repetições (0..repeat) e a região, para o fragmento
    // repetir a imagem dentro dela.
    @location(2) tile: vec2<f32>,
    @location(3) @interpolate(flat) region: vec4<f32>,
    @location(4) @interpolate(flat) repeat: vec2<f32>,
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
    out.clip_position = view.view_proj * vec4<f32>(world, 0.0, 1.0);
    // A imagem tem V crescendo para baixo. No mundo (Y para cima) o canto
    // y = 0 do quad é a base da imagem; na tela (Y para baixo), é o topo.
    let v = select(1.0 - corner.y, corner.y, view.y_down > 0.5);
    out.uv = inst.region.xy + vec2<f32>(corner.x, v) * inst.region.zw;
    out.color = inst.color;
    out.tile = vec2<f32>(corner.x, v) * inst.repeat;
    out.region = inst.region;
    out.repeat = inst.repeat;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Sem repetição (o caso comum), a coordenada vem pronta do vértice.
    var uv = in.uv;
    if (any(in.repeat != vec2<f32>(1.0))) {
        uv = in.region.xy + fract(in.tile) * in.region.zw;
    }
    // As derivadas vêm da coordenada contínua: sem costura entre as repetições.
    let gx = dpdx(in.tile) * in.region.zw;
    let gy = dpdy(in.tile) * in.region.zw;
    return textureSampleGrad(sprite_texture, sprite_sampler, uv, gx, gy) * in.color;
}
