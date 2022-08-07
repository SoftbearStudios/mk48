// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::infrastructure::Infrastructure;
use actix::{Handler, Message};
use core_protocol::rpc::StatusResponse;
use server_util::health::Health;
use std::time::{Duration, Instant};

/// Manages updating and reporting of server status.
pub struct StatusRepo {
    pub(crate) health: Health,
    uptime: Instant,
    /// Possibly overridden.
    pub(crate) client_hash: u64,
    /// Before being overridden.
    pub(crate) original_client_hash: u64,
}

impl StatusRepo {
    pub fn new(client_hash: u64) -> Self {
        Self {
            health: Health::default(),
            uptime: Instant::now(),
            client_hash,
            original_client_hash: client_hash,
        }
    }

    pub fn uptime(&self) -> Duration {
        self.uptime.elapsed()
    }
}

/// Asks the server if it and the underlying hardware and OS are healthy.
#[derive(Message)]
#[rtype(result = "StatusResponse")]
pub struct StatusRequest;

/// Reports whether infrastructure is healthy (hardware and actor are running properly).
impl<G: GameArenaService> Handler<StatusRequest> for Infrastructure<G> {
    type Result = StatusResponse;

    fn handle(&mut self, _request: StatusRequest, _: &mut Self::Context) -> Self::Result {
        StatusResponse {
            healthy: self.status.health.healthy(),
            region_id: self.region_id,
            redirect_server_id: self.admin.redirect_server_id_preference,
            client_hash: Some(self.status.client_hash),
            // TODO: In the future, this will sum players for all arenas.
            player_count: Some(self.context_service.context.players.real_players_live as u32),
            dying_server_ids: self
                .system
                .as_ref()
                .map(|s| {
                    s.servers
                        .iter()
                        .filter_map(|(&server_id, server)| {
                            if server.is_dying(self.region_id) {
                                Some(server_id)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        }
    }
}
