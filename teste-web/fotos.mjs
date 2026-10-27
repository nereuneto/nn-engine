// Abre cada cena no Chromium sem tela e fotografa o canvas.
// Uso: node fotos.mjs <url base> <quantas cenas> <pasta> <webgl|webgpu> <nomes separados por vírgula>
import { chromium } from "playwright";

const [, , base, n, pasta, modo, nomes] = process.argv;
const lista = nomes.split(",");
const browser = await chromium.launch({
  channel: "chromium",
  args: [
    "--use-angle=swiftshader",
    "--enable-unsafe-swiftshader",
    "--enable-unsafe-webgpu",
    "--enable-features=Vulkan",
    "--use-webgpu-adapter=swiftshader",
    "--use-vulkan=swiftshader",
    "--ignore-gpu-blocklist",
  ],
});
let falhou = false;
for (let i = 0; i < Number(n); i++) {
  const page = await browser.newPage({ viewport: { width: 400, height: 260 }, deviceScaleFactor: 1 });
  const logs = [];
  page.on("console", (m) => logs.push(m.text()));
  page.on("pageerror", (e) => logs.push("pageerror " + e.message));
  await page.goto(`${base}/index.html?cena=${i}${modo === "webgpu" ? "&webgpu" : ""}`);
  const fim = Date.now() + 30000;
  while (Date.now() < fim && !logs.some((l) => l.startsWith("NN pronto") || l.startsWith("NN erro") || l.startsWith("pageerror"))) {
    await page.waitForTimeout(100);
  }
  if (!logs.some((l) => l.startsWith("NN pronto"))) {
    console.log(`FALHA ${modo} cena ${i}: não ficou pronta\n  ${logs.join("\n  ")}`);
    falhou = true;
  } else {
    await page.locator("canvas").screenshot({ path: `${pasta}/${modo}-${i}-${lista[i]}.png` });
    const avisos = logs.filter((l) => !l.startsWith("NN "));
    console.log(`ok ${modo} cena ${i} (${lista[i]})${avisos.length ? "\n  " + avisos.join("\n  ") : ""}`);
  }
  await page.close();
}
await browser.close();
process.exit(falhou ? 1 : 0);
