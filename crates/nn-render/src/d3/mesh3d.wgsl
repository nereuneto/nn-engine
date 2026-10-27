// Malhas 3D: luz do sol (direcional) com sombra, mais luz ambiente do céu e
// do chão, as lâmpadas (e holofotes), o brilho próprio, sombreado por face
// (as normais vêm da malha) e névoa.

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
    sky_params: vec4<f32>,   // x = cosseno do raio do sol, y = 1 com céu, z = estrelas
    shadow: vec4<f32>,       // x = 1 com sombra, y = texel no mundo, z = força
    cam_right: vec4<f32>,    // direita da câmera (partículas)
    cam_up: vec4<f32>,       // cima da câmera
    time_wind: vec4<f32>,    // x = segundos, y = força do vento, zw = direção dele
    // As cascatas do sol (shadow.w = quantas; 0 = o mapa de um só).
    cascades: array<mat4x4<f32>, 3>,
    cascade_texel: vec4<f32>,
};

@group(0) @binding(0) var<uniform> view: View;

struct Lamp {
    pos_range: vec4<f32>,   // posição, alcance
    color: vec4<f32>,       // cor × força
    dir_cos: vec4<f32>,     // holofote: direção, cosseno da borda de fora
    cone: vec4<f32>,        // x = cosseno de onde começa a apagar, y = 1 num holofote,
                            // z = camada da sombra + 1 (0 = sem), w = 2 tan(meia abertura)
};

struct Lamps {
    count: vec4<u32>,
    shadow_mats: array<mat4x4<f32>, 5>,
    items: array<Lamp, 32>,
};

@group(0) @binding(1) var<uniform> lamps: Lamps;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
@group(2) @binding(0) var shadow_map: texture_depth_2d;
@group(2) @binding(1) var shadow_samp: sampler_comparison;
@group(2) @binding(2) var spot_shadow: texture_depth_2d_array;
@group(2) @binding(3) var sun_cascades: texture_depth_2d_array;

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
    // Brilho: rgb = cor que brilha por cima, a = quanto da própria cor.
    @location(14) glow: vec4<f32>,
    // x = 1 na água, y = quanto balança com o vento, z = o recorte por
    // transparência (0 = sem), w = 1 com as duas faces.
    @location(15) material: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) glow: vec4<f32>,
    @location(5) material: vec4<f32>,
};

// O vento: o ponto do mundo balança na direção do vento, mais quanto mais
// alto em relação à base do objeto (embaixo fica parado), com rajadas.
fn sway(world: vec4<f32>, base: vec3<f32>, amount: f32) -> vec4<f32> {
    let strength = view.time_wind.y;
    if (amount <= 0.0 || strength <= 0.0) {
        return world;
    }
    let h = max(world.y - base.y, 0.0);
    let t = view.time_wind.x;
    let phase = dot(base.xz, vec2<f32>(0.37, 0.23));
    let gust = 0.55 + 0.45 * sin(t * 0.7 + phase * 0.5);
    let wave = 0.6 + 0.4 * sin(t * 1.9 + phase + h * 0.6);
    let k = amount * strength * 0.05 * pow(h, 1.4) * gust * wave;
    return vec4<f32>(world.x + view.time_wind.z * k, world.y - k * 0.15 * h * 0.1, world.z + view.time_wind.w * k, world.w);
}

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let normal_m = mat3x3<f32>(v.n0.xyz, v.n1.xyz, v.n2.xyz);
    let world = sway(model * vec4<f32>(v.position, 1.0), v.m3.xyz, v.material.y);
    var out: VertexOut;
    out.clip = view.view_proj * world;
    out.normal = normalize(normal_m * v.normal);
    out.color = v.color * v.tint;
    out.world = world.xyz;
    out.uv = v.uv;
    out.glow = v.glow;
    out.material = v.material;
    return out;
}

// A luz das lâmpadas num ponto: some aos poucos até o alcance; o holofote
// só dentro do cone. Um pouco da luz chega mesmo às faces de lado (a luz
// rebatida), para o chão em volta de uma tocha não ficar preto.
fn lamp_light(world: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = min(lamps.count.x, 32u);
    for (var i = 0u; i < count; i++) {
        sum = sum + lamp_part(lamps.items[i], world, n).rgb;
    }
    return sum;
}

// Como `lamp_light`, com a sombra dos holofotes que fazem sombra (as
// partículas usam a sem sombra: o mapa não está no grupo delas).
fn lamp_light_shadowed(world: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var sum = vec3<f32>(0.0);
    let count = min(lamps.count.x, 32u);
    for (var i = 0u; i < count; i++) {
        let lamp = lamps.items[i];
        var part = lamp_part(lamp, world, n);
        if (lamp.cone.z > 0.5 && part.w > 0.0) {
            let d = distance(lamp.pos_range.xyz, world);
            // color.w: quanto da sombra vale (numa troca, acende aos poucos).
            let lit = spot_lit(u32(lamp.cone.z + 0.5) - 1u, world, n, d, lamp.cone.w);
            part = part * mix(1.0, lit, lamp.color.w);
        }
        sum = sum + part.rgb;
    }
    return sum;
}

// A luz de uma lâmpada num ponto (rgb) e quanto dela chega antes da
// sombra (a; 0 = fora do alcance ou do cone).
fn lamp_part(lamp: Lamp, world: vec3<f32>, n: vec3<f32>) -> vec4<f32> {
    let to = lamp.pos_range.xyz - world;
    let d2 = dot(to, to);
    let range = lamp.pos_range.w;
    if (d2 >= range * range) {
        return vec4<f32>(0.0);
    }
    let l = to * inverseSqrt(max(d2, 1e-8));
    let k = 1.0 - d2 / (range * range);
    var a = k * k;
    if (lamp.cone.y > 0.5) {
        let c = dot(-l, lamp.dir_cos.xyz);
        a = a * smoothstep(lamp.dir_cos.w, lamp.cone.x, c);
    }
    let diffuse = max(dot(n, l), 0.0) * 0.85 + 0.15;
    return vec4<f32>(lamp.color.rgb * (diffuse * a), a);
}

// 1 = a luz do holofote `i` chega, 0 = na sombra (bordas suavizadas).
// `dist` é a distância até ele e `k` o tamanho de um texel por metro.
fn spot_lit(i: u32, world: vec3<f32>, n: vec3<f32>, dist: f32, k: f32) -> f32 {
    let size = vec2<f32>(textureDimensions(spot_shadow).xy);
    // Um pouco para fora da superfície (mais longe, um texel maior).
    let texel = k * dist / size.x;
    let p = lamps.shadow_mats[i] * vec4<f32>(world + n * texel * 1.5, 1.0);
    if (p.w <= 0.0) {
        return 1.0;
    }
    let ndc = p.xyz / p.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0 || ndc.z < 0.0) {
        return 1.0;
    }
    let depth = ndc.z - 0.00002;
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            lit += textureSampleCompareLevel(spot_shadow, shadow_samp, uv + vec2<f32>(f32(x), f32(y)) / size, i32(i), depth);
        }
    }
    return mix(1.0, lit / 9.0, view.shadow.z);
}

// 1 = no sol, 0 = na sombra (com as bordas suavizadas).
fn sunlit(world: vec3<f32>, n: vec3<f32>) -> f32 {
    if (view.shadow.x < 0.5) {
        return 1.0;
    }
    if (view.shadow.w > 1.5) {
        return sunlit_cascades(world, n);
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

// Uma cascata do sol: x = 1 no sol, 0 na sombra; y = o quanto o ponto está
// perto da borda dela (0 no meio, 1 na borda; -1 = fora dela).
fn cascade_sample(i: i32, world: vec3<f32>, n: vec3<f32>) -> vec2<f32> {
    let p = view.cascades[i] * vec4<f32>(world + n * view.cascade_texel[i] * 1.5, 1.0);
    let ndc = p.xyz / p.w;
    let edge = max(abs(ndc.x), abs(ndc.y));
    if (edge > 1.0 || ndc.z > 1.0 || ndc.z < 0.0) {
        return vec2<f32>(1.0, -1.0);
    }
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    let size = vec2<f32>(textureDimensions(sun_cascades).xy);
    let depth = ndc.z - 0.0004;
    var lit = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            lit += textureSampleCompareLevel(sun_cascades, shadow_samp, uv + vec2<f32>(f32(x), f32(y)) / size, i, depth);
        }
    }
    return vec2<f32>(lit / 9.0, edge);
}

// A sombra do sol pelas cascatas: a de perto onde ela chega, passando para
// a seguinte aos poucos perto da borda; na borda da última, some aos poucos.
fn sunlit_cascades(world: vec3<f32>, n: vec3<f32>) -> f32 {
    let count = i32(view.shadow.w + 0.5);
    for (var i = 0; i < count; i++) {
        let s = cascade_sample(i, world, n);
        if (s.y < 0.0) {
            continue;
        }
        var lit = s.x;
        if (i == count - 1) {
            lit = mix(lit, 1.0, clamp((s.y - 0.85) / 0.15, 0.0, 1.0));
        } else if (s.y > 0.9) {
            let t = cascade_sample(i + 1, world, n);
            if (t.y >= 0.0) {
                lit = mix(lit, t.x, (s.y - 0.9) / 0.1);
            }
        }
        return mix(1.0, lit, view.shadow.z);
    }
    return 1.0;
}

// O tonemap (o "PBR Neutral" da Khronos): as cores até perto do branco
// quase não mudam; as acima são comprimidas, sem estourar em branco puro.
fn pbr_neutral(color: vec3<f32>) -> vec3<f32> {
    let start = 0.8 - 0.04;
    let desaturation = 0.15;
    let x = min(color.r, min(color.g, color.b));
    let offset = select(0.04, x - 6.25 * x * x, x < 0.08);
    var c = color - offset;
    let peak = max(c.r, max(c.g, c.b));
    if (peak < start) {
        return c;
    }
    let d = 1.0 - start;
    let new_peak = 1.0 - d * d / (peak + d - start);
    c = c * (new_peak / peak);
    let g = 1.0 - 1.0 / (desaturation * (peak - new_peak) + 1.0);
    return mix(c, vec3<f32>(new_peak), g);
}

// A cor que vai para a tela: com o tonemap, se ligado (sky_params.w).
fn finish(rgb: vec3<f32>) -> vec3<f32> {
    if (view.sky_params.w > 0.5) {
        return pbr_neutral(max(rgb, vec3<f32>(0.0)));
    }
    return rgb;
}

// As ondinhas da água: a normal da superfície mexendo com o tempo (somas de
// senos em direções diferentes).
fn wave(p: vec2<f32>, t: f32, dir: vec2<f32>, freq: f32, speed: f32, amp: f32) -> vec2<f32> {
    return dir * (cos(dot(dir, p) * freq + t * speed) * amp * freq);
}

fn water_normal(p: vec2<f32>, t: f32) -> vec3<f32> {
    let d = wave(p, t, vec2<f32>(0.8, 0.6), 1.3, 1.1, 0.09)
        + wave(p, t, vec2<f32>(-0.5, 0.9), 2.1, 1.5, 0.06)
        + wave(p, t, vec2<f32>(0.97, -0.24), 3.4, 2.0, 0.035)
        + wave(p, t, vec2<f32>(-0.3, -0.95), 5.2, 2.6, 0.02);
    return normalize(vec3<f32>(-d.x, 1.0, -d.y));
}

// A cor e, no segundo alvo, só o que brilha (para o halo do brilho).
struct FragOut {
    @location(0) color: vec4<f32>,
    @location(1) glow: vec4<f32>,
};

fn fog_amount(world: vec3<f32>) -> f32 {
    if (view.fog_range.y > view.fog_range.x) {
        let d = distance(world, view.camera.xyz);
        return clamp((d - view.fog_range.x) / (view.fog_range.y - view.fog_range.x), 0.0, 1.0);
    }
    return 0.0;
}

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> FragOut {
    // A amostra antes do `if`: o WGSL só deixa amostrar fora de desvios que
    // variam de pixel para pixel.
    var albedo = in.color * textureSample(tex, samp, in.uv);
    // Recorte (folhas, grades): abaixo do limite some; o resto fica opaco.
    if (in.material.z > 0.0) {
        if (albedo.a < in.material.z) {
            discard;
        }
        albedo.a = 1.0;
    }
    if (in.material.x > 0.5) {
        return water(in, albedo);
    }
    var n = normalize(in.normal);
    // Com as duas faces, a de trás é iluminada pelo lado dela.
    if (in.material.w > 0.5 && !front) {
        n = -n;
    }
    let f = fog_amount(in.world);
    var out: FragOut;
    if (f > 0.995) {
        // Toda na névoa: nem acende (o mar e o chão lá longe).
        out.color = vec4<f32>(finish(view.fog.rgb), albedo.a);
        out.glow = vec4<f32>(0.0, 0.0, 0.0, albedo.a);
        return out;
    }
    var sun = max(dot(n, -view.sun_dir.xyz), 0.0);
    if (sun > 0.0) {
        sun = sun * sunlit(in.world, n);
    }
    let up = n.y * 0.5 + 0.5;
    let ambient = mix(view.ground.rgb, view.sky.rgb, up);
    var rgb = albedo.rgb * (ambient + view.sun_color.rgb * sun + lamp_light_shadowed(in.world, n));
    // O brilho próprio: aparece mesmo no escuro.
    let own = albedo.rgb * in.glow.a + in.glow.rgb;
    rgb = rgb + own;
    rgb = mix(rgb, view.fog.rgb, f);
    out.color = vec4<f32>(finish(rgb), albedo.a);
    out.glow = vec4<f32>(min(own, vec3<f32>(1.0)) * (1.0 - f), albedo.a);
    return out;
}

// A água: a cor dela com a luz, o céu refletido (mais forte olhando de
// lado, pela direção da onda) e o brilho do sol ou da lua.
fn water(in: VertexOut, albedo: vec4<f32>) -> FragOut {
    let f = fog_amount(in.world);
    var out: FragOut;
    if (f > 0.995) {
        out.color = vec4<f32>(finish(view.fog.rgb), 1.0);
        out.glow = vec4<f32>(0.0, 0.0, 0.0, 1.0);
        return out;
    }
    let n = water_normal(in.world.xz, view.time_wind.x);
    let v = normalize(view.camera.xyz - in.world);
    // A sombra na água: uma conta só (serve para a luz e para o brilho).
    let lit = sunlit(in.world, vec3<f32>(0.0, 1.0, 0.0));
    let sun = max(dot(n, -view.sun_dir.xyz), 0.0) * lit;
    let ambient = mix(view.ground.rgb, view.sky.rgb, 0.85);
    var rgb = albedo.rgb * (ambient + view.sun_color.rgb * sun * 0.8 + lamp_light_shadowed(in.world, n));
    let r = reflect(-v, n);
    let fresnel = 0.1 + 0.55 * pow(1.0 - max(dot(n, v), 0.0), 3.0);
    // O céu refletido puxa um pouco para a cor de cima (a água não fica
    // branca de longe).
    var refl = mix(sky_color(vec3<f32>(r.x, abs(r.y), r.z)), view.sky_top.rgb, 0.35);
    if (view.sky_params.y < 0.5) {
        refl = view.sky.rgb;
    }
    rgb = mix(rgb, refl, fresnel);
    // O brilho do sol (ou da lua) nas ondas.
    let glint = pow(max(dot(r, -view.sun_dir.xyz), 0.0), 180.0) * 2.5;
    let sparkle = view.sun_color.rgb * glint * lit;
    let own = albedo.rgb * in.glow.a + in.glow.rgb;
    rgb = rgb + sparkle + own;
    var alpha = mix(albedo.a, 1.0, fresnel * 0.6);
    rgb = mix(rgb, view.fog.rgb, f);
    alpha = mix(alpha, 1.0, f);
    out.color = vec4<f32>(finish(rgb), alpha);
    // O brilho do sol nas ondas também ganha halo.
    out.glow = vec4<f32>(min(own + sparkle * 0.5, vec3<f32>(1.0)) * (1.0 - f), alpha);
    return out;
}

// --- Mapa de sombras: só a profundidade, vista do sol ---------------------

@vertex
fn vs_shadow(v: VertexIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return view.light_view_proj * sway(model * vec4<f32>(v.position, 1.0), v.m3.xyz, v.material.y);
}

// Sombra com recorte ou com as duas faces: a transparência da imagem decide
// o que faz sombra (as folhas, com os buracos).
struct ShadowOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // x = a transparência da cor, y = o recorte (0 = sem).
    @location(1) cut: vec2<f32>,
};

@vertex
fn vs_shadow_cut(v: VertexIn) -> ShadowOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    var out: ShadowOut;
    out.clip = view.light_view_proj * sway(model * vec4<f32>(v.position, 1.0), v.m3.xyz, v.material.y);
    out.uv = v.uv;
    out.cut = vec2<f32>(v.color.a * v.tint.a, v.material.z);
    return out;
}

@fragment
fn fs_shadow_cut(in: ShadowOut) {
    let a = textureSample(tex, samp, in.uv).a * in.cut.x;
    if (in.cut.y > 0.0 && a < in.cut.y) {
        discard;
    }
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
    let world = sway(model * skin * vec4<f32>(v.position, 1.0), v.m3.xyz, v.material.y);
    var out: VertexOut;
    out.clip = view.view_proj * world;
    out.normal = normalize(normal_m * (skin_n * v.normal));
    out.color = v.color * v.tint;
    out.world = world.xyz;
    out.uv = v.uv;
    out.glow = v.glow;
    out.material = v.material;
    return out;
}

@vertex
fn vs_shadow_skinned(v: VertexIn, s: SkinIn) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return view.light_view_proj * sway(model * skin_matrix(s) * vec4<f32>(v.position, 1.0), v.m3.xyz, v.material.y);
}

@vertex
fn vs_shadow_cut_skinned(v: VertexIn, s: SkinIn) -> ShadowOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    var out: ShadowOut;
    out.clip = view.light_view_proj * sway(model * skin_matrix(s) * vec4<f32>(v.position, 1.0), v.m3.xyz, v.material.y);
    out.uv = v.uv;
    out.cut = vec2<f32>(v.color.a * v.tint.a, v.material.z);
    return out;
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

// O degradê do céu numa direção (sem o sol e as estrelas).
fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    let h = dir.y;
    if (h >= 0.0) {
        return mix(view.sky_horizon.rgb, view.sky_top.rgb, pow(h, 0.6));
    }
    return mix(view.sky_horizon.rgb, view.sky_bottom.rgb, pow(min(-h * 4.0, 1.0), 0.7));
}

fn star_hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

@fragment
fn fs_sky(in: SkyOut) -> FragOut {
    let far = view.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let near = view.inv_view_proj * vec4<f32>(in.ndc, 0.0, 1.0);
    let dir = normalize(far.xyz / far.w - near.xyz / near.w);
    let h = dir.y;
    var c = sky_color(dir);
    // As estrelas: pontinhos sorteados pela direção, só acima do horizonte.
    let stars = view.sky_params.z;
    if (stars > 0.0 && h > 0.0) {
        let g = dir * 160.0;
        let cell = floor(g);
        let r = star_hash(cell);
        if (r > 0.993) {
            let f = fract(g) - vec3<f32>(0.5);
            let k = (1.0 - smoothstep(0.0, 0.45, length(f))) * (0.55 + 0.45 * star_hash(cell + vec3<f32>(1.7, 3.1, 5.3)));
            c = c + vec3<f32>(0.9, 0.95, 1.0) * (k * stars * smoothstep(0.0, 0.12, h));
        }
    }
    let s = dot(dir, -view.sun_dir.xyz);
    let cos_r = view.sky_params.x;
    var halo = vec3<f32>(0.0);
    if (cos_r < 1.0) {
        // Brilho em volta do sol e o disco.
        let glow = pow(max(s, 0.0), 48.0) * 0.35;
        c = c + view.sun_color.rgb * glow;
        let w = (1.0 - cos_r) * 0.25;
        let disk = smoothstep(cos_r - w, cos_r + w, s);
        let sol = min(view.sun_color.rgb * 1.1 + vec3<f32>(0.25), vec3<f32>(1.0));
        c = mix(c, sol, disk);
        // O disco (o sol ou a lua) ganha halo.
        halo = sol * disk * 0.8 * (1.0 - fog_amount(view.camera.xyz + dir * 1000.0));
    }
    var out: FragOut;
    out.color = vec4<f32>(finish(c), 1.0);
    out.glow = vec4<f32>(halo, 1.0);
    return out;
}

// --- Partículas: quadrados virados para a câmera ----------------------------

struct ParticleIn {
    @location(0) center_size: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) region: vec4<f32>,  // x, y, largura, altura (uv)
    @location(3) rot: vec4<f32>,     // x = giro em radianos, y = largura, zw = deslocamento
    @location(4) right: vec4<f32>,   // eixos do retângulo (w = 1); w = 0 usa os da câmera
    @location(5) up: vec4<f32>,
};

struct ParticleOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world: vec3<f32>,
    // 1 = recebe a luz do mundo (partículas; os textos, não).
    @location(3) lit: f32,
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
    let q = vec2<f32>(c.x * p.rot.y, c.y * p.center_size.w);
    let r = vec2<f32>(q.x * k - q.y * s, q.x * s + q.y * k) + p.rot.zw;
    var right = view.cam_right.xyz;
    var up = view.cam_up.xyz;
    if (p.right.w > 0.5) {
        right = p.right.xyz;
        up = p.up.xyz;
    }
    let world = p.center_size.xyz + right * r.x + up * r.y;
    var out: ParticleOut;
    out.clip = view.view_proj * vec4<f32>(world, 1.0);
    out.color = p.color;
    out.uv = p.region.xy + vec2<f32>(c.x + 0.5, 0.5 - c.y) * p.region.zw;
    out.world = world;
    out.lit = select(0.0, 1.0, p.up.w > 1.5);
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
fn fs_particle(in: ParticleOut) -> FragOut {
    let c = in.color * textureSample(tex, samp, in.uv);
    // A luz do mundo (o céu e o sol, mais as lâmpadas), no máximo 1: de dia,
    // como antes; de noite, a fumaça fica escura (ou da cor da tocha perto).
    let luz = view.sky.rgb + view.sun_color.rgb + lamp_light(in.world, vec3<f32>(0.0, 1.0, 0.0));
    let k = mix(vec3<f32>(1.0), clamp(luz, vec3<f32>(0.18), vec3<f32>(1.0)), in.lit);
    var out: FragOut;
    out.color = vec4<f32>(finish(mix(c.rgb * k, view.fog.rgb, particle_fog(in.world))), c.a);
    out.glow = vec4<f32>(0.0, 0.0, 0.0, c.a);
    return out;
}

@fragment
fn fs_particle_add(in: ParticleOut) -> FragOut {
    let c = in.color * textureSample(tex, samp, in.uv);
    var out: FragOut;
    out.color = vec4<f32>(c.rgb * (1.0 - particle_fog(in.world)), c.a);
    // O fogo e as faíscas (que somam luz) ganham halo.
    out.glow = vec4<f32>(c.rgb * 0.45 * (1.0 - particle_fog(in.world)), c.a);
    return out;
}
