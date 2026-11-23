//! Speaker output in the browser: the page plays the samples through Web Audio, each frame's
//! after the one before (`host::web`). Silent until the first touch, click or key, as browsers
//! require.
use crate::ports::AudioOut;

pub struct Audio {
    silent: bool,
}

impl Audio {
    pub fn silent() -> Audio {
        Audio { silent: true }
    }

    pub fn new() -> Audio {
        Audio { silent: false }
    }
}

impl AudioOut for Audio {
    fn sample_rate(&self) -> u32 {
        host::web::call(host::web::Call::SampleRate, 0.0) as u32
    }

    fn push(&self, samples: &[f32], volume: f32) {
        if !self.silent {
            host::web::audio_push(samples, volume);
        }
    }

    fn queued_seconds(&self) -> f32 {
        host::web::call(host::web::Call::QueuedSeconds, 0.0) as f32
    }
}
