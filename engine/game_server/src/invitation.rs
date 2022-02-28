// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::player::{PlayerData, PlayerRepo};
use crate::unwrap_or_return;
use atomic_refcell::AtomicRefMut;
use core_protocol::dto::InvitationDto;
use core_protocol::id::{ArenaId, InvitationId, PlayerId, ServerId};
use core_protocol::rpc::{InvitationRequest, InvitationUpdate};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Invitations, shared by all arenas.
pub struct InvitationRepo<G: GameArenaService> {
    // TODO: Prune.
    invitations: HashMap<InvitationId, Invitation>,
    _spooky: PhantomData<G>,
}

/// For routing invitations.
#[derive(Clone, Debug)]
pub struct Invitation {
    /// Sender arena id.
    pub arena_id: ArenaId,
    /// Sender.
    pub player_id: PlayerId,
}

/// Invitation related data stored in player.
#[derive(Debug)]
pub struct ClientInvitationData {
    /// Incoming invitation accepted by player.
    pub invitation_accepted: Option<InvitationDto>,
    /// Outgoing invitation created by player.
    pub invitation_created: Option<InvitationId>,
}

impl ClientInvitationData {
    pub fn new(invitation_accepted: Option<InvitationDto>) -> Self {
        Self {
            invitation_accepted,
            invitation_created: None,
        }
    }
}

impl<G: GameArenaService> InvitationRepo<G> {
    pub fn new() -> Self {
        Self {
            invitations: HashMap::new(),
            _spooky: PhantomData,
        }
    }

    /// Looks up an invitation by id.
    pub fn get(&self, invitation_id: InvitationId) -> Option<&Invitation> {
        self.invitations.get(&invitation_id)
    }

    /// Returns how many invitations are cached.
    pub fn len(&self) -> usize {
        self.invitations.len()
    }

    /// Forgets any invitation the player created.
    pub(crate) fn forget_player_invitation(&mut self, player: &mut AtomicRefMut<PlayerData<G>>) {
        let client = unwrap_or_return!(player.client_mut());
        if let Some(invitation_id) = client.invitation.invitation_created {
            let removed = self.invitations.remove(&invitation_id);
            debug_assert!(removed.is_some(), "invitation was cleared elsewhere");
            client.invitation.invitation_created = None;
        }
    }

    /// Requests an invitation id (new or recycled).
    fn create_invitation(
        &mut self,
        req_player_id: PlayerId,
        arena_id: ArenaId,
        server_id: Option<ServerId>,
        players: &mut PlayerRepo<G>,
    ) -> Result<InvitationUpdate, &'static str> {
        let mut req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("req player doesn't exist")?;

        let req_client = req_player
            .client_mut()
            .ok_or("only clients can request invitations")?;

        // Silently ignore case of previously created invitation id.
        let invitation_id = if let Some(invitation_id) = req_client.invitation.invitation_created {
            invitation_id
        } else {
            loop {
                let invitation_id = InvitationId::generate(server_id);
                if let Entry::Vacant(entry) = self.invitations.entry(invitation_id) {
                    entry.insert(Invitation {
                        arena_id,
                        player_id: req_player_id,
                    });
                    req_client.invitation.invitation_created = Some(invitation_id);
                    break invitation_id;
                }
            }
        };

        Ok(InvitationUpdate::InvitationCreated(invitation_id))
    }

    pub fn handle_invitation_request(
        &mut self,
        player_id: PlayerId,
        request: InvitationRequest,
        arena_id: ArenaId,
        server_id: Option<ServerId>,
        players: &mut PlayerRepo<G>,
    ) -> Result<InvitationUpdate, &'static str> {
        match request {
            InvitationRequest::CreateInvitation => {
                self.create_invitation(player_id, arena_id, server_id, players)
            }
        }
    }
}
