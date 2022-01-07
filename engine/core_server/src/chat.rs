// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::repo::Repo;
use crate::session::Session;
use crate::team::Team;
use core_protocol::dto::MessageDto;
use core_protocol::get_unix_time_now;
use core_protocol::id::{ArenaId, GameId, PlayerId, SessionId};
use core_protocol::name::{PlayerAlias, TeamName};
use log::{debug, error, warn};
use ringbuffer::RingBufferWrite;
use rustrict::{ContextProcessingOptions, ContextRateLimitOptions, trim_whitespace};
use std::fs::OpenOptions;
use std::rc::Rc;

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

    // Client reports the sender of a message.
    pub fn report_player(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
        player_id: PlayerId,
    ) -> bool {
        debug!(
            "report_player(arena={:?}, session={:?}, player={:?})",
            arena_id, session_id, player_id
        );
        let mut reported = false;
        let mut report_session_id = None;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            if let Some(session) = Session::get_mut(&mut arena.sessions, session_id) {
                if let Some(&sess) = self.players.get(&player_id) {
                    if let Some(play) = session.plays.last() {
                        // Throttle abuse of the feature.
                        if play.date_stop.is_none() && play.date_created > get_unix_time_now() + 30
                        {
                            if session.reported.insert(player_id) {
                                report_session_id = Some(sess);
                                reported = true;
                            }
                        }
                    }
                }
            }
            if let Some(report_session_id) = report_session_id {
                if let Some(report_session) =
                    Session::get_mut(&mut arena.sessions, report_session_id)
                {
                    report_session.chat_context.report();
                }
            }
        }

        if !reported {
            warn!(
                "mute_sender(arena={:?}, session={:?}, player={:?}) failed",
                arena_id, session_id, player_id
            );
        }

        reported
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
                    let trimmed = trim_whitespace(&message);
                    if play.date_stop.is_none() && !trimmed.is_empty() && trimmed.len() < 150 {
                        let options = ContextProcessingOptions {
                            rate_limit: if whisper {
                                None
                            } else {
                                Some(ContextRateLimitOptions::default())
                            },
                            ..Default::default()
                        };

                        match session.chat_context.process_with_options(message.to_owned(), &options) {
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
                            Err(reason) => {
                                let warning = MessageDto {
                                    alias: PlayerAlias::new("Server"),
                                    date_sent: get_unix_time_now(),
                                    player_id: None,
                                    team_captain: false,
                                    team_name: None,
                                    text: reason.contextual_string(),
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

    /// Admins can send chats from any alias (although no PlayerId).
    pub fn admin_send_chat(
        &mut self,
        arena_id: ArenaId,
        alias: PlayerAlias,
        message: &str,
    ) -> bool {
        debug!(
            "admin_send_chat(arena={:?}, alias={:?}): {}",
            arena_id, alias, &message
        );
        let mut sent = false;
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            let trimmed = trim_whitespace(message);

            let message = Rc::new(MessageDto {
                alias,
                date_sent: get_unix_time_now(),
                player_id: None,
                team_captain: false,
                team_name: None,
                text: String::from(trimmed),
                whisper: false,
            });

            for (_, session) in Session::iter_mut(&mut arena.sessions) {
                session.inbox.push(Rc::clone(&message));
                sent = true;
            }
            arena.newbie_messages.push(Rc::clone(&message));
        }
        if !sent {
            warn!(
                "send_chat(arena={:?}, alias={:?}) failed: {}",
                arena_id, alias, &message
            );
        }
        sent
    }
}

/// Logs a chat message to a file.
pub fn log_chat(
    chat_log: &str,
    game_id: Option<GameId>,
    whisper: bool,
    ok: bool,
    alias: PlayerAlias,
    message: &str,
) {
    match OpenOptions::new().create(true).append(true).open(chat_log) {
        Ok(file) => {
            let mut wtr = csv::Writer::from_writer(file);
            if let Err(e) = wtr.write_record(&[
                &format!("{}", get_unix_time_now()),
                &game_id
                    .map(|id| format!("{:?}", id))
                    .unwrap_or(String::from("-")),
                &format!("{:?}", whisper),
                &format!("{}", ok),
                alias.as_str(),
                message,
            ]) {
                error!("Error logging chat: {:?}", e);
            }
        }
        Err(e) => error!("Error logging chat: {:?}", e),
    }
}
