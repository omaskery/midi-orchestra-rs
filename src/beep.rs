use std::time::Duration;
use pitch_calc::Hz;
use rodio::Source;
use rodio;
use std;

pub struct Beeper {
    endpoint: rodio::Endpoint,
}

impl Beeper {
    pub fn new() -> Self {
        let endpoint = rodio::default_endpoint().unwrap();

        Self {
            endpoint,
        }
    }

    pub fn beep<H: Into<Hz>>(&self, frequency: H, duration: Duration, volume: f32) {
        let frequency = frequency.into().0;
        let source = SquareWave::new(frequency as u32);
        let source = source.amplify(volume).repeat_infinite().take_duration(duration);
        rodio::play_raw(&self.endpoint, source);
    }
}

#[derive(Clone, Debug)]
pub struct SquareWave {
    freq: f32,
    num_sample: usize,
}

impl SquareWave {
    /// The frequency of the sine.
    #[inline]
    pub fn new(freq: u32) -> Self {
        Self {
            freq: freq as f32,
            num_sample: 0,
        }
    }
}

impl Iterator for SquareWave {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        self.num_sample = self.num_sample.wrapping_add(1);

        let value = 2.0 * std::f32::consts::PI * self.freq * self.num_sample as f32 / 48000.0;
        let sin = value.sin();

        Some(if sin >= 0.0 { 1.0 } else { -1.0 })
    }
}

impl Source for SquareWave {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    #[inline]
    fn channels(&self) -> u16 {
        1
    }

    #[inline]
    fn samples_rate(&self) -> u32 {
        48000
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
