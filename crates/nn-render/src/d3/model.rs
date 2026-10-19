//! Modelos 3D de arquivos glTF (`.glb` ou `.gltf`), como os exportados pelo
//! Blender.
//!
//! O modelo é lido para a memória ([`ModelData`]) e depois enviado à GPU com
//! [`Renderer::create_model`](crate::Renderer::create_model). As
//! transformações dos nós do arquivo já vêm aplicadas: o modelo é desenhado
//! inteiro de uma vez, como uma malha só.

use glam::{Mat3, Mat4, UVec2, Vec2, Vec3};
use nn_core::Color;

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
            for tri in old.indices.chunks_exact(3) {
                let v = [tri[0], tri[1], tri[2]].map(|i| old.vertices[i as usize]);
                let p = v.map(|v| Vec3::from_array(v.position));
                let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
                let base = part.mesh.vertices.len() as u32;
                for mut vert in v {
                    vert.normal = n.to_array();
                    part.mesh.vertices.push(vert);
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
                return Err(ModelError::Invalid("dados binários menores que o esperado".into()));
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

        // As malhas, com as transformações dos nós aplicadas.
        let mut parts: Vec<ModelPart> = Vec::new();
        let scene = doc.default_scene().or_else(|| doc.scenes().next());
        let roots: Vec<gltf::Node> = match scene {
            Some(s) => s.nodes().collect(),
            None => doc.nodes().collect(),
        };
        let mut stack: Vec<(gltf::Node, Mat4)> =
            roots.into_iter().map(|n| (n, Mat4::IDENTITY)).collect();
        while let Some((node, parent)) = stack.pop() {
            let world = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
            if let Some(mesh) = node.mesh() {
                for prim in mesh.primitives() {
                    if prim.mode() != gltf::mesh::Mode::Triangles {
                        log::warn!("modelo: parte que não é de triângulos ignorada");
                        continue;
                    }
                    add_primitive(&prim, &buffers, world, &image_of_texture, &mut parts)?;
                }
            }
            for child in node.children() {
                stack.push((child, world));
            }
        }
        parts.retain(|p| !p.mesh.indices.is_empty());
        if parts.is_empty() {
            return Err(ModelError::Empty);
        }
        Ok(Self { parts, images })
    }
}

fn add_primitive(
    prim: &gltf::Primitive,
    buffers: &[Vec<u8>],
    world: Mat4,
    image_of_texture: &std::collections::HashMap<usize, usize>,
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
        return Err(ModelError::Invalid("índice de vértice fora da malha".into()));
    }

    let material = prim.material();
    let pbr = material.pbr_metallic_roughness();
    let [r, g, b, a] = pbr.base_color_factor();
    // A cor do material vem em linear; as cores da engine são sRGB.
    let factor = Color::rgba(to_srgb(r), to_srgb(g), to_srgb(b), a);
    let image = pbr
        .base_color_texture()
        .and_then(|t| image_of_texture.get(&t.texture().index()).copied());

    let normal_matrix = Mat3::from_mat4(world).inverse().transpose();
    // Espelhado (escala negativa): a ordem dos vértices inverte.
    let flip = world.determinant() < 0.0;

    let part = match parts.iter_mut().find(|p| p.image == image) {
        Some(p) => p,
        None => {
            parts.push(ModelPart {
                mesh: Mesh::new(),
                image,
            });
            parts.last_mut().expect("parte criada")
        }
    };
    let base = part.mesh.vertices.len() as u32;
    let flat = normals.is_none();
    if flat {
        // Sem normais no arquivo: uma por face.
        for tri in indices.chunks_exact(3) {
            let mut t = [tri[0], tri[1], tri[2]];
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
            }
            part.mesh.indices.extend([start, start + 1, start + 2]);
        }
        return Ok(());
    }
    let normals = normals.expect("normais");
    for i in 0..count {
        let p = world.transform_point3(positions[i]);
        let n = (normal_matrix * normals.get(i).copied().unwrap_or(Vec3::Y)).normalize_or_zero();
        part.mesh
            .vertices
            .push(vertex(p, n, i, factor, &colors, &uvs));
    }
    for tri in indices.chunks_exact(3) {
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
    let bad = |what: &str| Err(ModelError::Invalid(format!("tipo de dado inválido em {what}")));
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

fn read_uri(uri: &str, load: &mut dyn FnMut(&str) -> Option<Vec<u8>>) -> Result<Vec<u8>, ModelError> {
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

/// Um modelo na GPU: uma malha por imagem usada.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub(crate) parts: Vec<(MeshId, Option<TextureId>)>,
    /// Caixa que envolve o modelo (mínimo, máximo).
    pub bounds: (Vec3, Vec3),
    pub triangles: usize,
}

impl Model {
    /// Tamanho do modelo (largura, altura, profundidade).
    pub fn size(&self) -> Vec3 {
        self.bounds.1 - self.bounds.0
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
