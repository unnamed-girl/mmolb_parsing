use std::collections::HashMap;

use crate::{enums::{Day, GameStat, LeagueScale, MaybeRecognized, SeasonStatus, Slot}, game::{Event, PitcherEntry, Weather}, utils::ExtraFields, AddedLater};
use serde::{Serialize, Deserialize};

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
    pub day: MaybeRecognized<Day>,
    pub state: String,

    pub weather: Weather,
    #[serde(rename = "Realm")]
    pub realm_id: String,
    /// TeamID -> PlayerID -> Stat -> Value
    pub stats: HashMap<String, HashMap<String, HashMap<MaybeRecognized<GameStat>, i32>>>,

    /// PitcherEntries were not retroactively added to old games
    /// 
    /// TeamID -> PitcherEntry for that team.
    #[serde(rename = "PitcherEntry", default, skip_serializing_if = "AddedLater::skip")]
    pub pitcher_entries: AddedLater<HashMap<String, PitcherEntry>>,
    
    /// PitchersUsed was not retroactively added to old games
    /// 
    /// TeamID -> List of pitchers for that team.
    #[serde(default, skip_serializing_if = "AddedLater::skip")]
    pub pitchers_used: AddedLater<HashMap<String, Vec<String>>>,

    pub away_lineup: Vec<MaybeRecognized<Slot>>,
    pub home_lineup: Vec<MaybeRecognized<Slot>>,
    #[serde(rename = "DayID")]
    pub day_id: String,
    #[serde(rename = "SeasonID")]
    pub season_id: String,
    pub season_status: MaybeRecognized<SeasonStatus>,
    #[serde(rename = "League")]
    pub league_scale: MaybeRecognized<LeagueScale>,

    pub event_log: Vec<Event>,

    #[serde(flatten)]
    pub extra_fields: ExtraFields,
}
