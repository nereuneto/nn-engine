// Abre a prova no Chromium (sem tela), usa o teclado e o mouse, tira fotos e
// grava o console. Uso: node navegador.mjs <url> <nome> <pasta>
import { chromium } from "playwright";
import { writeFileSync } from "node:fs";

const [, , url, nome, pasta] = process.argv;
const browser = await chromium.launch({
  // O Chromium inteiro no modo sem tela novo (o "headless shell" não tem WebGPU).
  channel: "chromium",
  args: [
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--enable-unsafe-webgpu",
    "--enable-features=Vulkan",
    "--use-webgpu-adapter=swiftshader",
    "--use-vulkan=swiftshader",
    "--ignore-gpu-blocklist",
    "--autoplay-policy=no-user-gesture-required",
  ],
});
const page = await browser.newPage({ viewport: { width: 820, height: 470 } });
const t0 = Date.now();
const logs = [];
const s = () => ((Date.now() - t0) / 1000).toFixed(2);
page.on("console", (m) => logs.push(`${s()} [${m.type()}] ${m.text()}`));
page.on("pageerror", (e) => logs.push(`${s()} [pageerror] ${e.message}`));
page.on("requestfailed", (r) => logs.push(`${s()} [requestfailed] ${r.url()}`));
// Guarda os AudioContext que a página criar, para ver se o som ligou.
await page.addInitScript(() => {
  const A = window.AudioContext;
  window.__audios = [];
  if (A) {
    window.AudioContext = class extends A {
      constructor(...a) {
        super(...a);
        window.__audios.push(this);
      }
    };
  }
});
const espera = async (re, ms) => {
  const fim = Date.now() + ms;
  while (Date.now() < fim) {
    if (logs.some((l) => re.test(l))) return true;
    await page.waitForTimeout(100);
  }
  return false;
};
let resumo = [];
try {
  await page.goto(url);
  const pronto = await espera(/NN (pronto|erro)/, 60000);
  resumo.push(`pronto: ${pronto} (${s()} s desde abrir a página)`);
  await page.waitForTimeout(2000);
  await page.screenshot({ path: `${pasta}/${nome}-1.png` });
  const canvas = await page.$("canvas");
  resumo.push(`canvas: ${canvas ? "sim" : "não"}`);
  if (canvas) {
    await canvas.click({ position: { x: 700, y: 300 } });
    await page.keyboard.down("ArrowRight");
    await page.waitForTimeout(800);
    await page.keyboard.up("ArrowRight");
    await page.keyboard.down("ArrowDown");
    await page.waitForTimeout(400);
    await page.keyboard.up("ArrowDown");
  }
  await page.waitForTimeout(2500);
  await page.screenshot({ path: `${pasta}/${nome}-2.png` });
  const audios = await page.evaluate(() =>
    (window.__audios || []).map((a) => `${a.state} ${a.sampleRate} Hz`),
  );
  resumo.push(`AudioContext: ${audios.length ? audios.join(", ") : "nenhum"}`);
} catch (e) {
  resumo.push(`erro do teste: ${e.message}`);
}
await browser.close();
const texto = `# Navegador: ${nome}\n\n${url}\n\n${resumo.map((r) => `- ${r}`).join("\n")}\n\n\`\`\`\n${logs.join("\n")}\n\`\`\`\n`;
writeFileSync(`${pasta}/${nome}.md`, texto);
console.log(texto);
