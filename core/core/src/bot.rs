// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::arena::Arena;
use crate::generate_id::generate_id_64;
use crate::repo::Repo;
use crate::session::{Play, Session};
use core_protocol::get_unix_time_now;
use core_protocol::id::{ArenaId, PlayerId, SessionId};
use core_protocol::name::PlayerAlias;
use log::{debug, info};
use rand::Rng;
use std::collections::hash_map::Entry;
use std::collections::HashSet;

lazy_static! {
    static ref BOT_NAMES: Box<[&'static str]> = include_str!("./famous_bots.txt")
        .split('\n')
        .filter(|s| s.len() > 0 && s.len() <= PlayerAlias::capacity())
        .collect();
}

pub fn next_name(excluded_aliases: &HashSet<PlayerAlias>) -> PlayerAlias {
    let mut alias = random_name();
    for _ in 0..5 {
        if excluded_aliases.contains(&alias) {
            break;
        }
        alias = random_name();
    }

    alias
}

/// random_name returns one of several possible bot names.
fn random_name() -> PlayerAlias {
    let s = BOT_NAMES[rand::thread_rng().gen_range(0..BOT_NAMES.len())];
    PlayerAlias::new(s)
}

impl Repo {
    // Assume caller reads newly available bots.
    pub fn read_available_bots(&mut self, arena_id: ArenaId) -> Option<Vec<(PlayerId, SessionId)>> {
        let mut available_bots: Vec<(PlayerId, SessionId)> = vec![];
        if let Some(arena) = Arena::get_mut(&mut self.arenas, &arena_id) {
            // Apply rules
            let mut player_count = 0;
            let mut bot_count = 0;
            for session in arena.sessions.values() {
                if let Some(play) = session.plays.last() {
                    if session.live || play.date_stop.is_none() {
                        if session.bot {
                            bot_count += 1;
                        } else {
                            player_count += 1;
                        }
                    }
                } else if session.bot
                    && get_unix_time_now().saturating_sub(session.date_created) < 1000
                {
                    // Give bots some time to start their session before spawning more.
                    bot_count += 1;
                }
            }

            let bots_wanted = arena
                .rules
                .bot_min
                .max((arena.rules.bot_percent * player_count) / 100);

            let bots_to_add = bots_wanted.saturating_sub(bot_count);

            if bots_to_add > 0 {
                info!("{} bots to add", bots_to_add);
                let mut excluded_aliases = HashSet::new();
                for (session_id, session) in arena.sessions.iter() {
                    if session.bot {
                        excluded_aliases.insert(session.alias);

                        if let Some(play) = session.plays.last() {
                            if play.date_stop.is_some() {
                                available_bots.push((session.player_id, *session_id));
                            }
                        }
                    }
                }
                let create_count = bots_to_add.saturating_sub(available_bots.len() as u32);
                if create_count > 0 {
                    info!("{} bots to create", create_count);
                    for _ in 0..create_count {
                        let alias = next_name(&excluded_aliases);
                        let (player_id, session_id) = loop {
                            let session_id = SessionId(generate_id_64());
                            if let Entry::Vacant(e) = arena.sessions.entry(session_id) {
                                let player_id = Self::create_entity(&mut self.players, session_id);
                                debug!(
                                    "create_bot_session(alias={:?}) => session={:?}, player={:?}",
                                    &alias, session_id, player_id
                                );
                                let bot = true;
                                let date_previous = None;
                                let previous_id = None;
                                let referrer = None;
                                let user_agent_id = None;
                                let mut session = Session::new(
                                    alias,
                                    arena_id,
                                    bot,
                                    date_previous,
                                    arena.game_id,
                                    player_id,
                                    previous_id,
                                    referrer,
                                    arena.server_id,
                                    user_agent_id,
                                );
                                session.plays.push(Play::new());
                                e.insert(session);
                                arena.broadcast_players.added(session_id); // Bot joins the roster.
                                break (player_id, session_id);
                            }
                        };
                        available_bots.push((player_id, session_id));
                    }
                }
            }

            // Don't add too many bots.
            available_bots.truncate(bots_to_add as usize);
        }

        if available_bots.is_empty() {
            None
        } else {
            Some(available_bots)
        }
    }
}
