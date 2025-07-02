use std::collections::HashMap;

use crate::{enums::{Day, GameStat, MaybeRecognized}, game::{event::RawEvent, weather::RawWeather, Event, PitcherEntry, Weather}, serde_utils::APIHistory};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoDiscriminant};

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

    pub(crate) pitcher_entries_format: PitcherEntryHistoryDiscriminants,
    /// TeamID -> PitcherEntry for that team.
    pub pitcher_entries: HashMap<String, PitcherEntry>,
    
    pub(crate) pitchers_used_format: PitchersUsedHistoryDiscriminants,
    /// TeamID -> List of pitchers for that team.
    pub pitchers_used: HashMap<String, Vec<String>>,


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

    #[serde(rename = "PitcherEntry", default, skip_serializing_if = "APIHistory::is_missing")]
    pub pitcher_entries: PitcherEntryHistory,
    #[serde(default, skip_serializing_if = "APIHistory::is_missing")]
    pub pitchers_used: PitchersUsedHistory,

    pub event_log: Vec<RawEvent>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum PitchersUsedHistory {
    #[default]
    Season0,
    Season2b(HashMap<String, Vec<String>>)
}
impl APIHistory for PitchersUsedHistory {
    fn is_missing(&self) -> bool {
        matches!(self, PitchersUsedHistory::Season0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum PitcherEntryHistory {
    #[default]
    Season0,
    Season2b(HashMap<String, PitcherEntry>)
}
impl APIHistory for PitcherEntryHistory {
    fn is_missing(&self) -> bool {
        matches!(self, PitcherEntryHistory::Season0)
    }
}

impl From<RawGame> for Game {
    fn from(value: RawGame) -> Self {
        let RawGame { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, pitcher_entries, event_log, extra_fields, pitchers_used } = value;
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

        let pitchers_used_format = pitchers_used.discriminant();
        let pitchers_used = match pitchers_used {
            PitchersUsedHistory::Season0 => HashMap::default(),
            PitchersUsedHistory::Season2b(pitchers_used) => pitchers_used,
        };

        let pitcher_entries_format = pitcher_entries.discriminant();
        let pitcher_entries = match pitcher_entries {
            PitcherEntryHistory::Season0 => HashMap::default(),
            PitcherEntryHistory::Season2b(pitcher_entries) => pitcher_entries
        };

        Game { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, pitcher_entries, event_log, extra_fields, pitchers_used, pitcher_entries_format, pitchers_used_format }
    }
}
impl From<Game> for RawGame {
    fn from(value: Game) -> Self {
        let Game { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, event_log, extra_fields, pitcher_entries, pitchers_used, pitchers_used_format, pitcher_entries_format } = value;
        let weather = weather.into();
        let event_log = event_log.into_iter().map(|event| event.into()).collect();
        let stats: HashMap<String, HashMap<String, HashMap<String, i32>>>  = stats.into_iter().map(|(team, players)|
            (team, players.into_iter().map(|(player, stats)|
                (player, stats.into_iter().map(|(stat, value)| (stat.to_string(), value)).collect())
            ).collect())
        ).collect();

        let pitcher_entries = match pitcher_entries_format {
            PitcherEntryHistoryDiscriminants::Season0 => PitcherEntryHistory::Season0,
            PitcherEntryHistoryDiscriminants::Season2b => PitcherEntryHistory::Season2b(pitcher_entries)
        };

        let pitchers_used = match pitchers_used_format {
            PitchersUsedHistoryDiscriminants::Season0 => PitchersUsedHistory::Season0,
            PitchersUsedHistoryDiscriminants::Season2b => PitchersUsedHistory::Season2b(pitchers_used)
        };

        RawGame { away_sp, away_team_abbreviation, away_team_color, away_team_emoji, away_team_id, away_team_name, home_sp, home_team_abbreviation, home_team_color, home_team_emoji, home_team_id, home_team_name, season, day, state, weather, realm_id, stats, event_log, pitcher_entries, extra_fields, pitchers_used }
    }
}
