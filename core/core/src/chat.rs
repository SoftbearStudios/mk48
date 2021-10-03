// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::repo::Repo;
use crate::session::Session;
use crate::team::Team;
use core_protocol::dto::MessageDto;
use core_protocol::id::{ArenaId, PlayerId, SessionId};
use core_protocol::name::{trim_spaces, TeamName};
use core_protocol::{get_unix_time_now, UnixTime};
use log::debug;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferExt, RingBufferWrite};
use rustrict::{Censor, Type};
use std::rc::Rc;

pub struct ChatHistory {
    /// Total message count, faded out over time.
    total: f32,

    /// Inappropriate message count, faded out over time.
    inappropriate: f32,

    /// Content based filtering.
    recent_lengths: ConstGenericRingBuffer<u8, 8>,

    /// Time last faded out in milliseconds.
    date_updated: UnixTime,
}

impl ChatHistory {
    pub fn new() -> Self {
        Self {
            date_updated: 0,
            inappropriate: 0.0,
            recent_lengths: ConstGenericRingBuffer::new(),
            total: 0.0,
        }
    }

    /// Returns censored text and whether to block it entirely.
    pub fn update(&mut self, message: &str, whisper: bool) -> (String, bool) {
        let (censored, analysis) = Censor::from_str(message).censor_and_analyze();

        self.total += 1.0;
        let inappropriate = analysis.is(Type::INAPPROPRIATE);
        let severely_inappropriate = analysis.is(Type::INAPPROPRIATE & Type::SEVERE);

        if inappropriate {
            self.inappropriate += 1.0;
        }

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
            let fade_rate = if self.inappropriate > 5.0 && inappropriate_fraction > 0.5 {
                0.999999 // days
            } else if self.inappropriate > 4.0 && inappropriate_fraction > 0.4 {
                0.99999 // hours
            } else if self.inappropriate > 3.0 && inappropriate_fraction > 0.3 {
                0.9999 // minutes
            } else if inappropriate_fraction > 0.2 {
                0.999
            } else if inappropriate_fraction > 0.1 {
                0.99
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
        let any_spam = frequency_spam || inappropriate_spam || repetition_spam;

        let block = severely_inappropriate || any_spam && !whisper;

        (censored, !block)
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

        return muted;
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
                            team_name = Some(existing_team_name.clone());
                        }
                    }
                    let trimmed = trim_spaces(&message);
                    if play.date_stop.is_none() && trimmed.len() > 0 && trimmed.len() < 150 {
                        let (text, allow) = session.chat_history.update(trimmed, whisper);
                        if allow {
                            let message = MessageDto {
                                alias: session.alias.clone(),
                                date_sent: get_unix_time_now(),
                                player_id: session.player_id,
                                team_captain: play.team_captain,
                                team_name,
                                text,
                                whisper,
                            };
                            maybe_message_tuple = Some((play.team_id, Rc::new(message)));
                        }
                    }
                }
            } // sessions.get_mut

            if let Some((maybe_team_id, message)) = maybe_message_tuple {
                let player_id = message.player_id;
                if whisper {
                    if let Some(whisper_team_id) = maybe_team_id {
                        for (_, session) in Session::iter_mut(&mut arena.sessions) {
                            if session.muted.contains(&message.player_id) {
                                continue;
                            }
                            if let Some(play) = session.plays.last_mut() {
                                if let Some(team_id) = play.team_id {
                                    if team_id == whisper_team_id
                                        && !session.muted.contains(&message.player_id)
                                    {
                                        session.inbox.push(Rc::clone(&message));
                                        if sent.is_none() {
                                            sent = Some(player_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    for (_, session) in Session::iter_mut(&mut arena.sessions) {
                        if !session.muted.contains(&message.player_id) {
                            session.inbox.push(Rc::clone(&message));
                        }
                    }
                    arena.newbie_messages.push(Rc::clone(&message));
                    sent = Some(player_id);
                }
            }
        }
        if sent.is_none() {
            debug!("message not sent");
        }
        return sent;
    }
}
