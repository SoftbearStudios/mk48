// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::context::{BotData, PlayerData, PlayerTuple};
use crate::game_service::{Bot, GameArenaService};
use common_util::ticks::Ticks;
use core_protocol::id::PlayerId;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};

/// Manages the storage and updating of bots.
pub struct BotZoo<G: GameArenaService> {
    bots: Vec<BotData<G>>,
    min_players: usize,
    bot_percent: usize,
}

impl<G: GameArenaService> BotZoo<G> {
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
    pub fn update_count(&mut self, clients: usize, service: &mut G) {
        let count = self.min_players.max((self.bot_percent * clients) / 100);
        self.set_count(count, service);
    }

    /// Changes number of bots by spawning/despawning.
    fn set_count(&mut self, count: usize, service: &mut G) {
        let mut governor = 32;

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
                service.player_joined(&bot.player_tuple);
                self.bots.push(bot);
            } else {
                debug_assert!(false, "should not run out of ids");
            }
        }
    }

    fn bot_data(player_id: PlayerId) -> BotData<G> {
        BotData::new(PlayerTuple::new(PlayerData::new(player_id, None)))
    }
}
