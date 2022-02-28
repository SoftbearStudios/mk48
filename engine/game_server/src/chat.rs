// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::metric::MetricRepo;
use crate::player::PlayerRepo;
use crate::team::TeamRepo;
use core_protocol::dto::MessageDto;
use core_protocol::get_unix_time_now;
use core_protocol::id::PlayerId;
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{ChatRequest, ChatUpdate};
use heapless::HistoryBuffer;
use log::error;
use rustrict::{BlockReason, ContextProcessingOptions, ContextRateLimitOptions};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::marker::PhantomData;
use std::sync::Arc;

/// Component of [`Context`] dedicated to chat.
pub struct ChatRepo<G> {
    /// For new players' chat to start full.
    recent: HistoryBuffer<Arc<MessageDto>, 16>,
    /// Log all chats here.
    log_path: Option<String>,
    _spooky: PhantomData<G>,
}

/// Component of client data encompassing chat information.
#[derive(Debug, Default)]
pub struct ClientChatData {
    /// Chat context for censoring purposes.
    pub(crate) context: rustrict::Context,
    /// Players this client has muted.
    muted: HashSet<PlayerId>,
    /// Messages that need to be sent to the client.
    inbox: HistoryBuffer<Arc<MessageDto>, 16>,
}

impl ClientChatData {
    /// Call when it is reasonable to assume client has forgotten state (and will receive
    /// recent messages anyway).
    pub fn forget_state(&mut self) {
        self.inbox.clear()
    }

    /// Receives a message (unless the sender is muted).
    pub fn receive(&mut self, message: &Arc<MessageDto>) {
        if message
            .player_id
            .map(|p| self.muted.contains(&p))
            .unwrap_or(false)
        {
            // Muted.
            return;
        }
        self.inbox.write(Arc::clone(message));
    }

    /// Gets all messages that need to be sent.
    fn take_inbox(&mut self) -> HistoryBuffer<Arc<MessageDto>, 16> {
        std::mem::take(&mut self.inbox)
    }
}

impl<G: GameArenaService> ChatRepo<G> {
    pub fn new(log_path: Option<String>) -> Self {
        Self {
            recent: HistoryBuffer::new(),
            log_path,
            _spooky: PhantomData,
        }
    }

    /// Indicate a preference to not receive further messages from a given player.
    fn mute_player(
        &mut self,
        req_player_id: PlayerId,
        mute_player_id: PlayerId,
        players: &mut PlayerRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        if req_player_id == mute_player_id {
            return Err("cannot mute self");
        }
        if !players.contains(mute_player_id) {
            return Err("cannot mute nonexistent player");
        }
        let mut req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("nonexistent player")?;
        let req_client = req_player.client_mut().ok_or("only clients can mute")?;
        if req_client.chat.muted.insert(mute_player_id) {
            Ok(ChatUpdate::Muted(mute_player_id))
        } else {
            Err("already muted")
        }
    }

    /// Indicate a preference to receive further messages from a given player.
    fn unmute_player(
        &mut self,
        req_player_id: PlayerId,
        unmute_player_id: PlayerId,
        players: &mut PlayerRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        if req_player_id == unmute_player_id {
            return Err("cannot unmute self");
        }
        let mut req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("nonexistent player")?;
        let req_client = req_player.client_mut().ok_or("only clients can unmute")?;
        if req_client.chat.muted.remove(&unmute_player_id) {
            Ok(ChatUpdate::Unmuted(unmute_player_id))
        } else {
            Err("player wasn't muted")
        }
    }

    /// Send a chat to all players, or one's team (whisper).
    fn send_chat(
        &mut self,
        req_player_id: PlayerId,
        message: String,
        whisper: bool,
        players: &mut PlayerRepo<G>,
        teams: &TeamRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        let mut req_player = players
            .borrow_player_mut(req_player_id)
            .ok_or("nonexistent player")?;
        let team = req_player.team_id().and_then(|t| teams.get(t));

        if !req_player.is_alive() {
            return Err("must be alive to chat");
        }

        if whisper && team.is_none() {
            return Err("no one to whisper to");
        }

        // If the team no longer exists, no members should exist.
        debug_assert_eq!(req_player.team_id().is_some(), team.is_some());

        let options = ContextProcessingOptions {
            rate_limit: if whisper {
                None
            } else {
                Some(ContextRateLimitOptions::default())
            },
            ..Default::default()
        };

        let (result, was_toxic) = if let Some(req_client) = req_player.client_mut() {
            let before = req_client.chat.context.total_inappropriate();

            let result = req_client
                .chat
                .context
                .process_with_options(message.clone(), &options);

            let was_toxic = req_client.chat.context.total_inappropriate() > before;
            metrics.mutate_with(|m| m.toxicity.push(was_toxic), &req_client.metrics);

            (result, was_toxic)
        } else {
            (Ok(message.clone()), false)
        };

        let verdict = match &result {
            Ok(_) if was_toxic => "toxic",
            Ok(_) => "ok",
            Err(BlockReason::Inappropriate(_)) => "inappropriate",
            Err(BlockReason::Unsafe(_)) => "unsafe",
            Err(BlockReason::Repetitious(_)) => "repetitious",
            Err(BlockReason::Spam(_)) => "spam",
            Err(BlockReason::Muted(_)) => "muted",
            Err(BlockReason::Empty) => "empty",
            _ => "???",
        };

        self.log_chat(req_player.alias(), &message, whisper, verdict);

        match result {
            Ok(text) => {
                let message = Arc::new(MessageDto {
                    alias: req_player.alias(),
                    date_sent: get_unix_time_now(),
                    player_id: Some(req_player.player_id),
                    team_captain: team.map(|t| t.is_captain(req_player_id)).unwrap_or(false),
                    team_name: team.map(|t| t.name),
                    text,
                    whisper,
                });

                // We are about to borrow the players to send to them.
                drop(req_player);

                if whisper {
                    if let Some(team) = team {
                        for member in team.members.iter() {
                            if let Some(mut player) = players.borrow_player_mut(member) {
                                if let Some(client) = player.client_mut() {
                                    client.chat.receive(&message)
                                }
                            } else {
                                debug_assert!(false, "team member {:?} doesn't exist", member);
                            }
                        }
                    } else {
                        // Incorrect, but harmless.
                        debug_assert!(false, "should have returned early");
                    }
                } else {
                    self.broadcast_message(message, players);
                }
            }
            Err(reason) => {
                if let Some(req_client) = req_player.client_mut() {
                    let warning = MessageDto {
                        alias: G::authority_alias(),
                        date_sent: get_unix_time_now(),
                        player_id: None,
                        team_captain: false,
                        team_name: None,
                        text: reason.contextual_string(),
                        whisper,
                    };

                    req_client.chat.receive(&Arc::new(warning));
                } else {
                    debug_assert!(false, "non-clients cannot end up here");
                }
            }
        }
        Ok(ChatUpdate::Sent)
    }

    /// Broadcasts a message to all players (including queuing it for those who haven't joined yet).
    pub fn broadcast_message(&mut self, message: Arc<MessageDto>, players: &mut PlayerRepo<G>) {
        for mut player in players.iter_borrow_mut() {
            if let Some(client) = player.client_mut() {
                client.chat.receive(&message);
            }
        }
        self.recent.write(message);
    }

    /// Process any [`ChatRequest`].
    pub(crate) fn handle_chat_request(
        &mut self,
        req_player_id: PlayerId,
        request: ChatRequest,
        players: &mut PlayerRepo<G>,
        teams: &TeamRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        match request {
            ChatRequest::Mute(player_id) => self.mute_player(req_player_id, player_id, players),
            ChatRequest::Unmute(player_id) => self.unmute_player(req_player_id, player_id, players),
            ChatRequest::Send { message, whisper } => {
                self.send_chat(req_player_id, message, whisper, players, teams, metrics)
            }
        }
    }

    /// Back-fills player with recent messages.
    /// Returns true iff successful.
    pub fn initialize_client(&self, player_id: PlayerId, players: &mut PlayerRepo<G>) -> bool {
        let mut player = match players.borrow_player_mut(player_id) {
            Some(player) => player,
            None => return false,
        };
        let client = match player.client_mut() {
            Some(client) => client,
            None => return false,
        };
        for msg in self.recent.oldest_ordered() {
            client.chat.receive(msg);
        }
        true
    }

    /// Gets chat update, consisting of new messages, for a player. Borrows the player mutably.
    pub fn player_delta(player_id: PlayerId, players: &PlayerRepo<G>) -> Option<ChatUpdate> {
        let mut player = players.borrow_player_mut(player_id)?;
        let client = player.client_mut()?;
        if client.chat.inbox.is_empty() {
            None
        } else {
            let messages = client
                .chat
                .take_inbox()
                .oldest_ordered()
                .map(|arc| Arc::clone(arc))
                .collect();
            Some(ChatUpdate::Received(messages))
        }
    }

    /// Logs a chat message to a file (provided that a logging path has been configured).
    pub(crate) fn log_chat(&self, alias: PlayerAlias, message: &str, whisper: bool, verdict: &str) {
        if let Some(log_path) = &self.log_path {
            let ctx = if whisper { "team" } else { "global" };

            match OpenOptions::new().create(true).append(true).open(log_path) {
                Ok(file) => {
                    let mut wtr = csv::Writer::from_writer(file);
                    if let Err(e) = wtr.write_record(&[
                        &format!("{}", get_unix_time_now()),
                        &format!("{:?}", G::GAME_ID),
                        ctx,
                        verdict,
                        alias.as_str(),
                        message,
                    ]) {
                        error!("error logging chat: {:?}", e);
                    }
                }
                Err(e) => error!("error logging chat: {:?}", e),
            }
        }
    }
}
