//! A dica do dia, no Início: um atalho ou um jeito mais rápido de fazer algo.

use eframe::egui::{self, RichText};

/// As dicas, uma por vez.
pub const TIPS: &[&str] = &[
    "Com o mouse na cena, Shift+A abre uma busca: digite \"luz\", \"cubo\" ou o nome de um prefab e aperte Enter. O objeto nasce onde o mouse está.",
    "O botão direito na cena cria no ponto do clique. Em cima de um objeto, mostra o menu dele: duplicar, focar, copiar a posição...",
    "Duplicou e moveu a cópia? O próximo Ctrl+D põe a nova o mesmo tanto adiante. Uma cerca sai com Ctrl+D, Ctrl+D, Ctrl+D.",
    "Alt + arrastar um objeto leva uma cópia e deixa o original. No 3D, segure Alt e arraste uma seta do Mover.",
    "Shift+D faz uma cópia que segue o mouse até você clicar. Com Encaixar ligado, ela anda na grade.",
    "Selecione vários objetos e mude um valor no Inspetor: muda em todos. O ≠ marca o que está diferente entre eles.",
    "Algo que você monta toda hora? Botão direito no objeto > ⭐ Guardar nos favoritos. Ele aparece no topo do ➕ Objeto.",
    "Ctrl+C e Ctrl+V copiam objetos, também de uma cena para outra. Com o mouse na cena, o colado vai para onde o mouse está.",
    "No menu ☰ de um componente: Copiar componente e, em outro objeto, Colar valores. Bom para deixar várias luzes iguais.",
    "F com o mouse na cena enquadra o selecionado. Perdeu-se no 3D? Aperte F.",
    "Ctrl+Shift+P abre a paleta com todos os comandos, e Ctrl+P acha qualquer arquivo pelo nome.",
    "▶ Testar daqui (F6) roda o jogo começando na cena aberta. Com o jogo aberto, o que você muda na cena aparece nele na hora.",
    "Em Exibir > Preferências dá para trocar a cor de destaque, ligar sons da interface e desligar as animações.",
];

fn tip_id() -> egui::Id {
    egui::Id::new("dica-do-dia")
}

/// A dica que aparece agora (muda a cada dia, ou no "Próxima dica").
pub fn current(ctx: &egui::Context) -> usize {
    let offset: usize = ctx.data_mut(|d| *d.get_persisted_mut_or_default::<usize>(tip_id()));
    let day = (crate::settings::now_secs() / 86_400) as usize;
    (day + offset) % TIPS.len()
}

/// O quadro "Você sabia?" com a dica e o botão de passar.
pub fn tip_ui(ui: &mut egui::Ui) {
    let i = current(ui.ctx());
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_max_width(560.0);
        ui.label(RichText::new("💡 Você sabia?").size(17.0).strong());
        ui.add_space(2.0);
        ui.label(TIPS[i]);
        ui.add_space(2.0);
        if ui.small_button("Próxima dica ⏵").clicked() {
            ui.ctx().data_mut(|d| {
                *d.get_persisted_mut_or_default::<usize>(tip_id()) += 1;
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxima_dica_passa_e_da_a_volta() {
        let ctx = egui::Context::default();
        let a = current(&ctx);
        ctx.data_mut(|d| *d.get_persisted_mut_or_default::<usize>(tip_id()) += 1);
        assert_eq!(current(&ctx), (a + 1) % TIPS.len());
        assert!(TIPS.iter().all(|t| t.len() < 220), "dicas curtas");
    }
}
