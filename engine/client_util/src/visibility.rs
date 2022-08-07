// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::apply::Apply;

/// Any type of visibility event.
#[derive(Debug)]
pub enum VisibilityEvent {
    Visible(bool),
}

/// The state of the page visibility.
/// TODO: Consider expanding to include whether focused.
pub struct VisibilityState {
    visible: bool,
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self {
            // Be safe if initial visible event lost.
            visible: true,
        }
    }
}

impl Apply<VisibilityEvent> for VisibilityState {
    fn apply(&mut self, event: VisibilityEvent) {
        match event {
            VisibilityEvent::Visible(visible) => {
                self.visible = visible;
            }
        }
    }
}

impl VisibilityState {
    /// The page is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// The page is not visible.
    pub fn is_hidden(&self) -> bool {
        !self.visible
    }
}
