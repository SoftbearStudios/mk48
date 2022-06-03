use aws_sdk_dynamodb::model::AttributeValue;
use core_protocol::dto::{MetricsDataPointDto, MetricsSummaryDto};
use core_protocol::id::{ArenaId, GameId, PlayerId, ServerId, SessionId, UserAgentId};
use core_protocol::metrics::{
    ContinuousExtremaMetric, DiscreteMetric, HistogramMetric, Metric, RatioMetric,
};
use core_protocol::name::{PlayerAlias, Referrer};
use core_protocol::serde_util::StrVisitor;
use core_protocol::UnixTime;
use derive_more::Add;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
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
    /// Unlike RAM cache Session, not optional because storing localhost sessions in the database
    /// makes no sense.
    pub server_id: ServerId,
    /// Range key.
    pub session_id: SessionId,
}

#[derive(Clone, Debug, Default, Add, Deserialize, Serialize)]
pub struct Metrics {
    /// Number of active abuse reports.
    #[serde(default)]
    pub abuse_reports: DiscreteMetric,
    /// How many arenas are in cache.
    #[serde(default)]
    pub arenas_cached: DiscreteMetric,
    /// How many megabits per second received.
    #[serde(default)]
    pub bandwidth_rx: ContinuousExtremaMetric,
    /// How many megabits per second transmitted.
    #[serde(default)]
    pub bandwidth_tx: ContinuousExtremaMetric,
    /// Ratio of new players that leave without ever playing.
    #[serde(default)]
    pub bounce: RatioMetric,
    /// How many concurrent players.
    #[serde(default)]
    pub concurrent: ContinuousExtremaMetric,
    /// How many connections are open.
    #[serde(default)]
    pub connections: ContinuousExtremaMetric,
    /// Fraction of total CPU time used by processes in the current operating system.
    #[serde(default)]
    pub cpu: ContinuousExtremaMetric,
    /// Fraction of total CPU time stolen by the hypervisor.
    #[serde(default)]
    pub cpu_steal: ContinuousExtremaMetric,
    /// Ratio of new players that play only once and leave quickly.
    #[serde(default)]
    pub flop: RatioMetric,
    /// Client frames per second.
    #[serde(default)]
    pub fps: ContinuousExtremaMetric,
    /// Ratio of new players who were invited to new players who were not.
    #[serde(default)]
    pub invited: RatioMetric,
    /// Number of invitations in RAM cache.
    #[serde(default)]
    pub invitations_cached: DiscreteMetric,
    /// Ratio of players with FPS below 24 to all players.
    #[serde(default)]
    pub low_fps: RatioMetric,
    /// Minutes per completed play (a measure of engagement).
    #[serde(default)]
    pub minutes_per_play: ContinuousExtremaMetric,
    /// Minutes played, per visit, during the metrics period.
    #[serde(default)]
    pub minutes_per_visit: ContinuousExtremaMetric,
    /// Ratio of unique players that are new to players that are not.
    #[serde(default)]
    pub new: RatioMetric,
    /// Ration of players with no referrer to all players.
    #[serde(default)]
    pub no_referrer: RatioMetric,
    /// Ratio of previous players that leave without playing (e.g. to peek at player count).
    #[serde(default)]
    pub peek: RatioMetric,
    /// How many players (for now, [`PlayerId`]) are in memory cache.
    #[serde(default)]
    pub players_cached: DiscreteMetric,
    /// Plays per visit (a measure of engagement).
    #[serde(default)]
    pub plays_per_visit: ContinuousExtremaMetric,
    /// Plays total (aka impressions).
    #[serde(default)]
    pub plays_total: DiscreteMetric,
    /// Percent of available server RAM required by service.
    #[serde(default)]
    pub ram: ContinuousExtremaMetric,
    /// Number of times session was renewed.
    #[serde(default)]
    pub renews: DiscreteMetric,
    /// Player retention in days.
    #[serde(default)]
    pub retention_days: ContinuousExtremaMetric,
    /// Player retention histogram.
    #[serde(default)]
    pub retention_histogram: HistogramMetric,
    /// Network latency round trip time in seconds.
    #[serde(default)]
    pub rtt: ContinuousExtremaMetric,
    /// Score per completed play.
    #[serde(default)]
    pub score: ContinuousExtremaMetric,
    /// Total sessions in cache.
    #[serde(default)]
    pub sessions_cached: DiscreteMetric,
    /// Seconds per tick.
    #[serde(default)]
    pub spt: ContinuousExtremaMetric,
    /// Ratio of plays that end team-less to plays that don't.
    #[serde(default)]
    pub teamed: RatioMetric,
    /// Ratio of inappropriate messages to total.
    #[serde(default)]
    pub toxicity: RatioMetric,
    /// Server ticks per second.
    #[serde(default)]
    pub tps: ContinuousExtremaMetric,
    /// Uptime in (fractional) days.
    #[serde(default)]
    pub uptime: ContinuousExtremaMetric,
    /// Visits
    #[serde(default)]
    pub visits: DiscreteMetric,
}

impl Metrics {
    pub fn summarize(&self) -> MetricsSummaryDto {
        MetricsSummaryDto {
            abuse_reports: self.abuse_reports.summarize(),
            arenas_cached: self.arenas_cached.summarize(),
            bandwidth_rx: self.bandwidth_rx.summarize(),
            bandwidth_tx: self.bandwidth_tx.summarize(),
            bounce: self.bounce.summarize(),
            concurrent: self.concurrent.summarize(),
            connections: self.connections.summarize(),
            cpu: self.cpu.summarize(),
            cpu_steal: self.cpu_steal.summarize(),
            flop: self.flop.summarize(),
            fps: self.fps.summarize(),
            invited: self.invited.summarize(),
            invitations_cached: self.invitations_cached.summarize(),
            low_fps: self.low_fps.summarize(),
            minutes_per_play: self.minutes_per_play.summarize(),
            minutes_per_visit: self.minutes_per_visit.summarize(),
            new: self.new.summarize(),
            no_referrer: self.no_referrer.summarize(),
            peek: self.peek.summarize(),
            players_cached: self.players_cached.summarize(),
            plays_per_visit: self.plays_per_visit.summarize(),
            plays_total: self.plays_total.summarize(),
            ram: self.ram.summarize(),
            renews: self.renews.summarize(),
            retention_days: self.retention_days.summarize(),
            retention_histogram: self.retention_histogram.summarize(),
            rtt: self.rtt.summarize(),
            score: self.score.summarize(),
            sessions_cached: self.sessions_cached.summarize(),
            spt: self.spt.summarize(),
            teamed: self.teamed.summarize(),
            toxicity: self.toxicity.summarize(),
            tps: self.tps.summarize(),
            uptime: self.uptime.summarize(),
            visits: self.visits.summarize(),
        }
    }

    pub fn data_point(&self) -> MetricsDataPointDto {
        MetricsDataPointDto {
            abuse_reports: self.abuse_reports.data_point(),
            arenas_cached: self.arenas_cached.data_point(),
            bandwidth_rx: self.bandwidth_rx.data_point(),
            bandwidth_tx: self.bandwidth_tx.data_point(),
            bounce: self.bounce.data_point(),
            concurrent: self.concurrent.data_point(),
            connections: self.connections.data_point(),
            cpu: self.cpu.data_point(),
            cpu_steal: self.cpu_steal.data_point(),
            flop: self.flop.data_point(),
            fps: self.fps.data_point(),
            invited: self.invited.data_point(),
            invitations_cached: self.invitations_cached.data_point(),
            low_fps: self.low_fps.data_point(),
            minutes_per_play: self.minutes_per_play.data_point(),
            minutes_per_visit: self.minutes_per_visit.data_point(),
            new: self.new.data_point(),
            no_referrer: self.no_referrer.data_point(),
            peek: self.peek.data_point(),
            players_cached: self.players_cached.data_point(),
            plays_per_visit: self.plays_per_visit.data_point(),
            plays_total: self.plays_total.data_point(),
            ram: self.ram.data_point(),
            renews: self.renews.data_point(),
            retention_days: self.retention_days.data_point(),
            rtt: self.rtt.data_point(),
            score: self.score.data_point(),
            sessions_cached: self.sessions_cached.data_point(),
            spt: self.spt.data_point(),
            teamed: self.teamed.data_point(),
            toxicity: self.toxicity.data_point(),
            tps: self.tps.data_point(),
            uptime: self.uptime.data_point(),
            visits: self.visits.data_point(),
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

#[derive(Clone, Serialize, Deserialize)]
pub struct MetricsItem {
    /// Hash key.
    pub game_id: GameId,
    /// Sort key.
    pub timestamp: UnixTime,
    #[serde(flatten)]
    pub metrics: Metrics,
}
