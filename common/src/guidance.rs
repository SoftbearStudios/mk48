// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::angle::Angle;

use crate::velocity::Velocity;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Guidance {
    pub direction_target: Angle,
    pub velocity_target: Velocity,
}

impl Guidance {
    /// new returns a zero Guidance.
    pub fn new() -> Self {
        Self::default()
    }
}
