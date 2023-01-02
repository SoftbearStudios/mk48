#[derive(Clone, Debug)]
pub struct Sensors {
    pub visual: Sensor,
    pub radar: Sensor,
    pub sonar: Sensor,
}

impl Sensors {
    /// any returns if any of the sensors have a non-zero range.
    pub fn any(&self) -> bool {
        self.visual.range != 0.0 || self.radar.range != 0.0 || self.sonar.range != 0.0
    }

    /// max_range returns the maximum range of all sensors.
    pub fn max_range(&self) -> f32 {
        self.visual
            .range
            .max(self.radar.range.max(self.sonar.range))
    }
}

#[derive(Clone, Debug)]
pub struct Sensor {
    pub range: f32,
}
