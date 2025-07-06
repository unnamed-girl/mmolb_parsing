use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use tracing::error;

use crate::{enums::{GameStat, MaybeRecognized, PositionType, RecordType, Slot}, feed_event::FeedEvent, utils::AddedLaterMarker};
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

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feed: Option<Vec<FeedEvent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motes_used: Option<u8>,

    pub location: String,
    pub full_location: String,
    pub league: String,

    /// no modifications have been seen
    modifications: Vec<serde_json::Value>,
    pub name: String,

    pub motto: Option<String>,

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
        if !extra_fields.is_empty() {
            error!("Deserialization of Team found extra fields: {:?}", extra_fields);
        }
        let feed_format = AddedLaterMarker::new(&feed);
        let feed = feed.unwrap_or_default();
        let players = players.into_iter().map(TeamPlayer::from).collect();

        if !modifications.is_empty() {
            error!("Expected all modifications lists to be empty, found {modifications:?}");
        }

        Team { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields, feed_format }
    }
}

impl From<Team> for RawTeam {
    fn from(value: Team) -> Self {
        let Team { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields, feed_format } = value;
        let feed = feed_format.wrap(feed);
        let players = players.into_iter().map(RawTeamPlayer::from).collect();
        RawTeam { _id, abbreviation, active, augments, championships, color, emoji, feed, motes_used, location, full_location, league, modifications, name, motto, owner_id, players, record, season_records, extra_fields }
    }
}



#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_type: Option<MaybeRecognized<PositionType>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<HashMap<MaybeRecognized<GameStat>, i32>>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

impl From<RawTeamPlayer> for TeamPlayer {
    fn from(value: RawTeamPlayer) -> Self {
        let RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields } = value;

        let position_type_format = AddedLaterMarker::new(&position_type);
        let position_type = position_type.unwrap_or_else(|| MaybeRecognized::<PositionType>::from(position.as_str()));

        // Undrafted player's positions are just their slot
        let position = (player_id != "#").then(|| position.as_str().into());

        let stats_format = AddedLaterMarker::new(&stats);
        let stats = stats.unwrap_or_default();

        if !extra_fields.is_empty() {
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
            (None, AddedLaterMarker(true)) => position_type.to_string(),
            (None, AddedLaterMarker(false)) => slot.to_string()
        };

        let position_type = position_type_format.wrap(position_type);

        let stats = stats_format.wrap(stats);
        RawTeamPlayer { emoji, first_name, last_name, number, player_id, position, slot, position_type, stats, extra_fields }
    }
}
