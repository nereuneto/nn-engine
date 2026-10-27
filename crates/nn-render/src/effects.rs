//! Efeitos: shaders que mudam como uma imagem aparece (tons de cinza,
//! pixelar, CRT, onda, contorno, brilho) e os do jogo, escritos em WGSL.
//!
//! Um efeito é só a função `fn efeito(uv: vec2<f32>) -> vec4<f32>`, que diz a
//! cor do ponto `uv` (0 a 1, com 0, 0 no canto de cima à esquerda da imagem).
//! Ela pode ler a imagem com `amostra(uv)` (fora dela, transparente), o
//! tamanho dela em pixels com `tamanho()`, o tempo em segundos com `tempo()`
//! e os parâmetros em `params.nome`. Os parâmetros vêm de um comentário no
//! começo, com os valores padrão:
//!
//! ```wgsl
//! // parâmetros: forca = 1, velocidade = 2
//! fn efeito(uv: vec2<f32>) -> vec4<f32> {
//!     let c = amostra(uv);
//!     return vec4<f32>(c.rgb * params.forca, c.a);
//! }
//! ```
//!
//! O efeito vira o fragmento do shader dos sprites: vale para qualquer
//! sprite, canvas ou a tela inteira (que é desenhada num canvas antes).

/// Quantos parâmetros um efeito pode ter.
pub const MAX_PARAMS: usize = 16;

/// Os efeitos prontos: nome e o código (com os parâmetros).
pub const BUILTIN: &[(&str, &str)] = &[
    (
        "cinza",
        r"// parâmetros: forca = 1
fn efeito(uv: vec2<f32>) -> vec4<f32> {
    let c = amostra(uv);
    let g = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    return vec4<f32>(mix(c.rgb, vec3<f32>(g), clamp(params.forca, 0.0, 1.0)), c.a);
}
",
    ),
    (
        "pixelar",
        r"// parâmetros: tamanho = 4
fn efeito(uv: vec2<f32>) -> vec4<f32> {
    let bloco = max(params.tamanho, 1.0) / tamanho();
    return amostra((floor(uv / bloco) + 0.5) * bloco);
}
",
    ),
    (
        "crt",
        r"// parâmetros: curvatura = 0.15, linhas = 0.35, vinheta = 0.3
fn efeito(uv: vec2<f32>) -> vec4<f32> {
    var p = uv * 2.0 - 1.0;
    p = p * (1.0 + params.curvatura * dot(p.yx, p.yx) * 0.5);
    let q = p * 0.5 + 0.5;
    if (any(q < vec2<f32>(0.0)) || any(q > vec2<f32>(1.0))) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let c = amostra(q);
    // Uma linha mais escura a cada dois pixels.
    let linha = 0.5 + 0.5 * cos((q.y * tamanho().y - 0.5) * 3.14159265);
    let v = clamp(1.0 - params.vinheta * dot(p, p) * 0.5, 0.0, 1.0);
    return vec4<f32>(c.rgb * (1.0 - params.linhas * linha) * v, c.a);
}
",
    ),
    (
        "onda",
        r"// parâmetros: amplitude = 4, frequencia = 3, velocidade = 2
fn efeito(uv: vec2<f32>) -> vec4<f32> {
    let onda = sin(uv.y * params.frequencia * 6.2831853 + tempo() * params.velocidade);
    return amostra(vec2<f32>(uv.x + onda * params.amplitude / tamanho().x, uv.y));
}
",
    ),
    (
        "contorno",
        r"// parâmetros: espessura = 1, cor_r = 1, cor_g = 1, cor_b = 1, cor_a = 1
fn efeito(uv: vec2<f32>) -> vec4<f32> {
    let c = amostra(uv);
    let d = max(params.espessura, 0.0) / tamanho();
    var a = 0.0;
    for (var i = 0; i < 8; i = i + 1) {
        let ang = f32(i) * 0.78539816;
        a = max(a, amostra(uv + vec2<f32>(cos(ang), sin(ang)) * d).a);
    }
    let borda = params.cor_a * a;
    // A imagem por cima da borda.
    let cor = mix(vec3<f32>(params.cor_r, params.cor_g, params.cor_b), c.rgb, c.a);
    return vec4<f32>(cor, c.a + borda * (1.0 - c.a));
}
",
    ),
    (
        "brilho",
        r"// parâmetros: forca = 0.8, raio = 4
fn efeito(uv: vec2<f32>) -> vec4<f32> {
    let c = amostra(uv);
    let d = max(params.raio, 0.0) / tamanho();
    // A luz em volta: um disco de amostras, as de perto pesando mais.
    var soma = vec3<f32>(0.0);
    var peso = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let ang = f32(i) * 0.39269908 + f32(i % 2) * 0.19634954;
        let dir = vec2<f32>(cos(ang), sin(ang));
        for (var k = 1; k <= 3; k = k + 1) {
            let r = f32(k) / 3.0;
            let w = 1.0 - r * 0.6;
            let a = amostra(uv + dir * d * r);
            soma = soma + a.rgb * a.a * w;
            peso = peso + w;
        }
    }
    let brilho = soma / peso * params.forca;
    // Por cima da imagem (com o alpha multiplicado), e de volta ao normal.
    let luz = clamp(max(brilho.r, max(brilho.g, brilho.b)), 0.0, 1.0);
    let a = c.a + luz * (1.0 - c.a);
    let rgb = c.rgb * c.a + brilho;
    return vec4<f32>(rgb / max(a, 0.0039), a);
}
",
    ),
];

/// Um efeito que não muda nada: o que fica no lugar de um efeito com erro
/// (o jogo segue sem ele).
pub const PASSTHROUGH: &str =
    "fn efeito(uv: vec2<f32>) -> vec4<f32> {\n    return amostra(uv);\n}\n";

/// O código de um efeito pronto.
pub fn builtin_source(name: &str) -> Option<&'static str> {
    if name == "nenhum" {
        return Some(PASSTHROUGH);
    }
    BUILTIN.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// Um erro num efeito: a linha (no arquivo do efeito, a partir de 1) e o
/// que houve, em português.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{}", self.describe())]
pub struct EffectError {
    pub line: Option<usize>,
    pub message: String,
}

impl EffectError {
    fn describe(&self) -> String {
        match self.line {
            Some(l) => format!("linha {l}: {}", self.message),
            None => self.message.clone(),
        }
    }
}

/// Os parâmetros do comentário `// parâmetros: a = 1, b` (também
/// `parametros:` e `params:`), com o valor padrão (0 se faltar).
pub fn parse_params(source: &str) -> Result<Vec<(String, f32)>, EffectError> {
    let mut out: Vec<(String, f32)> = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("//") else {
            continue;
        };
        let rest = rest.trim();
        let lower = rest.to_lowercase();
        let Some(list) = ["parâmetros:", "parametros:", "params:"]
            .iter()
            .find_map(|p| lower.strip_prefix(p).map(|_| &rest[p.len()..]))
        else {
            continue;
        };
        for item in list.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (name, value) = match item.split_once('=') {
                Some((n, v)) => (n.trim(), Some(v.trim())),
                None => (item, None),
            };
            let valid = name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !valid {
                return Err(EffectError {
                    line: Some(i + 1),
                    message: format!(
                        "\"{name}\" não serve de nome de parâmetro (use letras sem acento, números e _)"
                    ),
                });
            }
            // Uma cor (#RRGGBB ou #RRGGBBAA) vira quatro: nome_r, nome_g,
            // nome_b e nome_a, em cor linear (como o shader trabalha).
            if let Some(hex) = value.and_then(|v| v.strip_prefix('#')) {
                let rgba = parse_hex(hex).ok_or_else(|| EffectError {
                    line: Some(i + 1),
                    message: format!(
                        "a cor de \"{name}\" precisa ser #RRGGBB ou #RRGGBBAA (veio \"#{hex}\")"
                    ),
                })?;
                for (k, suffix) in ["r", "g", "b", "a"].iter().enumerate() {
                    let full = format!("{name}_{suffix}");
                    if out.iter().any(|(n, _)| *n == full) {
                        return Err(EffectError {
                            line: Some(i + 1),
                            message: format!("o parâmetro \"{full}\" aparece duas vezes"),
                        });
                    }
                    let v = if k < 3 {
                        srgb_to_linear(rgba[k])
                    } else {
                        rgba[k]
                    };
                    out.push((full, v));
                }
                continue;
            }
            let value = match value {
                Some(v) => v.parse::<f32>().map_err(|_| EffectError {
                    line: Some(i + 1),
                    message: format!(
                        "o valor de \"{name}\" precisa ser um número ou uma cor #RRGGBB (veio \"{v}\")"
                    ),
                })?,
                None => 0.0,
            };
            if out.iter().any(|(n, _)| n == name) {
                return Err(EffectError {
                    line: Some(i + 1),
                    message: format!("o parâmetro \"{name}\" aparece duas vezes"),
                });
            }
            out.push((name.to_string(), value));
        }
    }
    if out.len() > MAX_PARAMS {
        return Err(EffectError {
            line: None,
            message: format!(
                "um efeito pode ter até {MAX_PARAMS} parâmetros (este tem {})",
                out.len()
            ),
        });
    }
    Ok(out)
}

/// "RRGGBB" ou "RRGGBBAA" em 0 a 1.
fn parse_hex(hex: &str) -> Option<[f32; 4]> {
    if !(hex.len() == 6 || hex.len() == 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16)
            .ok()
            .map(|b| f32::from(b) / 255.0)
    };
    Some([
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if hex.len() == 8 { byte(6)? } else { 1.0 },
    ])
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Um efeito que não muda nada, com os parâmetros de `source` (o que fica no
/// lugar de um efeito com erro: o jogo continua mudando os parâmetros dele).
pub fn passthrough_for(source: &str) -> String {
    match parse_params(source) {
        Ok(params) if !params.is_empty() => {
            let list: Vec<String> = params.iter().map(|(n, v)| format!("{n} = {v}")).collect();
            format!("// parâmetros: {}\n{PASSTHROUGH}", list.join(", "))
        }
        _ => PASSTHROUGH.to_string(),
    }
}

/// O shader inteiro: o dos sprites, o que o efeito usa e o efeito. Devolve
/// também quantas linhas vêm antes do efeito (para as linhas dos erros).
pub(crate) fn full_source(source: &str, params: &[(String, f32)]) -> (String, usize) {
    let mut fields = String::new();
    for (name, _) in params {
        fields.push_str(&format!("    {name}: f32,\n"));
    }
    for k in params.len()..MAX_PARAMS {
        fields.push_str(&format!("    nn_livre_{k}: f32,\n"));
    }
    let prelude = format!(
        "{sprite}
// ---- efeito ----
struct NnTempo {{
    segundos: f32,
    nn_a: f32,
    nn_b: f32,
    nn_c: f32,
}};
struct NnParametros {{
{fields}}};
@group(2) @binding(0) var<uniform> nn_tempo: NnTempo;
@group(2) @binding(1) var<uniform> params: NnParametros;
var<private> nn_regiao: vec4<f32>;
var<private> nn_canvas: bool;
fn tempo() -> f32 {{
    return nn_tempo.segundos;
}}
fn tamanho() -> vec2<f32> {{
    return max(vec2<f32>(textureDimensions(sprite_texture)) * nn_regiao.zw, vec2<f32>(1.0));
}}
fn amostra(uv: vec2<f32>) -> vec4<f32> {{
    let dentro = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    let c = textureSampleLevel(sprite_texture, sprite_sampler, nn_regiao.xy + clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)) * nn_regiao.zw, 0.0);
    // Um canvas guarda a cor com o alpha já multiplicado: volta ao normal.
    let normal = select(c, vec4<f32>(c.rgb / max(c.a, 0.0039), c.a), nn_canvas);
    return select(vec4<f32>(0.0), normal, dentro);
}}
",
        sprite = include_str!("sprite.wgsl"),
    );
    let lines = prelude.lines().count();
    let full = format!(
        "{prelude}{source}
@fragment
fn fs_efeito(in: VertexOut) -> @location(0) vec4<f32> {{
    nn_regiao = in.region;
    nn_canvas = in.flags > 0.5;
    return efeito(in.local) * in.color;
}}
"
    );
    (full, lines)
}

/// Algumas mensagens do naga, em português.
fn translate(message: &str, params: &[(String, f32)]) -> String {
    let m = message.trim();
    // "Function [3] 'efeito' is invalid: ..." conta só o que importa.
    let m = match m.split_once(" is invalid: ") {
        Some((head, rest)) if head.starts_with("Function ") => rest,
        _ => m,
    };
    let quoted = |s: &str| {
        s.split('`')
            .nth(1)
            .map(|x| x.to_string())
            .unwrap_or_default()
    };
    if m.starts_with("invalid field accessor") {
        let field = quoted(m);
        let names: Vec<&str> = params
            .iter()
            .map(|(n, _)| n.as_str())
            .filter(|n| !n.starts_with("nn_livre_"))
            .collect();
        return if names.is_empty() {
            format!("\"{field}\" não existe aqui (o efeito não tem parâmetros)")
        } else {
            format!(
                "\"{field}\" não existe aqui (os parâmetros são: {})",
                names.join(", ")
            )
        };
    }
    if m.contains("does not match the declared return type") {
        return "o return não devolve o tipo que a função promete (o efeito devolve vec4<f32>)"
            .into();
    }
    if m.starts_with("no definition in scope for identifier") {
        return format!("\"{}\" não existe aqui", quoted(m));
    }
    if let Some(rest) = m.strip_prefix("expected ") {
        if let Some((a, b)) = rest.split_once(", found ") {
            return format!("esperava {a}, mas veio {b}");
        }
        return format!("esperava {rest}");
    }
    if m.starts_with("redefinition of") {
        return format!("\"{}\" foi definido duas vezes", quoted(m));
    }
    if let Some(rest) = m.strip_prefix("automatic conversions cannot convert ")
        && let Some((a, b)) = rest.split_once(" to ")
    {
        return format!("não dá para usar {a} onde se espera {b}");
    }
    if m.contains("unknown type") {
        return format!("tipo desconhecido: \"{}\"", quoted(m));
    }
    format!("erro no shader: {m}")
}

/// Confere o efeito: o WGSL, a validação e a tradução para o GLSL do
/// navegador (WebGL2). Devolve o módulo pronto.
pub(crate) fn check(source: &str) -> Result<(String, Vec<(String, f32)>), EffectError> {
    let params = parse_params(source)?;
    if !source.contains("fn efeito") {
        return Err(EffectError {
            line: None,
            message: "falta a função do efeito: fn efeito(uv: vec2<f32>) -> vec4<f32> { ... }"
                .into(),
        });
    }
    let (full, before) = full_source(source, &params);
    let line_of = |loc: Option<naga::SourceLocation>| {
        loc.map(|l| l.line_number as usize)
            .filter(|l| *l > before)
            .map(|l| l - before)
    };
    let module = naga::front::wgsl::parse_str(&full).map_err(|e| {
        // A primeira marca que cai no código do efeito.
        let line = e
            .labels()
            .filter(|(span, _)| span.is_defined() && span.to_range().is_some_and(|r| !r.is_empty()))
            .find_map(|(span, _)| line_of(Some(span.location(&full))))
            .or_else(|| line_of(e.location(&full)));
        EffectError {
            line,
            message: translate(e.message(), &params),
        }
    })?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| {
        let mut message = e.as_inner().to_string();
        let mut source_err = std::error::Error::source(e.as_inner());
        while let Some(s) = source_err {
            message = format!("{message}: {s}");
            source_err = s.source();
        }
        // A marca mais de dentro (a última) que cai no código do efeito.
        let line = e
            .spans()
            .filter_map(|(span, _)| line_of(Some(span.location(&full))))
            .last()
            .or_else(|| line_of(e.location(&full)));
        EffectError {
            line,
            message: translate(&message, &params),
        }
    })?;
    // O navegador (WebGL2) recebe o shader em GLSL ES 3.0.
    let mut glsl = String::new();
    let options = naga::back::glsl::Options {
        version: naga::back::glsl::Version::Embedded {
            version: 300,
            is_webgl: true,
        },
        ..Default::default()
    };
    let pipeline = naga::back::glsl::PipelineOptions {
        shader_stage: naga::ShaderStage::Fragment,
        entry_point: "fs_efeito".into(),
        multiview: None,
    };
    naga::back::glsl::Writer::new(
        &mut glsl,
        &module,
        &info,
        &options,
        &pipeline,
        naga::proc::BoundsCheckPolicies::default(),
    )
    .and_then(|mut w| w.write())
    .map_err(|e| EffectError {
        line: None,
        message: format!("o efeito não roda no navegador (WebGL2): {e}"),
    })?;
    Ok((full, params))
}

// ----------------------------------------------------------------------
// Na placa de vídeo
// ----------------------------------------------------------------------

use std::collections::HashMap;

/// Um shader de efeito compilado (de [`crate::Renderer::compile_effect`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectShader(pub(crate) u32);

/// Um efeito em uso, com os valores dos parâmetros dele (de
/// [`crate::Renderer::create_effect`]). Dois sprites com o mesmo efeito usam
/// os mesmos valores; para valores diferentes, crie outro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(pub(crate) u32);

#[derive(Debug)]
pub(crate) struct ShaderData {
    pub alpha: wgpu::RenderPipeline,
    pub add: wgpu::RenderPipeline,
    pub canvas_alpha: wgpu::RenderPipeline,
    pub canvas_add: wgpu::RenderPipeline,
    pub params: Vec<(String, f32)>,
}

#[derive(Debug)]
pub(crate) struct Instance {
    pub shader: usize,
    buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    values: [f32; MAX_PARAMS],
    dirty: bool,
}

#[derive(Debug)]
pub(crate) struct EffectGpu {
    layout: wgpu::BindGroupLayout,
    time_buffer: wgpu::Buffer,
    pub shaders: Vec<ShaderData>,
    pub instances: Vec<Option<Instance>>,
    builtins: HashMap<String, EffectShader>,
    pub time: f32,
    time_written: Option<f32>,
}

impl EffectGpu {
    pub fn new(device: &wgpu::Device) -> Self {
        let entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn efeito"),
            entries: &[entry(0), entry(1)],
        });
        let time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn efeito tempo"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            layout,
            time_buffer,
            shaders: Vec::new(),
            instances: Vec::new(),
            builtins: HashMap::new(),
            time: 0.0,
            time_written: None,
        }
    }

    /// Compila o efeito (já conferido) nos dois jeitos de misturar.
    #[allow(clippy::too_many_arguments)]
    pub fn compile(
        &mut self,
        device: &wgpu::Device,
        source: &str,
        format: wgpu::TextureFormat,
        view_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        vertex: wgpu::VertexBufferLayout<'static>,
        blends: [wgpu::BlendState; 4],
    ) -> Result<EffectShader, EffectError> {
        let (full, params) = check(source)?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nn efeito"),
            source: wgpu::ShaderSource::Wgsl(full.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn efeito"),
            bind_group_layouts: &[Some(view_layout), Some(texture_layout), Some(&self.layout)],
            immediate_size: 0,
        });
        let make = |blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("nn efeito"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(vertex.clone())],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("fs_efeito"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        self.shaders.push(ShaderData {
            alpha: make(blends[0]),
            add: make(blends[1]),
            canvas_alpha: make(blends[2]),
            canvas_add: make(blends[3]),
            params,
        });
        Ok(EffectShader(self.shaders.len() as u32 - 1))
    }

    pub fn builtin(&self, name: &str) -> Option<EffectShader> {
        self.builtins.get(name).copied()
    }

    pub fn remember_builtin(&mut self, name: &str, shader: EffectShader) {
        self.builtins.insert(name.to_string(), shader);
    }

    pub fn create(&mut self, device: &wgpu::Device, shader: EffectShader) -> EffectId {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn efeito parâmetros"),
            size: (MAX_PARAMS * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn efeito"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.time_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        let mut values = [0.0; MAX_PARAMS];
        for (i, (_, v)) in self.shaders[shader.0 as usize].params.iter().enumerate() {
            values[i] = *v;
        }
        let instance = Instance {
            shader: shader.0 as usize,
            buffer,
            bind_group,
            values,
            dirty: true,
        };
        if let Some(i) = self.instances.iter().position(Option::is_none) {
            self.instances[i] = Some(instance);
            return EffectId(i as u32);
        }
        self.instances.push(Some(instance));
        EffectId(self.instances.len() as u32 - 1)
    }

    pub fn get(&self, id: EffectId) -> Option<&Instance> {
        self.instances.get(id.0 as usize).and_then(Option::as_ref)
    }

    pub fn params(&self, id: EffectId) -> &[(String, f32)] {
        match self.get(id) {
            Some(i) => &self.shaders[i.shader].params,
            None => &[],
        }
    }

    pub fn set(&mut self, id: EffectId, name: &str, value: f32) -> bool {
        let Some(inst) = self
            .instances
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
        else {
            return false;
        };
        let Some(i) = self.shaders[inst.shader]
            .params
            .iter()
            .position(|(n, _)| n == name)
        else {
            return false;
        };
        if inst.values[i] != value {
            inst.values[i] = value;
            inst.dirty = true;
        }
        true
    }

    pub fn value(&self, id: EffectId, name: &str) -> Option<f32> {
        let inst = self.get(id)?;
        let i = self.shaders[inst.shader]
            .params
            .iter()
            .position(|(n, _)| n == name)?;
        Some(inst.values[i])
    }

    /// Troca o shader (ao recarregar o arquivo), mantendo os valores dos
    /// parâmetros de mesmo nome.
    pub fn replace_shader(&mut self, id: EffectId, shader: EffectShader) {
        let new = &self.shaders[shader.0 as usize].params;
        let Some(inst) = self
            .instances
            .get_mut(id.0 as usize)
            .and_then(Option::as_mut)
        else {
            return;
        };
        let old = &self.shaders[inst.shader].params;
        let mut values = [0.0; MAX_PARAMS];
        for (i, (name, default)) in new.iter().enumerate() {
            values[i] = old
                .iter()
                .position(|(n, _)| n == name)
                .map_or(*default, |j| inst.values[j]);
        }
        inst.values = values;
        inst.shader = shader.0 as usize;
        inst.dirty = true;
    }

    pub fn free(&mut self, id: EffectId) {
        if let Some(slot) = self.instances.get_mut(id.0 as usize) {
            *slot = None;
        }
    }

    /// Manda para a placa o tempo e os parâmetros que mudaram.
    pub fn upload(&mut self, queue: &wgpu::Queue) {
        if self.time_written != Some(self.time) {
            queue.write_buffer(
                &self.time_buffer,
                0,
                bytemuck::cast_slice(&[self.time, 0.0, 0.0, 0.0]),
            );
            self.time_written = Some(self.time);
        }
        for inst in self.instances.iter_mut().flatten() {
            if inst.dirty {
                queue.write_buffer(&inst.buffer, 0, bytemuck::cast_slice(&inst.values));
                inst.dirty = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_prontos_compilam_e_rodam_no_navegador() {
        for (name, source) in BUILTIN {
            check(source).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    #[test]
    fn parametros_do_comentario() {
        let p = parse_params("// parâmetros: forca = 1.5, cor, x_2 = -3\nfn efeito() {}").unwrap();
        assert_eq!(
            p,
            [
                ("forca".to_string(), 1.5),
                ("cor".to_string(), 0.0),
                ("x_2".to_string(), -3.0)
            ]
        );
        assert!(parse_params("// params: força = 1").is_err());
        assert!(parse_params("// params: a = b").is_err());
        assert!(parse_params("// params: a, a").is_err());
    }

    #[test]
    fn cor_nos_parametros() {
        let p = parse_params("// parâmetros: fundo = #FF0000, forca = 2").unwrap();
        let names: Vec<&str> = p.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["fundo_r", "fundo_g", "fundo_b", "fundo_a", "forca"]);
        assert_eq!(p[0].1, 1.0);
        assert_eq!(p[1].1, 0.0);
        assert_eq!(p[3].1, 1.0);
        assert!(parse_params("// parâmetros: fundo = #FF00").is_err());
        // O que não muda nada, com os mesmos parâmetros.
        let fake = passthrough_for("// parâmetros: a = 1, cor = #FFFFFF\nnada");
        let (_, params) = check(&fake).unwrap();
        assert_eq!(params.len(), 5);
    }

    #[test]
    fn mensagens_dos_erros_de_tipo() {
        let e = check(
            "// parâmetros: forca = 1\nfn efeito(uv: vec2<f32>) -> vec4<f32> {\n    let c = amostra(uv);\n    return c * params.forsa;\n}\n",
        )
        .unwrap_err();
        assert!(
            e.message
                .contains("\"forsa\" não existe aqui (os parâmetros são: forca)"),
            "{e}"
        );
        assert_eq!(e.line, Some(4), "{e}");
        let e = check("fn efeito(uv: vec2<f32>) -> vec4<f32> {\n    let x = 1.0;\n    return vec3<f32>(x);\n}\n").unwrap_err();
        assert!(e.message.contains("o return não devolve o tipo"), "{e}");
        assert_eq!(e.line, Some(3), "{e}");
    }

    #[test]
    fn erro_com_a_linha_em_portugues() {
        let e = check(
            "// parâmetros: forca = 1\nfn efeito(uv: vec2<f32>) -> vec4<f32> {\n    let c = amostra(uv);\n    return c * forsa;\n}\n",
        )
        .unwrap_err();
        assert_eq!(e.line, Some(4));
        assert!(e.message.contains("\"forsa\" não existe"), "{}", e.message);
        let e = check("fn outra() {}").unwrap_err();
        assert!(e.message.contains("falta a função"), "{}", e.message);
        let e = check("fn efeito(uv: vec2<f32>) -> vec4<f32> {\n    return 1.0;\n}\n").unwrap_err();
        assert_eq!(e.line, Some(2), "{e}");
    }
}
