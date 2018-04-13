const ONE_SECOND_NS: u64 = 1_000_000_000;

use std::time::Duration;

pub fn seconds_to_duration(seconds: f64) -> Duration {
    nanoseconds_to_duration((seconds * (ONE_SECOND_NS as f64)) as u64)
}

pub fn nanoseconds_to_duration(mut nanoseconds: u64) -> Duration {
    let mut seconds = 0;
    while nanoseconds >= ONE_SECOND_NS {
        nanoseconds -= ONE_SECOND_NS;
        seconds += 1;
    }

    Duration::new(seconds, nanoseconds as u32)
}

pub fn duration_to_nanoseconds(duration: Duration) -> u64 {
    (duration.as_secs() * ONE_SECOND_NS) + duration.subsec_nanos() as u64
}
