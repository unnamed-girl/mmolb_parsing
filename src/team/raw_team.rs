use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoDiscriminant};
use tracing::error;

use crate::{enums::{GameStat, MaybeRecognized, PositionType, RecordType, Slot}, feed_event::FeedEvent, serde_utils::APIHistory};
use super::team::{Team, TeamPlayer, TeamRecord};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawTeam {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    _id: Option<String>,
    pub abbreviation: String,
    pub active: bool,
    pub augments: u16,
    pub championships: u8,
    pub color: String,
    pub emoji: String,

    #[serde(default, skip_serializing_if = "APIHistory::is_missing")]
    pub feed: FeedHistory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motes_used: Option<u8>,

    pub location: String,
    pub full_location: String,
    pub league: String,

    /// no modifications have been seen
    modifications: Vec<serde_json::Value>,
    pub name: String,

    // / no mottos have been seen
    motto: Option<String>,

    #[serde(rename = "OwnerID")]
    pub owner_id: Option<String>,

    pub players: Vec<RawTeamPlayer>,
    pub record: HashMap<MaybeRecognized<RecordType>, TeamRecord>,
    pub season_records: HashMap<String, String>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawTeam> for Team {
    fn from(value: RawTeam) -> Self {
        let RawTeam { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields } = value;
        if extra_fields.len() > 0 {
            error!("Deserialization of Team found extra fields: {:?}", extra_fields);
        }
        let feed_format = feed.discriminant();
        let feed = match feed {
            FeedHistory::Season0 => Vec::new(),
            FeedHistory::Season1(feed) => feed
        };
        let players = players.into_iter().map(TeamPlayer::from).collect();

        if modifications.len() > 0 {
            error!("Expected all modifications lists to be empty, found {modifications:?}");
        }

        Team { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields, feed_format }
    }
}

impl From<Team> for RawTeam {
    fn from(value: Team) -> Self {
        let Team { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields, feed_format } = value;
        let feed = match feed_format {
            FeedHistoryDiscriminants::Season0 => FeedHistory::Season0,
            FeedHistoryDiscriminants::Season1 => FeedHistory::Season1(feed)
        };
        let players = players.into_iter().map(RawTeamPlayer::from).collect();
        RawTeam { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum FeedHistory {
    #[default]
    Season0,
    Season1(Vec<FeedEvent>)
}

impl APIHistory for FeedHistory {
    fn is_missing(&self) -> bool {
        matches!(self, Self::Season0)
    }
}


#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawTeamPlayer {
    pub emoji: String,
    pub first_name: String,
    pub last_name: String,
    pub number: u8,
    #[serde(rename = "PlayerID")]
    pub player_id: String,
    pub position: String,
    pub slot: MaybeRecognized<Slot>,
    #[serde(default, skip_serializing_if = "APIHistory::is_missing")]
    pub position_type: PositionTypeHistory,

    #[serde(default, skip_serializing_if = "APIHistory::is_missing")]
    pub stats: StatsHistory,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawTeamPlayer> for TeamPlayer {
    fn from(value: RawTeamPlayer) -> Self {
        let RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields } = value;

        let position_type_format = position_type.discriminant();
        let position_type = match position_type {
            PositionTypeHistory::Season0a => MaybeRecognized::<PositionType>::from(position.as_str()),
            PositionTypeHistory::Season0b(position) => position
        };

        // Undrafted player's positions are just their slot
        let position = (player_id != "#").then(|| position.as_str().into());

        let stats_format = stats.discriminant();
        let stats = match stats {
            StatsHistory::Undrafted => HashMap::default(),
            StatsHistory::Drafted(stats) => stats,
        };

        if extra_fields.len() > 0 {
            error!("Deserialization of TeamPlayer found extra fields: {:?}", extra_fields)
        }

        TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields, position_type_format, stats_format }
    }
}

impl From<TeamPlayer> for RawTeamPlayer {
    fn from(value: TeamPlayer) -> Self {
        let TeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields, position_type_format, stats_format } = value;

        let position = match (position, position_type_format) {
            (Some(position), _) => position.to_string(),
            (None, PositionTypeHistoryDiscriminants::Season0a) => position_type.to_string(),
            (None, PositionTypeHistoryDiscriminants::Season0b) => slot.to_string()
        };

        let position_type = match position_type_format {
            PositionTypeHistoryDiscriminants::Season0a => PositionTypeHistory::Season0a,
            PositionTypeHistoryDiscriminants::Season0b => PositionTypeHistory::Season0b(position_type)
        };

        let stats = match stats_format {
            StatsHistoryDiscriminants::Undrafted => StatsHistory::Undrafted,
            StatsHistoryDiscriminants::Drafted => StatsHistory::Drafted(stats)
        };

        RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum StatsHistory {
    #[default]
    Undrafted,
    Drafted(HashMap<MaybeRecognized<GameStat>, i32>)
}
impl APIHistory for StatsHistory {
    fn is_missing(&self) -> bool {
        matches!(self, StatsHistory::Undrafted)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum PositionTypeHistory {
    #[default]
    Season0a,
    Season0b(MaybeRecognized<PositionType>)
}

impl APIHistory for PositionTypeHistory {
    fn is_missing(&self) -> bool {
        matches!(self, Self::Season0a)
    }
}
