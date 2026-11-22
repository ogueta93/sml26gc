// SPDX-License-Identifier: GPL-3.0-or-later
// One of the editor's capture workers: runs sml26gc_capture.wasm (crates/capture-worker), which
// captures one job line at a time and answers its result line. Messages in: { program, rom,
// original } once, then { job }. Messages out: { ready } once, then { done }.
let program = null;

function fill(bytes) {
    const at = program.capture_buffer(bytes.length);
    new Uint8Array(program.memory.buffer, at, bytes.length).set(bytes);
}

onmessage = async e => {
    const m = e.data;
    if (m.program) {
        try {
            const { instance } = await WebAssembly.instantiateStreaming(fetch(m.program), {});
            program = instance.exports;
            fill(m.rom);
            fill(m.original);
            postMessage({ ready: program.capture_init() === 1 });
        } catch (error) {
            console.warn("a capture worker could not start", error);
            postMessage({ ready: false });
        }
        return;
    }
    fill(new TextEncoder().encode(m.job));
    const len = program.capture_run();
    postMessage({ done: new TextDecoder().decode(new Uint8Array(program.memory.buffer, program.capture_result(), len)) });
};
