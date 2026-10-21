// O halo do brilho: o alvo com o que brilha é reduzido, borrado e somado
// por cima da imagem.

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct Params {
    // Borrar: a direção, em texels. Somar: o tamanho do alvo, em pixels.
    dir: vec2<f32>,
    strength: f32,
    _pad: f32,
};
@group(0) @binding(2) var<uniform> params: Params;

struct Out {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) i: u32) -> Out {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: Out;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    out.uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

// Reduz: a média de 4 amostras (cada uma já mistura 4 texels).
@fragment
fn fs_down(in: Out) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    var c = textureSample(src, samp, in.uv + vec2<f32>(-t.x, -t.y));
    c = c + textureSample(src, samp, in.uv + vec2<f32>(t.x, -t.y));
    c = c + textureSample(src, samp, in.uv + vec2<f32>(-t.x, t.y));
    c = c + textureSample(src, samp, in.uv + vec2<f32>(t.x, t.y));
    return vec4<f32>(c.rgb * 0.25, 1.0);
}

// Borra numa direção (gaussiana de 9 amostras).
@fragment
fn fs_blur(in: Out) -> @location(0) vec4<f32> {
    let t = params.dir / vec2<f32>(textureDimensions(src));
    var c = textureSample(src, samp, in.uv).rgb * 0.227;
    c = c + (textureSample(src, samp, in.uv + t * 1.385).rgb + textureSample(src, samp, in.uv - t * 1.385).rgb) * 0.316;
    c = c + (textureSample(src, samp, in.uv + t * 3.231).rgb + textureSample(src, samp, in.uv - t * 3.231).rgb) * 0.070;
    return vec4<f32>(c, 1.0);
}

// Soma o halo por cima da imagem (o alvo do 3D usa a mesma área).
@fragment
fn fs_add(in: Out) -> @location(0) vec4<f32> {
    // (Aqui, dir é o tamanho do alvo em pixels.)
    let uv = in.clip.xy / params.dir;
    let c = textureSample(src, samp, uv).rgb * params.strength;
    return vec4<f32>(c, 1.0);
}
