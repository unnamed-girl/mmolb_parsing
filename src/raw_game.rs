use std::collections::HashMap;

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct RawGame {
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
    pub day: u32,
    pub state: String,

    pub weather: RawWeather,
    pub realm: String,
    /// TeamID -> PlayerID -> Stat -> Value
    pub stats: HashMap<String, HashMap<String, HashMap<String, i32>>>,

    pub event_log: Vec<RawEvent>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct RawWeather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawEvent {
    /// 0 is before the game has started
    pub inning: u8,
    
    /// 2 when the game is over
    pub inning_side: u8,

    pub away_score: u8,
    pub home_score: u8,

    pub balls: Option<u8>,
    pub strikes: Option<u8>,
    pub outs: Option<u8>,

    pub on_1b: bool,
    pub on_2b: bool,
    pub on_3b: bool,
    
    /// Empty string between innings, null before game
    pub on_deck: Option<String>,
    /// Empty string between innings, null before game
    pub batter: Option<String>,
    /// Empty string between innings, null before game
    pub pitcher: Option<String>,

    /// Empty if none
    pub pitch_info: String,
    // RawZone::String("") when none, else RawZone::Number(n)
    pub zone: RawZone,

    pub event: String,
    pub message: String,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RawZone {
    String(String),
    Number(u8)
}