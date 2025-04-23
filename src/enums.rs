use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display, PartialEq, Eq, Hash)]
pub enum EventType {
    PitchingMatchup,
    MoundVisit,
    GameOver,
    Field,
    HomeLineup,
    Recordkeeping,
    LiveNow,
    InningStart,
    Pitch,
    AwayLineup,
    InningEnd,
    PlayBall,
    NowBatting
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Side {
    Home,
    Away,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Inning {
    BeforeGame,
    DuringGame {number: u8, side: Side},
    AfterGame { total_inning_count: u8 }
}
impl Inning {
    pub fn number(self) -> Option<u8> {
        if let Inning::DuringGame { number, .. } = self {
            Some(number)
        } else {
            None
        }
    }
    pub fn side(self) -> Option<Side> {
        if let Inning::DuringGame { side, .. } = self {
            Some(side)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct NotASide(u8);
impl Display for NotASide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} is not 0 or 1 (2 means gameover, should not reach here)", self.0)
    }
}
impl TryFrom<u8> for Side {
    type Error = NotASide;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Home),
            1 => Ok(Self::Away),
            _ => Err(NotASide(value))
        }
    }
}
impl Into<u8> for Side {
    fn into(self) -> u8 {
        match self {
            Self::Home => 0,
            Self::Away => 1,
        }
    }
}

#[derive(EnumString, Display, Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum Position {
    #[strum(to_string = "P")]
    Pitcher,
    #[strum(to_string = "C")]
    Catcher,
    #[strum(to_string = "1B")]
    FirstBaseman,
    #[strum(to_string = "2B")]
    SecondBaseman,
    #[strum(to_string = "3B")]
    ThirdBaseman,
    #[strum(to_string = "SS")]
    ShortStop,
    #[strum(to_string = "LF")]
    LeftField,
    #[strum(to_string = "CF")]
    CenterField,
    #[strum(to_string = "RF")]
    RightField,
    #[strum(to_string = "SP")]
    StartingPitcher,
    #[strum(to_string = "RP")]
    ReliefPitcher,
    #[strum(to_string = "CL")]
    Closer,
    #[strum(to_string = "DH")]
    DesignatedHitter
}


#[derive(EnumString, Display, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum HitDestination {
    #[strum(to_string = "the shortstop")]
    ShortStop,
    #[strum(to_string = "the catcher")]
    Catcher,
    #[strum(to_string = "the pitcher")]
    Pitcher,

    #[strum(to_string = "first base")]
    FirstBase,
    #[strum(to_string = "second base")]
    SecondBase,
    #[strum(to_string = "third base")]
    ThirdBase,
    #[strum(to_string = "left field")]
    LeftField,
    #[strum(to_string = "center field")]
    CenterField,
    #[strum(to_string = "right field")]
    RightField,
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum HitType {
    #[strum(to_string = "ground ball")]
    GroundBall,
    #[strum(to_string = "fly ball")]
    FlyBall,
    #[strum(to_string = "line drive")]
    LineDrive,
    #[strum(to_string = "popup")]
    Popup,
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum PitchType {
    Fastball,
    Sinker,
    Slider,
    Changeup,
    Curveball,
    Cutter,
    Sweeper,
    #[strum(to_string = "Knuckle Curve")]
    KnuckleCurve,
    Splitter
}


#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StrikeType {
    #[strum(to_string = "looking")]
    Looking,
    #[strum(to_string = "swinging")]
    Swinging
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FielderError {
    Throwing,
    Fielding
}