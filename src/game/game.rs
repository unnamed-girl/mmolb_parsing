use std::collections::HashMap;

use crate::{enums::{Day, GameStat, LeagueScale, SeasonStatus, Slot}, game::{Event, PitcherEntry, Weather}, utils::{AddedLaterResult, ExtraFields, MaybeRecognizedResult}};
use crate::utils::{MaybeRecognizedHelper, AddedLaterHelper};

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
    #[serde(rename = "PitcherEntry", default = "AddedLaterHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "AddedLaterHelper<_>")]
    pub pitcher_entries: AddedLaterResult<HashMap<String, PitcherEntry>>,
    
    /// PitchersUsed was not retroactively added to old games
    /// 
    /// TeamID -> List of pitchers for that team.
    #[serde(default = "AddedLaterHelper::default_result", skip_serializing_if = "AddedLaterResult::is_err")]
    #[serde_as(as = "AddedLaterHelper<_>")]
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

    pub event_log: Vec<Event>,

    #[serde(flatten)]
    pub extra_fields: ExtraFields,
}
