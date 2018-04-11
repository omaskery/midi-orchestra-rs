use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};
use cpal::{EventLoop, StreamId};
use std::thread::spawn;
use pitch_calc::Hz;
use std::sync::Arc;
use cpal;
use std;

#[derive(Copy, Clone, Debug)]
struct Beep {
    frequency: f32,
    deadline: Instant,
}

pub struct Beeper {
    event_loop: Arc<EventLoop>,
    stream_id: StreamId,
    tx_beep: Sender<Beep>,
}

impl Drop for Beeper {
    fn drop(&mut self) {
        let stream_id = self.stream_id.clone();
        self.event_loop.destroy_stream(stream_id);
    }
}

impl Beeper {
    pub fn new() -> Self {
        let device = cpal::default_output_device().expect("no audio device");
        let mut supported_formats_range = device.supported_output_formats()
            .expect("error while querying formats");
        let format = supported_formats_range.next()
            .expect("no supported formats")
            .with_max_sample_rate();

        let event_loop = Arc::new(cpal::EventLoop::new());
        let stream_id = event_loop.build_output_stream(&device, &format)
            .expect("failed to build output stream");

        println!("format: {:#?}", format);

        event_loop.play_stream(stream_id.clone());

        let (tx_beep, rx_beep) = channel();

        let event_loop_inner = event_loop.clone();
        spawn(move || {
            let sample_rate = format.sample_rate.0 as f32;
            let mut sample_clock = 0f32;
            let mut current_beep: Option<Beep> = None;

            let mut next_value = |frequency: f32| {
                sample_clock = (sample_clock + 1.0) % sample_rate;
                (sample_clock * frequency * 2.0 * std::f32::consts::PI / sample_rate).sin()
            };

            event_loop_inner.run(move |_stream_id, stream_data| {
                let now = Instant::now();

                match rx_beep.try_recv() {
                    Ok(beep) => {
                        current_beep = Some(beep);
                    },
                    _ => {},
                }

                current_beep = match current_beep {
                    Some(beep) if now <= beep.deadline => Some(beep),
                    _ => None,
                };

                let mut value_calc = || {
                    if let Some(beep) = current_beep {
                        next_value(beep.frequency)
                    } else {
                        0.0
                    }
                };

                match stream_data {
                    cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::U16(mut buffer) } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let value = ((value_calc() * 0.5 + 0.5) * std::u16::MAX as f32) as u16;
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    },
                    cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let value = (value_calc() * std::i16::MAX as f32) as i16;
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    },
                    cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                        for sample in buffer.chunks_mut(format.channels as usize) {
                            let value = value_calc();
                            for out in sample.iter_mut() {
                                *out = value;
                            }
                        }
                    },
                    _ => {},
                };
            });
        });

        Self {
            event_loop,
            stream_id,
            tx_beep,
        }
    }

    pub fn beep<H: Into<Hz>>(&self, frequency: H, duration: Duration) {
        let deadline = Instant::now() + duration;
        self.beep_until(frequency, deadline);
    }

    pub fn beep_until<H: Into<Hz>>(&self, frequency: H, deadline: Instant) {
        let frequency = frequency.into().0;

        self.tx_beep.send(Beep {
            frequency,
            deadline,
        }).unwrap();
    }
}
