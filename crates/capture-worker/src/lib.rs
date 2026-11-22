//! The editor's capture workers in the browser: each Web Worker (`packaging/web/capture-worker.js`)
//! runs this program, which captures one job at a time (`look_capture`). The page's script
//! writes the cartridges and each job line into buffers this program gives it, and reads the
//! result line back.
use look_capture::{done_line, new_baselines, parse_job, Capturer};
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static CAPTURER: RefCell<Option<Capturer>> = const { RefCell::new(None) };
    static BUFFERS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A buffer of `len` bytes for the page to fill: its address. Buffers live until `capture_init`
/// (the cartridges) or the next `capture_run` (a job) takes them.
#[unsafe(no_mangle)]
pub extern "C" fn capture_buffer(len: usize) -> *mut u8 {
    BUFFERS.with(|b| {
        let mut b = b.borrow_mut();
        b.push(vec![0; len]);
        b.last_mut().map_or(std::ptr::null_mut(), |v| v.as_mut_ptr())
    })
}

/// Take the two buffers given last (the project's cartridge, then the base one) and boot the
/// console: 1 when it reached the world map.
#[unsafe(no_mangle)]
pub extern "C" fn capture_init() -> u32 {
    let (original, rom) = BUFFERS.with(|b| {
        let mut b = b.borrow_mut();
        (b.pop().unwrap_or_default(), b.pop().unwrap_or_default())
    });
    let capturer = Capturer::new(Arc::new(rom), Arc::new(original), new_baselines());
    let ready = capturer.is_some();
    CAPTURER.with(|c| *c.borrow_mut() = capturer);
    ready as u32
}

/// Run the job line in the buffer given last; the result line's length (`capture_result`).
#[unsafe(no_mangle)]
pub extern "C" fn capture_run() -> usize {
    let line = BUFFERS.with(|b| b.borrow_mut().pop()).unwrap_or_default();
    let result = parse_job(&String::from_utf8_lossy(&line))
        .and_then(|job| CAPTURER.with(|c| c.borrow().as_ref().map(|c| done_line(&job, &c.run(&job)))))
        .unwrap_or_default();
    RESULT.with(|r| {
        *r.borrow_mut() = result.into_bytes();
        r.borrow().len()
    })
}

/// The address of the last result line.
#[unsafe(no_mangle)]
pub extern "C" fn capture_result() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}
