use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use serde_with::serde_as;

use crate::{enums::{BallparkSuffix, GameStat, Position, PositionType, RecordType, Slot}, feed_event::FeedEvent, player::PlayerEquipment, utils::{extra_fields_deserialize, AddedLaterResult, ExpectNone, MaybeRecognizedResult, NotRecognized}, RemovedLaterResult};
use crate::utils::{MaybeRecognizedHelper, SometimesMissingHelper};
use super::raw_team::{RawTeamPlayer};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Team {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub(super) _id: Option<String>,
    pub abbreviation: String,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub active: RemovedLaterResult<bool>,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub augments: RemovedLaterResult<u16>,
    pub championships: u8,
    pub color: String,
    pub emoji: String,

    /// Only present on some deleted teams
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub feed: AddedLaterResult<Vec<FeedEvent>>,

    /// Not present on some deleted teams.
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub motes_used: AddedLaterResult<u8>,

    pub location: String,
    pub full_location: String,
    pub league: Option<String>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_name: AddedLaterResult<String>,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    pub ballpark_suffix: AddedLaterResult<MaybeRecognizedResult<BallparkSuffix>>,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_use_city: AddedLaterResult<bool>,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_word_1: AddedLaterResult<Option<String>>,
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_word_2: AddedLaterResult<Option<String>>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub eligible: AddedLaterResult<bool>,

    /// no team modifications have been seen, so left as raw json
    #[serde_as(as = "Vec<ExpectNone<_>>")]
    pub modifications: Vec<Option<serde_json::Value>>,
    pub name: String,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub motto: RemovedLaterResult<Option<String>>,

    #[serde(rename = "OwnerID")]
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub owner_id: RemovedLaterResult<Option<String>>,

    pub players: Vec<TeamPlayer>,
    #[serde_as(as = "HashMap<MaybeRecognizedHelper<_>, _>")]
    pub record: HashMap<Result<RecordType, NotRecognized>, TeamRecord>,
    pub season_records: HashMap<String, String>,

    
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub inventory: AddedLaterResult<Vec<PlayerEquipment>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fund: Option<i32>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
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
    pub position: Option<MaybeRecognizedResult<Position>>,

    pub slot: AddedLaterResult<MaybeRecognizedResult<Slot>>,

    pub(crate) position_type_overridden: bool,
    pub position_type: MaybeRecognizedResult<PositionType>,


    pub stats: AddedLaterResult<HashMap<MaybeRecognizedResult<GameStat>, i32>>,
    
    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::{path::Path};

    use crate::{team::{Team, TeamPlayer}, utils::{assert_round_trip, no_tracing_errs}};

    #[test]
    fn team_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();

        assert_round_trip::<Team>(Path::new("test_data/s2_team.json"))?;
        assert_round_trip::<TeamPlayer>(Path::new("test_data/s2_team_player.json"))?;

        drop(no_tracing_errs);
        Ok(())
    }
}