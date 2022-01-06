// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::repo::Repo;
use core_protocol::id::*;
use log::debug;
use std::collections::hash_map::Entry;

#[derive(Clone, Debug)]
pub struct Invitation {
    pub arena_id: ArenaId,
    pub player_id: PlayerId,
}

impl Invitation {
    fn new(arena_id: ArenaId, player_id: PlayerId) -> Self {
        Self {
            arena_id,
            player_id,
        }
    }
}

impl Repo {
    /// Creates an invitation providing the specified session is the captain of a team.
    pub fn create_invitation(
        &mut self,
        arena_id: ArenaId,
        session_id: SessionId,
    ) -> Option<InvitationId> {
        if let Some(arena) = self.arenas.get_mut(&arena_id) {
            if let Some(session) = arena.sessions.get_mut(&session_id) {
                // Purposefully omit check for whether is live, to allow creating an invitation
                // before starting the first play (i.e. on the splash screen).
                if session.date_terminated.is_none() {
                    if let Some(invitation_id) = session.invitation_id {
                        // There are two possible behaviors:

                        // Remove the old one (to avoid storing infinite numbers of invitations).
                        // self.invitations.remove(&invitation_id);

                        // Don't create a new one.
                        return Some(invitation_id);
                    }
                    let invitation = Invitation::new(arena_id, session.player_id);
                    loop {
                        let invitation_id = InvitationId::generate(arena.server_id);
                        if let Entry::Vacant(entry) = self.invitations.entry(invitation_id) {
                            entry.insert(invitation);
                            session.invitation_id = Some(invitation_id);
                            return Some(invitation_id);
                        }
                    }
                }
            }
        }
        None
    }

    /// Prunes any invitations from non-live sessions.
    pub fn prune_invitations(&mut self) {
        // Workaround for using self in closure (borrow checker).
        let players = &mut self.players;
        let arenas = &mut self.arenas;
        self.invitations.retain(|invitation_id, invitation| {
            if let Some(session_id) = players.get(&invitation.player_id) {
                if let Some(arena) = arenas.get(&invitation.arena_id) {
                    if let Some(session) = arena.sessions.get(session_id) {
                        return session.live || session.plays.is_empty();
                    }
                }
            }
            debug!("pruning invitation {:?}", invitation_id);
            false
        });
    }
}
