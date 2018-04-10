extern crate pitch_calc;
extern crate ghakuf;
extern crate cpal;

use pitch_calc::{Step, Hz};

fn main() {
    let beeper = Beeper::new();

    let step_a: Step = Hz(440.0).to_step();
    let step_b: Step = Hz(880.0).to_step();

    println!("start");
    for index in 0..10 {
        println!("  repetition {}", index + 1);
        let duration_ms = 500;
        beeper.beep(step_a, duration_ms);
        beeper.beep(step_b, duration_ms);
    }
    println!("stop");
}

struct Beeper {
    device: cpal::Device,
    format: cpal::Format,
}

impl Beeper {
    fn new() -> Self {
        let device = cpal::default_output_device().expect("no audio device");
        let mut supported_formats_range = device.supported_output_formats()
            .expect("error while querying formats");
        let format = supported_formats_range.next()
            .expect("no supported formats")
            .with_max_sample_rate();

        Self {
            device,
            format,
        }
    }

    fn beep<H: Into<Hz>>(&self, frequency: H, duration_ms: u32) {
        let frequency = frequency.into().0;
        println!("beep {} for {}", frequency, duration_ms);

        let event_loop = std::sync::Arc::new(cpal::EventLoop::new());
        let stream_id = event_loop.build_output_stream(&self.device, &self.format)
            .expect("failed to build output stream");

        event_loop.play_stream(stream_id.clone());

        let format = self.format.clone();

        let event_loop_outer = event_loop.clone();
        std::thread::spawn(move || {
            let sample_rate = format.sample_rate.0 as f32;
            let mut sample_clock = 0f32;

            // Produce a sinusoid of maximum amplitude.
            let mut next_value = || {
                sample_clock = (sample_clock + 1.0) % sample_rate;
                (sample_clock * frequency * 2.0 * std::f32::consts::PI / sample_rate).sin()
            };

            event_loop.run(move |_stream_id, stream_data| {
                match stream_data {
                    cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::U16(mut buffer) } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let value = ((next_value() * 0.5 + 0.5) * std::u16::MAX as f32) as u16;
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    },
                    cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let value = (next_value() * std::i16::MAX as f32) as i16;
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    },
                    cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let value = next_value();
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    },
                    _ => {},
                };
            });
        });

        std::thread::sleep(std::time::Duration::from_millis(duration_ms as u64));
        event_loop_outer.destroy_stream(stream_id);
    }
}
