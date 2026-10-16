//! Liga o ícone ao executável no Windows e avisa o Cargo para compilar de
//! novo quando os jogos de exemplo (que vão dentro do executável) mudarem.

fn main() {
    println!("cargo:rerun-if-changed=../../jogos");
    println!("cargo:rerun-if-changed=assets/icone.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icone.ico");
        res.set("FileDescription", "NN Studio");
        res.set("ProductName", "NN Studio");
        // Sem as ferramentas de recurso do Windows, segue sem o ícone.
        if let Err(e) = res.compile() {
            println!("cargo:warning=ícone do executável não aplicado: {e}");
        }
    }
}
