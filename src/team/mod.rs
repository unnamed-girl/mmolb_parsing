use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{enums::{GameStat, MaybeRecognized, Position, PositionType, RecordType, Slot}, feed_event::FeedEvent, team::raw_team::PositionTypeHistoryDiscriminants};
use raw_team::RawTeamPlayer;

mod raw_team;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Team {
    // Cashews id
    #[serde(rename = "_id")]
    _id: Option<String>,
    pub abbreviation: String,
    pub active: bool,
    pub augments: u16,
    pub championships: u8,
    pub color: String,
    pub emoji: String,

    #[serde(default)] // Teams deleted before season 1 won't have feeds,
    pub feed: Vec<FeedEvent>,
    pub motes_used: Option<u8>,

    pub location: String,
    pub full_location: String,
    pub league: String,

    /// no modifications have been seen, so left as raw json
    pub modifications: Vec<Value>,
    pub name: String,

    // / no mottos have been seen, so left as raw json
    pub motto: Option<serde_json::Value>,

    #[serde(rename = "OwnerID")]
    pub owner_id: Option<String>,

    pub players: Vec<TeamPlayer>,
    pub record: HashMap<MaybeRecognized<RecordType>, TeamRecord>,
    pub season_records: HashMap<String, String>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct TeamRecord {
    pub losses: u16,
    pub run_differential: i32,
    pub wins: u16
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(from = "RawTeamPlayer", into = "RawTeamPlayer")]
pub struct TeamPlayer {
    pub emoji: String,
    pub first_name: String,
    pub last_name: String,
    pub number: u8,
    pub player_id: String,

    /// Undrafted player's positions are just their slot.
    pub position: Option<MaybeRecognized<Position>>,

    pub slot: MaybeRecognized<Slot>,

    position_type_format: PositionTypeHistoryDiscriminants,
    pub position_type: MaybeRecognized<PositionType>,


    pub stats: HashMap<MaybeRecognized<GameStat>, i32>,
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
