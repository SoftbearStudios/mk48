// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::bot::BotZoo;
use crate::game_service::GameArenaService;
use actix::Recipient;
use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use common_util::ticks::Ticks;
use core_protocol::dto::InvitationDto;
use core_protocol::id::{ArenaId, PlayerId, SessionId, TeamId};
use core_protocol::name::Location;
use server_util::observer::ObserverUpdate;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::time::Instant;

/// The message recipient of an actix actor corresponding to a client.
pub type ClientAddr<G> = Recipient<ObserverUpdate<<G as GameArenaService>::ClientUpdate>>;

pub struct BotData<G: GameArenaService> {
    pub(crate) player_tuple: Arc<PlayerTuple<G>>,
    /// Only Some during an update cycle.
    pub(crate) action_buffer: Option<G::Command>,
    pub(crate) bot: G::Bot,
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

pub struct ClientData<G: GameArenaService> {
    pub(crate) player_tuple: Arc<PlayerTuple<G>>,
    pub(crate) session_id: SessionId,
    pub(crate) limbo_expiry: Option<Instant>,
    pub(crate) last_status: Option<CoreStatus>,
    pub(crate) data: G::ClientData,
}

impl<G: GameArenaService> ClientData<G> {
    pub fn new(session_id: SessionId, player_tuple: PlayerTuple<G>) -> Self {
        Self {
            player_tuple: Arc::new(player_tuple),
            session_id,
            limbo_expiry: None,
            last_status: None,
            data: G::ClientData::default(),
        }
    }
}

/// Player tuple contains the Player and the EntityExtension.
///
/// The Player part is an AtomicRefCell because mutations are manually serialized.
///
/// The EntityExtension part is an UnsafeCell because mutators are forced to hold a mutable reference
/// to a unique structure (such as the player's vehicle).
pub struct PlayerTuple<G: GameArenaService> {
    pub player: AtomicRefCell<PlayerData<G>>,
    pub extension: G::PlayerExtension,
}

impl<G: GameArenaService> PlayerTuple<G> {
    pub fn new(player: PlayerData<G>) -> Self {
        PlayerTuple {
            player: AtomicRefCell::new(player),
            extension: G::PlayerExtension::default(),
        }
    }
}

impl<G: GameArenaService> PlayerTuple<G> {
    /// Borrows the player.
    pub fn borrow_player(&self) -> AtomicRef<PlayerData<G>> {
        self.player.borrow()
    }

    /// Mutably borrows the player.
    pub fn borrow_player_mut(&self) -> AtomicRefMut<PlayerData<G>> {
        self.player.borrow_mut()
    }
}

impl<G: GameArenaService> PartialEq for PlayerTuple<G> {
    fn eq(&self, other: &Self) -> bool {
        self.player.as_ptr() == other.player.as_ptr()
    }
}

impl<G: GameArenaService> Debug for PlayerTuple<G> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.borrow_player().data)
    }
}

// These are intended to be 100% sound (TODO: Explain why).
unsafe impl<G: GameArenaService> Send for PlayerTuple<G> {}
unsafe impl<G: GameArenaService> Sync for PlayerTuple<G> {}

pub struct Context<G: GameArenaService> {
    pub arena_id: Option<ArenaId>,
    /// Wrapping counter.
    pub counter: Ticks,
    pub(crate) clients: HashMap<ClientAddr<G>, ClientData<G>>,
    pub(crate) bots: BotZoo<G>,
}

/// The status of a player from the perspective of the core.
#[derive(Copy, Clone, Debug)]
pub struct CoreStatus {
    pub location: Location,
    pub score: u32,
}

impl Eq for CoreStatus {}
impl PartialEq for CoreStatus {
    fn eq(&self, other: &Self) -> bool {
        const THRESHOLD: f32 = 100.0;
        self.location.distance_squared(other.location) <= THRESHOLD.powi(2)
            && self.score == other.score
    }
}

#[derive(Debug)]
pub struct PlayerData<G: GameArenaService> {
    pub player_id: PlayerId,
    pub team_id: Option<TeamId>,
    pub score: u32,
    pub invitation: Option<InvitationDto>,
    pub data: G::PlayerData,
}

impl<G: GameArenaService> PlayerData<G> {
    pub fn new(player_id: PlayerId, invitation: Option<InvitationDto>) -> Self {
        Self {
            player_id,
            team_id: None,
            score: 0,
            invitation,
            data: G::PlayerData::default(),
        }
    }

    pub fn is_bot(&self) -> bool {
        self.player_id.is_bot()
    }
}
