//! Efeitos de partículas prontos: fogo, fumaça, faíscas, explosão...
//! Cada um é só um jeito de preencher o componente Partículas.

use crate::scene_data::{Component, Prop};

#[derive(Debug)]
enum V {
    N(f64),
    S(&'static str),
    B(bool),
}
use V::{B, N, S};

/// Um efeito pronto.
#[derive(Debug)]
pub struct Preset {
    /// Nome do objeto criado com ele.
    pub id: &'static str,
    pub label: &'static str,
    pub tip: &'static str,
    values: &'static [(&'static str, V)],
}

impl Preset {
    /// O componente Partículas com este efeito.
    pub fn component(&self) -> Component {
        let mut c = Component::new("particles");
        for (k, v) in self.values {
            c.set(
                k,
                match v {
                    N(n) => Prop::Num(*n),
                    S(s) => Prop::Str((*s).to_string()),
                    B(b) => Prop::Bool(*b),
                },
            );
        }
        c
    }
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "fogo",
        label: "Fogo",
        tip: "Chamas subindo: tochas, fogueiras",
        values: &[
            ("rate", N(45.0)),
            ("lifetime", N(0.8)),
            ("lifetime_var", N(0.4)),
            ("speed_min", N(30.0)),
            ("speed_max", N(80.0)),
            ("direction", N(90.0)),
            ("spread", N(25.0)),
            ("gravity", N(80.0)),
            ("area", S("circulo")),
            ("area_radius", N(10.0)),
            ("size_start", N(26.0)),
            ("size_end", N(4.0)),
            ("size_var", N(0.3)),
            ("color_start", S("#FFE066")),
            ("color_mid", S("#FF7A1A")),
            ("color_end", S("#B0201000")),
        ],
    },
    Preset {
        id: "fumaca",
        label: "Fumaça",
        tip: "Fumaça subindo devagar: chaminés, coisas queimando",
        values: &[
            ("rate", N(12.0)),
            ("lifetime", N(2.2)),
            ("lifetime_var", N(0.3)),
            ("speed_min", N(20.0)),
            ("speed_max", N(45.0)),
            ("direction", N(90.0)),
            ("spread", N(30.0)),
            ("gravity", N(25.0)),
            ("drag", N(0.3)),
            ("area", S("circulo")),
            ("area_radius", N(8.0)),
            ("size_start", N(18.0)),
            ("size_end", N(60.0)),
            ("color_start", S("#9A9A9AAA")),
            ("color_end", S("#5A5A5A00")),
            ("spin", N(30.0)),
        ],
    },
    Preset {
        id: "faiscas",
        label: "Faíscas",
        tip: "Faíscas pulando e caindo: solda, máquina, fogueira",
        values: &[
            ("rate", N(25.0)),
            ("lifetime", N(0.6)),
            ("lifetime_var", N(0.5)),
            ("speed_min", N(120.0)),
            ("speed_max", N(260.0)),
            ("direction", N(90.0)),
            ("spread", N(70.0)),
            ("gravity", N(-500.0)),
            ("size_start", N(6.0)),
            ("size_end", N(1.0)),
            ("color_start", S("#FFFFFF")),
            ("color_mid", S("#FFD166")),
            ("color_end", S("#FF6B0000")),
            ("blend", S("add")),
        ],
    },
    Preset {
        id: "explosao",
        label: "Explosão",
        tip: "Estoura de uma vez e o objeto some: bom para um prefab",
        values: &[
            ("mode", S("explosao")),
            ("amount", N(60.0)),
            ("when_done", S("destruir")),
            ("lifetime", N(0.7)),
            ("lifetime_var", N(0.5)),
            ("speed_min", N(80.0)),
            ("speed_max", N(380.0)),
            ("spread", N(360.0)),
            ("drag", N(3.5)),
            ("size_start", N(28.0)),
            ("size_end", N(2.0)),
            ("size_var", N(0.4)),
            ("color_start", S("#FFF4D0")),
            ("color_mid", S("#FF9A2E")),
            ("color_end", S("#B0301000")),
        ],
    },
    Preset {
        id: "poeira",
        label: "Poeira",
        tip: "Um pouco de poeira no chão; solte com a ação Soltar partículas (ao pular, ao cair)",
        values: &[
            ("mode", S("explosao")),
            ("amount", N(10.0)),
            ("on_start", B(false)),
            ("lifetime", N(0.45)),
            ("lifetime_var", N(0.4)),
            ("speed_min", N(30.0)),
            ("speed_max", N(90.0)),
            ("direction", N(90.0)),
            ("spread", N(140.0)),
            ("gravity", N(-60.0)),
            ("drag", N(3.0)),
            ("area", S("retangulo")),
            ("area_w", N(30.0)),
            ("area_h", N(4.0)),
            ("size_start", N(12.0)),
            ("size_end", N(2.0)),
            ("color_start", S("#D8C7A8CC")),
            ("color_end", S("#B8A88800")),
        ],
    },
    Preset {
        id: "brilho",
        label: "Brilho",
        tip: "Brilho rápido em volta: pegar moeda, item, power-up (e some)",
        values: &[
            ("mode", S("explosao")),
            ("amount", N(24.0)),
            ("when_done", S("destruir")),
            ("lifetime", N(0.6)),
            ("lifetime_var", N(0.4)),
            ("speed_min", N(40.0)),
            ("speed_max", N(160.0)),
            ("spread", N(360.0)),
            ("drag", N(2.5)),
            ("size_start", N(10.0)),
            ("size_end", N(0.0)),
            ("color_start", S("#FFFFFF")),
            ("color_mid", S("#FFE27A")),
            ("color_end", S("#FFD16600")),
            ("blend", S("add")),
        ],
    },
    Preset {
        id: "bolhas",
        label: "Bolhas",
        tip: "Bolhas subindo: água, poção, aquário",
        values: &[
            ("rate", N(6.0)),
            ("lifetime", N(2.5)),
            ("lifetime_var", N(0.3)),
            ("speed_min", N(20.0)),
            ("speed_max", N(50.0)),
            ("direction", N(90.0)),
            ("spread", N(30.0)),
            ("gravity", N(20.0)),
            ("area", S("retangulo")),
            ("area_w", N(40.0)),
            ("area_h", N(6.0)),
            ("size_start", N(8.0)),
            ("size_end", N(14.0)),
            ("size_var", N(0.5)),
            ("color_start", S("#BFE9FF99")),
            ("color_end", S("#BFE9FF00")),
        ],
    },
    Preset {
        id: "chuva",
        label: "Chuva",
        tip: "Chuva caindo numa faixa larga: ponha no alto da cena",
        values: &[
            ("rate", N(140.0)),
            ("lifetime", N(1.4)),
            ("speed_min", N(650.0)),
            ("speed_max", N(750.0)),
            ("direction", N(260.0)),
            ("spread", N(3.0)),
            ("area", S("retangulo")),
            ("area_w", N(1500.0)),
            ("area_h", N(10.0)),
            ("size_start", N(4.0)),
            ("size_end", N(4.0)),
            ("color_start", S("#A8C8FFCC")),
            ("color_end", S("#A8C8FF66")),
            ("max", N(1000.0)),
        ],
    },
    Preset {
        id: "neve",
        label: "Neve",
        tip: "Neve caindo devagar numa faixa larga: ponha no alto da cena",
        values: &[
            ("rate", N(30.0)),
            ("lifetime", N(7.0)),
            ("speed_min", N(30.0)),
            ("speed_max", N(70.0)),
            ("direction", N(270.0)),
            ("spread", N(40.0)),
            ("area", S("retangulo")),
            ("area_w", N(1500.0)),
            ("area_h", N(10.0)),
            ("size_start", N(7.0)),
            ("size_end", N(7.0)),
            ("size_var", N(0.5)),
            ("color_start", S("#FFFFFF")),
            ("color_mid", S("#FFFFFF")),
            ("color_end", S("#FFFFFF00")),
        ],
    },
    Preset {
        id: "confete",
        label: "Confete",
        tip: "Confete colorido de uma vez: vitória, fim de fase",
        values: &[
            ("mode", S("explosao")),
            ("amount", N(80.0)),
            ("lifetime", N(2.5)),
            ("lifetime_var", N(0.3)),
            ("speed_min", N(220.0)),
            ("speed_max", N(440.0)),
            ("direction", N(90.0)),
            ("spread", N(70.0)),
            ("gravity", N(-350.0)),
            ("drag", N(1.2)),
            ("size_start", N(10.0)),
            ("size_end", N(10.0)),
            ("size_var", N(0.3)),
            ("color_start", S("#FF4D6D")),
            ("color_mid", S("#FFD166")),
            ("color_end", S("#06D6A0")),
            ("spin", N(360.0)),
        ],
    },
];

/// Os mesmos efeitos numa cena 3D: em metros, saindo para o "cima" do
/// objeto (gire o objeto para apontar).
pub const PRESETS_3D: &[Preset] = &[
    Preset {
        id: "fogo",
        label: "Fogo",
        tip: "Chamas subindo: tochas, fogueiras",
        values: &[
            ("rate", N(60.0)),
            ("lifetime", N(0.8)),
            ("lifetime_var", N(0.4)),
            ("speed_min", N(0.6)),
            ("speed_max", N(1.6)),
            ("spread", N(25.0)),
            ("gravity", N(1.5)),
            ("area", S("circulo")),
            ("area_radius", N(0.25)),
            ("size_start", N(0.5)),
            ("size_end", N(0.1)),
            ("size_var", N(0.3)),
            ("color_start", S("#FFE066")),
            ("color_mid", S("#FF7A1A")),
            ("color_end", S("#B0201000")),
            ("blend", S("add")),
        ],
    },
    Preset {
        id: "fumaca",
        label: "Fumaça",
        tip: "Fumaça subindo devagar: chaminés, fogueiras",
        values: &[
            ("rate", N(10.0)),
            ("lifetime", N(3.0)),
            ("lifetime_var", N(0.3)),
            ("speed_min", N(0.5)),
            ("speed_max", N(1.0)),
            ("spread", N(30.0)),
            ("gravity", N(0.4)),
            ("drag", N(0.3)),
            ("area", S("circulo")),
            ("area_radius", N(0.2)),
            ("size_start", N(0.4)),
            ("size_end", N(1.8)),
            ("color_start", S("#9A9A9AAA")),
            ("color_end", S("#5A5A5A00")),
            ("spin", N(30.0)),
        ],
    },
    Preset {
        id: "faiscas",
        label: "Faíscas",
        tip: "Faíscas pulando e caindo",
        values: &[
            ("rate", N(25.0)),
            ("lifetime", N(0.7)),
            ("lifetime_var", N(0.5)),
            ("speed_min", N(2.5)),
            ("speed_max", N(5.0)),
            ("spread", N(70.0)),
            ("gravity", N(-12.0)),
            ("size_start", N(0.12)),
            ("size_end", N(0.03)),
            ("color_start", S("#FFFFFF")),
            ("color_mid", S("#FFD166")),
            ("color_end", S("#FF6B0000")),
            ("blend", S("add")),
        ],
    },
    Preset {
        id: "explosao",
        label: "Explosão",
        tip: "Estoura de uma vez e o objeto some: bom para um prefab",
        values: &[
            ("mode", S("explosao")),
            ("amount", N(70.0)),
            ("when_done", S("destruir")),
            ("lifetime", N(0.8)),
            ("lifetime_var", N(0.5)),
            ("speed_min", N(2.0)),
            ("speed_max", N(7.0)),
            ("spread", N(360.0)),
            ("gravity", N(-4.0)),
            ("drag", N(2.0)),
            ("size_start", N(0.6)),
            ("size_end", N(0.0)),
            ("size_var", N(0.4)),
            ("color_start", S("#FFF3B0")),
            ("color_mid", S("#FF7A1A")),
            ("color_end", S("#40303000")),
            ("blend", S("add")),
        ],
    },
    Preset {
        id: "poeira",
        label: "Poeira",
        tip: "Nuvem de poeira no chão: ao pular, ao cair, passos",
        values: &[
            ("mode", S("explosao")),
            ("amount", N(14.0)),
            ("on_start", B(false)),
            ("lifetime", N(0.6)),
            ("lifetime_var", N(0.4)),
            ("speed_min", N(0.8)),
            ("speed_max", N(2.0)),
            ("spread", N(200.0)),
            ("gravity", N(0.5)),
            ("drag", N(4.0)),
            ("area", S("circulo")),
            ("area_radius", N(0.3)),
            ("size_start", N(0.35)),
            ("size_end", N(0.6)),
            ("color_start", S("#E8DCC4CC")),
            ("color_end", S("#C8B89800")),
        ],
    },
    Preset {
        id: "brilho",
        label: "Brilho",
        tip: "Estrelinhas em volta: itens, magia, um lugar especial",
        values: &[
            ("rate", N(8.0)),
            ("lifetime", N(1.2)),
            ("lifetime_var", N(0.4)),
            ("speed_min", N(0.1)),
            ("speed_max", N(0.4)),
            ("spread", N(360.0)),
            ("area", S("circulo")),
            ("area_radius", N(0.6)),
            ("size_start", N(0.2)),
            ("size_end", N(0.0)),
            ("color_start", S("#FFFFFF")),
            ("color_mid", S("#FFE066")),
            ("color_end", S("#FFD16600")),
            ("blend", S("add")),
            ("spin", N(180.0)),
        ],
    },
    Preset {
        id: "chuva",
        label: "Chuva",
        tip: "Chuva caindo numa área grande: suba o objeto uns 10 m",
        values: &[
            ("rate", N(400.0)),
            ("lifetime", N(1.2)),
            ("speed_min", N(12.0)),
            ("speed_max", N(14.0)),
            ("spread", N(5.0)),
            ("area", S("retangulo")),
            ("area_w", N(30.0)),
            ("area_h", N(30.0)),
            ("size_start", N(0.08)),
            ("size_end", N(0.08)),
            ("color_start", S("#C8DDF0AA")),
            ("color_end", S("#C8DDF066")),
            ("max", N(3000.0)),
        ],
    },
    Preset {
        id: "neve",
        label: "Neve",
        tip: "Flocos caindo devagar: suba o objeto uns 10 m",
        values: &[
            ("rate", N(60.0)),
            ("lifetime", N(8.0)),
            ("speed_min", N(0.8)),
            ("speed_max", N(1.4)),
            ("spread", N(30.0)),
            ("area", S("retangulo")),
            ("area_w", N(30.0)),
            ("area_h", N(30.0)),
            ("size_start", N(0.12)),
            ("size_end", N(0.12)),
            ("size_var", N(0.4)),
            ("color_start", S("#FFFFFF")),
            ("color_end", S("#FFFFFF88")),
            ("spin", N(90.0)),
            ("max", N(2000.0)),
        ],
    },
];

/// Um componente Partículas novo para uma cena 3D: os padrões em metros.
pub fn component_3d() -> Component {
    let mut c = Component::new("particles");
    for (k, v) in [
        ("speed_min", 1.0),
        ("speed_max", 3.0),
        ("size_start", 0.2),
        ("size_end", 0.0),
        ("area_w", 2.0),
        ("area_h", 2.0),
        ("area_radius", 0.5),
        ("spread", 60.0),
    ] {
        c.set(k, Prop::Num(v));
    }
    c
}

/// Os efeitos do tipo de cena (2D ou 3D).
pub fn presets(three_d: bool) -> &'static [Preset] {
    if three_d { PRESETS_3D } else { PRESETS }
}

/// O efeito pelo nome.
pub fn find(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// O efeito pelo nome, no tipo de cena.
pub fn find_in(id: &str, three_d: bool) -> Option<&'static Preset> {
    presets(three_d).iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn efeitos_usam_campos_da_engine() {
        let schema = crate::scene_data::schema();
        let fields: Vec<&str> = schema
            .fields("particles")
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(PRESETS.len(), 10);
        assert_eq!(PRESETS_3D.len(), 8);
        for p in PRESETS.iter().chain(PRESETS_3D) {
            for (k, _) in p.values {
                assert!(fields.contains(k), "{}: {k}", p.id);
            }
            // O tipo de cada valor é o mesmo do padrão.
            let c = p.component();
            for (k, v) in &c.props {
                let d = schema.default_of("particles", k).unwrap();
                assert_eq!(
                    std::mem::discriminant(v),
                    std::mem::discriminant(d),
                    "{}: {k}",
                    p.id
                );
            }
        }
    }
}
