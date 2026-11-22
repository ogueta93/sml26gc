// SPDX-License-Identifier: GPL-3.0-or-later
// The page's side of the web build: what the program asks of the browser through its `sml_*`
// imports (crates/host/src/web.rs). Before the program starts, every file kept for this site is
// read from IndexedDB; the program takes them, and later the files another page of the site
// changed, from `queue`. Its writes go to IndexedDB and to the site's other pages.
"use strict";

const sml = (() => {
    const DB_NAME = "sml26gc";
    const STORE = "files";
    const NOT_KEPT = ["/incoming/"];
    const ARGUMENTS = ["project", "level", "start", "point", "test", "lives", "coins", "power", "lang", "cheats"];
    const FLAGS = ["test", "cheats"];
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();

    let db = null;
    const queue = [];
    const arrived = [];
    const pending = {};
    const fetching = {};
    const rects = {};
    const channel = "BroadcastChannel" in window ? new BroadcastChannel("sml26gc-files") : null;
    let audio = null;
    let audioAt = 0;
    let chooserOpenedAt = 0;
    // A cartridge on its way: the chooser open (until a file or its closing), files being read,
    // or read and not yet taken by the program. No other chooser opens meanwhile.
    let choosing = false;
    let reading = 0;
    const coming = () => choosing || reading > 0 || arrived.length > 0;
    const chooser = document.createElement("input");
    chooser.type = "file";
    chooser.style.display = "none";

    const memory = () => new Uint8Array(wasm_memory.buffer);
    const text = (ptr, len) => decoder.decode(memory().subarray(ptr, ptr + len));
    const bytes = (ptr, len) => memory().slice(ptr, ptr + len);

    function openDb() {
        return new Promise((resolve, reject) => {
            const req = indexedDB.open(DB_NAME, 1);
            req.onupgradeneeded = () => req.result.createObjectStore(STORE);
            req.onsuccess = () => resolve(req.result);
            req.onerror = () => reject(req.error);
        });
    }

    function readAll() {
        return new Promise((resolve, reject) => {
            const tx = db.transaction(STORE, "readonly");
            const req = tx.objectStore(STORE).openCursor();
            req.onsuccess = () => {
                const cursor = req.result;
                if (!cursor) return resolve();
                queue.push({ path: encoder.encode(cursor.key), data: cursor.value.data, time: cursor.value.time });
                cursor.continue();
            };
            req.onerror = () => reject(req.error);
        });
    }

    function store(path, data, time) {
        if (NOT_KEPT.some(p => path.startsWith(p)) || !db) return;
        const tx = db.transaction(STORE, "readwrite");
        if (data) tx.objectStore(STORE).put({ data, time }, path);
        else tx.objectStore(STORE).delete(path);
    }

    function changed(path, data, time) {
        store(path, data, time);
        if (channel) channel.postMessage({ path, data, time });
    }

    function arrive(name, data) {
        queue.push({ path: encoder.encode("/incoming/" + name), data, time: Date.now() });
        arrived.push(name);
    }

    async function take(files) {
        choosing = false;
        reading++;
        try {
            for (const file of files) arrive(file.name, new Uint8Array(await file.arrayBuffer()));
        } finally {
            reading--;
        }
    }

    // The chooser closed: a chosen file's change event comes first, or (on some phones, while
    // the file is copied) a moment later; without one, it was cancelled.
    function chooserClosed() {
        if (!choosing) return;
        setTimeout(() => {
            if (choosing && !chooser.files.length) choosing = false;
        }, 1500);
    }

    // Only within a tap or click the browser allows (a touch's start does not); the tap's own
    // handler (`tapped`) opens it then.
    function choose(accept) {
        const activation = navigator.userActivation;
        if (activation && !activation.isActive) return;
        if (coming() || Date.now() - chooserOpenedAt < 1000) return;
        chooserOpenedAt = Date.now();
        choosing = true;
        chooser.accept = accept;
        chooser.value = "";
        chooser.click();
    }

    function fetchFont(name) {
        if (fetching[name]) return;
        fetching[name] = true;
        fetch("fonts/" + name).then(r => r.ok ? r.arrayBuffer() : Promise.reject(r.status)).then(buf => {
            const data = new Uint8Array(buf);
            const path = "/fonts/" + name;
            const time = Date.now();
            queue.push({ path: encoder.encode(path), data, time });
            store(path, data, time);
        }).catch(() => { fetching[name] = false; });
    }

    function audioContext() {
        if (!audio) {
            const Context = window.AudioContext || window.webkitAudioContext;
            audio = Context ? new Context() : null;
        }
        return audio;
    }

    function unlockAudio() {
        const ctx = audioContext();
        if (ctx && ctx.state !== "running") ctx.resume();
    }

    function pushAudio(ptr, len, volume) {
        const ctx = audioContext();
        if (!ctx || ctx.state !== "running" || len < 2) return;
        const now = ctx.currentTime;
        if (audioAt < now + 0.01) audioAt = now + 0.03;
        if (audioAt - now > 0.15) return;
        const samples = new Float32Array(wasm_memory.buffer, ptr, len);
        const frames = len >> 1;
        const buffer = ctx.createBuffer(2, frames, ctx.sampleRate);
        const left = buffer.getChannelData(0), right = buffer.getChannelData(1);
        for (let i = 0; i < frames; i++) {
            left[i] = samples[2 * i] * volume;
            right[i] = samples[2 * i + 1] * volume;
        }
        const source = ctx.createBufferSource();
        source.buffer = buffer;
        source.connect(ctx.destination);
        source.start(audioAt);
        audioAt += frames / ctx.sampleRate;
    }

    function fullscreenElement() {
        return document.fullscreenElement || document.webkitFullscreenElement || null;
    }

    function canFullscreen() {
        return !!(document.fullscreenEnabled || document.webkitFullscreenEnabled);
    }

    function setFullscreen(on) {
        const page = document.documentElement;
        if (on && !fullscreenElement()) {
            const go = page.requestFullscreen || page.webkitRequestFullscreen;
            if (go) Promise.resolve(go.call(page)).catch(() => {});
        } else if (!on && fullscreenElement()) {
            const leave = document.exitFullscreen || document.webkitExitFullscreen;
            if (leave) Promise.resolve(leave.call(document)).catch(() => {});
        }
    }

    function standalone() {
        return matchMedia("(display-mode: fullscreen)").matches || matchMedia("(display-mode: standalone)").matches || navigator.standalone === true;
    }

    function ios() {
        return /iPhone|iPad|iPod/.test(navigator.userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1);
    }

    function argumentsText() {
        const lines = [];
        for (const [name, value] of new URLSearchParams(location.search)) {
            if (!ARGUMENTS.includes(name)) continue;
            lines.push("--" + name);
            if (!FLAGS.includes(name)) lines.push(value);
        }
        return lines.join("\n");
    }

    function textOf(what) {
        switch (what) {
            case 0: return argumentsText();
            case 1: return (navigator.languages || [navigator.language || "en"]).join("\n");
            case 2: return arrived.length ? arrived.shift() : null;
        }
        return null;
    }

    function padState(ptr, len) {
        const pads = navigator.getGamepads ? Array.from(navigator.getGamepads()) : [];
        const pad = pads.find(p => p && p.connected);
        if (!pad) return 0;
        const out = new Float32Array(wasm_memory.buffer, ptr, len);
        out.fill(0);
        pad.buttons.slice(0, 17).forEach((b, i) => out[i] = b.pressed ? 1 : b.value);
        pad.axes.slice(0, 4).forEach((a, i) => out[17 + i] = a);
        return 1;
    }

    // A tap on a rectangle the program gave (`UiRect`) is handled here, inside the browser's
    // event, where it allows fullscreen and the file chooser.
    function tapped(event) {
        const box = canvas.getBoundingClientRect();
        const x = event.clientX - box.left, y = event.clientY - box.top;
        const inside = r => r && r.w > 0 && x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h;
        if (inside(rects[0])) setFullscreen(!fullscreenElement());
        if (inside(rects[1])) choose(".gb,.gbc");
    }

    const imports = {
        sml_now_ms: () => Date.now(),
        sml_storage_next: sizes => {
            if (!queue.length) return 0;
            const sizesView = new Uint32Array(wasm_memory.buffer, sizes, 2);
            sizesView[0] = queue[0].path.length;
            sizesView[1] = queue[0].data ? queue[0].data.length : 0xFFFFFFFF;
            return 1;
        },
        sml_storage_take: (pathPtr, dataPtr) => {
            const item = queue.shift();
            memory().set(item.path, pathPtr);
            if (item.data) memory().set(item.data, dataPtr);
            return item.time;
        },
        sml_storage_put: (pathPtr, pathLen, dataPtr, dataLen, time) => changed(text(pathPtr, pathLen), bytes(dataPtr, dataLen), time),
        sml_storage_delete: (pathPtr, pathLen) => changed(text(pathPtr, pathLen), null, Date.now()),
        sml_text_len: what => {
            const t = textOf(what);
            if (t === null) return -1;
            pending[what] = encoder.encode(t);
            return pending[what].length;
        },
        sml_text_take: (what, out) => {
            memory().set(pending[what] || new Uint8Array(0), out);
            delete pending[what];
        },
        sml_call: (what, value) => {
            switch (what) {
                case 0: { const ctx = audioContext(); return ctx ? ctx.sampleRate : 44100; }
                case 1: { const ctx = audioContext(); return ctx && ctx.state === "running" ? Math.max(0, audioAt - ctx.currentTime) : 0.05; }
                case 2: if (navigator.vibrate) navigator.vibrate(value); return 0;
                case 3: return canFullscreen() ? 1 : 0;
                case 4: return fullscreenElement() ? 1 : 0;
                case 5: setFullscreen(value !== 0); return 0;
                case 6: return ios() && !standalone() && !canFullscreen() ? 1 : 0;
                case 7: return matchMedia("(pointer: fine)").matches ? 1 : 0;
                case 8: return navigator.maxTouchPoints > 0 ? 1 : 0;
                case 9: return coming() ? 1 : 0;
            }
            return 0;
        },
        sml_call_text: (what, ptr, len) => {
            const t = text(ptr, len);
            if (what === 0) choose(t);
            if (what === 1) fetchFont(t);
            return 0;
        },
        sml_audio_push: pushAudio,
        sml_pad_state: padState,
        sml_ui_rect: (what, x, y, w, h) => { rects[what] = { x, y, w, h }; },
    };

    function listen() {
        document.body.appendChild(chooser);
        chooser.addEventListener("change", () => chooser.files.length ? take(chooser.files) : (choosing = false));
        chooser.addEventListener("cancel", () => { choosing = false; });
        window.addEventListener("focus", chooserClosed);
        document.addEventListener("visibilitychange", () => { if (!document.hidden) chooserClosed(); });
        document.addEventListener("drop", e => { if (e.dataTransfer && e.dataTransfer.files.length) take(e.dataTransfer.files); });
        for (const kind of ["pointerdown", "keydown", "touchend"]) window.addEventListener(kind, unlockAudio, { capture: true });
        canvas.addEventListener("pointerdown", e => { if (e.pointerType === "mouse") tapped(e); });
        canvas.addEventListener("pointerup", e => { if (e.pointerType !== "mouse") tapped(e); });
        if (channel) channel.onmessage = e => queue.push({ path: encoder.encode(e.data.path), data: e.data.data, time: e.data.time });
    }

    // Load the site's files, then the program.
    async function start(wasm) {
        try {
            db = await openDb();
            await readAll();
        } catch (e) {
            console.warn("the browser's storage could not be opened; nothing will be kept", e);
        }
        if (navigator.storage && navigator.storage.persist) navigator.storage.persist();
        listen();
        miniquad_add_plugin({ name: "sml26gc", version: 1, register_plugin: importObject => Object.assign(importObject.env, imports) });
        load(wasm);
    }

    return { start };
})();
