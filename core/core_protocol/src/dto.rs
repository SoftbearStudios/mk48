// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::id::*;
use crate::metrics::*;
use crate::name::*;
use crate::UnixTime;
use serde::{Deserialize, Serialize};

/// The Survey Data Transfer Object (DTO) collects user feedback.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SurveyDto {
    pub star_id: StarId,
    pub detail: Option<SurveyDetail>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InvitationDto {
    pub player_id: PlayerId,
}

/// The Leaderboard Data Transfer Object (DTO) is a single line on a leaderboard.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaderboardDto {
    pub alias: PlayerAlias,
    pub score: u32,
}

/// The Liveboard Data Transfer Object (DTO) is a single line on a liveboard.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveboardDto {
    pub team_captain: bool,
    pub team_id: Option<TeamId>,
    pub player_id: PlayerId,
    pub score: u32,
}

/// The Member Data Transfer Object (DTO) binds a player to a team.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemberDto {
    pub player_id: PlayerId,
    pub team_id: Option<TeamId>,
}

/// The Message Data Transfer Object (DTO) is used for chats.
#[derive(Clone, Serialize, Deserialize)]
pub struct MessageDto {
    pub alias: PlayerAlias, // For display in case alias is changed or player quits.
    pub date_sent: UnixTime,
    pub player_id: PlayerId, // For muting sender.
    pub team_captain: bool,
    pub team_name: Option<TeamName>, // Don't use team_id in case team is deleted or ID re-used.
    pub text: String,
    pub whisper: bool,
}

/// The Metrics Data Transfer Object (DTO) contains core server metrics.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MetricsDto {
    pub arenas_cached: <DiscreteMetric as Metric>::Summary,
    pub bounce: <RatioMetric as Metric>::Summary,
    pub concurrent: <ContinuousExtremaMetric as Metric>::Summary,
    pub cpu: <ContinuousExtremaMetric as Metric>::Summary,
    pub flop: <RatioMetric as Metric>::Summary,
    pub invited: <RatioMetric as Metric>::Summary,
    pub minutes_per_play: <ContinuousExtremaMetric as Metric>::Summary,
    pub minutes_per_session: <ContinuousExtremaMetric as Metric>::Summary,
    pub new: <RatioMetric as Metric>::Summary,
    pub peek: <RatioMetric as Metric>::Summary,
    pub plays_per_session: <ContinuousExtremaMetric as Metric>::Summary,
    pub plays_total: <DiscreteMetric as Metric>::Summary,
    pub ram: <ContinuousExtremaMetric as Metric>::Summary,
    pub retention: <ContinuousExtremaMetric as Metric>::Summary,
    pub score: <ContinuousExtremaMetric as Metric>::Summary,
    pub sessions_cached: <DiscreteMetric as Metric>::Summary,
    pub teamed: <RatioMetric as Metric>::Summary,
    pub toxicity: <RatioMetric as Metric>::Summary,
    pub uptime: <ContinuousExtremaMetric as Metric>::Summary,
}

/// The Player Data Transfer Object (DTO) binds player ID to player data.
#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerDto {
    pub alias: PlayerAlias,
    pub player_id: PlayerId,
    pub team_captain: bool,
    pub team_id: Option<TeamId>,
}

/// The Region Data Transfer Object (DTO) binds region ID to region data.
#[derive(Clone, Serialize, Deserialize)]
pub struct RegionDto {
    pub player_count: u32,
    pub region_id: RegionId,
    pub server_id: Option<ServerId>,
}

/// The Restart Data Transfer Object (DTO) contains restart parameters.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RestartDto {
    pub max_hour: u32,
    pub max_players: Option<u32>,
    pub max_score: Option<u32>,
    pub min_hour: u32,
}

/// The Rules Data Transfer Object (DTO) specifies arena rules.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RulesDto {
    /// Minimum number of players, to be reached by adding bots.
    pub bot_min: u32,
    /// Multiply real players by this as a percent to get minimum bots.
    pub bot_percent: u32,
    /// Start play's score at this.
    pub default_score: Option<u32>,
    /// Do bots appear on the live leaderboard? (bots never appear on the persistent leaderboard)
    pub show_bots_on_liveboard: bool,
    /// Maximum number of players in a team before no more can be accepted.
    pub team_size_max: u32,
}

impl Default for RulesDto {
    fn default() -> Self {
        Self {
            bot_min: 0,
            bot_percent: 0,
            default_score: None,
            show_bots_on_liveboard: false,
            team_size_max: 6,
        }
    }
}

/// The Team Data Transfer Object (DTO) binds team ID to team name.
#[derive(Clone, Serialize, Deserialize)]
pub struct TeamDto {
    pub team_id: TeamId,
    pub team_name: TeamName,
}
