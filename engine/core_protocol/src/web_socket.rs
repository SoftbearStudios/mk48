// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// Possible websocket protocols.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Display, EnumString)]
pub enum WebSocketProtocol {
    /// Serde bincode.
    Binary,
    /// Serde json.
    /// Make sure this is after Binary so if it's disabled the bincoding of Binary stays the same.
    #[cfg(feature = "json")]
    Json,
}

impl Default for WebSocketProtocol {
    fn default() -> Self {
        Self::Binary
    }
}
