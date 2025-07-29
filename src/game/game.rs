use std::collections::HashMap;

use crate::{enums::{Day, GameStat, LeagueScale, SeasonStatus, Slot}, game::{Event, PitcherEntry, Weather}, utils::{AddedLaterResult, extra_fields_deserialize, MaybeRecognizedResult}};
use crate::utils::{MaybeRecognizedHelper, SometimesMissingHelper, ExpectNone};

use serde::{Serialize, Deserialize};
use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Game {
    #[serde(rename = "AwaySP")]
    pub away_sp: String,
    pub away_team_abbreviation: String,
    pub away_team_color: String,
    pub away_team_emoji: String,
    #[serde(rename = "AwayTeamID")]
    pub away_team_id: String,
    pub away_team_name: String,

    #[serde(rename = "HomeSP")]
    pub home_sp: String,
    pub home_team_abbreviation: String,
    pub home_team_color: String,
    pub home_team_emoji: String,
    #[serde(rename = "HomeTeamID")]
    pub home_team_id: String,
    pub home_team_name: String,

    pub season: u32,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub day: MaybeRecognizedResult<Day>,
    pub state: String,

    pub weather: Weather,
    #[serde(rename = "Realm")]
    pub realm_id: String,
    
    /// TeamID -> PlayerID -> Stat -> Value
    #[serde_as(as = "HashMap<_, HashMap<_, HashMap<MaybeRecognizedHelper<_>, _>>>")]
    pub stats: HashMap<String, HashMap<String, HashMap<MaybeRecognizedResult<GameStat>, i32>>>,

    /// PitcherEntries were not retroactively added to old games
    /// 
    /// TeamID -> PitcherEntry for that team.
    #[serde(rename = "PitcherEntry", default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub pitcher_entries: AddedLaterResult<HashMap<String, PitcherEntry>>,
    
    /// PitchersUsed was not retroactively added to old games
    /// 
    /// TeamID -> List of pitchers for that team.
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub pitchers_used: AddedLaterResult<HashMap<String, Vec<String>>>,

    #[serde_as(as = "Vec<MaybeRecognizedHelper<_>>")]
    pub away_lineup: Vec<MaybeRecognizedResult<Slot>>,
    #[serde_as(as = "Vec<MaybeRecognizedHelper<_>>")]
    pub home_lineup: Vec<MaybeRecognizedResult<Slot>>,
    #[serde(rename = "DayID")]
    pub day_id: String,
    #[serde(rename = "SeasonID")]
    pub season_id: String,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub season_status: MaybeRecognizedResult<SeasonStatus>,
    #[serde(rename = "League")]
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub league_scale: MaybeRecognizedResult<LeagueScale>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "SometimesMissingHelper<ExpectNone<_>>")]
    pub(super) hype_end_index: AddedLaterResult<Option<serde_json::Value>>,

    
    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub aurora_photos: AddedLaterResult<Vec<AuroraPhoto>>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    /// ids
    pub ejected_players: AddedLaterResult<Vec<String>>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    pub geomagnetic_pending: AddedLaterResult<bool>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    /// Team id => bench
    pub original_bench: AddedLaterResult<HashMap<String, Bench>>,

    #[serde(default = "SometimesMissingHelper::default_result", skip_serializing_if = "Result::is_err")]
    #[serde_as(as = "SometimesMissingHelper<_>")]
    /// Team id => slot => player id
    pub original_rosters: AddedLaterResult<HashMap<String, HashMap<Slot, String>>>,

    pub event_log: Vec<Event>,

    #[serde(flatten, deserialize_with = "extra_fields_deserialize")]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuroraPhoto {
    pub luck: f64,
    /// id
    pub player: String,
    pub slot: Slot,
    pub team: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Bench {
    /// ids
    pub batters: Vec<String>,
    /// ids
    pub pitchers: Vec<String>
}
