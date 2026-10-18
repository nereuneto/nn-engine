// Malhas 3D: luz do sol (direcional) mais luz ambiente do céu e do chão,
// sombreado por face (as normais vêm da malha) e névoa opcional.

struct View {
    view_proj: mat4x4<f32>,
    camera: vec4<f32>,      // posição da câmera (w sem uso)
    sun_dir: vec4<f32>,     // direção PARA onde a luz vai (normalizada)
    sun_color: vec4<f32>,   // cor × intensidade
    sky: vec4<f32>,         // ambiente vindo de cima
    ground: vec4<f32>,      // ambiente vindo de baixo
    fog: vec4<f32>,         // cor da névoa (rgb) e... (a sem uso)
    fog_range: vec4<f32>,   // começo, fim (0, 0 = sem névoa)
};

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
    // Instância: matriz do modelo, matriz das normais e cor.
    @location(4) m0: vec4<f32>,
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
    @location(8) n0: vec4<f32>,
    @location(9) n1: vec4<f32>,
    @location(10) n2: vec4<f32>,
    @location(11) tint: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>,
    @location(3) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let normal_m = mat3x3<f32>(v.n0.xyz, v.n1.xyz, v.n2.xyz);
    let world = model * vec4<f32>(v.position, 1.0);
    var out: VertexOut;
    out.clip = view.view_proj * world;
    out.normal = normalize(normal_m * v.normal);
    out.color = v.color * v.tint;
    out.world = world.xyz;
    out.uv = v.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let albedo = in.color * textureSample(tex, samp, in.uv);
    let sun = max(dot(n, -view.sun_dir.xyz), 0.0);
    let up = n.y * 0.5 + 0.5;
    let ambient = mix(view.ground.rgb, view.sky.rgb, up);
    var rgb = albedo.rgb * (ambient + view.sun_color.rgb * sun);
    if (view.fog_range.y > view.fog_range.x) {
        let d = distance(in.world, view.camera.xyz);
        let f = clamp((d - view.fog_range.x) / (view.fog_range.y - view.fog_range.x), 0.0, 1.0);
        rgb = mix(rgb, view.fog.rgb, f);
    }
    return vec4<f32>(rgb, albedo.a);
}
