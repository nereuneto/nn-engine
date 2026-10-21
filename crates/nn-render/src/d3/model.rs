//! Modelos 3D de arquivos glTF (`.glb` ou `.gltf`), como os exportados pelo
//! Blender.
//!
//! O modelo é lido para a memória ([`ModelData`]) e depois enviado à GPU com
//! [`Renderer::create_model`](crate::Renderer::create_model). As
//! transformações dos nós do arquivo já vêm aplicadas: o modelo é desenhado
//! inteiro de uma vez, como uma malha só. Modelos com ossos (*skins*) ou
//! animações também trazem o esqueleto ([`Skeleton`]).

use std::sync::Arc;

use glam::{Mat3, Mat4, Quat, UVec2, Vec2, Vec3, Vec4};
use nn_core::Color;

use super::skin::{Animation, Channel, MAX_JOINTS, Property, SkelNode, Skeleton, SkinVertex, Trs};
use super::{Mesh, MeshId, Vertex3};
use crate::TextureId;

/// Erro ao ler um modelo.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("o arquivo não é um modelo glTF válido: {0}")]
    Invalid(String),
    #[error("o modelo usa o arquivo \"{0}\", que não foi encontrado")]
    MissingFile(String),
    #[error("imagem do modelo inválida: {0}")]
    Image(#[from] image::ImageError),
    #[error("o modelo não tem nenhuma malha")]
    Empty,
}

/// Uma parte do modelo: os triângulos que usam a mesma imagem.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelPart {
    pub mesh: Mesh,
    /// Índice em [`ModelData::images`], se a parte tiver textura.
    pub image: Option<usize>,
    /// Os ossos de cada vértice (vazio num modelo sem esqueleto).
    pub skin: Vec<SkinVertex>,
    /// A cor que a parte emite (brilha mesmo no escuro); preto = nada. Vem
    /// do `emissiveFactor` do material.
    pub emissive: Color,
}

/// Imagem de um modelo, já decodificada (RGBA 8 bits).
#[derive(Debug, Clone, PartialEq)]
pub struct ModelImage {
    pub size: UVec2,
    pub rgba: Vec<u8>,
    /// Pixel art (sem suavizar ao ampliar).
    pub nearest: bool,
}

/// Um modelo lido do arquivo, ainda na memória.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ModelData {
    pub parts: Vec<ModelPart>,
    pub images: Vec<ModelImage>,
    /// Esqueleto e animações (modelos com ossos ou animações).
    pub skeleton: Option<Skeleton>,
}

impl ModelData {
    pub fn triangle_count(&self) -> usize {
        self.parts.iter().map(|p| p.mesh.triangle_count()).sum()
    }

    /// Caixa que envolve o modelo (mínimo, máximo).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut out: Option<(Vec3, Vec3)> = None;
        for p in &self.parts {
            if p.mesh.vertices.is_empty() {
                continue;
            }
            let (a, b) = p.mesh.bounds();
            out = Some(match out {
                Some((min, max)) => (min.min(a), max.max(b)),
                None => (a, b),
            });
        }
        out.unwrap_or((Vec3::ZERO, Vec3::ZERO))
    }

    /// Separa todos os triângulos (cada um com os seus vértices e a normal da
    /// face): o visual low-poly, mesmo se o arquivo tiver sombreado suave.
    pub fn flatten(&mut self) {
        for part in &mut self.parts {
            let old = std::mem::take(&mut part.mesh);
            let old_skin = std::mem::take(&mut part.skin);
            for tri in old.indices.as_chunks::<3>().0 {
                let v = tri.map(|i| old.vertices[i as usize]);
                let p = v.map(|v| Vec3::from_array(v.position));
                let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                let base = part.mesh.vertices.len() as u32;
                for mut vert in v {
                    vert.normal = n.to_array();
                    part.mesh.vertices.push(vert);
                }
                if !old_skin.is_empty() {
                    part.skin
                        .extend(tri.map(|i| old_skin.get(i as usize).copied().unwrap_or_default()));
                }
                part.mesh.indices.extend([base, base + 1, base + 2]);
            }
        }
    }

    /// Lê um `.glb` ou `.gltf`. `load` busca os arquivos que o modelo cita
    /// (o `.bin` e as imagens de um `.gltf`), pelo caminho escrito no arquivo.
    pub fn from_gltf(
        bytes: &[u8],
        load: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, ModelError> {
        let gltf = gltf::Gltf::from_slice(bytes).map_err(|e| ModelError::Invalid(e.to_string()))?;
        let doc = &gltf.document;

        // Os dados binários.
        let mut buffers = Vec::new();
        for buffer in doc.buffers() {
            let data = match buffer.source() {
                gltf::buffer::Source::Bin => gltf
                    .blob
                    .clone()
                    .ok_or_else(|| ModelError::Invalid("faltou o bloco binário do .glb".into()))?,
                gltf::buffer::Source::Uri(uri) => read_uri(uri, load)?,
            };
            if data.len() < buffer.length() {
                return Err(ModelError::Invalid(
                    "dados binários menores que o esperado".into(),
                ));
            }
            buffers.push(data);
        }

        // As imagens (só as usadas como cor base).
        let mut image_of_texture = std::collections::HashMap::new();
        let mut images = Vec::new();
        for material in doc.materials() {
            let Some(info) = material.pbr_metallic_roughness().base_color_texture() else {
                continue;
            };
            let texture = info.texture();
            if image_of_texture.contains_key(&texture.index()) {
                continue;
            }
            let encoded = match texture.source().source() {
                gltf::image::Source::View { view, .. } => {
                    let b = &buffers[view.buffer().index()];
                    b.get(view.offset()..view.offset() + view.length())
                        .ok_or_else(|| ModelError::Invalid("imagem fora dos dados".into()))?
                        .to_vec()
                }
                gltf::image::Source::Uri { uri, .. } => read_uri(uri, load)?,
            };
            let img = image::load_from_memory(&encoded)?.into_rgba8();
            let nearest = matches!(
                texture.sampler().mag_filter(),
                Some(gltf::texture::MagFilter::Nearest)
            );
            image_of_texture.insert(texture.index(), images.len());
            images.push(ModelImage {
                size: UVec2::new(img.width(), img.height()),
                rgba: img.into_raw(),
                nearest,
            });
        }

        // Os nós da cena, cada pai antes dos filhos.
        let scene = doc.default_scene().or_else(|| doc.scenes().next());
        let roots: Vec<gltf::Node> = match scene {
            Some(s) => s.nodes().collect(),
            None => doc.nodes().collect(),
        };
        let mut order: Vec<(gltf::Node, Option<usize>)> = Vec::new();
        let mut stack: Vec<(gltf::Node, Option<usize>)> =
            roots.into_iter().rev().map(|n| (n, None)).collect();
        while let Some((node, parent)) = stack.pop() {
            let me = order.len();
            for child in node.children().collect::<Vec<_>>().into_iter().rev() {
                stack.push((child, Some(me)));
            }
            order.push((node, parent));
        }
        let mut index_of = std::collections::HashMap::new();
        for (i, (node, _)) in order.iter().enumerate() {
            index_of.insert(node.index(), i);
        }

        // Esqueleto: só em modelos com ossos ou animações.
        let animated = doc.skins().next().is_some() || doc.animations().next().is_some();
        let mut skeleton = animated.then(|| Skeleton {
            nodes: order
                .iter()
                .map(|(node, parent)| {
                    let (t, r, s) = node.transform().decomposed();
                    SkelNode {
                        name: node.name().unwrap_or("").to_string(),
                        parent: *parent,
                        rest: Trs {
                            translation: Vec3::from_array(t),
                            rotation: Quat::from_array(r).normalize(),
                            scale: Vec3::from_array(s),
                        },
                    }
                })
                .collect(),
            ..Default::default()
        });
        let rest_globals = skeleton
            .as_ref()
            .map(|s| s.globals(&s.rest_pose()))
            .unwrap_or_default();
        // Ossos das skins: os índices de cada skin viram índices da lista
        // geral de ossos.
        let mut skin_joints: Vec<Vec<u32>> = Vec::new();
        if let Some(sk) = skeleton.as_mut() {
            for skin in doc.skins() {
                if let Some(a) = skin.inverse_bind_matrices()
                    && !(a.dimensions() == gltf::accessor::Dimensions::Mat4
                        && a.data_type() == gltf::accessor::DataType::F32)
                {
                    return Err(ModelError::Invalid(
                        "tipo de dado inválido nas matrizes dos ossos".into(),
                    ));
                }
                let reader = skin.reader(|b| buffers.get(b.index()).map(|d| d.as_slice()));
                let ibms: Vec<Mat4> = reader
                    .read_inverse_bind_matrices()
                    .map(|m| m.map(|c| Mat4::from_cols_array_2d(&c)).collect())
                    .unwrap_or_default();
                let mut map = Vec::new();
                for (k, joint) in skin.joints().enumerate() {
                    let node = *index_of
                        .get(&joint.index())
                        .ok_or_else(|| ModelError::Invalid("osso fora da cena do modelo".into()))?;
                    map.push(sk.joints.len() as u32);
                    sk.joints
                        .push((node, ibms.get(k).copied().unwrap_or(Mat4::IDENTITY)));
                }
                skin_joints.push(map);
            }
        }

        // As malhas, com as transformações dos nós aplicadas.
        let mut parts: Vec<ModelPart> = Vec::new();
        let mut world_of: Vec<Mat4> = Vec::with_capacity(order.len());
        for (i, (node, parent)) in order.iter().enumerate() {
            let local = Mat4::from_cols_array_2d(&node.transform().matrix());
            let world = match parent {
                Some(p) => world_of[*p] * local,
                None => local,
            };
            world_of.push(world);
            let Some(mesh) = node.mesh() else {
                continue;
            };
            // Num modelo animado, a parte segue os ossos da skin ou o nó dela.
            let skin = match (&mut skeleton, node.skin()) {
                (Some(_), Some(s)) => SkinSource::Skinned(&skin_joints[s.index()]),
                (Some(sk), None) => {
                    let inverse = rest_globals[i].inverse();
                    let joint = match sk.joints.iter().position(|(n, _)| *n == i) {
                        Some(j) if sk.joints[j].1.abs_diff_eq(inverse, 1e-5) => j,
                        _ => {
                            sk.joints.push((i, inverse));
                            sk.joints.len() - 1
                        }
                    };
                    SkinSource::Rigid(joint as u32)
                }
                _ => SkinSource::None,
            };
            for prim in mesh.primitives() {
                if prim.mode() != gltf::mesh::Mode::Triangles {
                    log::warn!("modelo: parte que não é de triângulos ignorada");
                    continue;
                }
                // Malha com skin: os vértices já estão na pose dos ossos (o
                // nó dela não conta).
                let w = if matches!(skin, SkinSource::Skinned(_)) {
                    Mat4::IDENTITY
                } else {
                    world
                };
                add_primitive(&prim, &buffers, w, &image_of_texture, skin, &mut parts)?;
            }
        }
        parts.retain(|p| !p.mesh.indices.is_empty());
        if parts.is_empty() {
            return Err(ModelError::Empty);
        }

        if let Some(sk) = skeleton.as_mut() {
            sk.animations = read_animations(doc, &buffers, &index_of)?;
        }
        if skeleton
            .as_ref()
            .is_some_and(|s| s.joints.len() > MAX_JOINTS)
        {
            log::warn!("modelo com mais de {MAX_JOINTS} ossos: desenhado parado, sem animação");
            skeleton = None;
            for p in &mut parts {
                p.skin.clear();
            }
        }
        Ok(Self {
            parts,
            images,
            skeleton,
        })
    }
}

/// De onde vêm os ossos dos vértices de uma parte.
#[derive(Clone, Copy)]
enum SkinSource<'a> {
    /// Modelo sem esqueleto.
    None,
    /// Preso inteiro a um osso (o nó da parte).
    Rigid(u32),
    /// Os ossos do arquivo (JOINTS_0 e WEIGHTS_0), com os índices da skin
    /// levados para a lista geral.
    Skinned(&'a [u32]),
}

fn read_animations(
    doc: &gltf::Document,
    buffers: &[Vec<u8>],
    index_of: &std::collections::HashMap<usize, usize>,
) -> Result<Vec<Animation>, ModelError> {
    use gltf::accessor::{DataType as T, Dimensions as D};
    use gltf::animation::util::ReadOutputs;
    let mut out = Vec::new();
    for (n, anim) in doc.animations().enumerate() {
        let mut channels = Vec::new();
        let mut duration = 0.0f32;
        for ch in anim.channels() {
            let target = ch.target();
            let Some(&node) = index_of.get(&target.node().index()) else {
                continue;
            };
            let property = match target.property() {
                gltf::animation::Property::Translation => Property::Translation,
                gltf::animation::Property::Rotation => Property::Rotation,
                gltf::animation::Property::Scale => Property::Scale,
                gltf::animation::Property::MorphTargetWeights => continue,
            };
            let sampler = ch.sampler();
            let (input, output) = (sampler.input(), sampler.output());
            let ok_out = match property {
                Property::Rotation => {
                    output.dimensions() == D::Vec4
                        && matches!(output.data_type(), T::F32 | T::I8 | T::U8 | T::I16 | T::U16)
                }
                _ => output.dimensions() == D::Vec3 && output.data_type() == T::F32,
            };
            if input.dimensions() != D::Scalar
                || input.data_type() != T::F32
                || !ok_out
                || input.sparse().is_some()
                || output.sparse().is_some()
            {
                return Err(ModelError::Invalid(
                    "tipo de dado inválido numa animação".into(),
                ));
            }
            let reader = ch.reader(|b| buffers.get(b.index()).map(|d| d.as_slice()));
            let times: Vec<f32> = match reader.read_inputs() {
                Some(t) => t.collect(),
                None => continue,
            };
            let mut values: Vec<Vec4> = match reader.read_outputs() {
                Some(ReadOutputs::Translations(v)) | Some(ReadOutputs::Scales(v)) => {
                    v.map(|v| Vec3::from_array(v).extend(0.0)).collect()
                }
                Some(ReadOutputs::Rotations(r)) => r.into_f32().map(Vec4::from_array).collect(),
                _ => continue,
            };
            let interpolation = sampler.interpolation();
            if interpolation == gltf::animation::Interpolation::CubicSpline {
                // Cada quadro tem (tangente de entrada, valor, de saída): fica
                // só o valor, com transição linear.
                values = values.chunks(3).filter_map(|c| c.get(1).copied()).collect();
            }
            if let Some(&last) = times.last() {
                duration = duration.max(last);
            }
            channels.push(Channel {
                node,
                property,
                step: interpolation == gltf::animation::Interpolation::Step,
                times,
                values,
            });
        }
        out.push(Animation {
            name: anim
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("animacao{}", n + 1)),
            duration,
            channels,
        });
    }
    Ok(out)
}

fn add_primitive(
    prim: &gltf::Primitive,
    buffers: &[Vec<u8>],
    world: Mat4,
    image_of_texture: &std::collections::HashMap<usize, usize>,
    skin: SkinSource,
    parts: &mut Vec<ModelPart>,
) -> Result<(), ModelError> {
    check_accessors(prim)?;
    let reader = prim.reader(|b| buffers.get(b.index()).map(|d| d.as_slice()));
    let Some(positions) = reader.read_positions() else {
        return Ok(());
    };
    let positions: Vec<Vec3> = positions.map(Vec3::from_array).collect();
    let count = positions.len();
    let normals: Option<Vec<Vec3>> = reader
        .read_normals()
        .map(|n| n.map(Vec3::from_array).collect());
    let colors: Option<Vec<[f32; 4]>> = reader.read_colors(0).map(|c| c.into_rgba_f32().collect());
    let uvs: Option<Vec<Vec2>> = reader
        .read_tex_coords(0)
        .map(|t| t.into_f32().map(Vec2::from_array).collect());
    let indices: Vec<u32> = match reader.read_indices() {
        Some(i) => i.into_u32().collect(),
        None => (0..count as u32).collect(),
    };
    if indices.iter().any(|&i| i as usize >= count) {
        return Err(ModelError::Invalid(
            "índice de vértice fora da malha".into(),
        ));
    }
    // Os ossos de cada vértice.
    let skins: Option<Vec<SkinVertex>> = match skin {
        SkinSource::None => None,
        SkinSource::Rigid(j) => Some(vec![SkinVertex::rigid(j); count]),
        SkinSource::Skinned(map) => {
            let joints: Vec<[u16; 4]> = reader
                .read_joints(0)
                .map(|j| j.into_u16().collect())
                .unwrap_or_default();
            let weights: Vec<[f32; 4]> = reader
                .read_weights(0)
                .map(|w| w.into_f32().collect())
                .unwrap_or_default();
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let j = joints.get(i).copied().unwrap_or([0; 4]);
                let mut w = weights.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
                let sum: f32 = w.iter().sum();
                if sum > 1e-6 {
                    w = w.map(|x| x / sum);
                } else {
                    w = [1.0, 0.0, 0.0, 0.0];
                }
                let mut v = SkinVertex {
                    joints: [0; 4],
                    weights: w,
                };
                for k in 0..4 {
                    match map.get(j[k] as usize) {
                        Some(&g) => v.joints[k] = g,
                        None if w[k] > 0.0 => {
                            return Err(ModelError::Invalid(
                                "vértice preso a um osso que não existe".into(),
                            ));
                        }
                        None => {}
                    }
                }
                out.push(v);
            }
            Some(out)
        }
    };

    let material = prim.material();
    let pbr = material.pbr_metallic_roughness();
    let [r, g, b, a] = pbr.base_color_factor();
    // A cor do material vem em linear; as cores da engine são sRGB.
    let factor = Color::rgba(to_srgb(r), to_srgb(g), to_srgb(b), a);
    let image = pbr
        .base_color_texture()
        .and_then(|t| image_of_texture.get(&t.texture().index()).copied());
    // O brilho (em linear no arquivo), com a força da extensão
    // KHR_materials_emissive_strength quando houver.
    let [er, eg, eb] = material.emissive_factor();
    let strength = material.emissive_strength().unwrap_or(1.0).max(0.0);
    let emissive = Color::rgb(
        to_srgb(er * strength),
        to_srgb(eg * strength),
        to_srgb(eb * strength),
    );

    let normal_matrix = Mat3::from_mat4(world).inverse().transpose();
    // Espelhado (escala negativa): a ordem dos vértices inverte.
    let flip = world.determinant() < 0.0;

    let part = match parts
        .iter_mut()
        .find(|p| p.image == image && p.emissive == emissive)
    {
        Some(p) => p,
        None => {
            parts.push(ModelPart {
                mesh: Mesh::new(),
                image,
                skin: Vec::new(),
                emissive,
            });
            parts.last_mut().expect("parte criada")
        }
    };
    let base = part.mesh.vertices.len() as u32;
    let flat = normals.is_none();
    if flat {
        // Sem normais no arquivo: uma por face.
        for tri in indices.as_chunks::<3>().0 {
            let mut t = *tri;
            if flip {
                t.swap(1, 2);
            }
            let p = t.map(|i| world.transform_point3(positions[i as usize]));
            let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            let start = part.mesh.vertices.len() as u32;
            for (k, &i) in t.iter().enumerate() {
                part.mesh
                    .vertices
                    .push(vertex(p[k], n, i as usize, factor, &colors, &uvs));
                if let Some(s) = &skins {
                    part.skin.push(s[i as usize]);
                }
            }
            part.mesh.indices.extend([start, start + 1, start + 2]);
        }
        return Ok(());
    }
    let normals = normals.expect("normais");
    for (i, &pos) in positions.iter().enumerate() {
        let p = world.transform_point3(pos);
        let n = (normal_matrix * normals.get(i).copied().unwrap_or(Vec3::Y)).normalize_or_zero();
        part.mesh
            .vertices
            .push(vertex(p, n, i, factor, &colors, &uvs));
        if let Some(s) = &skins {
            part.skin.push(s[i]);
        }
    }
    for tri in indices.as_chunks::<3>().0 {
        if flip {
            part.mesh
                .indices
                .extend([base + tri[0], base + tri[2], base + tri[1]]);
        } else {
            part.mesh
                .indices
                .extend([base + tri[0], base + tri[1], base + tri[2]]);
        }
    }
    Ok(())
}

/// Confere os tipos dos dados antes de ler (o leitor do `gltf` para o
/// programa com um tipo inesperado, em vez de dar erro).
fn check_accessors(prim: &gltf::Primitive) -> Result<(), ModelError> {
    use gltf::Semantic;
    use gltf::accessor::{DataType as T, Dimensions as D};
    let bad = |what: &str| {
        Err(ModelError::Invalid(format!(
            "tipo de dado inválido em {what}"
        )))
    };
    if let Some(a) = prim.indices()
        && !(a.dimensions() == D::Scalar && matches!(a.data_type(), T::U8 | T::U16 | T::U32))
    {
        return bad("índices");
    }
    for (semantic, a) in prim.attributes() {
        let ok = match semantic {
            Semantic::Positions | Semantic::Normals => {
                a.dimensions() == D::Vec3 && a.data_type() == T::F32
            }
            Semantic::Colors(_) => {
                matches!(a.dimensions(), D::Vec3 | D::Vec4)
                    && matches!(a.data_type(), T::U8 | T::U16 | T::F32)
            }
            Semantic::TexCoords(_) => {
                a.dimensions() == D::Vec2 && matches!(a.data_type(), T::U8 | T::U16 | T::F32)
            }
            Semantic::Joints(_) => {
                a.dimensions() == D::Vec4 && matches!(a.data_type(), T::U8 | T::U16)
            }
            Semantic::Weights(_) => {
                a.dimensions() == D::Vec4 && matches!(a.data_type(), T::U8 | T::U16 | T::F32)
            }
            _ => true,
        };
        if !ok {
            return bad(&format!("{semantic:?}"));
        }
        if a.sparse().is_some() {
            return Err(ModelError::Invalid("dados esparsos não são aceitos".into()));
        }
    }
    Ok(())
}

fn vertex(
    p: Vec3,
    n: Vec3,
    i: usize,
    factor: Color,
    colors: &Option<Vec<[f32; 4]>>,
    uvs: &Option<Vec<Vec2>>,
) -> Vertex3 {
    let c = match colors.as_ref().and_then(|c| c.get(i)) {
        // Cores de vértice também vêm em linear.
        Some([r, g, b, a]) => Color::rgba(
            to_srgb(*r) * factor.r,
            to_srgb(*g) * factor.g,
            to_srgb(*b) * factor.b,
            a * factor.a,
        ),
        None => factor,
    };
    let uv = uvs
        .as_ref()
        .and_then(|u| u.get(i).copied())
        .unwrap_or(Vec2::ZERO);
    Vertex3::new(p, n, c).with_uv(uv)
}

fn to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn read_uri(
    uri: &str,
    load: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Result<Vec<u8>, ModelError> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let (_, data) = rest
            .split_once(";base64,")
            .ok_or_else(|| ModelError::Invalid("dados embutidos sem base64".into()))?;
        return base64(data).ok_or_else(|| ModelError::Invalid("base64 inválido".into()));
    }
    let path = percent_decode(uri);
    load(&path).ok_or(ModelError::MissingFile(path))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn base64(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let clean: Vec<u8> = s
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut n = 0u32;
        for (k, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * k);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        match chunk.len() {
            4 => out.extend_from_slice(&bytes),
            3 => out.extend_from_slice(&bytes[..2]),
            2 => out.push(bytes[0]),
            _ => return None,
        }
    }
    Some(out)
}

/// Os triângulos de um modelo, na memória (para a física: colisão exata).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CollisionMesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

impl ModelData {
    /// Todos os triângulos do modelo juntos (posições só).
    pub fn collision_mesh(&self) -> CollisionMesh {
        let mut out = CollisionMesh::default();
        for p in &self.parts {
            let base = out.vertices.len() as u32;
            out.vertices
                .extend(p.mesh.vertices.iter().map(|v| Vec3::from_array(v.position)));
            out.triangles.extend(
                p.mesh
                    .indices
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|t| t.map(|i| base + i)),
            );
        }
        out
    }
}

/// Um modelo na GPU: uma malha por imagem usada.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// Malha, textura e a cor que ela emite.
    pub(crate) parts: Vec<(MeshId, Option<TextureId>, Color)>,
    /// Caixa que envolve o modelo (mínimo, máximo).
    pub bounds: (Vec3, Vec3),
    pub triangles: usize,
    /// Os triângulos, para a colisão exata da física.
    pub collision: Arc<CollisionMesh>,
    /// Esqueleto e animações (modelos animados).
    pub skeleton: Option<Arc<Skeleton>>,
    /// As matrizes dos ossos na pose de descanso (desenho sem animação).
    pub(crate) rest_joints: Arc<Vec<Mat4>>,
}

impl Model {
    /// Tamanho do modelo (largura, altura, profundidade).
    pub fn size(&self) -> Vec3 {
        self.bounds.1 - self.bounds.0
    }

    /// Os nomes das animações (vazio num modelo sem animação).
    pub fn animation_names(&self) -> Vec<String> {
        self.skeleton
            .as_ref()
            .map(|s| s.animations.iter().map(|a| a.name.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_e_percent() {
        assert_eq!(base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64("aGk").unwrap(), b"hi");
        assert_eq!(percent_decode("minha%20casa.bin"), "minha casa.bin");
    }

    #[test]
    fn srgb() {
        assert!((to_srgb(0.214_041) - 0.5).abs() < 1e-3);
        assert!((to_srgb(1.0) - 1.0).abs() < 1e-5);
    }
}
