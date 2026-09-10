use crate::utils::TimestampHelper;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::HashMap;

use super::raw_team::RawTeamPlayer;
use crate::enums::{Attribute, BenchRole, FullSlot, FullSlotLabel, SlotType};
use crate::player::{BoonCollection, FoodBuff, Modification, PendingLevelUp};
use crate::utils::{maybe_recognized_from_str, MaybeRecognizedHelper, SometimesMissingHelper};
use crate::{
    enums::{BallparkSuffix, GameStat, Position, PositionType, RecordType},
    feed_event::FeedEvent,
    player::PlayerEquipment,
    utils::{
        extra_fields_deserialize, AddedLaterResult, ExpectNone, MaybeRecognizedResult,
        NotRecognized,
    },
    RemovedLaterResult,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TeamPlayerCollection {
    Vec(Vec<TeamPlayer>),
    // Using this third-party map type instead of HashMap to preserve key order
    Map(indexmap::IndexMap<String, TeamPlayer>),
}

impl From<TeamPlayerCollection> for Vec<TeamPlayer> {
    fn from(val: TeamPlayerCollection) -> Self {
        match val {
            TeamPlayerCollection::Vec(v) => v,
            TeamPlayerCollection::Map(m) => m
                .into_iter()
                .map(|(k, mut v)| {
                    v.slot = Ok(maybe_recognized_from_str(&k));
                    v
                })
                .collect(),
        }
    }
}

impl From<Vec<TeamPlayer>> for TeamPlayerCollection {
    fn from(value: Vec<TeamPlayer>) -> Self {
        TeamPlayerCollection::Vec(value)
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Team {
    // Cashews id
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub(super) _id: Option<String>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub abbreviation: RemovedLaterResult<String>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub active: RemovedLaterResult<bool>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub augments: RemovedLaterResult<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub championships: Option<u8>,
    pub color: String,
    pub emoji: String,

    /// Only present on some deleted teams
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub feed: AddedLaterResult<Vec<FeedEvent>>,

    /// Not present on some deleted teams.
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub motes_used: AddedLaterResult<u8>,

    pub location: String,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub full_location: RemovedLaterResult<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub league: Option<String>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_name: AddedLaterResult<String>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<MaybeRecognizedHelper<_>>")]
    pub ballpark_suffix: AddedLaterResult<MaybeRecognizedResult<BallparkSuffix>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_use_city: AddedLaterResult<bool>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_word_1: AddedLaterResult<Option<String>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub ballpark_word_2: AddedLaterResult<Option<String>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub eligible: AddedLaterResult<bool>,

    /// no team modifications have been seen, so left as raw json
    ///    TODO: The above is now incorrect. Add team modifications support.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<Vec<ExpectNone<_>>>")]
    pub modifications: Option<Vec<Option<serde_json::Value>>>,
    pub name: String,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub motto: RemovedLaterResult<Option<String>>,

    #[serde(rename = "OwnerID")]
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub owner_id: RemovedLaterResult<Option<String>>,

    /// For all current teams this is a Vec. For some historical team versions this was a map.
    pub players: TeamPlayerCollection,
    #[serde_as(as = "HashMap<MaybeRecognizedHelper<_>, _>")]
    pub record: HashMap<Result<RecordType, NotRecognized>, TeamRecord>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_records: Option<HashMap<String, String>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub inventory: AddedLaterResult<Vec<PlayerEquipment>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub fund: Option<i32>,

    // This appears to have been `null` (rather than just missing)
    // for newly created teams during the first half of Season 10.
    // Whatever caused this seems to have been fixed during the
    // Season 10 Superstar Break.
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub away_games: AddedLaterResult<Option<u32>>,
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "AddedLaterResult::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub bench: AddedLaterResult<Bench>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub manager: AddedLaterResult<String>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub auto_accept_exhibitions: AddedLaterResult<bool>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub is_playing: AddedLaterResult<bool>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Option<TimestampHelper>>")]
    pub last_roster_swap_at: AddedLaterResult<Option<DateTime<Utc>>>,

    // Err(_) => key does not exist
    // Ok(None) => key exists with value `null`
    // Ok(Some(Err())) => key exists with unrecognized value
    // Ok(Some(Ok())) => key exists with recognized value
    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<Option<MaybeRecognizedHelper<_>>>")]
    pub lineup_priority: AddedLaterResult<Option<MaybeRecognizedResult<Attribute>>>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub swap_available: AddedLaterResult<bool>,

    #[serde(
        default = "SometimesMissingHelper::default_result",
        skip_serializing_if = "Result::is_err"
    )]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub swap_season_restricted: AddedLaterResult<bool>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct TeamRecord {
    pub losses: u16,
    pub run_differential: i32,
    pub wins: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(from = "RawTeamPlayer", into = "RawTeamPlayer")]
pub struct TeamPlayer {
    pub emoji: String,
    pub first_name: String,
    pub last_name: String,
    pub suffix: AddedLaterResult<Option<String>>,
    pub number: u8,
    pub player_id: String,

    /// Undrafted player's positions are deeply unreliable.
    pub position: Option<MaybeRecognizedResult<Position>>,
    pub(crate) actual_position: String,

    pub slot_type: AddedLaterResult<MaybeRecognizedResult<SlotType>>,
    pub slot: AddedLaterResult<MaybeRecognizedResult<FullSlot>>,
    pub slot_label: AddedLaterResult<MaybeRecognizedResult<FullSlotLabel>>,

    pub position_type: AddedLaterResult<MaybeRecognizedResult<PositionType>>,

    pub stats: AddedLaterResult<HashMap<MaybeRecognizedResult<GameStat>, i32>>,

    pub bench_index: AddedLaterResult<Option<u32>>,
    pub bench_role: AddedLaterResult<Option<BenchRole>>,

    pub food_buffs: AddedLaterResult<Vec<FoodBuff>>,

    pub greater_boon: AddedLaterResult<BoonCollection>,
    pub lesser_boon: AddedLaterResult<BoonCollection>,
    pub modifications: AddedLaterResult<Vec<Modification>>,

    pub level: AddedLaterResult<u32>,
    pub pending_level_ups: AddedLaterResult<Vec<PendingLevelUp>>,
    pub free_recomp: Option<bool>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Bench {
    pub batters: Vec<TeamPlayer>,
    pub pitchers: Vec<TeamPlayer>,
    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::{
        team::{Team, TeamPlayer},
        utils::{assert_round_trip, no_tracing_errs},
    };

    #[test]
    fn team_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();

        assert_round_trip::<Team>(Path::new("test_data/s2_team.json"))?;
        assert_round_trip::<TeamPlayer>(Path::new("test_data/s2_team_player.json"))?;

        drop(no_tracing_errs);
        Ok(())
    }
}
