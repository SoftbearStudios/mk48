// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::{Bot, BotAction, GameArenaService};
use crate::player::{PlayerData, PlayerRepo, PlayerTuple};
use core_protocol::id::PlayerId;
use maybe_parallel_iterator::IntoMaybeParallelRefMutIterator;
use std::sync::Arc;

/// Data stored per bot.
pub struct BotData<G: GameArenaService> {
    player_tuple: Arc<PlayerTuple<G>>,
    /// Only Some during an update cycle.
    action_buffer: BotAction<G::GameRequest>,
    bot: G::Bot,
}

impl<G: GameArenaService> BotData<G> {
    pub fn new(player_tuple: PlayerTuple<G>) -> Self {
        Self {
            bot: G::Bot::default(),
            player_tuple: Arc::new(player_tuple),
            action_buffer: BotAction::None,
        }
    }
}

/// Manages the storage and updating of bots.
pub struct BotRepo<G: GameArenaService> {
    /// Collection of bots, indexed corresponding to player id.
    bots: Vec<BotData<G>>,
    /// Minimum number of bots (always less than or equal to max_bots).
    pub(crate) min_bots: usize,
    /// Maximum number of bots.
    max_bots: usize,
    /// This percent of real players will help determine the target bot quantity.
    bot_percent: usize,
}

impl<G: GameArenaService> BotRepo<G> {
    /// Creates a new bot zoo.
    pub fn new(min_bots: usize, max_bots: usize, bot_percent: usize) -> Self {
        let min_bots = min_bots.min(max_bots);
        Self {
            bots: Vec::with_capacity(min_bots),
            min_bots,
            max_bots,
            bot_percent,
        }
    }

    pub fn new_from_options(
        min_bots: Option<usize>,
        max_bots: Option<usize>,
        bot_percent: Option<usize>,
    ) -> Self {
        Self::new(
            min_bots.unwrap_or(G::Bot::DEFAULT_MIN_BOTS),
            max_bots.unwrap_or(G::Bot::DEFAULT_MAX_BOTS),
            bot_percent.unwrap_or(G::Bot::DEFAULT_BOT_PERCENT),
        )
    }

    /// Updates all bots.
    pub fn update(&mut self, service: &G, players: &PlayerRepo<G>) {
        self.bots
            .maybe_par_iter_mut()
            .with_min_sequential(64)
            .for_each(|bot_data: &mut BotData<G>| {
                let update = G::Bot::get_input(service, &bot_data.player_tuple, &players);
                bot_data.action_buffer = bot_data.bot.update(
                    update,
                    bot_data.player_tuple.player.borrow().player_id,
                    players,
                )
            });
    }

    /// Call after `GameService::post_update` to avoid sending commands between `GameService::tick` and it.
    pub fn post_update(&mut self, service: &mut G, players: &PlayerRepo<G>) {
        for bot_data in &mut self.bots {
            match std::mem::take(&mut bot_data.action_buffer) {
                BotAction::Some(command) => {
                    let _ = service.player_command(command, &bot_data.player_tuple, players);
                }
                BotAction::None => {}
                BotAction::Quit => {
                    // Recycle.
                    service.player_left(&bot_data.player_tuple, players);
                    let player_id = bot_data.player_tuple.player.borrow().player_id;
                    *bot_data = Self::bot_data(player_id);
                    service.player_joined(&bot_data.player_tuple, players);
                }
            };
        }
    }

    /// Spawns/despawns bots based on number of (real) player clients.
    pub fn update_count(&mut self, service: &mut G, players: &mut PlayerRepo<G>) {
        let count = (self.bot_percent * players.real_players_live / 100)
            .clamp(self.min_bots, self.max_bots);
        self.set_count(count, service, players);
    }

    /// Changes number of bots by spawning/despawning.
    fn set_count(&mut self, count: usize, service: &mut G, players: &mut PlayerRepo<G>) {
        // Give server 3 seconds (50 ticks) to create all testing bots.
        let mut governor = 4.max(self.min_bots / 50);

        while count < self.bots.len() && governor > 0 {
            governor -= 1;

            if let Some(last) = self.bots.pop() {
                service.player_left(&last.player_tuple, &*players);
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
                service.player_joined(&bot.player_tuple, &*players);
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
