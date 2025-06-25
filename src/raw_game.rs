use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

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
pub(crate) struct RawWeather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct RawEvent {
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

    #[serde(with = "none_as_empty_string")]
    pub zone: Option<u8>,

    pub event: String,
    pub message: String,

    #[serde(default, skip_serializing_if = "APIHistory::is_missing")]
    pub index: IndexHistory,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

trait APIHistory {
    fn is_missing(&self) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants, Default)]
#[strum_discriminants(derive(Serialize, Deserialize))]
#[serde(untagged)]
pub(crate) enum IndexHistory {
    #[default]
    Season0,
    #[serde(with = "none_as_empty_string")]
    Season2(Option<u16>)
}

impl APIHistory for IndexHistory {
    fn is_missing(&self) -> bool {
        if let IndexHistory::Season0 = self {
            true
        } else {
            false
        }
    }
}

mod none_as_empty_string {
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, T: Deserialize<'de>, D>(d: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ValueOrEmptyString<'a, T> {
            String(String),
            S(&'a str),
            R(T),
        }

        match ValueOrEmptyString::deserialize(d) {
            Ok(ValueOrEmptyString::R(r)) => Ok(Some(r)),
            Ok(ValueOrEmptyString::S(s)) if s.is_empty() => Ok(None),
            Ok(ValueOrEmptyString::S(_)) => Err(D::Error::custom("only empty strings may be provided")),
            Ok(ValueOrEmptyString::String(s)) if s.is_empty() => Ok(None),
            Ok(ValueOrEmptyString::String(_)) => Err(D::Error::custom("only empty strings may be provided")),
            Err(err) => Err(err),
        }
    }

    pub fn serialize<S, T: Serialize>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match value {
            Some(t) => t.serialize(serializer),
            None => "".serialize(serializer)
        }
    }
}