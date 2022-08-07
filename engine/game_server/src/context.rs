// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::BotRepo;
use crate::chat::ChatRepo;
use crate::client::ClientRepo;
use crate::game_service::GameArenaService;
use crate::liveboard::LiveboardRepo;
use crate::player::PlayerRepo;
use crate::team::TeamRepo;
use core_protocol::id::ArenaId;
use server_util::rate_limiter::RateLimiterProps;

/// Things that go along with every instance of a [`GameArenaService`].
pub struct Context<G: GameArenaService> {
    pub arena_id: ArenaId,
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
        bots: BotRepo<G>,
        chat_log: Option<String>,
        trace_log: Option<String>,
        client_authenticate: RateLimiterProps,
    ) -> Self {
        Context {
            arena_id,
            clients: ClientRepo::new(trace_log, client_authenticate),
            bots,
            players: PlayerRepo::new(),
            teams: TeamRepo::new(),
            chat: ChatRepo::new(chat_log),
            liveboard: LiveboardRepo::new(),
        }
    }
}
