use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{enums::{EventType, Inning, MaybeRecognized}, game::{MaybePlayer, Pitch}, serde_utils::{AddedLaterMarker, SomeOrEmptyString}};

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

    pub zone: SomeOrEmptyString<u8>,

    pub event: String,
    pub message: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<SomeOrEmptyString<u16>>,

    #[serde(flatten)]
    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub pitcher: MaybePlayer<String>,

    pub pitch: Option<Pitch>,

    pub event: MaybeRecognized<EventType>,
    pub message: String,

    /// Event Index, introduced in S2
    index_format: AddedLaterMarker,
    pub index: Option<u16>,

    pub extra_fields: serde_json::Map<String, serde_json::Value>,
}
impl From<RawEvent> for Event {
    fn from(value: RawEvent) -> Self {
        let inning = match (value.inning, value.inning_side) {
            (0, 1) => Inning::BeforeGame,
            (0, 2) => Inning::AfterGame { final_inning_number: 1 },
            (number, 2) => Inning::AfterGame { final_inning_number: number - 1 },
            (number, side) => Inning::DuringGame { number, batting_side: side.try_into().unwrap() }
        };
        let pitch_info = (!value.pitch_info.is_empty()).then_some(value.pitch_info);

        let batter = value.batter.into();
        let on_deck = value.on_deck.into();
        let pitcher = value.pitcher.into();

        let pitch = pitch_info.zip(value.zone.into()).map(|(pitch_info, zone)| Pitch::new(pitch_info, zone));
        
        let event = value.event.as_str().into();

        if !value.extra_fields.is_empty() {
            error!("Deserialization found extra fields: {:?}", value.extra_fields)
        }

        let index_format = AddedLaterMarker::new(&value.index);
        let index = value.index.unwrap_or_default().into();

        Self {index_format, inning, pitch, batter, pitcher, on_deck, event, away_score: value.away_score, home_score: value.home_score, balls: value.balls, strikes: value.strikes, outs: value.outs, on_1b: value.on_1b, on_2b: value.on_2b, on_3b: value.on_3b, message: value.message, extra_fields: value.extra_fields, index }
    }
}
impl From<Event> for RawEvent {
    fn from(value: Event) -> Self {
        let (inning, inning_side) = match value.inning {
            Inning::BeforeGame => (0, 1),
            Inning::DuringGame { number, batting_side: side } => (number, side.into()),
            Inning::AfterGame { final_inning_number: 1 } => (0, 2),
            Inning::AfterGame { final_inning_number } => (final_inning_number + 1, 2)
        };
        let (pitch_info, zone) = value.pitch.map(Pitch::unparse).map(|(pitch, zone)| (pitch, Some(zone))).unwrap_or(("".to_string(), None));
        let event = value.event.to_string();

        let batter = value.batter.unparse();
        let on_deck = value.on_deck.unparse();
        let pitcher = value.pitcher.unparse();

        let index = value.index_format.wrap(value.index.into());

        let zone = zone.into();

        Self {inning, inning_side, pitch_info, zone, event, batter, on_deck, pitcher, away_score: value.away_score, home_score: value.home_score, balls: value.balls, strikes: value.strikes, outs: value.outs, on_1b: value.on_1b, on_2b: value.on_2b, on_3b: value.on_3b, message: value.message, extra_fields: value.extra_fields, index }
    }
}
