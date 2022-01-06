// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::dto::*;
use crate::id::*;
use crate::name::*;
use crate::UnixTime;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;
use std::sync::Arc;

// Admin requests are from the admin interface to the core service.
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    RequestDay {
        game_id: GameId,
        referrer: Option<Referrer>,
        user_agent_id: Option<UserAgentId>,
    },
    RequestGames,
    RequestSeries {
        game_id: GameId,
        period_start: Option<UnixTime>,
        period_stop: Option<UnixTime>,
        // Resolution in hours.
        resolution: Option<NonZeroU8>,
    },
    RequestStatus,
    RequestSummary {
        game_id: GameId,
        referrer: Option<Referrer>,
        user_agent_id: Option<UserAgentId>,
        period_start: Option<UnixTime>,
        period_stop: Option<UnixTime>,
    },
    RequestReferrers,
    RequestRestart {
        conditions: RestartDto,
    },
    RequestUserAgents,
    SendChat {
        // If None, goes to all arenas.
        arena_id: Option<ArenaId>,
        alias: PlayerAlias,
        message: String,
    },
    RequestRedirect,
    SetRedirect {
        server_id: Option<ServerId>,
    },
}

// Client requests are from the browser to the core service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientRequest {
    AcceptPlayer {
        player_id: PlayerId,
    },
    AssignCaptain {
        player_id: PlayerId,
    },
    CreateInvitation,
    CreateSession {
        game_id: GameId,
        invitation_id: Option<InvitationId>,
        referrer: Option<Referrer>,
        saved_session_tuple: Option<(ArenaId, SessionId)>,
    },
    CreateTeam {
        team_name: TeamName,
    },
    IdentifySession {
        alias: PlayerAlias,
    },
    KickPlayer {
        player_id: PlayerId,
    },
    MuteSender {
        enable: bool,
        player_id: PlayerId,
    },
    ReportPlayer {
        player_id: PlayerId,
    },
    QuitTeam,
    RejectPlayer {
        player_id: PlayerId,
    },
    RequestJoin {
        team_id: TeamId,
    },
    SendChat {
        message: String,
        whisper: bool,
    },
    SubmitSurvey {
        survey: SurveyDto,
    },
    TallyFps {
        fps: f32,
    },
    Trace {
        message: String,
    },
}

// Server requests are from the game server to the core service.
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerRequest {
    BotRequest {
        session_id: SessionId,
        request: ClientRequest,
    },
    // This should be called when a web socket it dropped regardless of whether client is playing.
    DropSession {
        session_id: SessionId,
    },
    SetStatus {
        session_id: SessionId,
        #[serde(default)]
        location: Option<Location>,
        #[serde(default)]
        score: Option<u32>,
    },
    StartArena {
        game_id: GameId,
        region: RegionId,
        rules: Option<RulesDto>,
        saved_arena_id: Option<ArenaId>,
        server_id: Option<ServerId>,
    },
    StartPlay {
        session_id: SessionId,
    },
    StopArena,
    StopPlay {
        session_id: SessionId,
        // In the future, may also add Option<ExitState>
    },
    TallyUps {
        ups: f32,
    },
    ValidateSession {
        session_id: SessionId,
    },
}

#[cfg_attr(feature = "server", derive(actix::Message))]
#[cfg_attr(feature = "server", rtype(result = "()"))]
#[cfg_attr(feature = "client", derive(actix::Message))]
#[cfg_attr(feature = "client", rtype(result = "()"))]
#[derive(Debug, Serialize, Deserialize)]
pub enum AdminUpdate {
    ChatSent {
        sent: bool,
    },
    DayRequested {
        series: Arc<[(UnixTime, MetricsDataPointDto)]>,
    },
    GamesRequested {
        games: Arc<[(GameId, f32)]>,
    },
    ReferrersRequested {
        referrers: Arc<[(Referrer, f32)]>,
    },
    RestartRequested,
    RedirectRequested {
        server_id: Option<ServerId>,
    },
    RedirectSet {
        server_id: Option<ServerId>,
    },
    SeriesRequested {
        series: Arc<[(UnixTime, MetricsDataPointDto)]>,
    },
    SummaryRequested {
        metrics: MetricsSummaryDto,
    },
    StatusRequested,
    UserAgentsRequested {
        user_agents: Arc<[(UserAgentId, f32)]>,
    },
}

#[cfg_attr(feature = "server", derive(actix::Message))]
#[cfg_attr(feature = "server", rtype(result = "()"))]
#[derive(Serialize, Deserialize)]
pub enum ClientUpdate {
    CaptainAssigned {
        player_id: PlayerId,
    },
    ChatSent {
        player_id: Option<PlayerId>,
    },
    InvitationCreated {
        invitation_id: InvitationId,
    },
    JoinRequested {
        team_id: TeamId,
    },
    // The following is for the team captain only.
    JoinersUpdated {
        added: Arc<[PlayerId]>,
        removed: Arc<[PlayerId]>,
    },
    // The following is for the joiner only.
    JoinsUpdated {
        added: Arc<[TeamId]>,
        removed: Arc<[TeamId]>,
    },
    // The leaderboard contains high score players, but not teams, for prior periods.
    LeaderboardUpdated {
        leaderboard: Arc<[LeaderboardDto]>,
        period: PeriodId,
    },
    // The liveboard contains high score players and their teams in the current game.
    LiveboardUpdated {
        added: Arc<[LiveboardDto]>,
        removed: Arc<[PlayerId]>,
    },
    MessagesUpdated {
        added: Arc<[MessageDto]>,
    },
    PlayerAccepted {
        player_id: PlayerId,
    },
    PlayerKicked {
        player_id: PlayerId,
    },
    PlayerRejected {
        player_id: PlayerId,
    },
    PlayersUpdated {
        /// Total count, *excluding* bots.
        count: u32,
        /// Added and/or changed players, including bots.
        added: Arc<[PlayerDto]>,
        /// Removed players, including bots.
        removed: Arc<[PlayerId]>,
    },
    RegionsUpdated {
        added: Arc<[RegionDto]>,
        removed: Arc<[RegionId]>,
    },
    SenderMuted {
        enable: bool,
        player_id: PlayerId,
    },
    PlayerReported {
        player_id: PlayerId,
    },
    SessionCreated {
        arena_id: ArenaId,
        server_id: Option<ServerId>,
        session_id: SessionId,
        player_id: PlayerId,
    },
    SessionIdentified {
        alias: PlayerAlias,
    },
    SurveySubmitted,
    TeamCreated {
        team_id: TeamId,
    },
    TeamQuit,
    TeamsUpdated {
        added: Arc<[TeamDto]>,
        removed: Arc<[TeamId]>,
    },
    Traced,
}

#[cfg_attr(feature = "server", derive(actix::Message))]
#[cfg_attr(feature = "server", rtype(result = "()"))]
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerUpdate {
    ArenaStarted {
        arena_id: ArenaId,
    },
    ArenaStopped,
    ArmageddonStarted {
        arena_id: ArenaId,
    },
    BotReady {
        player_id: PlayerId,
        session_id: SessionId,
    },
    MembersChanged {
        changes: Arc<[MemberDto]>,
    },
    PlayStarted {
        player_id: PlayerId,
        // In the future, may also add Option<ExitState>
    },
    PlayStopped,
    SessionDropped,
    SessionValid {
        elapsed: u32,
        player_id: PlayerId,
        invitation: Option<InvitationDto>,
        score: u32,
    },
    StatusSet,
}
