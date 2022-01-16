// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use client_util::audio::AudioLayer;

impl Mk48Game {
    /// Gets the volume at a distance from the center of the screen.
    pub fn volume_at(distance: f32) -> f32 {
        debug_assert!(distance >= 0.0);
        1.0 / (1.0 + 0.05 * distance)
    }

    /// Plays music if it is not already playing, automatically preempting lower priority music.
    pub fn play_music(name: &'static str, audio_player: &AudioLayer) {
        // Highest to lowest.
        let music_priorities = ["achievement", "dodge", "intense"];

        let index = music_priorities
            .iter()
            .position(|&m| m == name)
            .expect("name must be one of available music");

        for (i, music) in music_priorities.iter().enumerate() {
            if audio_player.is_playing(music) {
                if i <= index {
                    // Preempted by higher priority music, or already playing.
                    return;
                } else {
                    // Preempt lower priority music.
                    audio_player.stop_playing(music);
                }
            }
        }

        audio_player.play(name);
    }
}
