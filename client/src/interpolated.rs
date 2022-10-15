// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

/// For interpolating fixed time steps to client fps.
/// Adds a delay of 1 fixed update.
pub struct Interpolated {
    previous: Option<f32>,
    last: f32,           // last new value
    last_time: f32,      // time of last new value
    delta: f32,          // change in value
    delta_time: f32,     // time delta took
    max_delta_time: f32, // aka update rate.
}

impl Interpolated {
    pub fn new(max_delta_time: f32) -> Self {
        Self {
            previous: None,
            last: 0.0,
            last_time: 0.0,
            delta: 0.0,
            delta_time: 0.0,
            max_delta_time,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.max_delta_time);
    }

    // Gets the interpolated value from the current value and the time.
    pub fn update(&mut self, current: f32, time: f32) -> f32 {
        // Initialize with current if None.
        // Must pre-borrow mut refs until rustc fixes this.
        let last_mut = &mut self.last;
        let last_time_mut = &mut self.last_time;
        self.previous.get_or_insert_with(|| {
            *last_mut = current;
            *last_time_mut = time;
            current
        });

        // Record deltas.
        if current != self.last {
            let l = self.now(time);
            self.previous = Some(l);
            let last = l;
            self.delta = current - last;
            self.delta_time = (time - self.last_time).min(self.max_delta_time);
            self.last = current;
            self.last_time = time;
        }

        self.now(time)
    }

    // Gets the current value at a time.
    // Panics if previous is None.
    fn now(&self, time: f32) -> f32 {
        let previous = self.previous.unwrap();
        if self.delta_time == 0.0 {
            return previous;
        }

        let per_second = self.delta / self.delta_time;
        let seconds = time - self.last_time;
        let now = previous + per_second * seconds;

        // Don't extrapolate.
        if per_second.is_sign_positive() {
            now.min(self.last)
        } else {
            now.max(self.last)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::interpolated::Interpolated;

    #[test]
    fn test_interpolate() {
        let mut time = 0.0;
        let step = 1.0;
        let sub_step = 0.1;

        let mut interpolated = Interpolated::new(step);

        let mut value = 4.0;
        interpolated.update(value, time);
        time += step;
        value *= 2.0;
        interpolated.update(value, time);

        for _ in 0..12 {
            let i = interpolated.update(value, time);
            println!("part1 t: {}, i: {}", time, i);
            time += sub_step
        }

        time += step;
        value *= 0.5;
        interpolated.update(value, time);

        for _ in 0..12 {
            let i = interpolated.update(value, time);
            println!("part2 t: {}, i: {}", time, i);
            time += sub_step
        }
    }
}
