// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::{Bot, GameArenaService};
use crate::player::{PlayerData, PlayerRepo, PlayerTuple};
use common_util::ticks::Ticks;
use core_protocol::id::PlayerId;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::sync::Arc;

/// Data stored per bot.
pub struct BotData<G: GameArenaService> {
    player_tuple: Arc<PlayerTuple<G>>,
    /// Only Some during an update cycle.
    action_buffer: Option<G::Command>,
    bot: G::Bot,
}

impl<G: GameArenaService> BotData<G> {
    pub fn new(player_tuple: PlayerTuple<G>) -> Self {
        Self {
            bot: G::Bot::default(),
            player_tuple: Arc::new(player_tuple),
            action_buffer: None,
        }
    }
}

/// Manages the storage and updating of bots.
pub struct BotRepo<G: GameArenaService> {
    bots: Vec<BotData<G>>,
    min_players: usize,
    bot_percent: usize,
}

impl<G: GameArenaService> BotRepo<G> {
    /// Creates a new bot zoo.
    pub fn new(min_players: usize, bot_percent: usize) -> Self {
        Self {
            bots: Vec::with_capacity(min_players.max(bot_percent * 5)),
            min_players,
            bot_percent,
        }
    }

    /// Updates all bots.
    pub fn update(&mut self, counter: Ticks, service: &mut G) {
        {
            let service = &service;

            self.bots
                .par_iter_mut()
                .with_min_len(64)
                .for_each(|bot_data: &mut BotData<G>| {
                    let update = service.get_bot_update(counter, &bot_data.player_tuple);
                    bot_data.action_buffer = bot_data
                        .bot
                        .update(update, bot_data.player_tuple.player.borrow().player_id)
                });
        }

        for bot_data in &mut self.bots {
            if let Some(command) = bot_data.action_buffer.take() {
                service.player_command(command, &bot_data.player_tuple);
            } else {
                // Recycle.
                service.player_left(&bot_data.player_tuple);
                let player_id = bot_data.player_tuple.player.borrow().player_id;
                *bot_data = Self::bot_data(player_id);
                service.player_joined(&bot_data.player_tuple);
            };
        }
    }

    /// Spawns/despawns bots based on number of (real) player clients.
    pub fn update_count(&mut self, service: &mut G, players: &mut PlayerRepo<G>) {
        let count = self
            .min_players
            .max((self.bot_percent * players.real_players_live) / 100);
        self.set_count(count, service, players);
    }

    /// Changes number of bots by spawning/despawning.
    fn set_count(&mut self, count: usize, service: &mut G, players: &mut PlayerRepo<G>) {
        // Give server 3 seconds (50 ticks) to create all testing bots.
        let mut governor = 4.max(self.min_players / 50);

        while count < self.bots.len() && governor > 0 {
            governor -= 1;

            if let Some(last) = self.bots.pop() {
                service.player_left(&last.player_tuple);
            } else {
                break;
            }
        }

        while count > self.bots.len() && governor > 0 {
            governor -= 1;

            if let Some(next_id) = PlayerId::nth_bot(self.bots.len()) {
                debug_assert!(next_id.is_bot());
                let bot = Self::bot_data(next_id);
                // This player will never be forgotten by PlayerRepo.
                players.insert(next_id, Arc::clone(&bot.player_tuple));
                service.player_joined(&bot.player_tuple);
                self.bots.push(bot);
            } else {
                debug_assert!(false, "should not run out of ids");
            }
        }
    }

    fn bot_data(player_id: PlayerId) -> BotData<G> {
        let player_data = PlayerData::new(player_id, None);
        BotData::new(PlayerTuple::new(player_data))
    }
}
