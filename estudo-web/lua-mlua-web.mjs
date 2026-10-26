// Roda o lua-mlua-web.wasm no Node com o WASI do próprio Node (na página,
// seria um shim pequeno: escrever no console, relógio e o resto "não tem").
// Uso: node lua-mlua-web.mjs <arquivo.wasm>
import { readFileSync } from "node:fs";
import { WASI } from "node:wasi";

const bytes = readFileSync(process.argv[2]);
const module = await WebAssembly.compile(bytes);
const imports = WebAssembly.Module.imports(module);
console.log("imports do wasm:\n" + imports.map((i) => `- ${i.module}.${i.name}`).join("\n") + "\n");
const wasi = new WASI({ version: "preview1", args: [], env: {}, returnOnExit: true });
const outros = new Proxy({}, { get: (_, nome) => () => { throw new Error(`import sem implementação: ${nome}`); } });
const env = { agora_ms: () => performance.now() };
const importObject = { wasi_snapshot_preview1: wasi.wasiImport, env: new Proxy(env, { get: (t, n) => t[n] ?? outros[n] }) };
const instance = await WebAssembly.instantiate(module, importObject);
if (instance.exports._initialize) wasi.initialize(instance);
const r = instance.exports.rodar();
const ptr = Number(r & 0xffffffffn);
const len = Number(r >> 32n);
console.log(new TextDecoder().decode(new Uint8Array(instance.exports.memory.buffer, ptr, len)));
