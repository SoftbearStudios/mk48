// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::id::*;
use crate::name::*;
use crate::owned::Owned;
use crate::UnixTime;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct InvitationDto {
    /// Who sent it.
    pub player_id: PlayerId,
}

/// The Leaderboard Data Transfer Object (DTO) is a single line on a leaderboard.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LeaderboardDto {
    pub alias: PlayerAlias,
    pub score: u32,
}

impl PartialOrd for LeaderboardDto {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LeaderboardDto {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .cmp(&other.score)
            .then_with(|| self.alias.cmp(&other.alias))
    }
}

/// The Liveboard Data Transfer Object (DTO) is a single line on a liveboard.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct LiveboardDto {
    pub player_id: PlayerId,
    pub score: u32,
    pub team_captain: bool,
    pub team_id: Option<TeamId>,
}

impl PartialOrd for LiveboardDto {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// NOTE: Recently changed so that larger scores are treated as greater.
impl Ord for LiveboardDto {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score).then_with(|| {
            self.player_id.cmp(&other.player_id).then_with(|| {
                self.team_id
                    .cmp(&other.team_id)
                    .then_with(|| self.team_captain.cmp(&other.team_captain))
            })
        })
    }
}

#[cfg(test)]
mod test {
    use crate::dto::LiveboardDto;
    use crate::id::{PlayerId, TeamId};
    use std::num::NonZeroU32;

    #[test]
    fn sort_order() {
        assert!(
            LiveboardDto {
                player_id: PlayerId(NonZeroU32::new(2).unwrap()),
                score: 3,
                team_captain: true,
                team_id: Some(TeamId(NonZeroU32::new(1).unwrap())),
            } < LiveboardDto {
                player_id: PlayerId(NonZeroU32::new(1).unwrap()),
                score: 5,
                team_captain: false,
                team_id: None,
            }
        )
    }
}

/// The Member Data Transfer Object (DTO) binds a player to a team.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemberDto {
    pub player_id: PlayerId,
    pub team_id: Option<TeamId>,
}

/// The Message Data Transfer Object (DTO) is used for chats.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDto {
    /// For display in case alias is changed or player quits.
    pub alias: PlayerAlias,
    pub date_sent: UnixTime,
    /// For muting sender. None if from server.
    pub player_id: Option<PlayerId>,
    pub team_captain: bool,
    /// Don't use team_id in case team is deleted or ID re-used.
    pub team_name: Option<TeamName>,
    pub text: String,
    /// Whether message is directed to team only.
    pub whisper: bool,
}
/// The Player Data Transfer Object (DTO) binds player ID to player data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerDto {
    pub alias: PlayerAlias,
    pub moderator: bool,
    pub player_id: PlayerId,
    pub team_captain: bool,
    pub team_id: Option<TeamId>,
}

/// The Server Data Transfer Object (DTO) binds server ID to server data.
/// It is assumed to be reachable, healthy, having an ip mapped to server_id via DNS, and having
/// a compatible client version.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ServerDto {
    pub server_id: ServerId,
    pub region_id: RegionId,
    pub player_count: u32,
}

impl PartialOrd for ServerDto {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ServerDto {
    fn cmp(&self, other: &Self) -> Ordering {
        self.server_id.cmp(&other.server_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SnippetDto {
    pub cohort_id: Option<CohortId>,
    pub referrer: Option<Referrer>,
    pub snippet: Owned<str>,
}

impl PartialOrd for SnippetDto {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SnippetDto {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut diff = self.cohort_id.cmp(&other.cohort_id);
        if diff == Ordering::Equal {
            diff = self.referrer.cmp(&other.referrer)
        }
        diff
    }
}

/// The Team Data Transfer Object (DTO) binds team ID to team name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TeamDto {
    pub team_id: TeamId,
    pub name: TeamName,
    /// Maximum number of numbers reached.
    pub full: bool,
    /// Closed to additional requests.
    pub closed: bool,
}

/// Filter daily metrics.
// TODO: Not a DTO?
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MetricFilter {
    CohortId(CohortId),
    Referrer(Referrer),
    RegionId(RegionId),
    UserAgentId(UserAgentId),
}

#[cfg(feature = "admin")]
pub use admin::*;
#[cfg(feature = "admin")]
mod admin {
    use super::*;
    use crate::metrics::*;
    use std::net::IpAddr;
    use std::num::NonZeroU64;

    /// The Player Admin Data Transfer Object (DTO) binds player ID to admin player data (for real players, not bots).
    #[derive(Clone, Debug, PartialEq, Serialize)]
    pub struct AdminPlayerDto {
        pub alias: PlayerAlias,
        pub player_id: PlayerId,
        pub team_id: Option<TeamId>,
        pub region_id: Option<RegionId>,
        pub discord_id: Option<NonZeroU64>,
        pub ip_address: IpAddr,
        pub moderator: bool,
        pub score: u32,
        pub plays: u32,
        pub fps: Option<f32>,
        pub rtt: Option<u16>,
        pub messages: usize,
        pub inappropriate_messages: usize,
        pub abuse_reports: usize,
        /// Remaining minutes muted.
        pub mute: usize,
        /// Remaining minutes restricted.
        pub restriction: usize,
    }

    /// Like [`ServerDto`] but more details.
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct AdminServerDto {
        pub server_id: ServerId,
        pub redirect_server_id: Option<ServerId>,
        pub region_id: Option<RegionId>,
        pub ip: IpAddr,
        /// Routed by DNS to the home page.
        pub home: bool,
        pub reachable: bool,
        /// Round trip time in milliseconds.
        pub rtt: u16,
        pub healthy: bool,
        pub client_hash: Option<u64>,
        pub player_count: Option<u32>,
    }

    impl PartialOrd for AdminServerDto {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for AdminServerDto {
        fn cmp(&self, other: &Self) -> Ordering {
            self.server_id.cmp(&other.server_id)
        }
    }

    /// The Metrics Data Transfer Object (DTO) contains core server metrics.
    #[derive(Clone, Copy, Debug, Serialize)]
    pub struct MetricsSummaryDto {
        pub abuse_reports: <DiscreteMetric as Metric>::Summary,
        pub arenas_cached: <DiscreteMetric as Metric>::Summary,
        pub bandwidth_rx: <ContinuousExtremaMetric as Metric>::Summary,
        pub bandwidth_tx: <ContinuousExtremaMetric as Metric>::Summary,
        pub banner_ads: <DiscreteMetric as Metric>::Summary,
        pub bounce: <RatioMetric as Metric>::Summary,
        pub concurrent: <ContinuousExtremaMetric as Metric>::Summary,
        pub connections: <ContinuousExtremaMetric as Metric>::Summary,
        pub cpu: <ContinuousExtremaMetric as Metric>::Summary,
        pub cpu_steal: <ContinuousExtremaMetric as Metric>::Summary,
        pub flop: <RatioMetric as Metric>::Summary,
        pub fps: <ContinuousExtremaMetric as Metric>::Summary,
        pub invited: <RatioMetric as Metric>::Summary,
        pub invitations_cached: <DiscreteMetric as Metric>::Summary,
        pub low_fps: <RatioMetric as Metric>::Summary,
        pub minutes_per_play: <ContinuousExtremaMetric as Metric>::Summary,
        pub minutes_per_visit: <ContinuousExtremaMetric as Metric>::Summary,
        pub new: <RatioMetric as Metric>::Summary,
        pub no_referrer: <RatioMetric as Metric>::Summary,
        pub peek: <RatioMetric as Metric>::Summary,
        pub players_cached: <DiscreteMetric as Metric>::Summary,
        pub plays_per_visit: <ContinuousExtremaMetric as Metric>::Summary,
        pub plays_total: <DiscreteMetric as Metric>::Summary,
        pub ram: <ContinuousExtremaMetric as Metric>::Summary,
        pub renews: <DiscreteMetric as Metric>::Summary,
        pub retention_days: <ContinuousExtremaMetric as Metric>::Summary,
        pub retention_histogram: <HistogramMetric as Metric>::Summary,
        pub rewarded_ads: <DiscreteMetric as Metric>::Summary,
        pub rtt: <ContinuousExtremaMetric as Metric>::Summary,
        pub score: <ContinuousExtremaMetric as Metric>::Summary,
        pub sessions_cached: <DiscreteMetric as Metric>::Summary,
        pub spt: <ContinuousExtremaMetric as Metric>::Summary,
        pub teamed: <RatioMetric as Metric>::Summary,
        pub toxicity: <RatioMetric as Metric>::Summary,
        pub tps: <ContinuousExtremaMetric as Metric>::Summary,
        pub uptime: <ContinuousExtremaMetric as Metric>::Summary,
        pub video_ads: <DiscreteMetric as Metric>::Summary,
        pub visits: <DiscreteMetric as Metric>::Summary,
    }

    #[derive(Clone, Copy, Debug, Serialize)]
    pub struct MetricsDataPointDto {
        pub abuse_reports: <DiscreteMetric as Metric>::DataPoint,
        pub arenas_cached: <DiscreteMetric as Metric>::DataPoint,
        pub bandwidth_rx: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub bandwidth_tx: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub banner_ads: <DiscreteMetric as Metric>::DataPoint,
        pub bounce: <RatioMetric as Metric>::DataPoint,
        pub concurrent: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub connections: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub cpu: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub cpu_steal: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub flop: <RatioMetric as Metric>::DataPoint,
        pub fps: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub invited: <RatioMetric as Metric>::DataPoint,
        pub invitations_cached: <DiscreteMetric as Metric>::DataPoint,
        pub low_fps: <RatioMetric as Metric>::DataPoint,
        pub minutes_per_play: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub minutes_per_visit: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub new: <RatioMetric as Metric>::DataPoint,
        pub no_referrer: <RatioMetric as Metric>::DataPoint,
        pub peek: <RatioMetric as Metric>::DataPoint,
        pub players_cached: <DiscreteMetric as Metric>::DataPoint,
        pub plays_per_visit: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub plays_total: <DiscreteMetric as Metric>::DataPoint,
        pub ram: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub renews: <DiscreteMetric as Metric>::DataPoint,
        pub retention_days: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub rewarded_ads: <DiscreteMetric as Metric>::DataPoint,
        pub rtt: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub score: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub sessions_cached: <DiscreteMetric as Metric>::DataPoint,
        pub spt: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub teamed: <RatioMetric as Metric>::DataPoint,
        pub toxicity: <RatioMetric as Metric>::DataPoint,
        pub tps: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub uptime: <ContinuousExtremaMetric as Metric>::DataPoint,
        pub video_ads: <DiscreteMetric as Metric>::DataPoint,
        pub visits: <DiscreteMetric as Metric>::DataPoint,
    }
}
