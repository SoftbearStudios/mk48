// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::repo::Repo;
use crate::session::Session;
use crate::team::Team;
use core_protocol::dto::MessageDto;
use core_protocol::id::{ArenaId, PlayerId, SessionId};
use core_protocol::metrics::RatioMetric;
use core_protocol::name::{trim_spaces, PlayerAlias, TeamName};
use core_protocol::{get_unix_time_now, UnixTime};
use log::{debug, warn};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};
use rustrict::{Censor, Type};
use std::rc::Rc;

#[derive(Default)]
pub struct ChatHistory {
    /// Total message count, faded out over time.
    total: f32,

    /// Inappropriate message count, faded out over time.
    inappropriate: f32,

    /// Content based filtering.
    recent_lengths: ConstGenericRingBuffer<u8, 8>,

    /// Time last faded out in milliseconds.
    date_updated: UnixTime,

    /// Ratio of inappropriate messages to all messages.
    pub toxicity: RatioMetric,
}

impl ChatHistory {
    /// Returns censored text and whether to block it entirely.
    pub fn update(&mut self, message: &str, whisper: bool) -> Result<String, &'static str> {
        let threshold = if whisper {
            // Allow moderately mean words and spam.
            Type::INAPPROPRIATE
        } else {
            // Don't allow moderately mean words and spam.
            Type::INAPPROPRIATE | (Type::MEAN & Type::MODERATE) | (Type::SPAM & Type::MODERATE)
        };

        let (censored, analysis) = Censor::from_str(message)
            .with_censor_threshold(threshold)
            .with_censor_first_character_threshold(if self.inappropriate >= 0.8 {
                threshold
            } else {
                Type::OFFENSIVE & Type::SEVERE
            })
            .censor_and_analyze();

        self.total += 1.0;

        let inappropriate = analysis.is(threshold);
        let severely_inappropriate = analysis.is(Type::INAPPROPRIATE & Type::SEVERE);

        if inappropriate {
            if analysis.is(Type::SEVERE) {
                self.inappropriate += 1.0;
            } else if analysis.is(Type::MODERATE) {
                self.inappropriate += 0.8;
            } else {
                self.inappropriate += 0.6;
            }
        }

        self.toxicity.push(inappropriate);

        let inappropriate_fraction = self.inappropriate / self.total;

        // Length of message capped at 255
        let n = message.len().min(u8::MAX as usize) as u8;

        self.recent_lengths.push(n);

        let average_length = self.recent_lengths.iter().map(|x| *x as f32).sum::<f32>()
            / self.recent_lengths.len() as f32;

        // Deviation of this comment
        let mut length_specific_deviation = n as i32 - average_length as i32;
        if length_specific_deviation < 0 {
            length_specific_deviation = -length_specific_deviation
        }

        let length_standard_deviation: f32 = self
            .recent_lengths
            .iter()
            .map(|x| (average_length - *x as f32).powi(2))
            .sum::<f32>()
            / self.recent_lengths.len() as f32;

        // Count whole number of seconds since last update
        let now = get_unix_time_now();
        let seconds = ((now - self.date_updated) / 1000) as f32;

        if self.date_updated == 0 {
            self.date_updated = now;
        } else if seconds > 0.0 {
            let fade_rate = if self.inappropriate > 3.0 && inappropriate_fraction > 0.3 {
                0.999
            } else if self.inappropriate > 2.0 && inappropriate_fraction > 0.2 {
                0.99
            } else if inappropriate_fraction > 0.1 {
                0.98
            } else {
                0.95
            } as f32;

            let fade = fade_rate.powf(seconds);

            // Fade in equal proportions to not distort inappropriate_fraction
            self.total *= fade;
            self.inappropriate *= fade;

            self.date_updated = now;
        }

        let repetition_threshold_total = 3;

        let frequency_spam = self.total >= 7.0;
        let inappropriate_spam = self.inappropriate > 2.0 && inappropriate_fraction > 0.20;
        let repetition_spam = self.total as i32 > repetition_threshold_total
            && length_standard_deviation < 3.0
            && length_specific_deviation < 3;

        if severely_inappropriate {
            Err("Message held for severe profanity")
        } else if inappropriate_spam {
            Err("You have been temporarily muted due to profanity/spam")
        } else if (frequency_spam || repetition_spam) && !whisper {
            Err("You have been temporarily muted due to excessive frequency")
        } else {
            Ok(censored)
        }
    }
}

impl Repo {
    // Client un/mutes sender in chat messages.
    pub fn mute_sender(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        enable: bool,
        player_id: PlayerId,
    ) -> bool {
        debug!(
            "mute_sender(arena={:?}, session={:?}, enable={:?}, player={:?})",
            arena_id, session_id, enable, player_id
        );
        let mut muted = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if self.players.contains_key(&player_id) {
                    session.muted.insert(player_id);
                    muted = true;
                }
            }
        }

        if !muted {
            warn!(
                "mute_sender(arena={:?}, session={:?}, enable={:?}, player={:?}) failed",
                arena_id, session_id, enable, player_id
            );
        }

        muted
    }

    // Client is chatty.
    pub fn send_chat(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        message: String,
        whisper: bool,
    ) -> Option<PlayerId> {
        debug!(
            "send_chat(arena={:?}, session={:?}): {}",
            arena_id, session_id, &message
        );
        let mut sent = None;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            let mut maybe_message_tuple = None;
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if let Some(play) = session.plays.last() {
                    let mut team_name: Option<TeamName> = None;
                    if let Some(team_id) = play.team_id {
                        if let Some(Team {
                            team_name: existing_team_name,
                            ..
                        }) = arena.teams.get(&team_id)
                        {
                            team_name = Some(*existing_team_name);
                        }
                    }
                    let trimmed = trim_spaces(&message);
                    if play.date_stop.is_none() && !trimmed.is_empty() && trimmed.len() < 150 {
                        match session.chat_history.update(trimmed, whisper) {
                            Ok(text) => {
                                let message = MessageDto {
                                    alias: session.alias,
                                    date_sent: get_unix_time_now(),
                                    player_id: Some(session.player_id),
                                    team_captain: play.team_captain,
                                    team_name,
                                    text,
                                    whisper,
                                };
                                maybe_message_tuple = Some((play.team_id, Rc::new(message)));
                            }
                            Err(text) => {
                                let warning = MessageDto {
                                    alias: PlayerAlias::new("Server"),
                                    date_sent: get_unix_time_now(),
                                    player_id: None,
                                    team_captain: false,
                                    team_name: None,
                                    text: String::from(text),
                                    whisper,
                                };

                                // Send warning to sending player only.
                                session.inbox.push(Rc::new(warning));
                            }
                        }
                    }
                }
            } // sessions.get_mut

            if let Some((maybe_team_id, message)) = maybe_message_tuple {
                if whisper {
                    if let Some(whisper_team_id) = maybe_team_id {
                        for (_, session) in Session::iter_mut(&mut arena.sessions) {
                            // Must be live to receive whisper, otherwise your team affilation isn't real.
                            if !session.live
                                || message
                                    .player_id
                                    .map(|id| session.muted.contains(&id))
                                    .unwrap_or(false)
                            {
                                continue;
                            }
                            if let Some(play) = session.plays.last_mut() {
                                if play.team_id == Some(whisper_team_id) {
                                    session.inbox.push(Rc::clone(&message));
                                    sent = Some(session.player_id);
                                }
                            }
                        }
                    }
                } else {
                    for (_, session) in Session::iter_mut(&mut arena.sessions) {
                        if !message
                            .player_id
                            .map(|id| session.muted.contains(&id))
                            .unwrap_or(false)
                        {
                            session.inbox.push(Rc::clone(&message));
                            sent = Some(session.player_id);
                        }
                    }
                    arena.newbie_messages.push(Rc::clone(&message));
                }
            }
        }
        if sent.is_none() {
            warn!(
                "send_chat(arena={:?}, session={:?}) failed: {}",
                arena_id, session_id, &message
            );
        }
        sent
    }
}
