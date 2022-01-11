// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::observer::ObserverUpdate;
use crate::unused_game_server2d::GameArenaService;
use actix::Recipient;
use common_util::unused_ticks::Ticks;
use core_protocol::id::{ArenaId, PlayerId, SessionId, TeamId};
use core_protocol::name::Location;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct BotData<G: GameArenaService> {
    bot: G::Bot,
    player_id: PlayerId,
}

pub type ClientAddr<G: GameArenaService> = Recipient<ObserverUpdate<G::ClientUpdate>>;

pub struct ClientData<G: GameArenaService> {
    player_id: PlayerId,
    session_id: SessionId,
    data: G::ClientData,
}

pub struct Context<G: GameArenaService> {
    arena_id: Option<ArenaId>,
    /// Wrapping counter.
    counter: Ticks,
    clients: HashMap<ClientAddr<G>, ClientData<G>>,
    bots: HashMap<SessionId, BotData<G>>,
    players: HashMap<PlayerId, Arc<PlayerData<G>>>,
}

/// The status of an player from the perspective of the core.
#[derive(Copy, Clone, Debug)]
pub struct CoreStatus {
    location: Location,
    score: u32,
}

pub struct PlayerData<G: GameArenaService> {
    team_id: Option<TeamId>,
    last_status: Option<CoreStatus>,
    limbo_expiry: Option<Instant>,
    data: G::PlayerData,
}
