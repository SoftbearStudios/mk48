// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use arrayvec::ArrayString;
use core_protocol::name::slice_up_to;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct UserAgent(pub ArrayString<384>);

impl UserAgent {
    pub fn new(str: &str) -> Self {
        Self(slice_up_to(str))
    }
}
