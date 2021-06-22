importScripts('/wasm_exec.js');
const go = new Go();
WebAssembly.instantiateStreaming(fetch('/server.wasm'), go.importObject).then(result => go.run(result.instance));
