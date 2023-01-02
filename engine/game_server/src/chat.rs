// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::game_service::GameArenaService;
use crate::metric::MetricRepo;
use crate::player::PlayerRepo;
use crate::team::TeamRepo;
use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use core_protocol::dto::MessageDto;
use core_protocol::get_unix_time_now;
use core_protocol::id::PlayerId;
use core_protocol::name::PlayerAlias;
use core_protocol::rpc::{ChatRequest, ChatUpdate};
use heapless::HistoryBuffer;
use log::error;
use rustrict::{BlockReason, ContextProcessingOptions, ContextRateLimitOptions};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs::OpenOptions;
use std::io::Write;
use std::marker::PhantomData;
use std::net::IpAddr;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Component of [`Context`] dedicated to chat.
pub struct ChatRepo<G> {
    /// For new players' chat to start full.
    recent: HistoryBuffer<Arc<MessageDto>, 16>,
    /// Safe mode (profanity filter setting) is on until this time.
    safe_mode_until: Option<Instant>,
    /// Slow mode (more aggressive rate limits for all players) is on until this time.
    slow_mode_until: Option<Instant>,
    emoji_replacer: AhoCorasick<u32>,
    /// Log all chats here.
    log_path: Option<Arc<str>>,
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

engine_macros::include_emoji!();

impl<G: GameArenaService> ChatRepo<G> {
    pub fn new(log_path: Option<String>) -> Self {
        let emoji_replacer = AhoCorasickBuilder::new()
            .dfa(true)
            .build_with_size(EMOJI_FIND)
            .unwrap();

        Self {
            recent: HistoryBuffer::new(),
            safe_mode_until: None,
            slow_mode_until: None,
            emoji_replacer,
            log_path: log_path.map(Into::into),
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

    /// Clamps minutes to a day, and then returns an instant in the future (if overflow occurs, returns old instant).
    fn minutes_to_instant(minutes: u32, old: Option<Instant>) -> Option<Instant> {
        let new = Instant::now().checked_add(Duration::from_secs(minutes as u64 * 60));
        new.or(old)
    }

    fn restrict_player(
        &mut self,
        req_player_id: PlayerId,
        restrict_player_id: PlayerId,
        minutes: u32,
        players: &PlayerRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        if req_player_id == restrict_player_id {
            return Err("cannot restrict self");
        }
        let req_player = players
            .borrow_player(req_player_id)
            .ok_or("nonexistent player")?;
        let req_client = req_player.client().ok_or("not a real player")?;
        if !req_client.moderator {
            return Err("permission denied");
        }
        let mut restrict_player = players
            .borrow_player_mut(restrict_player_id)
            .ok_or("nonexistent player")?;
        let restrict_client = restrict_player.client_mut().ok_or("not a real player")?;
        let minutes = minutes.min(1440);
        if let Some(restrict_until) =
            Self::minutes_to_instant(minutes, restrict_client.chat.context.restricted_until())
        {
            restrict_client.chat.context.restrict_until(restrict_until);
            Ok(ChatUpdate::PlayerRestricted {
                player_id: restrict_player_id,
                minutes,
            })
        } else {
            Err("overflow")
        }
    }

    fn set_safe_mode(
        &mut self,
        req_player_id: PlayerId,
        minutes: u32,
        players: &PlayerRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        let req_player = players
            .borrow_player(req_player_id)
            .ok_or("nonexistent player")?;
        let req_client = req_player.client().ok_or("not a real player")?;
        if !req_client.moderator {
            return Err("permission denied");
        }
        let clamped = minutes.min(60);
        self.safe_mode_until = Self::minutes_to_instant(clamped, None);
        Ok(ChatUpdate::SafeModeSet(clamped))
    }

    fn set_slow_mode(
        &mut self,
        req_player_id: PlayerId,
        minutes: u32,
        players: &PlayerRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        let req_player = players
            .borrow_player(req_player_id)
            .ok_or("nonexistent player")?;
        let req_client = req_player.client().ok_or("not a real player")?;
        if !req_client.moderator {
            return Err("permission denied");
        }
        let clamped = minutes.min(120);
        self.slow_mode_until = Self::minutes_to_instant(clamped, None);
        Ok(ChatUpdate::SlowModeSet(clamped))
    }

    /// Send a chat to all players, or one's team (whisper).
    fn send_chat(
        &mut self,
        req_player_id: PlayerId,
        message: String,
        whisper: bool,
        service: &mut G,
        players: &mut PlayerRepo<G>,
        teams: &TeamRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        if let Some(text) = self.try_execute_command(req_player_id, &message, service, players) {
            if let Some(mut req_player) = players.borrow_player_mut(req_player_id) {
                let alias = req_player.alias();
                if let Some(req_client) = req_player.client_mut() {
                    self.log_chat(req_client.ip_address, alias, &message, whisper, "executed");
                    let message = MessageDto {
                        alias: G::authority_alias(),
                        date_sent: get_unix_time_now(),
                        player_id: None,
                        team_captain: false,
                        team_name: None,
                        text,
                        whisper,
                    };
                    req_client.chat.receive(&Arc::new(message));
                } else {
                    debug_assert!(false, "bot issued command");
                }
            } else {
                debug_assert!(false, "nonexistent player issued command");
            }
            return Ok(ChatUpdate::Sent);
        }

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

        let result = if let Some(req_client) = req_player.client_mut() {
            let options = ContextProcessingOptions {
                character_limit: NonZeroUsize::new(150),
                safe_mode_until: self.safe_mode_until.filter(|_| !req_client.moderator),
                rate_limit: if whisper {
                    None
                } else {
                    Some(
                        if self
                            .slow_mode_until
                            .map(|t| !req_client.moderator && t > Instant::now())
                            .unwrap_or(false)
                        {
                            ContextRateLimitOptions::slow_mode()
                        } else {
                            ContextRateLimitOptions::default()
                        },
                    )
                },
                ..Default::default()
            };

            // Replace :smile: with 😄 (and others)
            let message =
                if message.len() <= 500 && message.bytes().filter(|b| *b == b':').count() >= 2 {
                    self.emoji_replacer.replace_all(&message, EMOJI_REPLACE)
                } else {
                    message
                };

            let before = req_client.chat.context.total_inappropriate();

            let result = req_client
                .chat
                .context
                .process_with_options(message.clone(), &options);

            let was_toxic = req_client.chat.context.total_inappropriate() > before;
            metrics.mutate_with(|m| m.toxicity.push(was_toxic), &req_client.metrics);

            let verdict = match &result {
                Ok(_) if was_toxic => "toxic",
                Ok(_) => "ok",
                Err(BlockReason::Inappropriate(_)) => "inappropriate",
                Err(BlockReason::Unsafe { .. }) => "unsafe",
                Err(BlockReason::Repetitious(_)) => "repetitious",
                Err(BlockReason::Spam(_)) => "spam",
                Err(BlockReason::Muted(_)) => "muted",
                Err(BlockReason::Empty) => "empty",
                _ => "???",
            };

            self.log_chat(
                req_client.ip_address,
                req_player.alias(),
                &message,
                whisper,
                verdict,
            );

            result
        } else {
            Ok(message)
        };

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
        service: &mut G,
        players: &mut PlayerRepo<G>,
        teams: &TeamRepo<G>,
        metrics: &mut MetricRepo<G>,
    ) -> Result<ChatUpdate, &'static str> {
        match request {
            ChatRequest::Mute(player_id) => self.mute_player(req_player_id, player_id, players),
            ChatRequest::Unmute(player_id) => self.unmute_player(req_player_id, player_id, players),
            ChatRequest::Send { message, whisper } => self.send_chat(
                req_player_id,
                message,
                whisper,
                service,
                players,
                teams,
                metrics,
            ),
            ChatRequest::SetSafeMode(minutes) => {
                self.set_safe_mode(req_player_id, minutes, &*players)
            }
            ChatRequest::SetSlowMode(minutes) => {
                self.set_slow_mode(req_player_id, minutes, &*players)
            }
            ChatRequest::RestrictPlayer { player_id, minutes } => {
                self.restrict_player(req_player_id, player_id, minutes, players)
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
    pub(crate) fn log_chat(
        &self,
        ip: IpAddr,
        alias: PlayerAlias,
        message: &str,
        whisper: bool,
        verdict: &str,
    ) {
        if let Some(log_path) = &self.log_path {
            let ctx = if whisper { "team" } else { "global" };
            let log_path = Arc::clone(log_path);
            let mut line = Vec::with_capacity(256);
            let mut writer = csv::Writer::from_writer(&mut line);
            if let Err(e) = writer.write_record(&[
                &format!("{}", get_unix_time_now()),
                &format!("{:?}", G::GAME_ID),
                &ip.to_string(),
                ctx,
                verdict,
                alias.as_str(),
                message,
            ]) {
                error!("error composing chat line: {:?}", e);
            }
            drop(writer);

            tokio::task::spawn_blocking(move || {
                if let Err(e) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&*log_path)
                    .and_then(move |mut file| file.write_all(&line))
                {
                    error!("error logging chat: {:?}", e);
                }
            });
        }
    }

    fn try_execute_command(
        &mut self,
        req_player_id: PlayerId,
        message: &str,
        service: &mut G,
        players: &PlayerRepo<G>,
    ) -> Option<String> {
        struct FormattedDuration(Duration);

        impl Display for FormattedDuration {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                // Don't round down immediately after setting time.
                let d = self.0.saturating_add(Duration::from_millis(500));
                if d >= Duration::from_secs(3600) {
                    write!(f, "{}h", d.as_secs() / 3600)
                } else if d >= Duration::from_secs(60) {
                    write!(f, "{}m", d.as_secs() / 60)
                } else {
                    write!(f, "{}s", d.as_secs().max(1))
                }
            }
        }

        fn parse_minutes(arg: &str) -> Option<u32> {
            if matches!(arg, "none" | "off") {
                Some(0)
            } else {
                arg.parse::<u32>()
                    .ok()
                    .or_else(|| arg.strip_suffix('m').and_then(|s| s.parse().ok()))
                    .or_else(|| {
                        arg.strip_suffix('h')
                            .and_then(|s| s.parse::<u32>().ok())
                            .and_then(|n| n.checked_mul(60))
                    })
            }
        }

        let print_until_status = |name: &str, until: Option<Instant>| {
            if let Some(duration) =
                until.and_then(|instant| instant.checked_duration_since(Instant::now()))
            {
                format!(
                    "{} enabled for the next {}",
                    name,
                    FormattedDuration(duration)
                )
            } else {
                format!("{} disabled", name)
            }
        };

        let command = message.strip_prefix('/')?;
        let mut words = command.split_ascii_whitespace();
        let first = words.next()?;

        macro_rules! until {
            ($name: literal, $getter: ident, $setter: ident) => {{
                match words.next() {
                    None => print_until_status($name, self.$getter),
                    Some(arg) => {
                        if let Some(minutes) = parse_minutes(arg) {
                            self.$setter(req_player_id, minutes, players)
                                .map(|_| print_until_status($name, self.$getter))
                                .map_err(String::from)
                                .into_ok_or_err()
                        } else {
                            String::from("failed to parse argument as minutes")
                        }
                    }
                }
            }};
        }

        Some(match first {
            "slow" => until!("slow mode", slow_mode_until, set_slow_mode),
            "safe" => until!("safe mode", safe_mode_until, set_safe_mode),
            _ => service
                .chat_command(command, req_player_id, players)
                .unwrap_or_else(|| String::from("unrecognized command")),
        })
    }
}
