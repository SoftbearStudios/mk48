// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// Possible websocket protocols.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WebSocketProtocol {
    /// Serde bincode.
    Binary,
    /// Serde json.
    Json,
}

impl Default for WebSocketProtocol {
    fn default() -> Self {
        Self::Binary
    }
}
