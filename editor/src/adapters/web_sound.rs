//! Speaker output in the browser, for listening to a level's music: the page plays the samples
//! through Web Audio (`host::web`), silent until the first click or key, as browsers require.
use crate::ports::SoundOut;

pub struct WebSound;

impl WebSound {
    pub fn new() -> WebSound {
        WebSound
    }
}

impl SoundOut for WebSound {
    fn sample_rate(&self) -> u32 {
        host::web::call(host::web::Call::SampleRate, 0.0) as u32
    }
    fn push(&self, samples: &[f32], volume: f32) {
        host::web::audio_push(samples, volume);
    }
    fn queued_seconds(&self) -> f32 {
        host::web::call(host::web::Call::QueuedSeconds, 0.0) as f32
    }
}
