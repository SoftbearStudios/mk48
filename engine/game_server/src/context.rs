// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::BotRepo;
use crate::chat::ChatRepo;
use crate::client::ClientRepo;
use crate::game_service::GameArenaService;
use crate::liveboard::LiveboardRepo;
use crate::player::PlayerRepo;
use crate::team::TeamRepo;
use common_util::ticks::Ticks;
use core_protocol::id::ArenaId;
use server_util::rate_limiter::RateLimiterProps;

/// Things that go along with every instance of a [`GameArenaService`].
pub struct Context<G: GameArenaService> {
    pub arena_id: ArenaId,
    /// Wrapping counter.
    pub counter: Ticks,
    pub players: PlayerRepo<G>,
    pub(crate) clients: ClientRepo<G>,
    pub(crate) bots: BotRepo<G>,
    pub(crate) chat: ChatRepo<G>,
    pub teams: TeamRepo<G>,
    pub(crate) liveboard: LiveboardRepo<G>,
}

impl<G: GameArenaService> Context<G> {
    pub fn new(
        arena_id: ArenaId,
        min_players: usize,
        chat_log: Option<String>,
        trace_log: Option<String>,
        client_authenticate: RateLimiterProps,
    ) -> Self {
        Context {
            arena_id,
            counter: Ticks::ZERO,
            clients: ClientRepo::new(trace_log, client_authenticate),
            bots: BotRepo::new(min_players, if min_players == 0 { 0 } else { 80 }),
            players: PlayerRepo::new(),
            teams: TeamRepo::new(),
            chat: ChatRepo::new(chat_log),
            liveboard: LiveboardRepo::new(),
        }
    }

    /// Increment tick count by one, wrapping on overflow.
    pub fn count_tick(&mut self) {
        self.counter = self.counter.wrapping_add(Ticks::ONE);
    }
}
