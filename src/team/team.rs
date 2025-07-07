use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::{enums::{GameStat, MaybeRecognized, Position, PositionType, RecordType, Slot}, feed_event::FeedEvent, utils::{ExpectNone, ExtraFields}, AddedLater};
use super::raw_team::{RawTeamPlayer};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Team {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub(super) _id: Option<String>,
    pub abbreviation: String,
    pub active: bool,
    pub augments: u16,
    pub championships: u8,
    pub color: String,
    pub emoji: String,

    /// Not present on some deleted teams.
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub feed: AddedLater<Vec<FeedEvent>>,

    /// Not present on some deleted teams.
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub motes_used: AddedLater<u8>,

    pub location: String,
    pub full_location: String,
    pub league: String,

    /// no modifications have been seen, so left as raw json
    pub(super) modifications: Vec<ExpectNone>,
    pub name: String,

    pub motto: Option<String>,

    #[serde(rename = "OwnerID")]
    pub owner_id: Option<String>,

    pub players: Vec<TeamPlayer>,
    pub record: HashMap<MaybeRecognized<RecordType>, TeamRecord>,
    pub season_records: HashMap<String, String>,

    #[serde(flatten)]
    pub extra_fields: ExtraFields,
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

    pub(crate) position_type_overidden: bool,
    pub position_type: MaybeRecognized<PositionType>,


    pub stats: AddedLater<HashMap<MaybeRecognized<GameStat>, i32>>,
    
    #[serde(flatten)]
    pub extra_fields: ExtraFields,
}

#[cfg(test)]
mod test {
    use std::{path::Path};

    use crate::{utils::assert_round_trip, team::{Team, TeamPlayer}};

    #[test]
    #[tracing_test::traced_test]
    fn team_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        assert_round_trip::<Team>(Path::new("test_data/s2_team.json"))?;
        assert_round_trip::<TeamPlayer>(Path::new("test_data/s2_team_player.json"))?;

        assert!(!logs_contain("not recognized"));

        Ok(())
    }
}