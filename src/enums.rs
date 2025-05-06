use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter, EnumString};

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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, EnumString, Display)]
pub enum TopBottom {
    #[strum(to_string = "top")]
    Top,
    #[strum(to_string = "bottom")]
    Bottom
}
impl TopBottom {
    pub fn flip(self) -> Self {
        match self {
            TopBottom::Top => TopBottom::Bottom,
            TopBottom::Bottom => TopBottom::Top,
        }
    }
    pub fn homeaway(self) -> HomeAway {
        match self {
            TopBottom::Top => HomeAway::Away,
            TopBottom::Bottom => HomeAway::Home,
        }
    }
    pub fn is_top(self) -> bool {
        match self {
            TopBottom::Top => true,
            TopBottom::Bottom => false,
        }
    }
    pub fn is_bottom(self) -> bool {
        match self {
            TopBottom::Top => false,
            TopBottom::Bottom => true,
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
impl TryFrom<u8> for TopBottom {
    type Error = NotASide;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Top),
            1 => Ok(Self::Bottom),
            _ => Err(NotASide(value))
        }
    }
}
impl From<TopBottom> for u8 {
    fn from(value: TopBottom) -> u8 {
        match value {
            TopBottom::Top => 0,
            TopBottom::Bottom => 1,
        }
    }
}

impl From<HomeAway> for TopBottom {
    fn from(value: HomeAway) -> Self {
        value.topbottom()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, EnumString, Display)]
pub enum HomeAway {
    Away,
    Home,
}
impl HomeAway {
    pub fn flip(self) -> Self {
        match self {
            Self::Away => Self::Home,
            Self::Home => Self::Away
        }
    }
    pub fn topbottom(self) -> TopBottom {
        match self {
            HomeAway::Away => TopBottom::Top,
            HomeAway::Home => TopBottom::Bottom
        }
    }
    pub fn is_home(self) -> bool {
        match self {
            HomeAway::Home => true,
            HomeAway::Away => false,
        }
    }
    pub fn is_away(self) -> bool {
        match self {
            HomeAway::Home => false,
            HomeAway::Away => true,
        }
    }
}

impl From<TopBottom> for HomeAway {
    fn from(value: TopBottom) -> Self {
        value.homeaway()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Inning {
    BeforeGame,
    DuringGame {number: u8, batting_side: TopBottom},
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
    pub fn batting_team(self) -> Option<HomeAway> {
        if let Inning::DuringGame { batting_side: side, .. } = self {
            match side {
                TopBottom::Top => Some(HomeAway::Away),
                TopBottom::Bottom => Some(HomeAway::Home),
            }
        } else {
            None
        }
    }
    pub fn pitching_team(self) -> Option<HomeAway> {
        if let Inning::DuringGame { batting_side: side, .. } = self {
            match side {
                TopBottom::Top => Some(HomeAway::Home),
                TopBottom::Bottom => Some(HomeAway::Away),
            }
        } else {
            None
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
pub enum FairBallDestination {
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
pub enum FairBallType {
    #[strum(to_string = "ground ball")]
    GroundBall,
    #[strum(to_string = "fly ball")]
    FlyBall,
    #[strum(to_string = "line drive")]
    LineDrive,
    #[strum(to_string = "popup")]
    Popup,
}
impl FairBallType {
    pub fn verb_name(self) -> &'static str {
        match self {
            Self::GroundBall => "grounds",
            Self::FlyBall => "flies",
            Self::LineDrive => "lines",
            Self::Popup => "pops",
        }
    }
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
pub enum FieldingErrorType {
    #[strum(ascii_case_insensitive)]
    Throwing,
    #[strum(ascii_case_insensitive)]
    Fielding
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FoulType {
    #[strum(to_string = "tip")]
    Tip,
    #[strum(to_string = "ball")]
    Ball
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Base {
    #[strum(to_string = "home")]
    Home,
    #[strum(to_string = "first")]
    First,
    #[strum(to_string = "second")]
    Second,
    #[strum(to_string = "third")]
    Third,
}
impl Base {
    /// To string, such that home is "home" rather than "home base"
    pub fn to_base_string(self) -> &'static str {
        match self {
            Base::First => "first base",
            Base::Second => "second base",
            Base::Third => "third base",
            Base::Home => "home"
        }
    }
}
impl From<BaseNameVariants> for Base {
    fn from(value: BaseNameVariants) -> Self {
        match value {
            BaseNameVariants::First => Base::First,
            BaseNameVariants::FirstBase => Base::First,
            BaseNameVariants::OneB => Base::First,
            BaseNameVariants::Second => Base::Second,
            BaseNameVariants::SecondBase => Base::Second,
            BaseNameVariants::TwoB => Base::Second,
            BaseNameVariants::Third => Base::Third,
            BaseNameVariants::ThirdBase => Base::Third,
            BaseNameVariants::ThreeB => Base::Third,
            BaseNameVariants::Home => Base::Home
        }
    }
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BaseNameVariants {
    #[strum(to_string = "first")]
    First,
    #[strum(to_string = "first base")]
    FirstBase,
    #[strum(to_string = "1B")]
    OneB,
    #[strum(to_string = "second")]
    Second,
    #[strum(to_string = "second base")]
    SecondBase,
    #[strum(to_string = "2B")]
    TwoB,
    #[strum(to_string = "third base")]
    ThirdBase,
    #[strum(to_string = "third")]
    Third,
    #[strum(to_string = "3B")]
    ThreeB,
    #[strum(to_string = "home")]
    Home,
}
impl From<Base> for BaseNameVariants {
    fn from(value: Base) -> Self {
        match value {
            Base::First => BaseNameVariants::First,
            Base::Second => BaseNameVariants::Second,
            Base::Third => BaseNameVariants::Third,
            Base::Home => BaseNameVariants::Home,
        }
    }
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Distance {
    #[strum(to_string = "singles")]
    Single,
    #[strum(to_string = "doubles")]
    Double,
    #[strum(to_string = "triples")]
    Triple,
}

#[derive(Clone, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NowBattingBoxScore {
    FirstPA,
    Stats {
        stats: Vec<BatterStat>
    },
    NoStats
}


#[derive(Clone, Copy, Display, Debug, EnumDiscriminants, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[strum_discriminants(derive(EnumString, Display))]
pub enum BatterStat {
    HitsForAtBats {
        hits: u8,
        at_bats: u8
    },
    #[strum_discriminants(strum(to_string = "1B"))]
    FirstBases(u8),
    #[strum_discriminants(strum(to_string = "2B"))]
    SecondBases(u8),
    #[strum_discriminants(strum(to_string = "3B"))]
    ThirdBases(u8),
    #[strum_discriminants(strum(to_string = "HR"))]
    HomeRuns(u8),
    #[strum_discriminants(strum(to_string = "SF"))]
    SacrificeFlies(u8),
    #[strum_discriminants(strum(to_string = "PO"))]
    PopOuts(u8),
    #[strum_discriminants(strum(to_string = "LO"))]
    LineOuts(u8),
    #[strum_discriminants(strum(to_string = "SO"))]
    StrikeOuts(u8),
    #[strum_discriminants(strum(to_string = "FO"))]
    ForceOuts(u8),
    #[strum_discriminants(strum(to_string = "BB"))]
    BaseOnBalls(u8),
    #[strum_discriminants(strum(to_string = "HBP"))]
    HitByPitchs(u8),
    #[strum_discriminants(strum(to_string = "GIDP"))]
    GroundIntoDoublePlays(u8),
    #[strum_discriminants(strum(to_string = "CDP"))]
    CaughtDoublePlays(u8),
    #[strum_discriminants(strum(to_string = "FC"))]
    FieldersChoices(u8),
    #[strum_discriminants(strum(to_string = "F"))]
    Fouls(u8),
}
impl BatterStat {
    pub fn unparse(self) -> String {
        match self {
            BatterStat::FirstBases(count) |
            BatterStat::SecondBases(count) |
            BatterStat::ThirdBases(count) |
            BatterStat::LineOuts(count) |
            BatterStat::PopOuts(count) |
            BatterStat::Fouls(count) |
            BatterStat::ForceOuts(count) |
            BatterStat::HomeRuns(count) |
            BatterStat::BaseOnBalls(count) |
            BatterStat::GroundIntoDoublePlays(count) |
            BatterStat::SacrificeFlies(count) |
            BatterStat::CaughtDoublePlays(count) |
            BatterStat::FieldersChoices(count) |
            BatterStat::HitByPitchs(count) |
            BatterStat::StrikeOuts(count) => {
                format!("{count} {}", BatterStatDiscriminants::from(self))
            }
            BatterStat::HitsForAtBats { hits, at_bats } => {
                format!("{hits} for {at_bats}")
            }
        }
    }
}