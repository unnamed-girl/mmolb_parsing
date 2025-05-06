use std::{collections::HashMap, str::FromStr};
use serde::{Deserialize, Serialize};

use crate::{enums::{EventType, GameStats, Inning, PitchType}, raw_game::{RawEvent, RawGame, RawWeather, RawZone}};


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
    pub day: u32,
    pub state: String,

    pub weather: Weather,
    pub realm_id: String,
    /// TeamID -> PlayerID -> Stat -> Value
    pub stats: HashMap<String, HashMap<String, HashMap<GameStats, i32>>>,

    pub event_log: Vec<Event>,
}
impl From<RawGame> for Game {
    fn from(value: RawGame) -> Self {
        let weather = value.weather.into();
        let event_log = value.event_log.into_iter().map(|event| event.into()).collect();
        let realm_id = value.realm;
        let stats  = value.stats.into_iter().map(|(team, players)|
            (team, players.into_iter().map(|(player, stats)|
                (player, stats.into_iter().map(|(stat, value)| (GameStats::from_str(&stat).expect(&stat), value)).collect())
            ).collect())
        ).collect();
        Self { away_sp: value.away_sp, away_team_abbreviation: value.away_team_abbreviation, away_team_color: value.away_team_color, away_team_emoji: value.away_team_emoji, away_team_id: value.away_team_id, away_team_name: value.away_team_name, home_sp: value.home_sp, home_team_abbreviation: value.home_team_abbreviation, home_team_color: value.home_team_color, home_team_emoji: value.home_team_emoji, home_team_id: value.home_team_id, home_team_name: value.home_team_name, season: value.season, day: value.day, state: value.state, 
            weather, event_log, realm_id, stats
        }
    }
}
impl From<Game> for RawGame {
    fn from(value: Game) -> Self {
        let weather = value.weather.into();
        let event_log = value.event_log.into_iter().map(|event| event.into()).collect();
        let realm = value.realm_id;
        let stats  = value.stats.into_iter().map(|(team, players)|
            (team, players.into_iter().map(|(player, stats)|
                (player, stats.into_iter().map(|(stat, value)| (stat.to_string(), value)).collect())
            ).collect())
        ).collect();
        Self { away_sp: value.away_sp, away_team_abbreviation: value.away_team_abbreviation, away_team_color: value.away_team_color, away_team_emoji: value.away_team_emoji, away_team_id: value.away_team_id, away_team_name: value.away_team_name, home_sp: value.home_sp, home_team_abbreviation: value.home_team_abbreviation, home_team_color: value.home_team_color, home_team_emoji: value.home_team_emoji, home_team_id: value.home_team_id, home_team_name: value.home_team_name, season: value.season, day: value.day, state: value.state, 
            weather, event_log, realm, stats
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Weather {
    pub emoji: String,
    pub name: String,
    pub tooltip: String
}
impl From<RawWeather> for Weather {
    fn from(value: RawWeather) -> Self {
        Self { emoji: value.emoji, name: value.name, tooltip: value.tooltip }
    }
}
impl From<Weather> for RawWeather {
    fn from(value: Weather) -> Self {
        Self { emoji: value.emoji, name: value.name, tooltip: value.tooltip }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub inning: Inning,

    pub away_score: u8,
    pub home_score: u8,

    pub balls: Option<u8>,
    pub strikes: Option<u8>,
    pub outs: Option<u8>,

    pub on_1b: bool,
    pub on_2b: bool,
    pub on_3b: bool,
    
    pub on_deck: MaybePlayer<String>,
    pub batter: MaybePlayer<String>,

    pub pitch: Option<Pitch>,

    pub event: EventType,
    pub message: String
}
impl From<RawEvent> for Event {
    fn from(value: RawEvent) -> Self {
        let inning = match (value.inning, value.inning_side) {
            (0, 1) => Inning::BeforeGame,
            (number, 2) => Inning::AfterGame { total_inning_count: number - 1 },
            (number, side) => Inning::DuringGame { number, batting_side: side.try_into().unwrap() }
        };
        let pitch_info = (!value.pitch_info.is_empty()).then_some(value.pitch_info);
        let zone = if let RawZone::Number(n) = value.zone {Some(n)} else {None};

        let batter = value.batter.into();
        let on_deck = value.on_deck.into();

        let pitch = pitch_info.zip(zone).map(|(pitch_info, zone)| Pitch::new(pitch_info, zone));
        
        let event = EventType::from_str(&value.event).expect("Events to be known");
        Self {inning, pitch, batter, on_deck, event, away_score: value.away_score, home_score: value.home_score, balls: value.balls, strikes: value.strikes, outs: value.outs, on_1b: value.on_1b, on_2b: value.on_2b, on_3b: value.on_3b, message: value.message }
    }
}
impl From<Event> for RawEvent {
    fn from(value: Event) -> Self {
        let (inning, inning_side) = match value.inning {
            Inning::BeforeGame => (0, 1),
            Inning::DuringGame { number, batting_side: side } => (number, side.into()),
            Inning::AfterGame { total_inning_count } => (total_inning_count + 1, 2)
        };
        let (pitch_info, zone) = value.pitch.map(Pitch::unparse).map(|(pitch, zone)| (pitch, RawZone::Number(zone))).unwrap_or(("".to_string(), RawZone::String("".to_string())));
        let event = value.event.to_string();

        let batter = value.batter.unparse();
        let on_deck = value.on_deck.unparse();

        Self {inning, inning_side, pitch_info, zone, event, batter, on_deck, away_score: value.away_score, home_score: value.home_score, balls: value.balls, strikes: value.strikes, outs: value.outs, on_1b: value.on_1b, on_2b: value.on_2b, on_3b: value.on_3b, message: value.message }
    }
}

/// mmmolb currently has three possible values for the batter and on_deck fields:
/// - The name of a batter (used when there is a batter)
/// - An empty string (used when there is no batter during the game)
/// - null (used before the game)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaybePlayer<S> {
    Player(S),
    EmptyString,
    Null
}
impl<S> MaybePlayer<S> {
    pub fn player(self) -> Option<S> {
        match self {
            MaybePlayer::Player(player) => Some(player),
            MaybePlayer::EmptyString => None,
            MaybePlayer::Null => None
        }
    }
}
impl MaybePlayer<String> {
    pub fn map_as_str(&self) -> MaybePlayer<&str> {
        match self {
            MaybePlayer::Player(player) => MaybePlayer::Player(player.as_str()),
            MaybePlayer::EmptyString => MaybePlayer::EmptyString,
            MaybePlayer::Null => MaybePlayer::Null
        }
    }
}
impl<S: From<&'static str>> MaybePlayer<S> {
    pub fn unparse(self) -> Option<S> {
        match self {
            MaybePlayer::Player(player) => Some(player),
            MaybePlayer::EmptyString => Some(S::from("")),
            MaybePlayer::Null => None
        }
    }
}
impl<S: PartialEq<&'static str>> From<Option<S>> for MaybePlayer<S> {
    fn from(value: Option<S>) -> Self {
        match value {
            Some(player) => if player == "" {
                MaybePlayer::EmptyString
            } else {
                MaybePlayer::Player(player)
            },
            None => MaybePlayer::Null
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pitch  {
    pub speed: f32,
    pub pitch_type: PitchType,
    pub zone: u8,
}
impl Pitch {
    pub fn new(pitch_info: String, zone: u8) -> Self {
        let mut iter = pitch_info.split(" MPH ");
        let pitch_speed = iter.next().unwrap().parse().unwrap();
        let pitch_type = iter.next().unwrap().try_into().unwrap();
        Self { speed: pitch_speed, pitch_type, zone }
    }
    pub fn unparse(self) -> (String, u8) {
        let speed = format!("{:.1}", self.speed);
        // let speed = speed.strip_suffix(".0").unwrap_or(speed.as_str());
        let pitch_info = format!("{speed} MPH {}", self.pitch_type);
        (pitch_info, self.zone)
    }
}