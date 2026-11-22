//! The browser's capture workers (`CapturePool`), run by the page's script: one Web Worker per
//! spare core, each running `sml26gc_capture.wasm` (`crates/capture-worker`).
use crate::ports::CapturePool;
use host::web::{self, Call, Text, TextCall};

pub struct WebCapture;

impl CapturePool for WebCapture {
    fn start(&self, rom: &[u8], original: &[u8]) -> Option<u32> {
        web::capture_start(rom, original)
    }
    fn idle(&self) -> Option<usize> {
        let n = web::call(Call::CaptureIdle, 0.0);
        (n >= 0.0).then_some(n as usize)
    }
    fn send(&self, job: &str) {
        web::call_text(TextCall::CaptureJob, job);
    }
    fn take(&self) -> Option<String> {
        web::text(Text::Captured)
    }
    fn stop(&self, generation: u32) {
        web::call(Call::CaptureStop, generation as f64);
    }
}
