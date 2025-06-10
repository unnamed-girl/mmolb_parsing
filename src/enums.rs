use std::fmt::Display;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumIter, EnumString};

/// Possible values of the "event" field of an mmolb event. 
#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display, PartialEq, Eq, Hash)]
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
    NowBatting,
    
    #[strum(default)]
    NotRecognized(String)
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Inning {
    BeforeGame,
    DuringGame {number: u8, batting_side: TopBottom},
    AfterGame { total_inning_count: u8 }
}
impl Inning {
    /// The next inning. If `continue_if_overtime`, go to extra innings instead of ending the game at the 9th.
    /// 
    /// ```
    /// use mmolb_parsing::enums::Inning;
    /// use mmolb_parsing::enums::TopBottom;
    /// 
    /// assert_eq!(Inning::BeforeGame.next(false), Some(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }));
    /// assert_eq!(Inning::DuringGame { number: 9, batting_side: TopBottom::Bottom }.next(false), Some(Inning::AfterGame { total_inning_count:9 }));
    /// assert_eq!(Inning::DuringGame { number: 9, batting_side: TopBottom::Bottom }.next(true), Some(Inning::DuringGame { number: 10, batting_side: TopBottom::Top }));
    /// assert_eq!(Inning::AfterGame { total_inning_count: 9 }.next(true), None);
    /// ```
    pub fn next(self, continue_if_overtime: bool) -> Option<Self> {
        match self {
            Inning::BeforeGame => Some(Inning::DuringGame { number: 1, batting_side: TopBottom::Top }),
            Inning::DuringGame { number, batting_side } => {
                if number >= 9 && !continue_if_overtime {
                    Some(Inning::AfterGame { total_inning_count: number })
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
    /// assert_eq!(Inning::AfterGame {total_inning_count: 9}.number(), None);
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
    /// assert_eq!(Inning::AfterGame {total_inning_count: 9}.batting_team(), None);
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
    /// assert_eq!(Inning::AfterGame {total_inning_count: 9}.pitching_team(), None);
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

/// Places that a batter can hit a ball towards.
/// 
/// ```
/// use mmolb_parsing::enums::FairBallDestination;
/// 
/// assert_eq!(FairBallDestination::ShortStop.to_string(), "the shortstop");
/// ```
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


/// A characterisation of a fair ball.
///
/// ```
/// use mmolb_parsing::enums::FairBallType;
/// 
/// assert_eq!(FairBallType::GroundBall.to_string(), "ground ball");
/// assert_eq!(FairBallType::GroundBall.verb_name(), "grounds");
/// ```
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

/// ```
/// use mmolb_parsing::enums::StrikeType;
/// 
/// assert_eq!(StrikeType::Looking.to_string(), "looking");
/// ```
#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Distance {
    #[strum(to_string = "singles")]
    Single,
    #[strum(to_string = "doubles")]
    Double,
    #[strum(to_string = "triples")]
    Triple,
}

/// Possible followup to "Now batting: [BATTER]". (e.g. "(1st PA of game)")
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NowBattingStats {
    FirstPA,
    Stats {
        stats: Vec<BatterStat>
    },
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
#[derive(Clone, Debug, EnumDiscriminants, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    /// ```
    /// use mmolb_parsing::enums::{BatterStat, BatterStatDiscriminants};
    /// 
    /// assert_eq!(BatterStat::FirstBases(1).unparse(), "1 1B");
    /// assert_eq!(BatterStat::HitsForAtBats{hits: 1, at_bats: 1}.unparse(), "1 for 1");
    /// ```
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

/// ```
/// use mmolb_parsing::enums::GameStat;
/// 
/// assert_eq!(GameStat::GroundedIntoDoublePlay.to_string(), "grounded_into_double_play");
/// ```
#[derive(Clone, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[strum(serialize_all = "snake_case")]
pub enum GameStat {
    #[strum(default)]
    NotRecognized(String),

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
}

#[derive(Clone, Copy, EnumString, Display, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum GameOverMessage {
    /// Early season 0 "Game over." e.g. 6805db4bac48194de3cd42d2 
    #[strum(to_string = "Game over.")]
    GameOver,
    /// Season 0 "\"GAME OVER.\"" e.g. 680fec59555fc84a67ba0fda
    #[strum(to_string = "\"GAME OVER.\"")]
    QuotedGAMEOVER
}
