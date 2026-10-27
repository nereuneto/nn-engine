//! O render no navegador, conferido pelo CI.
//!
//! No desktop: `render-web referencias <pasta>` grava cada cena num PNG, e
//! `render-web comparar <referencias> <navegador>` diz se as fotos do
//! navegador batem com elas. No navegador (wasm): a página `?cena=N` desenha
//! a cena N num canvas e escreve "NN pronto" no console; `?webgpu` usa o
//! WebGPU em vez do WebGL2.

mod cenas;

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::path::Path;
    use std::process::ExitCode;

    use nn_render::{Camera2D, GpuContext, OffscreenTarget, Renderer};

    use crate::cenas::{self, CENAS, Recursos, SIZE};

    pub fn referencias(dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let gpu = pollster::block_on(GpuContext::new_headless()).map_err(|e| e.to_string())?;
        let target = OffscreenTarget::new(&gpu, SIZE);
        for (i, nome) in CENAS.iter().enumerate() {
            // Um renderer novo por cena, como no navegador (uma página por
            // cena): o que uma cena liga não vale na seguinte.
            let mut renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
            let rec = Recursos::new(&mut renderer);
            cenas::desenhar(i, &mut renderer, &rec);
            renderer.render(
                target.view(),
                SIZE,
                &Camera2D::default(),
                Some(cenas::fundo(i)),
            );
            let file = dir.join(format!("{i}-{nome}.png"));
            target.save_png(&gpu, &file).map_err(|e| e.to_string())?;
            println!("{}", file.display());
        }
        Ok(())
    }

    /// Pixels de uma foto (RGB), no tamanho das cenas.
    fn pixels(file: &Path) -> Result<Vec<[u8; 3]>, String> {
        let img = image::open(file)
            .map_err(|e| format!("{}: {e}", file.display()))?
            .into_rgb8();
        if img.width() != SIZE.x || img.height() != SIZE.y {
            return Err(format!(
                "{}: {}x{} em vez de {}x{}",
                file.display(),
                img.width(),
                img.height(),
                SIZE.x,
                SIZE.y
            ));
        }
        Ok(img.pixels().map(|p| p.0).collect())
    }

    /// Quanto as fotos podem diferir: a placa por software do navegador
    /// arredonda as bordas e o filtro um pouco diferente da do desktop, e o
    /// 3D (o céu, a luz) sai uns tons diferente no WebGL2.
    const PIXEL_DIFERENTE: u8 = 24;
    const MAX_DIFERENTES: f64 = 0.01;
    const MAX_MEDIA: f64 = 4.0;

    pub fn comparar(refs: &Path, nav: &Path, prefixo: &str) -> ExitCode {
        let mut ok = true;
        for (i, nome) in CENAS.iter().enumerate() {
            let a = refs.join(format!("{i}-{nome}.png"));
            let b = nav.join(format!("{prefixo}{i}-{nome}.png"));
            match (pixels(&a), pixels(&b)) {
                (Ok(a), Ok(b)) => {
                    let mut soma = 0u64;
                    let mut diferentes = 0usize;
                    // Onde ficam os pixels diferentes (para achar a causa).
                    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
                    let mut linhas = vec![0u32; SIZE.y as usize];
                    let mut colunas = vec![0u32; SIZE.x as usize];
                    for (i, (p, q)) in a.iter().zip(&b).enumerate() {
                        let d = (0..3).map(|c| p[c].abs_diff(q[c])).max().unwrap_or(0);
                        soma += (0..3).map(|c| u64::from(p[c].abs_diff(q[c]))).sum::<u64>();
                        if d > PIXEL_DIFERENTE {
                            diferentes += 1;
                            let (x, y) = (i as u32 % SIZE.x, i as u32 / SIZE.x);
                            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                            linhas[y as usize] += 1;
                            colunas[x as usize] += 1;
                        }
                    }
                    if diferentes > 0 {
                        let cheias = |v: &[u32], n: u32| {
                            v.iter()
                                .enumerate()
                                .filter(|(_, c)| **c * 2 > n)
                                .map(|(i, _)| i.to_string())
                                .collect::<Vec<_>>()
                                .join(" ")
                        };
                        println!(
                            "      diferentes em x {x0}..{x1}, y {y0}..{y1}; linhas quase todas diferentes: [{}]; colunas: [{}]",
                            cheias(&linhas, SIZE.x),
                            cheias(&colunas, SIZE.y)
                        );
                    }
                    let media = soma as f64 / (a.len() * 3) as f64;
                    let fracao = diferentes as f64 / a.len() as f64;
                    let passou = fracao <= MAX_DIFERENTES && media <= MAX_MEDIA;
                    println!(
                        "{} {prefixo}{i}-{nome}: {:.2}% dos pixels diferentes, diferença média {media:.2}",
                        if passou { "ok   " } else { "FALHA" },
                        fracao * 100.0
                    );
                    ok &= passou;
                }
                (Err(e), _) | (_, Err(e)) => {
                    println!("FALHA {prefixo}{i}-{nome}: {e}");
                    ok = false;
                }
            }
        }
        if ok {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    use std::path::Path;
    use std::process::ExitCode;
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["referencias", dir] => match desktop::referencias(Path::new(dir)) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("erro: {e}");
                ExitCode::FAILURE
            }
        },
        ["cenas"] => {
            println!("{}", cenas::CENAS.join(","));
            ExitCode::SUCCESS
        }
        ["comparar", refs, nav] => desktop::comparar(Path::new(refs), Path::new(nav), ""),
        ["comparar", refs, nav, prefixo] => {
            desktop::comparar(Path::new(refs), Path::new(nav), prefixo)
        }
        _ => {
            eprintln!(
                "uso: render-web referencias <pasta> | cenas | comparar <referencias> <navegador> [prefixo]"
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::main();
}
