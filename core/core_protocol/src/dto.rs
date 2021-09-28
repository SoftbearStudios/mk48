// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::id::*;
use crate::metrics::*;
use crate::name::*;
use crate::UnixTime;
use serde::{Deserialize, Serialize};

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
    pub bounce: <RatioMetric as Metric>::Summary,
    pub peek: <RatioMetric as Metric>::Summary,
    pub concurrent: <ExtremaMetric as Metric>::Summary,
    pub minutes: <ContinuousExtremaMetric as Metric>::Summary,
    pub plays: <ContinuousExtremaMetric as Metric>::Summary,
    pub play_minutes: <ContinuousExtremaMetric as Metric>::Summary,
    pub solo: <RatioMetric as Metric>::Summary,
    pub new: <RatioMetric as Metric>::Summary,
    pub retention: <ContinuousExtremaMetric as Metric>::Summary,
    pub score: <ContinuousExtremaMetric as Metric>::Summary,
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
    pub server_addr: ServerAddr,
}

/// The Rules Data Transfer Object (DTO) specifies arena rules.
#[derive(Debug, Serialize, Deserialize)]
pub struct RulesDto {
    pub bot_min: u32,
    pub bot_percent: u32,
    pub team_size_max: u32,
}

/// The Team Data Transfer Object (DTO) binds team ID to team name.
#[derive(Clone, Serialize, Deserialize)]
pub struct TeamDto {
    pub team_id: TeamId,
    pub team_name: TeamName,
}
