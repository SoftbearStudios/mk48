// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::observer::ObserverUpdate;
use crate::unused_context::BotData;
use crate::unused_context::Context;
use crate::unused_context::PlayerData;
use crate::unused_game_server2d::GameArenaService;
use actix::Handler;
use common_util::unused_ticks::Ticks;
use core_protocol::rpc::ServerUpdate;
use log::trace;
use std::sync::Arc;
use std::time::Instant;

pub struct Infrastructure<G: GameArenaService> {
    context: Context<G>,
    service: G,
    //ups_monitor: UpsMonitor::new(),
}

impl<G: GameArenaService> Infrastructure<G> {
    pub fn update(&mut self) {
        let now = Instant::now();

        self.counter = self.counter.wrapping_add(Ticks::ONE);

        self.service.update(Ticks::ONE);

        self.context.players.retain(|player_id, player_data| {
            player_data
                .limbo_expiry
                .map(|exp| exp > now)
                .unwrap_or(true)
        });
    }
}

impl<G: GameArenaService> Handler<ObserverUpdate<ServerUpdate>> for Infrastructure<G> {
    type Result = ();

    fn handle(
        &mut self,
        update: ObserverUpdate<ServerUpdate>,
        _: &mut Self::Context,
    ) -> Self::Result {
        trace!("Game server received server update: {:?}", update);
        if let ObserverUpdate::Send { message } = update {
            match message {
                ServerUpdate::ArenaStarted { arena_id } => {
                    self.context.arena_id = Some(arena_id);
                }
                ServerUpdate::ArmageddonStarted { .. } => {}
                ServerUpdate::ArenaStopped => {}
                ServerUpdate::PlayStarted { .. } => {}
                ServerUpdate::PlayStopped => {}
                ServerUpdate::SessionDropped => {}
                ServerUpdate::SessionValid { .. } => {}
                ServerUpdate::StatusSet => {}
                ServerUpdate::MembersChanged { changes } => {
                    for change in changes.iter() {
                        if let Some(player) = self
                            .context
                            .players
                            .get_mut(&change.player_id)
                            .as_deref_mut()
                        {
                            player.team_id = change.team_id;
                        }
                    }
                }
                ServerUpdate::BotReady {
                    session_id,
                    player_id,
                } => {
                    self.context
                        .bots
                        .entry(session_id)
                        .or_insert_with(|| BotData {
                            bot: G::Bot::default(),
                            player_id,
                        })
                        .player_id = player_id;

                    self.context.players.entry(player_id).or_insert_with(|| {
                        Arc::new(PlayerData {
                            team_id: None,
                            last_status: None,
                            limbo_expiry: None,
                            data: PlayerData::default(),
                        })
                    });
                }
            }
        }
    }
}
