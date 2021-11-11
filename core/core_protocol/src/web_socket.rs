// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WebSocketFormat {
    Binary,
    Json,
}

impl WebSocketFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Json => "json",
        }
    }
}

impl Default for WebSocketFormat {
    fn default() -> Self {
        Self::Binary
    }
}
