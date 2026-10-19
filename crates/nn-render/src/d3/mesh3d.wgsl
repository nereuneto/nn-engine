// Malhas 3D: luz do sol (direcional) com sombra, mais luz ambiente do céu e
// do chão, sombreado por face (as normais vêm da malha) e névoa.

struct View {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    light_view_proj: mat4x4<f32>,
    camera: vec4<f32>,       // posição da câmera
    sun_dir: vec4<f32>,      // direção PARA onde a luz vai (normalizada)
    sun_color: vec4<f32>,    // cor × intensidade
    sky: vec4<f32>,          // ambiente vindo de cima
    ground: vec4<f32>,       // ambiente vindo de baixo
    fog: vec4<f32>,          // cor da névoa (rgb)
    fog_range: vec4<f32>,    // começo, fim (iguais = sem névoa)
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    sky_bottom: vec4<f32>,
    sky_params: vec4<f32>,   // x = cosseno do raio do sol, y = 1 com céu
    shadow: vec4<f32>,       // x = 1 com sombra, y = texel no mundo, z = força
    cam_right: vec4<f32>,    // direita da câmera (partículas)
    cam_up: vec4<f32>,       // cima da câmera
};

@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_samp: sampler_comparison;

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

// 1 = no sol, 0 = na sombra (com as bordas suavizadas).
fn sunlit(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if (view.shadow.x < 0.5) {
        return 1.0;
    }
    // Um pouco para fora da superfície, para ela não sombrear a si mesma.
    let p = view.light_view_proj * vec4<f32>(world + n * view.shadow.y * 1.5, 1.0);
    let ndc = p.xyz / p.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0 || ndc.z < 0.0) {
        return 1.0;
    }
    let size = vec2<f32>(textureDimensions(shadow_map));
    let depth = ndc.z - 0.0004;
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            lit += textureSampleCompareLevel(shadow_map, shadow_samp, uv + vec2<f32>(f32(x), f32(y)) / size, depth);
        }
    }
    lit = lit / 9.0;
    // Perto da borda da área com sombra, ela some aos poucos.
    let edge = max(abs(ndc.x), abs(ndc.y));
    let fade = clamp((edge - 0.85) / 0.15, 0.0, 1.0);
    return mix(mix(1.0, lit, view.shadow.z), 1.0, fade);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let albedo = in.color * textureSample(tex, samp, in.uv);
    var sun = max(dot(n, -view.sun_dir.xyz), 0.0);
    if (sun > 0.0) {
        sun = sun * sunlit(in.world, n);
    }
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

// --- Mapa de sombras: só a profundidade, vista do sol ---------------------

@vertex
fn vs_shadow(v: VertexIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return view.light_view_proj * model * vec4<f32>(v.position, 1.0);
}

// --- Malhas com ossos (modelos animados) ----------------------------------

struct Joints {
    m: array<mat4x4<f32>, 128>,
};
@group(3) @binding(0) var<uniform> joints: Joints;

struct SkinIn {
    @location(12) joint: vec4<u32>,
    @location(13) weight: vec4<f32>,
};

// A matriz do vértice: a média dos ossos dele, pelo peso de cada um.
fn skin_matrix(s: SkinIn) -> mat4x4<f32> {
    return joints.m[s.joint.x] * s.weight.x + joints.m[s.joint.y] * s.weight.y
        + joints.m[s.joint.z] * s.weight.z + joints.m[s.joint.w] * s.weight.w;
}

@vertex
fn vs_skinned(v: VertexIn, s: SkinIn) -> VertexOut {
    let skin = skin_matrix(s);
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let normal_m = mat3x3<f32>(v.n0.xyz, v.n1.xyz, v.n2.xyz);
    let skin_n = mat3x3<f32>(skin[0].xyz, skin[1].xyz, skin[2].xyz);
    let world = model * skin * vec4<f32>(v.position, 1.0);
    var out: VertexOut;
    out.clip = view.view_proj * world;
    out.normal = normalize(normal_m * (skin_n * v.normal));
    out.color = v.color * v.tint;
    out.world = world.xyz;
    out.uv = v.uv;
    return out;
}

@vertex
fn vs_shadow_skinned(v: VertexIn, s: SkinIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return view.light_view_proj * model * skin_matrix(s) * vec4<f32>(v.position, 1.0);
}

// --- Céu: um triângulo que cobre a tela -----------------------------------

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) i: u32) -> SkyOut {
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    let ndc = uv * 2.0 - 1.0;
    var out: SkyOut;
    out.clip = vec4<f32>(ndc, 1.0, 1.0);
    out.ndc = ndc;
    return out;
}

@fragment
fn fs_sky(in: SkyOut) -> @location(0) vec4<f32> {
    let far = view.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let near = view.inv_view_proj * vec4<f32>(in.ndc, 0.0, 1.0);
    let dir = normalize(far.xyz / far.w - near.xyz / near.w);
    let h = dir.y;
    var c: vec3<f32>;
    if (h >= 0.0) {
        c = mix(view.sky_horizon.rgb, view.sky_top.rgb, pow(h, 0.6));
    } else {
        c = mix(view.sky_horizon.rgb, view.sky_bottom.rgb, pow(min(-h * 4.0, 1.0), 0.7));
    }
    let s = dot(dir, -view.sun_dir.xyz);
    let cos_r = view.sky_params.x;
    if (cos_r < 1.0) {
        // Brilho em volta do sol e o disco.
        let glow = pow(max(s, 0.0), 48.0) * 0.35;
        c = c + view.sun_color.rgb * glow;
        let w = (1.0 - cos_r) * 0.25;
        let disk = smoothstep(cos_r - w, cos_r + w, s);
        c = mix(c, min(view.sun_color.rgb * 1.1 + vec3<f32>(0.25), vec3<f32>(1.0)), disk);
    }
    return vec4<f32>(c, 1.0);
}

// --- Partículas: quadrados virados para a câmera ----------------------------

struct ParticleIn {
    @location(0) center_size: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) region: vec4<f32>,  // x, y, largura, altura (uv)
    @location(3) rot: vec4<f32>,     // x = giro em radianos
};

struct ParticleOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
};

@vertex
fn vs_particle(@builtin(vertex_index) i: u32, p: ParticleIn) -> ParticleOut {
    // Dois triângulos: os cantos de -0,5 a 0,5.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5),
    );
    let c = corners[i];
    let s = sin(p.rot.x);
    let k = cos(p.rot.x);
    let r = vec2<f32>(c.x * k - c.y * s, c.x * s + c.y * k) * p.center_size.w;
    let world = p.center_size.xyz + view.cam_right.xyz * r.x + view.cam_up.xyz * r.y;
    var out: ParticleOut;
    out.clip = view.view_proj * vec4<f32>(world, 1.0);
    out.color = p.color;
    out.uv = p.region.xy + vec2<f32>(c.x + 0.5, 0.5 - c.y) * p.region.zw;
    out.world = world;
    return out;
}

fn particle_fog(world: vec3<f32>) -> f32 {
    if (view.fog_range.y > view.fog_range.x) {
        let d = distance(world, view.camera.xyz);
        return clamp((d - view.fog_range.x) / (view.fog_range.y - view.fog_range.x), 0.0, 1.0);
    }
    return 0.0;
}

@fragment
fn fs_particle(in: ParticleOut) -> @location(0) vec4<f32> {
    let c = in.color * textureSample(tex, samp, in.uv);
    return vec4<f32>(mix(c.rgb, view.fog.rgb, particle_fog(in.world)), c.a);
}

@fragment
fn fs_particle_add(in: ParticleOut) -> @location(0) vec4<f32> {
    let c = in.color * textureSample(tex, samp, in.uv);
    return vec4<f32>(c.rgb * (1.0 - particle_fog(in.world)), c.a);
}
