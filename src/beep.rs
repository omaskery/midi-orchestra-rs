use std::time::Duration;
use pitch_calc::Hz;
use rodio::Source;
use rodio;

pub struct Beeper {
    endpoint: rodio::Endpoint,
}

impl Beeper {
    pub fn new() -> Self {
        let endpoint = rodio::default_endpoint().unwrap();

        Self {
            endpoint
        }
    }

    pub fn beep<H: Into<Hz>>(&self, frequency: H, duration: Duration, volume: f32) {
        let frequency = frequency.into().0;
        let source = rodio::source::SineWave::new(frequency as u32);
        let source = source.amplify(volume).repeat_infinite().take_duration(duration);
        rodio::play_raw(&self.endpoint, source);
    }
}
