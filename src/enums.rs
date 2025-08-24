use std::{convert::Infallible, fmt::{Debug, Display}, str::FromStr};

use nom::{branch::alt, bytes::complete::tag, character::complete::u8, combinator::{all_consuming, opt}, sequence::{preceded, separated_pair, terminated}, Parser};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use strum::{Display, EnumDiscriminants, EnumIter, EnumString, IntoDiscriminant, IntoStaticStr};
use serde_with::{SerializeDisplay, DeserializeFromStr};

/// Possible values of the "event" field of an mmolb event. 
#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, IntoStaticStr, Display, PartialEq, Eq, Hash, EnumIter)]
pub enum EventType {
    // Season 0
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
    NowBatting,

    // Season 1
    #[strum(to_string = "Weather_Delivery")]
    #[serde(rename = "Weather_Delivery")]
    WeatherDelivery,
    FallingStar,
    Weather,
    #[strum(to_string = "HRC_LiveNow")]
    #[serde(rename = "HRC_LiveNow")]
    HrcLiveNow,
    #[strum(to_string = "HRC_PitchingMatchup")]
    #[serde(rename = "HRC_PitchingMatchup")]
    HrcPitchingMatchup,
    #[strum(to_string = "HRC_BattingMatchup")]
    #[serde(rename = "HRC_BattingMatchup")]
    HrcBattingMatchup,
    #[strum(to_string = "HRC_PlayBall")]
    #[serde(rename = "HRC_PlayBall")]
    HrcPlayBall,
    #[strum(to_string = "HRC_Change")]
    #[serde(rename = "HRC_Change")]
    HrcChange,

    // Season 2
    #[strum(to_string = "Weather_Shipment")]
    #[serde(rename = "Weather_Shipment")]
    WeatherShipment,
    #[strum(to_string = "Weather_SpecialDelivery")]
    #[serde(rename = "Weather_SpecialDelivery")]
    WeatherSpecialDelivery,
    Balk,

    // Season 3
    #[strum(to_string = "Weather_Prosperity")]
    #[serde(rename = "Weather_Prosperity")]
    WeatherProsperity,

    // Season 4
    PhotoContest,

    // Season 5
    Party,
}

/// Top or bottom of an inning.
/// 
/// ```
/// use mmolb_parsing::enums::TopBottom;
/// use mmolb_parsing::enums::NotASide;
/// use mmolb_parsing::enums::HomeAway;
/// 
/// assert_eq!(TopBottom::Top.flip(), TopBottom::Bottom);
/// assert_eq!(TopBottom::Top.homeaway(), HomeAway::Away);
/// assert_eq!(TopBottom::Top.is_top(), true);
/// assert_eq!(TopBottom::Top.is_bottom(), false);
/// 
/// assert_eq!(TopBottom::from(HomeAway::Away), TopBottom::Top);
/// assert_eq!(TopBottom::try_from(0), Ok(TopBottom::Top));
/// assert_eq!(TopBottom::try_from(2), Err(NotASide(2)));
/// assert_eq!(u8::from(TopBottom::Bottom), 1);
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display, EnumIter, Default)]
pub enum TopBottom {
    #[default]
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
    
    /// Who is curently batting.
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

/// Error for TopBottom's TryFrom<u8> implementation: fails because the given number was not a valid side number.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NotASide(pub u8);
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

/// ```
/// use mmolb_parsing::enums::TopBottom;
/// use mmolb_parsing::enums::NotASide;
/// use mmolb_parsing::enums::HomeAway;
/// 
/// assert_eq!(HomeAway::Home.flip(), HomeAway::Away);
/// assert_eq!(HomeAway::Home.topbottom(), TopBottom::Bottom);
/// assert_eq!(HomeAway::Home.is_home(), true);
/// assert_eq!(HomeAway::Home.is_away(), false);
/// 
/// assert_eq!(HomeAway::from(TopBottom::Top), HomeAway::Away);
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display, EnumIter)]
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

    /// Converts to the side in which this team is batting.
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

/// Possible states for the current inning: before game/during game/after game. Inning number is 1-indexed: inning 0 is before the game.
/// 
/// ```
/// use mmolb_parsing::enums::Inning;
/// use mmolb_parsing::enums::TopBottom;
/// use mmolb_parsing::enums::HomeAway;
/// 
/// assert_eq!(Inning::DuringGame { number: 5, batting_side: TopBottom::Top }.next(false), Some(Inning::DuringGame { number: 5, batting_side: TopBottom::Bottom }));
/// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }.number(), Some(1));
/// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }.batting_team(), Some(HomeAway::Away));
/// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }.pitching_team(), Some(HomeAway::Home));
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum Inning {
    BeforeGame,
    DuringGame {number: u8, batting_side: TopBottom},
    AfterGame { final_inning_number: u8 }
}
impl Inning {
    /// The next inning. If `continue_if_overtime`, go to extra innings instead of ending the game at the 9th.
    /// 
    /// ```
    /// use mmolb_parsing::enums::Inning;
    /// use mmolb_parsing::enums::TopBottom;
    /// 
    /// assert_eq!(Inning::BeforeGame.next(false), Some(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }));
    /// assert_eq!(Inning::DuringGame { number: 9, batting_side: TopBottom::Bottom }.next(false), Some(Inning::AfterGame { final_inning_number:9 }));
    /// assert_eq!(Inning::DuringGame { number: 9, batting_side: TopBottom::Bottom }.next(true), Some(Inning::DuringGame { number: 10, batting_side: TopBottom::Top }));
    /// assert_eq!(Inning::AfterGame { final_inning_number: 9 }.next(true), None);
    /// ```
    pub fn next(self, continue_if_overtime: bool) -> Option<Self> {
        match self {
            Inning::BeforeGame => Some(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }),
            Inning::DuringGame { number, batting_side } => {
                if number >= 9 && !continue_if_overtime {
                    Some(Inning::AfterGame { final_inning_number: number })
                } else {
                    match batting_side {
                        TopBottom::Top => Some(Inning::DuringGame { number, batting_side: batting_side.flip() }),
                        TopBottom::Bottom => Some(Inning::DuringGame { number: number + 1, batting_side: batting_side.flip() })
                    }
                }
            }
            Inning::AfterGame { .. } => None
        }
    }
    /// The number of the current inning, during a game.
    /// 
    /// ```
    /// use mmolb_parsing::enums::Inning;
    /// use mmolb_parsing::enums::TopBottom;
    /// 
    /// assert_eq!(Inning::BeforeGame.number(), None);
    /// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }.number(), Some(1));
    /// assert_eq!(Inning::AfterGame {final_inning_number: 9}.number(), None);
    /// ```
    pub fn number(self) -> Option<u8> {
        if let Inning::DuringGame { number, .. } = self {
            Some(number)
        } else {
            None
        }
    }

    /// The side that is currently batting, during a game.
    /// 
    /// ```
    /// use mmolb_parsing::enums::Inning;
    /// use mmolb_parsing::enums::TopBottom;
    /// use mmolb_parsing::enums::HomeAway;
    /// 
    /// assert_eq!(Inning::BeforeGame.batting_team(), None);
    /// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }.batting_team(), Some(HomeAway::Away));
    /// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Bottom }.batting_team(), Some(HomeAway::Home));
    /// assert_eq!(Inning::AfterGame {final_inning_number: 9}.batting_team(), None);
    /// ```
    pub fn batting_team(self) -> Option<HomeAway> {
        if let Inning::DuringGame { batting_side, .. } = self {
            Some(batting_side.homeaway())
        } else {
            None
        }
    }

    /// The side that is currently pitching, during a game.
    /// 
    /// ```
    /// use mmolb_parsing::enums::Inning;
    /// use mmolb_parsing::enums::TopBottom;
    /// use mmolb_parsing::enums::HomeAway;
    /// 
    /// assert_eq!(Inning::BeforeGame.pitching_team(), None);
    /// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }.pitching_team(), Some(HomeAway::Home));
    /// assert_eq!(Inning::DuringGame { number: 1, batting_side: TopBottom::Bottom }.pitching_team(), Some(HomeAway::Away));
    /// assert_eq!(Inning::AfterGame {final_inning_number: 9}.pitching_team(), None);
    /// ```
    pub fn pitching_team(self) -> Option<HomeAway> {
        if let Inning::DuringGame { batting_side, .. } = self {
            Some(batting_side.flip().homeaway())
        } else {
            None
        }
    }
}


/// Player roster/fielding positions.
/// 
/// ```
/// use mmolb_parsing::enums::Position;
/// 
/// assert_eq!(Position::FirstBaseman.to_string(), "1B");
/// ```
#[derive(EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum Position {
    #[strum(to_string = "P")]
    #[serde(rename = "P")]
    Pitcher,
    #[strum(to_string = "C")]
    #[serde(rename = "C")]
    Catcher,
    #[strum(to_string = "1B")]
    #[serde(rename = "1B")]
    FirstBaseman,
    #[strum(to_string = "2B")]
    #[serde(rename = "2B")]
    SecondBaseman,
    #[strum(to_string = "3B")]
    #[serde(rename = "3B")]
    ThirdBaseman,
    #[strum(to_string = "SS")]
    #[serde(rename = "SS")]
    ShortStop,
    #[strum(to_string = "LF")]
    #[serde(rename = "LF")]
    LeftField,
    #[strum(to_string = "CF")]
    #[serde(rename = "CF")]
    CenterField,
    #[strum(to_string = "RF")]
    #[serde(rename = "RF")]
    RightField,
    #[strum(to_string = "SP")]
    #[serde(rename = "SP")]
    StartingPitcher,
    #[strum(to_string = "RP")]
    #[serde(rename = "RP")]
    ReliefPitcher,
    #[strum(to_string = "CL")]
    #[serde(rename = "CL")]
    Closer,
}

/// Places that a batter can hit a ball towards.
/// 
/// ```
/// use mmolb_parsing::enums::FairBallDestination;
/// 
/// assert_eq!(FairBallDestination::ShortStop.to_string(), "the shortstop");
/// ```
#[derive(EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
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


/// A characterisation of a fair ball.
///
/// ```
/// use mmolb_parsing::enums::FairBallType;
/// 
/// assert_eq!(FairBallType::GroundBall.to_string(), "ground ball");
/// assert_eq!(FairBallType::GroundBall.verb_name(), "grounds");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
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
    /// ```
    /// use mmolb_parsing::enums::FairBallType;
    /// 
    /// assert_eq!(FairBallType::GroundBall.verb_name(), "grounds");
    /// ```
    pub fn verb_name(self) -> &'static str {
        match self {
            Self::GroundBall => "grounds",
            Self::FlyBall => "flies",
            Self::LineDrive => "lines",
            Self::Popup => "pops",
        }
    }
}

/// ```
/// use mmolb_parsing::enums::PitchType;
/// 
/// assert_eq!(PitchType::KnuckleCurve.to_string(), "Knuckle Curve");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
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

/// ```
/// use mmolb_parsing::enums::StrikeType;
/// 
/// assert_eq!(StrikeType::Looking.to_string(), "looking");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum StrikeType {
    #[strum(to_string = "looking")]
    Looking,
    #[strum(to_string = "swinging")]
    Swinging
}

/// ```
/// use mmolb_parsing::enums::FieldingErrorType;
/// 
/// assert_eq!(FieldingErrorType::Throwing.to_string(), "Throwing");
/// assert_eq!(FieldingErrorType::Throwing.uppercase(), "Throwing");
/// assert_eq!(FieldingErrorType::Throwing.lowercase(), "throwing");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum FieldingErrorType {
    #[strum(to_string="Throwing", serialize="throwing")]
    Throwing,
    #[strum(to_string="Fielding", serialize="fielding")]
    Fielding
}
impl FieldingErrorType {
    /// ```
    /// use mmolb_parsing::enums::FieldingErrorType;
    /// 
    /// assert_eq!(FieldingErrorType::Throwing.lowercase(), "throwing");
    /// ```
    pub fn lowercase(self) -> &'static str {
        match self {
            Self::Throwing => "throwing",
            Self::Fielding => "fielding",
        }
    }

    /// ```
    /// use mmolb_parsing::enums::FieldingErrorType;
    /// 
    /// assert_eq!(FieldingErrorType::Throwing.uppercase(), "Throwing");
    /// ```
    pub fn uppercase(self) -> &'static str {
        match self {
            Self::Throwing => "Throwing",
            Self::Fielding => "Fielding",
        }
    }
}

/// ```
/// use mmolb_parsing::enums::FoulType;
/// 
/// assert_eq!(FoulType::Tip.to_string(), "tip");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum FoulType {
    #[strum(to_string = "tip")]
    Tip,
    #[strum(to_string = "ball")]
    Ball
}

/// ```
/// use mmolb_parsing::enums::Base;
/// 
/// assert_eq!(Base::First.to_string(), "first");
/// assert_eq!(Base::First.to_base_str(), "first base");
/// assert_eq!(Base::Home.to_base_str(), "home");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
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
    /// ```
    /// use mmolb_parsing::enums::Base;
    /// 
    /// assert_eq!(Base::First.to_base_str(), "first base");
    /// assert_eq!(Base::Home.to_base_str(), "home");
    /// ```
    pub fn to_base_str(self) -> &'static str {
        match self {
            Base::First => "first base",
            Base::Second => "second base",
            Base::Third => "third base",
            Base::Home => "home"
        }
    }
}
impl From<BaseNameVariant> for Base {
    fn from(value: BaseNameVariant) -> Self {
        match value {
            BaseNameVariant::First => Base::First,
            BaseNameVariant::FirstBase => Base::First,
            BaseNameVariant::OneB => Base::First,
            BaseNameVariant::Second => Base::Second,
            BaseNameVariant::SecondBase => Base::Second,
            BaseNameVariant::TwoB => Base::Second,
            BaseNameVariant::Third => Base::Third,
            BaseNameVariant::ThirdBase => Base::Third,
            BaseNameVariant::ThreeB => Base::Third,
            BaseNameVariant::Home => Base::Home
        }
    }
}


/// ```
/// use mmolb_parsing::enums::BaseNameVariant;
/// use mmolb_parsing::enums::Base;
/// 
/// assert_eq!(BaseNameVariant::First.to_string(), "first");
/// assert_eq!(BaseNameVariant::OneB.to_string(), "1B");
/// assert_eq!(BaseNameVariant::basic_name(Base::First), BaseNameVariant::First);
/// assert_eq!(Base::from(BaseNameVariant::OneB), Base::First);
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum BaseNameVariant {
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
impl BaseNameVariant {
    /// The most basic name for a given base e.g. ("first")
    pub fn basic_name(base: Base) -> BaseNameVariant {
        match base {
            Base::First => BaseNameVariant::First,
            Base::Second => BaseNameVariant::Second,
            Base::Third => BaseNameVariant::Third,
            Base::Home => BaseNameVariant::Home,
        }
    }
}

/// ```
/// use mmolb_parsing::enums::Distance;
/// 
/// assert_eq!(Distance::Single.to_string(), "singles");
/// ```
#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum Distance {
    #[strum(to_string = "singles")]
    Single,
    #[strum(to_string = "doubles")]
    Double,
    #[strum(to_string = "triples")]
    Triple,
}

/// Possible followup to "Now batting: [BATTER]". (e.g. "(1st PA of game)")
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
pub enum NowBattingStats {
    FirstPA,
    Stats(Vec<BatterStat>),
    NoStats
}

/// ```
/// use mmolb_parsing::enums::{BatterStat, BatterStatDiscriminants};
/// 
/// assert_eq!(BatterStat::FirstBases(1).unparse(), "1 1B");
///
/// // EnumDiscrimanats with display is derived.
/// assert_eq!(BatterStatDiscriminants::FirstBases.to_string(), "1B");
/// assert_eq!(BatterStatDiscriminants::HitsForAtBats.to_string(), "HitsForAtBats"); // mmolb implies this stat, it doesn't have an acronym.
/// ```
#[derive(Clone, Debug, EnumDiscriminants, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
#[strum_discriminants(derive(EnumString, IntoStaticStr, Display))]
#[serde(tag = "stat", content = "value")]
pub enum BatterStat {
    // Season 0
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

    // Season 1
    #[strum_discriminants(strum(to_string = "GO"))]
    GroundOuts(u8),
}
impl BatterStat {
    /// ```
    /// use mmolb_parsing::enums::{BatterStat, BatterStatDiscriminants};
    /// 
    /// assert_eq!(BatterStat::FirstBases(1).unparse(), "1 1B");
    /// assert_eq!(BatterStat::HitsForAtBats{hits: 1, at_bats: 1}.unparse(), "1 for 1");
    /// ```
    pub fn unparse(&self) -> String {
        self.to_string()
    }
}

impl Display for BatterStat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            BatterStat::StrikeOuts(count) |
            BatterStat::GroundOuts(count) => {
                write!(f, "{count} {}", BatterStatDiscriminants::from(self))
            }
            BatterStat::HitsForAtBats { hits, at_bats } => {
                write!(f, "{hits} for {at_bats}")
            }
        }
    }
}

impl FromStr for BatterStat {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let a_u8_tag = |t| {
            all_consuming(terminated(u8, tag::<&str, &str, nom::error::Error<&str>>(t)))
        };
        alt((
            a_u8_tag(" 1B").map(BatterStat::FirstBases),
            a_u8_tag(" 2B").map(BatterStat::SecondBases),
            a_u8_tag(" 3B").map(BatterStat::ThirdBases),
            a_u8_tag(" LO").map(BatterStat::LineOuts),
            a_u8_tag(" SO").map(BatterStat::StrikeOuts),
            a_u8_tag(" FO").map(BatterStat::ForceOuts),
            a_u8_tag(" HR").map(BatterStat::HomeRuns),
            a_u8_tag(" FC").map(BatterStat::FieldersChoices),
            a_u8_tag(" SF").map(BatterStat::SacrificeFlies),
            a_u8_tag(" F").map(BatterStat::Fouls),
            a_u8_tag(" BB").map(BatterStat::BaseOnBalls),
            a_u8_tag(" FC").map(BatterStat::SacrificeFlies),
            a_u8_tag(" HBP").map(BatterStat::HitByPitchs),
            a_u8_tag(" GIDP").map(BatterStat::GroundIntoDoublePlays),
            a_u8_tag(" CDP").map(BatterStat::CaughtDoublePlays),
            a_u8_tag(" PO").map(BatterStat::PopOuts),
            a_u8_tag(" GO").map(BatterStat::GroundOuts),
            separated_pair(u8, tag(" for "), u8).map(|(hits, at_bats)| BatterStat::HitsForAtBats { hits, at_bats }),
        )).parse(s).map(|(_, o)| o).map_err(|_| "Batter stat not recognized")
    }
}


/// ```
/// use mmolb_parsing::enums::GameStat;
/// 
/// assert_eq!(GameStat::GroundedIntoDoublePlay.to_string(), "grounded_into_double_play");
/// ```
#[derive(Clone, Copy, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum GameStat {
    // Season 0
    GroundedIntoDoublePlay,
    LeftOnBaseRisp,
    StrikeoutsRisp,
    Groundout,
    AllowedStolenBases,
    FieldersChoice,
    SacFlies,
    Assists,
    RunsBattedIn,
    Popouts,
    HomeRunsRisp,
    AtBats,
    EarnedRunsRisp,
    Strikeouts,
    Losses,
    StolenBasesRisp,
    HomeRunsAllowedRisp,
    ForceOuts,
    FieldersChoiceRisp,
    SacFliesRisp,
    Shutouts,
    BattersFaced,
    EarnedRuns,
    FieldOut,
    TriplesRisp,
    StolenBases,
    Walked,
    MoundVisits,
    FieldOutRisp,
    UnearnedRunsRisp,
    InheritedRunnersRisp,
    RunsRisp,
    QualityStarts,
    GroundedIntoDoublePlayRisp,
    Wins,
    RunsBattedInRisp,
    HitsAllowed,
    RunnersCaughtStealing,
    StruckOut,
    AssistsRisp,
    Saves,
    Walks,
    ReachedOnError,
    BlownSaves,
    CaughtDoublePlayRisp,
    LeftOnBase,
    LineoutsRisp,
    ReachedOnErrorRisp,
    UnearnedRuns,
    PlateAppearancesRisp,
    Triples,
    SacrificeDoublePlays,
    Starts,
    InheritedRunsAllowed,
    NoHitters,
    GamesFinished,
    CaughtStealingRisp,
    RunnersCaughtStealingRisp,
    BattersFacedRisp,
    DoublePlays,
    ForceOutsRisp,
    SinglesRisp,
    Singles,
    Lineouts,
    PlateAppearances,
    AtBatsRisp,
    DoublePlaysRisp,
    CaughtStealing,
    WalkedRisp,
    Putouts,
    HitBatters,
    HitByPitch,
    Errors,
    StruckOutRisp,
    PopoutsRisp,
    HomeRuns,
    HitByPitchRisp,
    Appearances,
    InheritedRunsAllowedRisp,
    WalksRisp,
    SacrificeDoublePlaysRisp,
    HitBattersRisp,
    Outs,
    Doubles,
    InheritedRunners,
    DoublesRisp,
    FlyoutsRisp,
    PitchesThrown,
    CompleteGames,
    Flyouts,
    PitchesThrownRisp,
    CaughtDoublePlay,
    HomeRunsAllowed,
    PutoutsRisp,
    GroundoutRisp,
    ErrorsRisp,
    Runs,
    HitsAllowedRisp,
    AllowedStolenBasesRisp,
    PerfectGames,

    // Season 1
    GroundoutsRisp,
    Groundouts,

    // Season 2
    Balks,
    BalksRisp,

    // Season 3
    HomeRunChallengeAppearances,
    HomeRunChallengeHomeRunsAllowed,
    HomeRunChallengeHomeRuns,

    // Season 4
    Ejected
}

#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum GameOverMessage {
    /// Early season 0 "Game over." e.g. 6805db4bac48194de3cd42d2 
    #[strum(to_string = "Game over.")]
    GameOver,
    /// Season 0 "\"GAME OVER.\"" e.g. 680fec59555fc84a67ba0fda
    #[strum(to_string = "\"GAME OVER.\"")]
    QuotedGAMEOVER
}

#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum ItemName {
    Cap,
    Gloves,
    #[strum(to_string = "T-Shirt")]
    #[serde(rename = "T-Shirt")]
    TShirt,
    Sneakers,
    Ring,
    #[strum(to_string = "Amplification Orb")]
    #[serde(rename = "Amplification Orb")]
    AmplificationOrb,
    #[strum(to_string = "Progress Orb")]
    #[serde(rename = "Progress Orb")]
    ProgressOrb,
    #[strum(to_string = "Ambition Orb")]
    #[serde(rename = "Ambition Orb")]
    AmbitionOrb
}

#[derive(Clone, Copy, EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum SpecialItemType {
    Material
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, IntoStaticStr, Display, PartialEq, Eq, Hash, EnumIter)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FeedEventType {
    Game,
    Augment,
    Release,
    Season
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, IntoStaticStr, Display, PartialEq, Eq, Hash, EnumIter)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Player,
    Game,
    Team
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, SerializeDisplay, DeserializeFromStr)]
pub enum SeasonStatus {
    RegularSeason,
    SuperstarBreak,
    HomeRunChallenge,
    SuperstarGame,
    Holiday,
    PostseasonRound(u8),
    SpecialEvent,
    Event,
    Election,
    Preseason,
    PostseasonPreview
}
impl FromStr for SeasonStatus {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Regular Season" => Ok(SeasonStatus::RegularSeason),
            "Superstar Break" => Ok(SeasonStatus::SuperstarBreak),
            "Home Run Challenge" => Ok(SeasonStatus::HomeRunChallenge),
            "Holiday" => Ok(SeasonStatus::Holiday),
            "Superstar Game" => Ok(SeasonStatus::SuperstarGame),
            "Special Event" => Ok(SeasonStatus::SpecialEvent),
            "Event" => Ok(SeasonStatus::Event),
            "Election" => Ok(SeasonStatus::Election),
            "Preseason" => Ok(SeasonStatus::Preseason),
            "Postseason Preview" => Ok(SeasonStatus::PostseasonPreview),
            s => s.strip_prefix("Postseason Round ")
                        .and_then(|s| s.parse().ok())
                        .map(SeasonStatus::PostseasonRound)
                        .ok_or(())
        }.map_err(|_| "Did not match any known FeedEventStatus variants")
    }
}

impl Display for SeasonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeasonStatus::RegularSeason => Display::fmt("Regular Season", f),
            SeasonStatus::SuperstarBreak => Display::fmt("Superstar Break", f),
            SeasonStatus::HomeRunChallenge => Display::fmt("Home Run Challenge", f),
            SeasonStatus::SuperstarGame => Display::fmt("Superstar Game", f),
            SeasonStatus::Holiday => Display::fmt("Holiday", f),
            SeasonStatus::PostseasonRound(i) => write!(f, "Postseason Round {i}"),
            SeasonStatus::SpecialEvent => write!(f, "Special Event"),
            SeasonStatus::Event => write!(f, "Event"),
            SeasonStatus::Election => write!(f, "Election"),
            SeasonStatus::Preseason => write!(f, "Preseason"),
            SeasonStatus::PostseasonPreview => write!(f, "Postseason Preview")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum Day {
    #[serde(rename = "Superstar Break")]
    SuperstarBreak,
    #[serde(rename = "Postseason Preview")]
    PostseasonPreview,
    #[serde(rename = "Superstar Game")]
    SuperstarGame,
    Holiday,
    Preseason,
    Election,
    Event,
    #[serde(rename = "Special Event")]
    SpecialEvent,
    #[serde(untagged)]
    Day(u16),
    #[serde(untagged, deserialize_with = "superstar_day_de", serialize_with = "superstar_day_ser")]
    SuperstarDay(u8),
    #[serde(untagged, deserialize_with = "postseason_round_de", serialize_with = "postseason_round_ser")]
    PostseasonRound(u8),
}
fn superstar_day_ser<S>(day: &u8, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    format!("Superstar Day {day}").serialize(serializer)
}

fn superstar_day_de<'de, D>(deserializer: D) -> Result<u8, D::Error> where D: Deserializer<'de> {
    <String>::deserialize(deserializer)?
        .strip_prefix("Superstar Day ")
        .ok_or(D::Error::custom("Didn't start with \"Superstar Day\""))?
        .parse::<u8>()
        .map_err(|_| D::Error::custom("Expected a number"))
}

fn postseason_round_ser<S>(round: &u8, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    format!("Postseason Round {round}").serialize(serializer)
}

fn postseason_round_de<'de, D>(deserializer: D) -> Result<u8, D::Error> where D: Deserializer<'de> {
    <String>::deserialize(deserializer)?
        .strip_prefix("Postseason Round ")
        .ok_or(D::Error::custom("Didn't start with \"Postseason Round\""))?
        .parse::<u8>()
        .map_err(|_| D::Error::custom("Expected a number"))
}

impl Display for Day {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SuperstarBreak => write!(f, "Superstar Break"),
            Self::Day(d) => write!(f, "{}", d),
            Self::Preseason => write!(f, "Preseason"),
            Self::Holiday => write!(f, "Holiday"),
            Self::Election => write!(f, "Election"),
            Self::SuperstarDay(d) => write!(f, "Superstar Day {d}"),
            Self::PostseasonPreview => write!(f, "Postseason Preview"),
            Self::PostseasonRound(r) => write!(f, "Postseason Round {r}"),
            Self::SpecialEvent => write!(f, "Special Event"),
            Self::Event => write!(f, "Event"),
            Self::SuperstarGame => write!(f, "Superstar Game"),
        }
    }
}

#[derive(Debug, Clone, Copy, SerializeDisplay, DeserializeFromStr, IntoStaticStr, PartialEq, Eq, Hash, EnumIter)]
pub enum RecordType {
    RegularSeason,
    Kumite,
    PostseasonRound(u8),
    SuperstarGame,
    HomeRunChallenge
}
impl FromStr for RecordType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Regular Season" => Ok(RecordType::RegularSeason),
            "Superstar Game" => Ok(RecordType::SuperstarGame),
            "Kumite" => Ok(RecordType::Kumite),
            "Home Run Challenge" => Ok(RecordType::HomeRunChallenge),
            s => s.strip_prefix("Postseason Round ")
                        .and_then(|s| s.parse().ok())
                        .map(RecordType::PostseasonRound)
                        .ok_or(())
        }.map_err(|_| "Did not match any known RecordType variants")
    }
}

impl Display for RecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordType::RegularSeason => Display::fmt("Regular Season", f),
            RecordType::PostseasonRound(i) => write!(f, "Postseason Round {i}"),
            RecordType::Kumite => write!(f, "Kumite"),
            RecordType::SuperstarGame => write!(f, "Superstar Game"),
            RecordType::HomeRunChallenge => write!(f, "Home Run Challenge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, IntoStaticStr, Display, PartialEq, Eq, Hash, EnumIter)]
pub enum PositionType {
    Pitcher,
    Batter,
}

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumDiscriminants, SerializeDisplay, DeserializeFromStr)]
#[strum_discriminants(derive(EnumString, IntoStaticStr, Display))]
pub enum Slot {
    #[strum_discriminants(strum(to_string = "C"))]
    Catcher,
    #[strum_discriminants(strum(to_string = "1B"))]
    FirstBaseman,
    #[strum_discriminants(strum(to_string = "2B"))]
    SecondBaseman,
    #[strum_discriminants(strum(to_string = "3B"))]
    ThirdBaseman,
    #[strum_discriminants(strum(to_string = "SS"))]
    ShortStop,
    #[strum_discriminants(strum(to_string = "LF"))]
    LeftField,
    #[strum_discriminants(strum(to_string = "CF"))]
    CenterField,
    #[strum_discriminants(strum(to_string = "RF"))]
    RightField,
    #[strum_discriminants(strum(to_string = "SP"))]
    StartingPitcher(u8),
    #[strum_discriminants(strum(to_string = "RP"))]
    ReliefPitcher(u8),
    #[strum_discriminants(strum(to_string = "CL"))]
    Closer,
    #[strum_discriminants(strum(to_string = "DH"))]
    DesignatedHitter
}

impl Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.discriminant())?;
        match self {
            Slot::StartingPitcher(i) | Slot::ReliefPitcher(i) => write!(f, "{}", i)?,
            _ => ()
        };
        Ok(())
    }
}

impl FromStr for Slot {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let a_tag = |t| {
            all_consuming(tag::<&str, &str, nom::error::Error<&str>>(t))
        };
        alt((
            a_tag("C").map(|_| Slot::Catcher),
            a_tag("1B").map(|_| Slot::FirstBaseman),
            a_tag("2B").map(|_| Slot::SecondBaseman),
            a_tag("3B").map(|_| Slot::ThirdBaseman),
            a_tag("LF").map(|_| Slot::LeftField),
            a_tag("CF").map(|_| Slot::CenterField),
            a_tag("RF").map(|_| Slot::RightField),
            a_tag("SS").map(|_| Slot::ShortStop),
            tag("DH").map(|_| Slot::DesignatedHitter),
            preceded(tag("SP"), u8).map(|i| Slot::StartingPitcher(i)),
            preceded(tag("RP"), u8).map(|i| Slot::ReliefPitcher(i)),
            a_tag("CL").map(|_| Slot::Closer),
        )).parse(s).map(|(_, o)| o).map_err(|_| "Player's slot didn't match known slots")
    }
}

#[derive(EnumString, IntoStaticStr, Display, Debug, SerializeDisplay, DeserializeFromStr, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum Attribute {
    Priority,
    Luck,
    Aiming,
    Contact,
    Cunning,
    Discipline,
    Insight,
    Intimidation,
    Lift,
    Vision,
    Determination,
    Wisdom,
    Muscle,
    Selflessness,
    Accuracy,
    Rotation,
    Presence,
    Persuasion,
    Stamina,
    Velocity,
    Control,
    Stuff,
    Defiance,
    Acrobatics,
    Agility,
    Arm,
    Awareness,
    Composure,
    Dexterity,
    Patience,
    Reaction,
    Greed,
    Performance,
    Speed,
    Stealth,
    Guts
}

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum AttributeCategory {
    Batting,
    Pitching,
    Defense,
    Baserunning,
    Generic
}

impl From<Attribute> for AttributeCategory {
    fn from(value: Attribute) -> Self {
        match value {
            Attribute::Priority | Attribute::Luck => AttributeCategory::Generic,
            Attribute::Aiming | Attribute::Contact | Attribute::Cunning | Attribute::Determination | Attribute::Discipline | Attribute::Insight | Attribute::Intimidation | Attribute::Lift | Attribute::Muscle | Attribute::Selflessness | Attribute::Vision | Attribute::Wisdom => AttributeCategory::Batting,
            Attribute::Greed | Attribute::Performance | Attribute::Speed | Attribute::Stealth => AttributeCategory::Baserunning,
            Attribute::Accuracy | Attribute::Control | Attribute::Defiance | Attribute::Guts | Attribute::Persuasion | Attribute::Presence | Attribute::Rotation | Attribute::Stamina | Attribute::Stuff | Attribute::Velocity | Attribute::Acrobatics | Attribute::Agility => AttributeCategory::Pitching,
            Attribute::Arm | Attribute::Awareness | Attribute::Composure | Attribute::Dexterity | Attribute::Patience | Attribute::Reaction => AttributeCategory::Defense
        }
    }
}

#[derive(EnumString, IntoStaticStr, Display, Debug, SerializeDisplay, DeserializeFromStr, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum ItemPrefix {
    Sharp,
    Consistent,
    Clever,
    Steadfast,
    Insightful,
    Menacing,
    Lofty,
    #[strum(to_string = "Eagle-Eyed")]
    EagleEyed,
    Stalwart,
    Wise,
    Mighty, 
    Selfless,
    True,
    Commanding,
    Charming,
    Courageous,
    Rebellious,
    Enduring,
    Rapid,
    Precise,
    Whirling,
    Filthy,
    Avaricious,
    Dazzling,
    Swift,
    Sneaky,
}

#[derive(EnumString, IntoStaticStr, Display, Debug, SerializeDisplay, DeserializeFromStr, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum ItemSuffix {
    #[strum(to_string = "of the Acrobat")]
    Acrobat,
    #[strum(to_string = "of the Cat")]
    Cat,
    #[strum(to_string = "of the Cannon")]
    Cannon,
    #[strum(to_string = "of Awareness")]
    Awareness,
    #[strum(to_string = "of Calm")]
    Calm,
    #[strum(to_string = "of Skill")]
    Skill,
    #[strum(to_string = "of Patience")]
    Patience,
    #[strum(to_string = "of Reflexes")]
    Reflexes,
    #[strum(to_string = "of Fortune")]
    Fortune,
}

/// The various places a player in a game has been said to be.
#[derive(Debug, SerializeDisplay, DeserializeFromStr, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumDiscriminants)]
#[strum_discriminants(derive(EnumString, IntoStaticStr, Display))]
pub enum Place {
    #[strum_discriminants(strum(to_string = "P"))]
    Pitcher,
    #[strum_discriminants(strum(to_string = "C"))]
    Catcher,
    #[strum_discriminants(strum(to_string = "1B"))]
    FirstBaseman,
    #[strum_discriminants(strum(to_string = "2B"))]
    SecondBaseman,
    #[strum_discriminants(strum(to_string = "3B"))]
    ThirdBaseman,
    #[strum_discriminants(strum(to_string = "SS"))]
    ShortStop,
    #[strum_discriminants(strum(to_string = "LF"))]
    LeftField,
    #[strum_discriminants(strum(to_string = "CF"))]
    CenterField,
    #[strum_discriminants(strum(to_string = "RF"))]
    RightField,
    #[strum_discriminants(strum(to_string = "SP"))]
    StartingPitcher(Option<u8>),
    #[strum_discriminants(strum(to_string = "RP"))]
    ReliefPitcher(Option<u8>),
    #[strum_discriminants(strum(to_string = "CL"))]
    Closer,
    #[strum_discriminants(strum(to_string = "DH"))]
    DesignatedHitter
}

impl From<Slot> for Place {
    fn from(value: Slot) -> Self {
        match value {
            Slot::Catcher => Place::Catcher,
            Slot::CenterField => Place::CenterField,
            Slot::Closer => Place::Closer,
            Slot::DesignatedHitter => Place::DesignatedHitter,
            Slot::FirstBaseman => Place::FirstBaseman,
            Slot::SecondBaseman => Place::SecondBaseman,
            Slot::ThirdBaseman => Place::ThirdBaseman,
            Slot::ShortStop => Place::ShortStop,
            Slot::LeftField => Place::LeftField,
            Slot::RightField => Place::RightField,
            Slot::StartingPitcher(i) => Place::StartingPitcher(Some(i)),
            Slot::ReliefPitcher(i) => Place::ReliefPitcher(Some(i)),
        }
    }
}
impl From<Position> for Place {
    fn from(value: Position) -> Self {
        match value {
            Position::Pitcher => Place::Pitcher,
            Position::Catcher => Place::Catcher,
            Position::FirstBaseman => Place::FirstBaseman,
            Position::SecondBaseman => Place::SecondBaseman,
            Position::ThirdBaseman => Place::ThirdBaseman,
            Position::ShortStop => Place::ShortStop,
            Position::LeftField => Place::LeftField,
            Position::CenterField => Place::CenterField,
            Position::RightField => Place::RightField,
            Position::StartingPitcher => Place::StartingPitcher(None),
            Position::ReliefPitcher => Place::ReliefPitcher(None),
            Position::Closer => Place::Closer,
        }
    }
}
impl FromStr for Place {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let a_tag = |t| {
            all_consuming(tag::<&str, &str, nom::error::Error<&str>>(t))
        };
        alt((
            a_tag("P").map(|_| Place::Pitcher),
            a_tag("C").map(|_| Place::Catcher),
            a_tag("1B").map(|_| Place::FirstBaseman),
            a_tag("2B").map(|_| Place::SecondBaseman),
            a_tag("3B").map(|_| Place::ThirdBaseman),
            a_tag("LF").map(|_| Place::LeftField),
            a_tag("CF").map(|_| Place::CenterField),
            a_tag("RF").map(|_| Place::RightField),
            a_tag("SS").map(|_| Place::ShortStop),
            tag("DH").map(|_| Place::DesignatedHitter),
            preceded(tag("SP"), opt(u8)).map(|i| Place::StartingPitcher(i)),
            preceded(tag("RP"), opt(u8)).map(|i| Place::ReliefPitcher(i)),
            a_tag("CL").map(|_| Place::Closer),
        )).parse(s).map(|(_, o)| o).map_err(|_| "Player's slot didn't match known slots")
    }
}
impl Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.discriminant())?;
        match self {
            Place::StartingPitcher(Some(i)) | Place::ReliefPitcher(Some(i)) => write!(f, "{}", i)?,
            _ => ()
        };
        Ok(())
    }
}

#[derive(EnumString, IntoStaticStr, Display, Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash)]
pub enum MoundVisitType {
    #[strum(to_string = "mound visit")]
    MoundVisit,
    #[strum(to_string = "pitching change")]
    PitchingChange
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum LeagueScale {
    Lesser,
    Greater,
    Special
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum Handedness {
    #[strum(to_string = "L")]
    #[serde(rename = "L")]
    Left,
    #[strum(to_string = "R")]
    #[serde(rename = "R")]
    Right,
    #[strum(to_string = "S")]
    #[serde(rename = "S")]
    Switch
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum EquipmentEffectType {
    FlatBonus
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum EquipmentRarity {
    Normal,
    Rare,
    Magic
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum EquipmentSlot {
    Accessory,
    Head,
    Feet,
    Hands,
    Body
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum FeedEventSource {
    Player,
    Team
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum BallparkSuffix {
    Field,
    Stadium,
    Fairgrounds,
    Dome,
    Park,
    Lot,
    Coliseum,
    Yards,
    Grounds
}

fn _check(_: &str) -> Infallible {
    unreachable!("This is dead code that exists for a strum parse_err_fn")
}

#[derive(Debug, Serialize, Deserialize, Clone, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
#[strum(
    parse_err_fn = check,
    parse_err_ty = Infallible
)]
pub enum ModificationType {
    #[strum(to_string = "Fire Elemental")]
    #[serde(rename = "Fire Elemental")]
    FireElemental,
    #[strum(to_string = "Air Elemental")]
    #[serde(rename = "Air Elemental")]
    AirElemental,
    #[strum(to_string = "Water Elemental")]
    #[serde(rename = "Water Elemental")]
    WaterElemental,
    #[strum(to_string = "Earth Elemental")]
    #[serde(rename = "Earth Elemental")]
    EarthElemental,
    Demonic,
    ROBO,
    Draconic,
    Angelic,
    Undead,
    Giant,
    Fae,

    #[strum(to_string = "Archer's Mark")]
    #[serde(rename = "Archer's Mark")]
    ArchersMark,
    Spectral,
    #[strum(to_string = "Tenacious Badger")]
    #[serde(rename = "Tenacious Badger")]
    TenaciousBadger,
    Stormrider,
    UFO,
    Psychic,
    #[strum(to_string = "One With All")]
    #[serde(rename = "One With All")]
    OneWithAll,
    Amphibian,
    #[strum(to_string = "Geometry Expert")]
    #[serde(rename = "Geometry Expert")]
    GeometryExpert,
    #[strum(to_string = "The Light")]
    #[serde(rename = "The Light")]
    TheLight,
    Insectoid,
    Shiny,
    Scooter,
    Calculated,
    Mer,
    Clean,    

    #[strum(default)]
    #[serde(untagged)]
    Unknown(String),
}

impl ModificationType {
    pub fn new(value: &str) -> Self {
        let r = ModificationType::from_str(value)
            .expect("This error type is infallible");

        if matches!(r, ModificationType::Unknown(_)) {
            tracing::warn!("Failed to match modification '{value}'");
        }

        r
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, EnumIter, PartialEq, Eq, Hash, EnumString, IntoStaticStr, Display)]
pub enum CelestialEnergyTier {
    #[strum(to_string = "began to glow brightly with celestial energy!")]
    BeganToGlow,
    #[strum(to_string = "was infused with a glimmer of celestial energy!")]
    Infused,
    #[strum(to_string = "was fully charged with an abundance of celestial energy!")]
    FullyCharged,
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use strum::IntoEnumIterator;


    use super::*;

    fn serde_round_trip_inner<T: IntoEnumIterator + PartialEq + Debug + Serialize + for<'de> Deserialize<'de>>() {
        for value in T::iter() {
            let ser = serde_json::to_string(&value).unwrap();
            let de = serde_json::from_str(&ser).map_err(|e| e.to_string());
            let message = format!("{value:?}");
            assert_eq!(Ok(value), de, "{message}");
        }
    }


    #[test]
    fn serde_round_trips() {
        serde_round_trip_inner::<EventType>();
        serde_round_trip_inner::<TopBottom>();
        serde_round_trip_inner::<HomeAway>();
        serde_round_trip_inner::<Inning>();
        serde_round_trip_inner::<Position>();
        serde_round_trip_inner::<FairBallDestination>();
        serde_round_trip_inner::<FairBallType>();
        serde_round_trip_inner::<PitchType>();
        serde_round_trip_inner::<StrikeType>();
        serde_round_trip_inner::<FieldingErrorType>();
        serde_round_trip_inner::<FoulType>();
        serde_round_trip_inner::<Base>();
        serde_round_trip_inner::<BaseNameVariant>();
        serde_round_trip_inner::<Distance>();
        serde_round_trip_inner::<NowBattingStats>();
        serde_round_trip_inner::<BatterStat>();
        serde_round_trip_inner::<GameStat>();
        serde_round_trip_inner::<GameOverMessage>();
        serde_round_trip_inner::<ItemName>();
        serde_round_trip_inner::<Day>();
        serde_round_trip_inner::<SeasonStatus>();
        serde_round_trip_inner::<FeedEventType>();
        serde_round_trip_inner::<RecordType>();
        serde_round_trip_inner::<PositionType>();
        serde_round_trip_inner::<Slot>();
        serde_round_trip_inner::<Attribute>();
        serde_round_trip_inner::<ItemPrefix>();
        serde_round_trip_inner::<ItemSuffix>();
        serde_round_trip_inner::<Place>();
        serde_round_trip_inner::<MoundVisitType>();
        serde_round_trip_inner::<LeagueScale>();
        serde_round_trip_inner::<Handedness>();
        serde_round_trip_inner::<ModificationType>();
        serde_round_trip_inner::<BallparkSuffix>();
    }
}