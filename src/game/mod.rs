use serde::{Deserialize, Deserializer, Serialize};
use serde_with::serde_as;

use crate::enums::{PitchType};
use crate::utils::{maybe_recognized_from_str, maybe_recognized_to_string, MaybeRecognizedHelper, MaybeRecognizedResult};

pub(crate) mod game;
pub(crate) mod event;
pub(crate) mod weather;

pub use event::Event;
pub use game::Game;
pub use weather::Weather;

/// mmmolb currently has three possible values for the batter and on_deck fields:
/// - The name of a batter (used when there is a batter)
/// - An empty string (used when there is no batter during the game)
/// - null (used before the game)
#[derive(Debug, Clone, PartialEq)]
pub enum MaybePlayer<S> {
    Player(S),
    EmptyString,
    Null
}
impl<T: Serialize> Serialize for MaybePlayer<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            MaybePlayer::Player(player) => player.serialize(serializer),
            MaybePlayer::EmptyString => "".serialize(serializer),
            MaybePlayer::Null => None::<T>.serialize(serializer)
        }
    }
}
impl<'de, T: Deserialize<'de> + PartialEq<&'static str>> Deserialize<'de> for MaybePlayer<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de> {
        let t = Option::<T>::deserialize(deserializer)?;
        Ok(MaybePlayer::from(t))
    }
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

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pitch  {
    pub speed: f32,
    #[serde_as(as = "MaybeRecognizedHelper<_>")]
    pub pitch_type: MaybeRecognizedResult<PitchType>,
    pub zone: u8,
}
impl Pitch {
    pub fn new(pitch_info: String, zone: u8) -> Self {
        let mut iter = pitch_info.split(" MPH ");
        let pitch_speed = iter.next().unwrap().parse().unwrap();
        let pitch_type = maybe_recognized_from_str(iter.next().unwrap());
        Self { speed: pitch_speed, pitch_type, zone }
    }
    pub fn unparse(self) -> (String, u8) {
        let speed = format!("{:.1}", self.speed);
        // let speed = speed.strip_suffix(".0").unwrap_or(speed.as_str());
        let pitch_info = format!("{speed} MPH {}", maybe_recognized_to_string(&self.pitch_type));
        (pitch_info, self.zone)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitcherEntry {
    #[serde(rename = "bf", deserialize_with = "bf_de")]
    pub batters_faced: u8,
    pub id: String
}

fn bf_de<'de, D>(deserializer: D) -> Result<u8, D::Error> where D: Deserializer<'de> {
    let r = u8::deserialize(deserializer);
    if let Ok(n)= r {
        if n > 0 {
            tracing::error!("Thought batters_faced is always 0")
        }
    }
    r
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use tracing_test::traced_test;

    use crate::{utils::{assert_round_trip, no_tracing_errs}, Game};


    #[test]
    fn game_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let no_tracing_errs = no_tracing_errs();
        assert_round_trip::<Game>(Path::new("test_data/s2_d240_game.json"))?;

        drop(no_tracing_errs);
        Ok(())
    }

    #[test]
    #[traced_test]
    fn extra_fields() -> Result<(), Box<dyn std::error::Error>> {
        assert_round_trip::<Game>(Path::new("test_data/game_extra_fields.json"))?;

        assert!(!logs_contain("not recognized"));

        logs_assert(|lines: &[&str]| {
            match lines.iter().filter(|line| line.contains("extra fields")).count() {
                2 => Ok(()),
                n => Err(format!("Expected two extra fields, but found {}", n)),
            }
        });
        Ok(())
    }
}
