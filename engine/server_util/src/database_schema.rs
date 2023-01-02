// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use aws_sdk_dynamodb::model::AttributeValue;
use common_util::serde::is_default;
use core_protocol::dto::{MetricFilter, MetricsDataPointDto, MetricsSummaryDto};
use core_protocol::id::{
    ArenaId, CohortId, GameId, LoginType, PlayerId, ServerId, SessionId, UserAgentId, UserId,
};
use core_protocol::metrics::{
    ContinuousExtremaMetric, DiscreteMetric, HistogramMetric, Metric, RatioMetric,
};
use core_protocol::name::{PlayerAlias, Referrer};
use core_protocol::serde_util::StrVisitor;
use core_protocol::UnixTime;
use derive_more::Add;
use serde::de::DeserializeOwned;
use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};
use std::iter::Sum;
use variant_count::VariantCount;

/// The type of leaderboard score, for a particular game.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, VariantCount)]
pub enum ScoreType {
    #[serde(rename = "player/all")]
    PlayerAllTime = 0,
    #[serde(rename = "player/week")]
    PlayerWeek = 1,
    #[serde(rename = "player/day")]
    PlayerDay = 2,
    #[serde(rename = "team/all")]
    TeamAllTime = 3,
    #[serde(rename = "team/week")]
    TeamWeek = 4,
    #[serde(rename = "team/day")]
    TeamDay = 5,
}

/// The type of leaderboard score, for any game. Serialized as "GameId/ScoreType".
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct GameIdScoreType {
    pub game_id: GameId,
    pub score_type: ScoreType,
}

impl Serialize for GameIdScoreType {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let av_game_id: AttributeValue = serde_dynamo::to_attribute_value(self.game_id).unwrap();
        let av_game_score_type: AttributeValue =
            serde_dynamo::to_attribute_value(self.score_type).unwrap();
        serializer.serialize_str(&format!(
            "{}/{}",
            av_game_id.as_s().unwrap(),
            av_game_score_type.as_s().unwrap()
        ))
    }
}

impl<'de> Deserialize<'de> for GameIdScoreType {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StrVisitor).and_then(|s| {
            let mut split = s.splitn(2, '/');
            if let Some((s_game_id, s_game_score_type)) = split.next().zip(split.next()) {
                let game_id_opt =
                    serde_dynamo::from_attribute_value(AttributeValue::S(String::from(s_game_id)))
                        .ok();
                let game_score_type_opt = serde_dynamo::from_attribute_value(AttributeValue::S(
                    String::from(s_game_score_type),
                ))
                .ok();
                return if let Some((game_id, game_score_type)) =
                    game_id_opt.zip(game_score_type_opt)
                {
                    Ok(Self {
                        game_id,
                        score_type: game_score_type,
                    })
                } else {
                    Err(de::Error::custom("parse error"))
                };
            }
            Err(de::Error::custom("wrong format"))
        })
    }
}

impl ScoreType {
    /// Returns corresponding period as unix timestamp seconds.
    pub fn period(self) -> Option<u64> {
        match self {
            Self::PlayerAllTime | Self::TeamAllTime => None,
            Self::PlayerWeek | Self::TeamWeek => Some(60 * 60 * 24 * 7),
            Self::PlayerDay | Self::TeamDay => Some(60 * 60 * 24),
        }
    }
}

/// A score of known score type.
#[derive(Debug, Clone)]
pub struct Score {
    pub alias: String,
    pub score: u32,
}

/// A database row storing a score.
#[derive(Serialize, Deserialize)]
pub struct ScoreItem {
    /// Hash key.
    pub game_id_score_type: GameIdScoreType,
    /// Range key.
    pub alias: String,
    pub score: u32,
    /// Unix seconds when DynamoDB should expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionItem {
    pub alias: PlayerAlias,
    /// Hash key.
    pub arena_id: ArenaId,
    #[serde(default)]
    pub cohort_id: CohortId,
    pub date_created: UnixTime,
    pub date_previous: Option<UnixTime>,
    pub date_renewed: UnixTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_terminated: Option<UnixTime>,
    pub game_id: GameId,
    pub player_id: PlayerId,
    pub plays: u32,
    pub previous_id: Option<SessionId>,
    pub referrer: Option<Referrer>,
    pub user_agent_id: Option<UserAgentId>,
    pub moderator: bool,
    /// Unlike RAM cache Session, not optional because storing localhost sessions in the database
    /// makes no sense.
    pub server_id: ServerId,
    /// Range key.
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Default, Add, Deserialize, Serialize)]
pub struct Metrics {
    /// Number of active abuse reports.
    #[serde(default, skip_serializing_if = "is_default")]
    pub abuse_reports: DiscreteMetric,
    /// How many arenas are in cache.
    #[serde(default, skip_serializing_if = "is_default")]
    pub arenas_cached: DiscreteMetric,
    /// How many megabits per second received.
    #[serde(default, skip_serializing_if = "is_default")]
    pub bandwidth_rx: ContinuousExtremaMetric,
    /// How many megabits per second transmitted.
    #[serde(default, skip_serializing_if = "is_default")]
    pub bandwidth_tx: ContinuousExtremaMetric,
    /// Number of banner advertisements shown.
    #[serde(default, skip_serializing_if = "is_default")]
    pub banner_ads: DiscreteMetric,
    /// Ratio of new players that leave without ever playing.
    #[serde(default, skip_serializing_if = "is_default")]
    pub bounce: RatioMetric,
    /// How many concurrent players.
    #[serde(default, skip_serializing_if = "is_default")]
    pub concurrent: ContinuousExtremaMetric,
    /// How many connections are open.
    #[serde(default, skip_serializing_if = "is_default")]
    pub connections: ContinuousExtremaMetric,
    /// Fraction of total CPU time used by processes in the current operating system.
    #[serde(default, skip_serializing_if = "is_default")]
    pub cpu: ContinuousExtremaMetric,
    /// Fraction of total CPU time stolen by the hypervisor.
    #[serde(default, skip_serializing_if = "is_default")]
    pub cpu_steal: ContinuousExtremaMetric,
    /// Ratio of new players that play only once and leave quickly.
    #[serde(default, skip_serializing_if = "is_default")]
    pub flop: RatioMetric,
    /// Client frames per second.
    #[serde(default, skip_serializing_if = "is_default")]
    pub fps: ContinuousExtremaMetric,
    /// Ratio of new players who were invited to new players who were not.
    #[serde(default, skip_serializing_if = "is_default")]
    pub invited: RatioMetric,
    /// Number of invitations in RAM cache.
    #[serde(default, skip_serializing_if = "is_default")]
    pub invitations_cached: DiscreteMetric,
    /// Ratio of players with FPS below 24 to all players.
    #[serde(default, skip_serializing_if = "is_default")]
    pub low_fps: RatioMetric,
    /// Minutes per completed play (a measure of engagement).
    #[serde(default, skip_serializing_if = "is_default")]
    pub minutes_per_play: ContinuousExtremaMetric,
    /// Minutes played, per visit, during the metrics period.
    #[serde(default, skip_serializing_if = "is_default")]
    pub minutes_per_visit: ContinuousExtremaMetric,
    /// Ratio of unique players that are new to players that are not.
    #[serde(default, skip_serializing_if = "is_default")]
    pub new: RatioMetric,
    /// Ratio of players with no referrer to all players.
    #[serde(default)]
    pub no_referrer: RatioMetric,
    /// Ratio of previous players that leave without playing (e.g. to peek at player count).
    #[serde(default, skip_serializing_if = "is_default")]
    pub peek: RatioMetric,
    /// How many players (for now, [`PlayerId`]) are in memory cache.
    #[serde(default, skip_serializing_if = "is_default")]
    pub players_cached: DiscreteMetric,
    /// Plays per visit (a measure of engagement).
    #[serde(default, skip_serializing_if = "is_default")]
    pub plays_per_visit: ContinuousExtremaMetric,
    /// Plays total (aka impressions).
    #[serde(default, skip_serializing_if = "is_default")]
    pub plays_total: DiscreteMetric,
    /// Percent of available server RAM required by service.
    #[serde(default, skip_serializing_if = "is_default")]
    pub ram: ContinuousExtremaMetric,
    /// Number of times session was renewed.
    #[serde(default, skip_serializing_if = "is_default")]
    pub renews: DiscreteMetric,
    /// Player retention in days.
    #[serde(default, skip_serializing_if = "is_default")]
    pub retention_days: ContinuousExtremaMetric,
    /// Player retention histogram.
    #[serde(default, skip_serializing_if = "is_default")]
    pub retention_histogram: HistogramMetric,
    /// Number of rewarded advertisements shown.
    #[serde(default, skip_serializing_if = "is_default")]
    pub rewarded_ads: DiscreteMetric,
    /// Network latency round trip time in seconds.
    #[serde(default, skip_serializing_if = "is_default")]
    pub rtt: ContinuousExtremaMetric,
    /// Score per completed play.
    #[serde(default, skip_serializing_if = "is_default")]
    pub score: ContinuousExtremaMetric,
    /// Total sessions in cache.
    #[serde(default, skip_serializing_if = "is_default")]
    pub sessions_cached: DiscreteMetric,
    /// Seconds per tick.
    #[serde(default, skip_serializing_if = "is_default")]
    pub spt: ContinuousExtremaMetric,
    /// Ratio of plays that end team-less to plays that don't.
    #[serde(default, skip_serializing_if = "is_default")]
    pub teamed: RatioMetric,
    /// Ratio of inappropriate messages to total.
    #[serde(default, skip_serializing_if = "is_default")]
    pub toxicity: RatioMetric,
    /// Server ticks per second.
    #[serde(default, skip_serializing_if = "is_default")]
    pub tps: ContinuousExtremaMetric,
    /// Uptime in (fractional) days.
    #[serde(default, skip_serializing_if = "is_default")]
    pub uptime: ContinuousExtremaMetric,
    /// Number of video advertisements shown.
    #[serde(default, skip_serializing_if = "is_default")]
    pub video_ads: DiscreteMetric,
    /// Visits
    #[serde(default, skip_serializing_if = "is_default")]
    pub visits: DiscreteMetric,
}

macro_rules! fields {
    ($me: ident, $st: ident, $f: ident, $($name: ident),*) => {
        {
            $st {
                $($name: $me.$name.$f()),*
            }
        }
    }
}

impl Metrics {
    pub fn summarize(&self) -> MetricsSummaryDto {
        fields!(
            self,
            MetricsSummaryDto,
            summarize,
            // Fields
            abuse_reports,
            arenas_cached,
            bandwidth_rx,
            bandwidth_tx,
            banner_ads,
            bounce,
            concurrent,
            connections,
            cpu,
            cpu_steal,
            flop,
            fps,
            invited,
            invitations_cached,
            low_fps,
            minutes_per_play,
            minutes_per_visit,
            new,
            no_referrer,
            peek,
            players_cached,
            plays_per_visit,
            plays_total,
            ram,
            renews,
            retention_days,
            retention_histogram,
            rewarded_ads,
            rtt,
            score,
            sessions_cached,
            spt,
            teamed,
            toxicity,
            tps,
            uptime,
            video_ads,
            visits
        )
    }

    pub fn data_point(&self) -> MetricsDataPointDto {
        fields! {
            self,
            MetricsDataPointDto,
            data_point,
            // Fields.
            abuse_reports,
            arenas_cached,
            bandwidth_rx,
            bandwidth_tx,
            banner_ads,
            bounce,
            concurrent,
            connections,
            cpu,
            cpu_steal,
            flop,
            fps,
            invited,
            invitations_cached,
            low_fps,
            minutes_per_play,
            minutes_per_visit,
            new,
            no_referrer,
            peek,
            players_cached,
            plays_per_visit,
            plays_total,
            ram,
            renews,
            retention_days,
            rewarded_ads,
            rtt,
            score,
            sessions_cached,
            spt,
            teamed,
            toxicity,
            tps,
            uptime,
            video_ads,
            visits
        }
    }
}

impl Sum for Metrics {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut total = Self::default();
        for item in iter {
            total = total + item;
        }
        total
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GameIdMetricFilter {
    pub game_id: GameId,
    pub metric_filter: Option<MetricFilter>,
}

impl Serialize for GameIdMetricFilter {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        fn ser<T: Serialize>(t: T) -> Option<String> {
            let av: AttributeValue = serde_dynamo::to_attribute_value(t).ok()?;
            if let AttributeValue::S(string) = av {
                Some(string)
            } else {
                None
            }
        }

        let game_id_string =
            ser(self.game_id).ok_or_else(|| ser::Error::custom("failed to serialize game id"))?;
        let string = match self.metric_filter {
            Some(filter) => match filter {
                MetricFilter::CohortId(cohort_id) => {
                    format!("{}/cohort_id/{}", game_id_string, cohort_id)
                }
                MetricFilter::Referrer(referrer) => {
                    format!("{}/referrer/{}", game_id_string, referrer)
                }
                MetricFilter::RegionId(region_id) => format!(
                    "{}/region_id/{}",
                    game_id_string,
                    ser(region_id)
                        .ok_or_else(|| ser::Error::custom("failed to serialize region id"))?,
                ),
                MetricFilter::UserAgentId(user_agent_id) => format!(
                    "{}/user_agent_id/{}",
                    game_id_string,
                    ser(user_agent_id)
                        .ok_or_else(|| ser::Error::custom("failed to serialize user agent id"))?,
                ),
            },
            None => game_id_string,
        };
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for GameIdMetricFilter {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(StrVisitor).and_then(|s| {
            fn de<T: DeserializeOwned>(s: &str) -> Option<T> {
                serde_dynamo::from_attribute_value(AttributeValue::S(String::from(s))).ok()
            }

            let mut split = s.split('/');
            let mut ret = Self {
                game_id: split
                    .next()
                    .and_then(de)
                    .ok_or_else(|| de::Error::custom("invalid game_id"))?,
                metric_filter: None,
            };

            if let Some(filter) = split.next() {
                let filter_value = split
                    .next()
                    .ok_or(de::Error::custom("missing filter value"))?;

                ret.metric_filter = Some(match filter {
                    "cohort_id" => MetricFilter::CohortId(
                        filter_value
                            .parse()
                            .map_err(|_| de::Error::custom("invalid cohort id"))?,
                    ),
                    "referrer" => MetricFilter::Referrer(
                        filter_value
                            .parse()
                            .map_err(|_| de::Error::custom("invalid referrer"))?,
                    ),
                    "region_id" => MetricFilter::RegionId(
                        de(filter_value).ok_or_else(|| de::Error::custom("invalid region id"))?,
                    ),
                    "user_agent_id" => MetricFilter::UserAgentId(
                        de(filter_value).ok_or_else(|| de::Error::custom("invalid user agent"))?,
                    ),
                    _ => return Err(de::Error::custom("invalid filter")),
                });
            }

            Ok(ret)
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MetricsItem {
    /// Hash key.
    // Rename for backwards compatibility.
    #[serde(rename = "game_id")]
    pub game_id_metric_filter: GameIdMetricFilter,
    /// Sort key.
    pub timestamp: UnixTime,
    #[serde(flatten)]
    pub metrics: Metrics,
}

#[derive(Serialize, Deserialize)]
pub struct UserItem {
    pub user_id: UserId,
}

#[derive(Serialize, Deserialize)]
pub struct LoginItem {
    pub login_type: LoginType,
    pub id: String,
    pub user_id: UserId,
}
