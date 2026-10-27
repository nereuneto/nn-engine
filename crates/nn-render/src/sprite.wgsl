// Shader de sprites com instancing: cada instância é um sprite e os seis
// vértices do quad (dois triângulos) são gerados aqui, sem vertex buffer.
//
// Uma instância com repeat.x < 0 é um triângulo de uma forma (linha,
// contorno, polígono): os cantos vêm em position, size e origin, e repeat.y
// diz quais lados são a borda da forma (bits: 1 = do canto 0 ao 1, 2 = do 1
// ao 2, 4 = do 2 ao 0). Esses lados ganham a borda suave.

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
    @location(7) offset: vec2<f32>, // deslocamento dentro das repetições
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
    @location(5) @interpolate(flat) offset: vec2<f32>,
    // Num triângulo de forma: as coordenadas baricêntricas e os lados de borda
    // (num sprite, edges = 0 e a borda não muda nada).
    @location(6) bary: vec3<f32>,
    @location(7) @interpolate(flat) edges: f32,
    // A posição dentro do sprite (0 a 1, com 0, 0 no canto de cima à
    // esquerda da imagem), para os efeitos.
    @location(8) local: vec2<f32>,
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
    out.bary = vec3<f32>(0.0);
    out.edges = 0.0;
    if (inst.repeat.x < 0.0) {
        // Triângulo de forma: os vértices 3 a 5 ficam no canto 2 (sem área).
        let i = min(vertex_index, 2u);
        var points = array<vec2<f32>, 3>(inst.position, inst.size, inst.origin);
        var barys = array<vec3<f32>, 3>(
            vec3<f32>(1.0, 0.0, 0.0),
            vec3<f32>(0.0, 1.0, 0.0),
            vec3<f32>(0.0, 0.0, 1.0),
        );
        out.clip_position = view.view_proj * vec4<f32>(points[i], 0.0, 1.0);
        out.uv = inst.region.xy + 0.5 * inst.region.zw;
        out.color = inst.color;
        out.tile = vec2<f32>(0.0);
        out.region = inst.region;
        out.repeat = vec2<f32>(1.0);
        out.offset = vec2<f32>(0.0);
        out.bary = barys[i];
        out.edges = inst.repeat.y;
        out.local = barys[i].xy;
        return out;
    }
    out.clip_position = view.view_proj * vec4<f32>(world, 0.0, 1.0);
    // A imagem tem V crescendo para baixo. No mundo (Y para cima) o canto
    // y = 0 do quad é a base da imagem; na tela (Y para baixo), é o topo.
    let v = select(1.0 - corner.y, corner.y, view.y_down > 0.5);
    out.uv = inst.region.xy + vec2<f32>(corner.x, v) * inst.region.zw;
    out.color = inst.color;
    out.tile = vec2<f32>(corner.x, v) * inst.repeat + inst.offset;
    out.region = inst.region;
    out.repeat = inst.repeat;
    out.offset = inst.offset;
    out.local = vec2<f32>(corner.x, v);
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    // Sem repetição (o caso comum), a coordenada vem pronta do vértice.
    var uv = in.uv;
    if (any(in.repeat != vec2<f32>(1.0)) || any(in.offset != vec2<f32>(0.0))) {
        uv = in.region.xy + fract(in.tile) * in.region.zw;
    }
    // As derivadas vêm da coordenada contínua: sem costura entre as repetições.
    let gx = dpdx(in.tile) * in.region.zw;
    let gy = dpdy(in.tile) * in.region.zw;
    // A borda suave dos triângulos de forma: quanto falta, em pixels, até
    // cada lado de borda (a coordenada baricêntrica do canto oposto).
    let fw = max(fwidth(in.bary), vec3<f32>(1e-6));
    let d = in.bary / fw + vec3<f32>(0.5);
    let bits = u32(in.edges + 0.5);
    var cover = 1.0;
    if ((bits & 1u) != 0u) { cover = min(cover, d.z); }
    if ((bits & 2u) != 0u) { cover = min(cover, d.x); }
    if ((bits & 4u) != 0u) { cover = min(cover, d.y); }
    let color = textureSampleGrad(sprite_texture, sprite_sampler, uv, gx, gy) * in.color;
    return vec4<f32>(color.rgb, color.a * clamp(cover, 0.0, 1.0));
}
