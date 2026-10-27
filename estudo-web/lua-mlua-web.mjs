// Roda o lua-mlua-web.wasm com um "WASI" mínimo escrito à mão: é o mesmo que
// iria na página (relógio, sorteio, escrever no console; o resto responde
// "não tem"). Uso: node lua-mlua-web.mjs <arquivo.wasm>
import { readFileSync } from "node:fs";

const module = await WebAssembly.compile(readFileSync(process.argv[2]));
console.log("imports do wasm:\n" + WebAssembly.Module.imports(module).map((i) => `- ${i.module}.${i.name}`).join("\n") + "\n");
let memory;
const view = () => new DataView(memory.buffer);
const ENOSYS = 52;
const EBADF = 8;
const wasi = {
  clock_time_get: (_id, _precision, out) => {
    view().setBigUint64(out, BigInt(Math.round(performance.now() * 1e6)), true);
    return 0;
  },
  random_get: (ptr, len) => {
    crypto.getRandomValues(new Uint8Array(memory.buffer, ptr, len));
    return 0;
  },
  environ_sizes_get: (count, size) => {
    view().setUint32(count, 0, true);
    view().setUint32(size, 0, true);
    return 0;
  },
  environ_get: () => 0,
  fd_write: (fd, iovs, n, written) => {
    let total = 0;
    let text = "";
    for (let i = 0; i < n; i++) {
      const ptr = view().getUint32(iovs + i * 8, true);
      const len = view().getUint32(iovs + i * 8 + 4, true);
      text += new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
      total += len;
    }
    if (fd === 1 || fd === 2) process.stdout.write(text);
    view().setUint32(written, total, true);
    return fd === 1 || fd === 2 ? 0 : EBADF;
  },
  fd_prestat_get: () => EBADF,
  proc_exit: (code) => {
    throw new Error(`proc_exit(${code})`);
  },
};
const naoTem = new Proxy(wasi, { get: (t, n) => t[n] ?? (() => ENOSYS) });
const instance = await WebAssembly.instantiate(module, {
  wasi_snapshot_preview1: naoTem,
  env: { agora_ms: () => performance.now() },
});
memory = instance.exports.memory;
const r = instance.exports.rodar();
const ptr = Number(r & 0xffffffffn);
const len = Number(r >> 32n);
console.log(new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len)));
