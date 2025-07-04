use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{enums::{GameStat, MaybeRecognized, Position, PositionType, RecordType, Slot}, feed_event::FeedEvent, serde_utils::AddedLaterMarker};
use super::raw_team::{RawTeam, RawTeamPlayer};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(from = "RawTeam", into = "RawTeam")]
pub struct Team {
    /// Cashews id
    pub(super) _id: Option<String>,
    pub abbreviation: String,
    pub active: bool,
    pub augments: u16,
    pub championships: u8,
    pub color: String,
    pub emoji: String,

    pub(super) feed_format: AddedLaterMarker,
    pub feed: Vec<FeedEvent>,
    pub motes_used: Option<u8>,

    pub location: String,
    pub full_location: String,
    pub league: String,

    /// no modifications have been seen, so left as raw json
    pub(super) modifications: Vec<serde_json::Value>,
    pub name: String,

    pub motto: Option<String>,

    pub owner_id: Option<String>,

    pub players: Vec<TeamPlayer>,
    pub record: HashMap<MaybeRecognized<RecordType>, TeamRecord>,
    pub season_records: HashMap<String, String>,

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

    pub(super) position_type_format: AddedLaterMarker,
    pub position_type: MaybeRecognized<PositionType>,


    pub(super) stats_format: AddedLaterMarker,
    pub stats: HashMap<MaybeRecognized<GameStat>, i32>,
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::{fs::File, io::Read, path::Path};

    use crate::team::{Team, TeamPlayer};

    #[test]
    fn round_trip_team() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = String::new(); 
        File::open(Path::new("test_data/s2_team.json"))?.read_to_string(&mut buf)?;

        let json: serde_json::Value = serde_json::from_str(&buf)?;
        let event: Team = serde_json::from_value(json.clone())?;
        let round_trip = serde_json::to_value(&event)?; 

        let diff = serde_json_diff::values(json, round_trip);
        assert!(diff.is_none(), "{diff:?}");

        Ok(())
    }


    #[test]
    fn round_trip_team_player() -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = String::new(); 
        File::open(Path::new("test_data/s2_team_player.json"))?.read_to_string(&mut buf)?;

        let json: serde_json::Value = serde_json::from_str(&buf)?;
        let event: TeamPlayer = serde_json::from_value(json.clone())?;
        let round_trip = serde_json::to_value(&event)?;

        let diff = serde_json_diff::values(json, round_trip);
        assert!(diff.is_none(), "{diff:?}");

        Ok(())
    }
}