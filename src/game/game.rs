use std::collections::HashMap;

use crate::{enums::{Day, GameStat, LeagueScale, MaybeRecognized, SeasonStatus, Slot}, game::{event::RawEvent, weather::RawWeather, Event, PitcherEntry, Weather}, serde_utils::AddedLaterMarker};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "RawGame", into = "RawGame")]
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
    pub realm_id: String,
    /// TeamID -> PlayerID -> Stat -> Value
    pub stats: HashMap<String, HashMap<String, HashMap<MaybeRecognized<GameStat>, i32>>>,

    pub(crate) pitcher_entries_format: AddedLaterMarker,
    /// TeamID -> PitcherEntry for that team.
    pub pitcher_entries: HashMap<String, PitcherEntry>,
    
    pub(crate) pitchers_used_format: AddedLaterMarker,
    /// TeamID -> List of pitchers for that team.
    pub pitchers_used: HashMap<String, Vec<String>>,

    pub away_lineup: Vec<MaybeRecognized<Slot>>,
    pub home_lineup: Vec<MaybeRecognized<Slot>>,
    pub day_id: String,
    pub season_id: String,
    pub season_status: MaybeRecognized<SeasonStatus>,
    pub league_scale: MaybeRecognized<LeagueScale>,

    pub event_log: Vec<Event>,

    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct RawGame {
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

    pub weather: RawWeather,
    #[serde(rename = "Realm")]
    pub realm_id: String,
    pub stats: HashMap<String, HashMap<String, HashMap<String, i32>>>,

    #[serde(rename = "PitcherEntry", default, skip_serializing_if = "Option::is_none")]
    pub pitcher_entries: Option<HashMap<String, PitcherEntry>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitchers_used: Option<HashMap<String, Vec<String>>>,

    pub away_lineup: Vec<MaybeRecognized<Slot>>,
    pub home_lineup: Vec<MaybeRecognized<Slot>>,
    #[serde(rename = "DayID")]
    pub day_id: String,
    #[serde(rename = "SeasonID")]
    pub season_id: String,
    pub season_status: MaybeRecognized<SeasonStatus>,
    #[serde(rename = "League")]
    pub league_scale: MaybeRecognized<LeagueScale>,

    pub event_log: Vec<RawEvent>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}


impl From<RawGame> for Game {
    fn from(value: RawGame) -> Self {
        let RawGame { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, pitcher_entries, event_log, extra_fields, pitchers_used, away_lineup, home_lineup, day_id, season_id, season_status, league_scale } = value;
        let weather = weather.into();
        let event_log: Vec<Event> = event_log.into_iter().map(|event| event.into()).collect();
        let stats  = stats.into_iter().map(|(team, players)| {
            let players = players.into_iter().map(|(player, stats)| {
                let stats = stats.into_iter().map(|(stat, value)| {
                    let stat: MaybeRecognized<GameStat> = stat.as_str().into();
                    (stat, value)
                }).collect();
                (player, stats)
            }).collect();
            (team, players)
            }
        ).collect();

        if extra_fields.len() > 0 {
            tracing::error!("Deserialization found extra fields: {:?}", extra_fields)
        }

        let pitchers_used_format = AddedLaterMarker::new(&pitchers_used);
        let pitchers_used = pitchers_used.unwrap_or_default();

        let pitcher_entries_format = AddedLaterMarker::new(&pitcher_entries);
        let pitcher_entries = pitcher_entries.unwrap_or_default();

        Game { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, pitcher_entries, event_log, extra_fields, pitchers_used, pitcher_entries_format, pitchers_used_format, away_lineup, home_lineup, day_id, season_id, season_status, league_scale }
    }
}
impl From<Game> for RawGame {
    fn from(value: Game) -> Self {
        let Game { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, event_log, extra_fields, pitcher_entries, pitchers_used, pitchers_used_format, pitcher_entries_format, away_lineup, home_lineup, day_id, season_id, season_status, league_scale } = value;
        let weather = weather.into();
        let event_log = event_log.into_iter().map(|event| event.into()).collect();
        let stats: HashMap<String, HashMap<String, HashMap<String, i32>>>  = stats.into_iter().map(|(team, players)|
            (team, players.into_iter().map(|(player, stats)|
                (player, stats.into_iter().map(|(stat, value)| (stat.to_string(), value)).collect())
            ).collect())
        ).collect();

        let pitcher_entries = pitcher_entries_format.wrap(pitcher_entries);
        let pitchers_used = pitchers_used_format.wrap(pitchers_used);

        RawGame { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, event_log, pitcher_entries, extra_fields, pitchers_used, away_lineup, home_lineup, day_id, season_id, season_status, league_scale }
    }
}
