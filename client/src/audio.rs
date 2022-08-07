// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game::Mk48Game;
use client_util::audio::AudioPlayer;

engine_macros::include_audio!("/sprites_audio.mp3" "./sprites_audio.json");

impl Mk48Game {
    /// Gets the volume at a distance from the center of the screen.
    pub fn volume_at(distance: f32) -> f32 {
        debug_assert!(distance >= 0.0);
        1.0 / (1.0 + 0.05 * distance)
    }

    /// Plays music if it is not already playing, automatically preempting lower priority music.
    pub fn play_music(audio: Audio, audio_player: &AudioPlayer<Audio>) {
        // Highest to lowest.
        let music_priorities = [Audio::Achievement, Audio::Dodge, Audio::Intense];

        let index = music_priorities
            .iter()
            .position(|&m| m == audio)
            .expect("name must be one of available music");

        for (i, music) in music_priorities.into_iter().enumerate() {
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

        audio_player.play(audio);
    }
}
