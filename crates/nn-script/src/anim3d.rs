//! Animações de modelos 3D no Lua: `modelo:animator()`.
//!
//! ```lua
//! local heroi = nn.assets.model("modelos/heroi.glb")
//! local anim = heroi:animator()
//! anim:play("andar")                       -- repete; troca suave (0,2 s)
//! -- no update:
//! anim:update(dt)
//! -- no draw:
//! nn.d3.draw(heroi, { x = 0, animator = anim })
//! ```

use std::sync::Arc;

use mlua::{Result, Table, UserData, UserDataFields, UserDataMethods};
use nn_render::d3::{Pose, Skeleton};

use crate::convert::{err, opt};

/// Uma animação tocando: qual, em que instante e se repete.
#[derive(Debug, Clone, Copy)]
struct Track {
    index: usize,
    time: f32,
    looped: bool,
}

/// Toca as animações de um modelo para um objeto.
#[derive(Debug, Clone)]
pub(crate) struct LAnimator {
    skeleton: Option<Arc<Skeleton>>,
    current: Option<Track>,
    /// A animação de antes, enquanto mistura com a nova.
    previous: Option<Track>,
    /// De 0 (só a de antes) a 1 (só a nova).
    mix: f32,
    mix_time: f32,
    speed: f32,
}

impl LAnimator {
    pub(crate) fn new(skeleton: Option<Arc<Skeleton>>) -> Self {
        Self {
            skeleton,
            current: None,
            previous: None,
            mix: 1.0,
            mix_time: 0.0,
            speed: 1.0,
        }
    }

    fn duration(&self, index: usize) -> f32 {
        self.skeleton
            .as_ref()
            .and_then(|s| s.animations.get(index))
            .map(|a| a.duration)
            .unwrap_or(0.0)
    }

    fn advance(&self, t: &mut Track, dt: f32) {
        let d = self.duration(t.index);
        t.time += dt;
        if d <= 0.0 {
            t.time = 0.0;
        } else if t.looped {
            t.time = t.time.rem_euclid(d);
        } else {
            t.time = t.time.clamp(0.0, d);
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        let dt = dt * self.speed;
        if let Some(mut t) = self.current {
            self.advance(&mut t, dt);
            self.current = Some(t);
        }
        if let Some(mut t) = self.previous {
            self.advance(&mut t, dt);
            self.previous = Some(t);
            self.mix += dt.abs() / self.mix_time.max(1e-4);
            if self.mix >= 1.0 {
                self.mix = 1.0;
                self.previous = None;
            }
        }
    }

    fn pose(&self) -> Option<(Pose, &Skeleton)> {
        let s = self.skeleton.as_deref()?;
        let mut pose = match self.current {
            Some(t) => s.pose(t.index, t.time),
            None => s.rest_pose(),
        };
        if let Some(p) = self.previous {
            let mut before = s.pose(p.index, p.time);
            before.blend(&pose, self.mix);
            pose = before;
        }
        Some((pose, s))
    }

    /// As matrizes dos ossos agora (vazio num modelo sem esqueleto).
    pub(crate) fn joints(&self) -> Vec<glam::Mat4> {
        match self.pose() {
            Some((pose, s)) => s.joint_matrices(&pose),
            None => Vec::new(),
        }
    }

    fn names(&self) -> Vec<String> {
        self.skeleton
            .as_ref()
            .map(|s| s.animations.iter().map(|a| a.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Começa a tocar `name`. `true` se mudou de animação.
    pub(crate) fn play(
        &mut self,
        name: &str,
        looped: bool,
        blend: f32,
        restart: bool,
    ) -> Result<bool> {
        let Some(s) = self.skeleton.as_ref() else {
            return Err(err("este modelo não tem animações"));
        };
        let Some(index) = s.animation(name) else {
            let names = self.names();
            return Err(err(format!(
                "o modelo não tem a animação \"{name}\" (tem: {})",
                if names.is_empty() {
                    "nenhuma".to_string()
                } else {
                    names.join(", ")
                }
            )));
        };
        if let Some(t) = &mut self.current
            && t.index == index
            && !restart
        {
            t.looped = looped;
            return Ok(false);
        }
        self.previous = if blend > 0.0 { self.current } else { None };
        self.mix = if self.previous.is_some() { 0.0 } else { 1.0 };
        self.mix_time = blend;
        self.current = Some(Track {
            index,
            time: 0.0,
            looped,
        });
        Ok(true)
    }

    fn finished(&self) -> bool {
        match self.current {
            Some(t) => !t.looped && t.time >= self.duration(t.index) - 1e-5,
            None => false,
        }
    }
}

impl UserData for LAnimator {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("animation", |_, this| {
            Ok(this.current.map(|t| {
                this.skeleton
                    .as_ref()
                    .and_then(|s| s.animations.get(t.index))
                    .map(|a| a.name.clone())
                    .unwrap_or_default()
            }))
        });
        f.add_field_method_get("time", |_, this| Ok(this.current.map_or(0.0, |t| t.time)));
        f.add_field_method_set("time", |_, this, v: f32| {
            if let Some(mut t) = this.current {
                t.time = 0.0;
                this.current = Some(t);
                this.update(v / this.speed.max(1e-6));
            }
            Ok(())
        });
        f.add_field_method_get("duration", |_, this| {
            Ok(this.current.map_or(0.0, |t| this.duration(t.index)))
        });
        f.add_field_method_get("speed", |_, this| Ok(this.speed));
        f.add_field_method_set("speed", |_, this, v: f32| {
            this.speed = v;
            Ok(())
        });
        f.add_field_method_get("looping", |_, this| {
            Ok(this.current.is_some_and(|t| t.looped))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        // play(nome, { loop = true, blend = 0.2, speed, restart = false })
        m.add_method_mut("play", |_, this, (name, opts): (String, Option<Table>)| {
            let o = opts.as_ref();
            let looped = opt::<bool>(o, "loop")?.unwrap_or(true);
            let blend = opt::<f32>(o, "blend")?.unwrap_or(0.2).max(0.0);
            let restart = opt::<bool>(o, "restart")?.unwrap_or(false);
            if let Some(s) = opt::<f32>(o, "speed")? {
                this.speed = s;
            }
            this.play(&name, looped, blend, restart)
        });
        m.add_method_mut("stop", |_, this, ()| {
            this.current = None;
            this.previous = None;
            this.mix = 1.0;
            Ok(())
        });
        m.add_method_mut("update", |_, this, dt: f32| {
            this.update(dt);
            Ok(())
        });
        m.add_method("finished", |_, this, ()| Ok(this.finished()));
        m.add_method("animations", |_, this, ()| Ok(this.names()));
        m.add_method("has", |_, this, name: String| {
            Ok(this
                .skeleton
                .as_ref()
                .is_some_and(|s| s.animation(&name).is_some()))
        });
        // Onde um osso está, no espaço do modelo (para prender uma espada na
        // mão: matriz_do_objeto * anim:bone("mao")).
        m.add_method("bone", |_, this, name: String| {
            let Some((pose, s)) = this.pose() else {
                return Ok(None);
            };
            Ok(s.node_matrix(&pose, &name).map(crate::d3::LMat4))
        });
        // Os nomes dos ossos (e das partes) do modelo, na ordem do arquivo.
        m.add_method("bones", |_, this, ()| {
            Ok(this
                .skeleton
                .as_ref()
                .map(|s| s.nodes.iter().map(|n| n.name.clone()).collect::<Vec<_>>())
                .unwrap_or_default())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esqueleto() -> Arc<Skeleton> {
        // Um modelo de verdade, para ter as animações.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../jogos/estrelas3d/modelos/heroi.glb");
        let bytes = std::fs::read(path).unwrap();
        let data = nn_render::d3::ModelData::from_gltf(&bytes, &mut |_| None).unwrap();
        Arc::new(data.skeleton.unwrap())
    }

    #[test]
    fn toca_repete_mistura_e_termina() {
        let mut a = LAnimator::new(Some(esqueleto()));
        assert!(a.play("andar", true, 0.0, false).unwrap());
        assert!(
            !a.play("andar", true, 0.0, false).unwrap(),
            "a mesma continua"
        );
        a.update(1.0);
        assert!(
            (a.current.unwrap().time - 0.2).abs() < 1e-4,
            "repete: 1,0 s em 0,8"
        );
        // Troca com mistura de 0,2 s: no meio, as duas.
        a.play("correr", true, 0.2, false).unwrap();
        a.update(0.1);
        assert!(a.previous.is_some() && (a.mix - 0.5).abs() < 1e-4);
        a.update(0.2);
        assert!(a.previous.is_none() && a.mix == 1.0);
        // Pular toca uma vez e para no fim.
        a.play("pular", false, 0.0, false).unwrap();
        assert!(!a.finished());
        a.update(1.0);
        assert!(a.finished());
        assert_eq!(a.joints().len(), 12);
        let e = a.play("voar", true, 0.0, false).unwrap_err().to_string();
        assert!(e.contains("parado, andar"), "{e}");
    }
}
