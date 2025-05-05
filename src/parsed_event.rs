use std::{fmt::{Display, Write}, iter::once};

use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use crate::enums::{Base, BaseNameVariants, Distance, EventType, FieldingErrorType, FoulType, FairBallDestination, FairBallType, HomeAway, Position, StrikeType, TopBottom};

/// S is the string type used. S = &'output str is used by the parser, 
/// but a mutable type is necessary when directly deserializing, because some players have escaped characters in their names
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
pub enum ParsedEventMessage<S> 
{
    ParseError {
        event_type: EventType,
        message: String,
    },

    LiveNow {
        away_team_name: S,
        away_team_emoji: S,
        home_team_name: S,
        home_team_emoji: S,
    },
    PitchingMatchup {
        away_team_name: S,
        away_team_emoji: S,
        home_team_name: S,
        home_team_emoji: S,
        home_pitcher: S,
        away_pitcher: S,
    },
    Lineup {
        side: HomeAway,
        players: Vec<PositionedPlayer<S>>
    },
    PlayBall,
    GameOver,
    Recordkeeping {
        winning_team_emoji: S,
        winning_team_name: S,
        losing_team_emoji: S,
        losing_team_name: S,
        winning_score: u8,
        losing_score: u8,
    },
    InningStart {
        number: u8,
        side: TopBottom,
        batting_team_emoji: S,
        batting_team_name: S,
        /// This message was only added halfway through season 0. This field does not currently track unannounced automatic runners.
        automatic_runner: Option<S>,
        pitcher_status: StartOfInningPitcher<S>,
    },
    NowBatting {
        batter: S,
        stats: Option<S>
    },
    InningEnd {
        number: u8,
        side: TopBottom
    },

    // Mound visits
    MoundVisit {
        emoji: S,
        team: S,
    },
    PitcherRemains {
        remaining_pitcher: PositionedPlayer<S>,
    },
    PitcherSwap {
        leaving_position: Position,
        leaving_pitcher: S,
        arriving_position: Position,
        arriving_pitcher: S,
    },

    // Pitch
    Ball { steals: Vec<BaseSteal<S>>, count:(u8, u8)},
    Strike { strike: StrikeType, steals: Vec<BaseSteal<S>>, count:(u8, u8)},
    Foul { foul: FoulType, steals: Vec<BaseSteal<S>>, count:(u8, u8) },
    Walk { batter: S, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    HitByPitch { batter: S, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    FairBall {
        batter: S,
        fair_ball_type: FairBallType,
        destination: FairBallDestination
    },  
    StrikeOut { foul: Option<FoulType>, batter: S, strike: StrikeType, steals: Vec<BaseSteal<S>> },

    // Field
    BatterToBase { batter: S, distance: Distance, fair_ball_type: FairBallType, fielder: PositionedPlayer<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    HomeRun { batter: S, fair_ball_type: FairBallType, destination: FairBallDestination, scores: Vec<S> },
    GrandSlam { batter: S, fair_ball_type: FairBallType, destination: FairBallDestination, scores: Vec<S> },
    CaughtOut { batter: S, fair_ball_type: FairBallType, caught_by: PositionedPlayer<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, sacrifice: bool, perfect: bool },
    GroundedOut { batter: S, fielders: Vec<PositionedPlayer<S>>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ForceOut { batter: S, fielders: Vec<PositionedPlayer<S>>, fair_ball_type: FairBallType, out:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ReachOnFieldersChoice { batter: S, fielders: Vec<PositionedPlayer<S>>, play:Play<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    DoublePlayGrounded { batter: S, fielders: Vec<PositionedPlayer<S>>, play_one:Play<S>, play_two:Play<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, sacrifice: bool },
    DoublePlayCaught { batter: S, fair_ball_type: FairBallType, fielders: Vec<PositionedPlayer<S>>, play:Play<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ReachOnFieldingError { batter: S, fielder:PositionedPlayer<S>, error: FieldingErrorType, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> }
}
impl<S: Display> ParsedEventMessage<S> {
    /// Recreate the event message this ParsedEvent was built out of.
    pub fn unparse(self) -> String {
        match self {
            Self::ParseError { event_type: _, message } => {
                message
            },
            Self::LiveNow { away_team_name, away_team_emoji, home_team_name, home_team_emoji } => format!("{} {} @ {} {}", away_team_emoji, away_team_name, home_team_emoji, home_team_name),
            Self::PitchingMatchup { away_team_name, away_team_emoji, home_team_name, home_team_emoji, home_pitcher, away_pitcher } => format!("{} {} {away_pitcher} vs. {} {} {home_pitcher}", away_team_emoji, away_team_name, home_team_emoji, home_team_name),
            Self::Lineup { side: _, players } => {
                players.into_iter().enumerate().fold(String::new(), |mut acc, (index, player)| {
                    let _ = write!(acc, "{}. {player}<br>", index + 1);
                    acc
                })
            },
            Self::PlayBall => "\"PLAY BALL.\"".to_string(),
            Self::GameOver => "\"GAME OVER.\"".to_string(),
            Self::Recordkeeping { winning_team_emoji, winning_team_name, losing_team_emoji, losing_team_name, winning_score, losing_score } => {
                format!("{winning_team_emoji} {winning_team_name} defeated {losing_team_emoji} {losing_team_name}. Final score: {winning_score}-{losing_score}")
            }
            Self::InningStart { number, side, batting_team_emoji, batting_team_name, automatic_runner, pitcher_status } => {
                let ordinal = match number {
                    0 => panic!("Should not have 0th innings"),
                    1 => "1st".to_string(),
                    2 => "2nd".to_string(),
                    3 => "3rd".to_string(),
                    4.. => format!("{number}th")
                };
                let pitcher_message = match pitcher_status {
                    StartOfInningPitcher::Same { emoji, name } => format!("{emoji} {name} pitching."),
                    StartOfInningPitcher::Different { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher } => {
                        format!("{leaving_position} {leaving_pitcher} is leaving the game. {arriving_position} {arriving_pitcher} takes the mound.")
                    }
                };
                let automatic_runner = match automatic_runner {
                    Some(runner) => format!(" {runner} starts the inning on second base."),
                    None => String::new()
                };
                format!("Start of the {side} of the {ordinal}. {batting_team_emoji} {batting_team_name} batting.{automatic_runner} {pitcher_message}")
            },
            Self::NowBatting {batter, stats} => {
                let stats = stats.map(|stats| format!(" ({stats})")).unwrap_or(String::new());
                format!("Now batting: {batter}{stats}")
            },
            Self::InningEnd {number, side} => {
                let ordinal = match number {
                    0 => panic!("Should not have 0th innings"),
                    1 => "1st".to_string(),
                    2 => "2nd".to_string(),
                    3 => "3rd".to_string(),
                    4.. => format!("{number}th")
                };
                format!("End of the {side} of the {ordinal}.")
            },        
            Self::MoundVisit {emoji, team } => {
                format!("The {emoji} {team} manager is making a mound visit.")
            },
            Self::PitcherRemains { remaining_pitcher } => {
                format!("{remaining_pitcher} remains in the game.")
            },
            Self::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher } => {
                format!("{leaving_position} {leaving_pitcher} is leaving the game. {arriving_position} {arriving_pitcher} takes the mound.")
            },

            Self::Ball { steals, count } => {
                let steals = once(String::new()).chain(steals.iter().map(BaseSteal::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");
                format!(" Ball. {}-{}.{steals}", count.0, count.1,)
            },
            Self::Strike { strike, steals, count } => {
                let steals: Vec<String> = once(String::new()).chain(steals.into_iter().map(|steal| steal.to_string())).collect();
                let steals = steals.join(" ");
                format!(" Strike, {strike}. {}-{}.{steals}", count.0, count.1)
            }
            Self::Foul { foul, steals, count } => {
                let steals: Vec<String> = once(String::new()).chain(steals.into_iter().map(|steal| steal.to_string())).collect();
                let steals = steals.join(" ");
                format!(" Foul {foul}. {}-{}.{steals}", count.0, count.1)
            }
            Self::Walk { batter, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!(" Ball 4. {batter} walks.{scores_and_advances}")
            }
            Self::HitByPitch { batter, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!(" {batter} was hit by the pitch and advances to first base.{scores_and_advances}")
            }
            Self::FairBall { batter, fair_ball_type, destination } => {
                format!(" {batter} hits a {fair_ball_type} to {destination}.")
            }
            Self::StrikeOut { foul, batter, strike, steals } => {
                let foul = match foul {
                    Some(foul) => format!("Foul {foul}. "),
                    None => String::new()
                };
                let steals: Vec<String> = once(String::new()).chain(steals.into_iter().map(|steal| steal.to_string())).collect();
                let steals = steals.join(" ");
                format!(" {foul}{batter} struck out {strike}.{steals}")
            }
            Self::BatterToBase { batter, distance, fair_ball_type, fielder, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!("{batter} {distance} on a {fair_ball_type} to {fielder}.{scores_and_advances}")
            }
            Self::HomeRun { batter, fair_ball_type, destination, scores } => {
                let scores = once(String::new()).chain(scores.into_iter().map(|runner| format!("<strong>{runner} scores!</strong>")))
                    .collect::<Vec<String>>()
                    .join(" ");
                format!("<strong>{batter} homers on a {fair_ball_type} to {destination}!</strong>{scores}")
            }
            Self::GrandSlam { batter, fair_ball_type, destination, scores } => {
                let scores = once(String::new()).chain(scores.into_iter().map(|runner| format!("<strong>{runner} scores!</strong>")))
                .collect::<Vec<String>>()
                .join(" ");
                format!("<strong>{batter} hits a grand slam on a {fair_ball_type} to {destination}!</strong>{scores}")
            }
            Self::CaughtOut { batter, fair_ball_type, caught_by: catcher, scores, advances, sacrifice, perfect } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let sacrifice = if sacrifice {"on a sacrifice fly "} else {""};
                let perfect = if perfect {" <strong>Perfect catch!</strong>"} else {""};
                
                format!("{batter} {fair_ball_type} out {sacrifice}to {catcher}.{perfect}{scores_and_advances}")
            }
            Self::GroundedOut { batter, fielders, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders(fielders);
                format!("{batter} grounds out{fielders}.{scores_and_advances}")
            }
            Self::ForceOut { batter, fielders, fair_ball_type, out, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders_for_play(fielders);
                let fair_ball_type = fair_ball_type.verb_name();
                format!("{batter} {fair_ball_type} into a force out{fielders}. {out}{scores_and_advances}")
            }
            Self::ReachOnFieldersChoice { batter, fielders, play, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                match play {
                    Play::Out {out} => {
                        let fielders = unparse_fielders_for_play(fielders);
                        
                        format!("{batter} reaches on a fielder's choice out{fielders}. {out}{scores_and_advances}")
                    }
                    Play::Error { fielder, error } => {
                        let fielder_long = fielders.first().unwrap();
                        format!("{batter} reaches on a fielder's choice, fielded by {fielder_long}.{scores_and_advances} {error} error by {fielder}.")
                    }
                }
            }
            Self::DoublePlayGrounded { batter, fielders, play_one, play_two, scores, advances, sacrifice } => {
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let sacrifice = if sacrifice {"sacrifice "} else {""};
                format!("{batter} grounded into a {sacrifice}double play{fielders}. {play_one} {play_two}{scores_and_advances}")
            }
            Self::DoublePlayCaught { batter, fair_ball_type, fielders, play, scores, advances } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!("{batter} {fair_ball_type} into a double play{fielders}. {play}{scores_and_advances}")
            }
            Self::ReachOnFieldingError { batter, fielder, error, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let error = error.to_string().to_lowercase();
                format!("{batter} reaches on a {error} error by {fielder}.{scores_and_advances}")
            }
        }
    }
}

fn unparse_fielders<S:Display>(fielders: Vec<PositionedPlayer<S>>) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!("to {}", fielders.first().unwrap()),
        _ => format!(", {}", fielders.iter().map(PositionedPlayer::to_string).collect::<Vec<_>>().join(" to "))
    }
}

fn unparse_fielders_for_play<S:Display>(fielders: Vec<PositionedPlayer<S>>) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!(", {} unassisted", fielders.first().unwrap()),
        _ => format!(", {}", fielders.iter().map(PositionedPlayer::to_string).collect::<Vec<_>>().join(" to "))
    }
}
fn unparse_scores_and_advances<S: Display>(scores: Vec<S>, advances:Vec<RunnerAdvance<S>>) -> String {
    once(String::new()).chain(scores.iter().map(|runner| format!("<strong>{runner} scores!</strong>"))
        .chain(advances.iter().map(|advance| advance.to_string())))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StartOfInningPitcher<S> {
    Same {emoji: S, name: S},
    Different {
        leaving_position: Position,
        leaving_pitcher: S,
        arriving_position: Position,
        arriving_pitcher: S,
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Play<S> {
    Out {
        out: RunnerOut<S>,
    },
    Error {
        fielder: S,
        error: FieldingErrorType
    }
}
impl<S:Display> Display for Play<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Out { out } => {
                write!(f, "{} out at {}.", out.runner, out.base)
            },
            Self::Error { fielder, error } => {
                write!(f, "{error} by {fielder}.")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PositionedPlayer<S> {
    pub name: S,
    pub position: Position
}
impl<S: Display> Display for PositionedPlayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.position, self.name)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RunnerOut<S> {
    pub runner: S,
    pub base: BaseNameVariants,
}
impl<S: Display> Display for RunnerOut<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} out at {}.", self.runner, self.base)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RunnerAdvance<S> {
    pub runner: S,
    pub base: Base
}
impl<S: Display> Display for RunnerAdvance<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} to {} base.", self.runner, self.base)
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct BaseSteal<S> {
    pub runner: S,
    pub base: Base,
    pub caught: bool
}
impl<S: Display> Display for BaseSteal<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.caught {
            true => write!(f, "{} is caught stealing {}.", self.runner, self.base.to_base_string()),
            false => match self.base {
                Base::Home => write!(f, "<strong>{} steals {}!</strong>", self.runner, self.base.to_base_string()),
                _ => write!(f, "{} steals {}!", self.runner, self.base.to_base_string()),
            }
        }
    }
}
impl<S> TryFrom<BaseSteal<S>> for RunnerOut<S> {
    type Error = ();
    fn try_from(value: BaseSteal<S>) -> Result<Self, Self::Error> {
        if value.caught {
            Ok(RunnerOut { runner: value.runner, base: value.base.into() })
        } else {
            Err(())
        }
    }
}
impl<S> TryFrom<BaseSteal<S>> for RunnerAdvance<S> {
    type Error = ();
    fn try_from(value: BaseSteal<S>) -> Result<Self, Self::Error> {
        if !value.caught {
            Ok(RunnerAdvance { runner: value.runner, base: value.base })
        } else {
            Err(())
        }
    }
}