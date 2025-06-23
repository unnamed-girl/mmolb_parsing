use std::{fmt::{Display, Write}, iter::once};

use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use crate::enums::{Base, BaseNameVariant, BatterStat, Distance, FairBallDestination, FairBallType, FieldingErrorType, FoulType, GameOverMessage, HomeAway, Item, NowBattingStats, Position, StrikeType, TopBottom};

/// S is the string type used. S = &'output str is used by the parser, 
/// but a mutable type is necessary when directly deserializing, because some players have escaped characters in their names
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
pub enum ParsedEventMessage<S> {
    // Season 0
    ParseError {
        event_type: String,
        message: String,
    },

    LiveNow {
        away_team: EmojiTeam<S>,
        home_team: EmojiTeam<S>
    },
    PitchingMatchup {
        away_team: EmojiTeam<S>,
        home_team: EmojiTeam<S>,
        home_pitcher: S,
        away_pitcher: S,
    },
    Lineup {
        side: HomeAway,
        players: Vec<PositionedPlayer<S>>
    },
    PlayBall,
    GameOver {
        message: GameOverMessage
    },
    Recordkeeping {
        winning_team: EmojiTeam<S>,
        losing_team: EmojiTeam<S>,
        winning_score: u8,
        losing_score: u8,
    },
    InningStart {
        number: u8,
        side: TopBottom,
        batting_team: EmojiTeam<S>,
        /// This message was only added halfway through season 0. This field does not currently track unannounced automatic runners.
        automatic_runner: Option<S>,
        pitcher_status: StartOfInningPitcher<S>,
    },
    NowBatting {
        batter: S,
        stats: NowBattingStats
    },
    InningEnd {
        number: u8,
        side: TopBottom
    },

    // Mound visits
    MoundVisit {
        team: EmojiTeam<S>
    },
    PitcherRemains {
        remaining_pitcher: PositionedPlayer<S>,
    },
    PitcherSwap {
        leaving_pitcher: PositionedPlayer<S>,
        arriving_pitcher: PositionedPlayer<S>,
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
    HomeRun { batter: S, fair_ball_type: FairBallType, destination: FairBallDestination, scores: Vec<S>, grand_slam: bool },
    CaughtOut { batter: S, fair_ball_type: FairBallType, caught_by: PositionedPlayer<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, sacrifice: bool, perfect: bool },
    GroundedOut { batter: S, fielders: Vec<PositionedPlayer<S>>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, perfect: bool },
    ForceOut { batter: S, fielders: Vec<PositionedPlayer<S>>, fair_ball_type: FairBallType, out:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ReachOnFieldersChoice { batter: S, fielders: Vec<PositionedPlayer<S>>, result:FieldingAttempt<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    DoublePlayGrounded { batter: S, fielders: Vec<PositionedPlayer<S>>, out_one:RunnerOut<S>, out_two:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, sacrifice: bool },
    DoublePlayCaught { batter: S, fair_ball_type: FairBallType, fielders: Vec<PositionedPlayer<S>>, out_two:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ReachOnFieldingError { batter: S, fielder:PositionedPlayer<S>, error: FieldingErrorType, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },

    // Season 1
    WeatherDelivery { team: EmojiTeam<S>, player: S, item_emoji: S, item :Item },
    WeatherDeliveryDiscard { item_emoji: S, item :Item },
}
impl<S: Display> ParsedEventMessage<S> {
    /// Recreate the event message this ParsedEvent was built out of.
    pub fn unparse(self) -> String {
        match self {
            Self::ParseError { event_type: _, message } => {
                message
            },
            Self::LiveNow { away_team, home_team } => format!("{} @ {}", away_team, home_team),
            Self::PitchingMatchup { away_team, home_team, home_pitcher, away_pitcher } => format!("{away_team} {away_pitcher} vs. {home_team} {home_pitcher}"),
            Self::Lineup { side: _, players } => {
                players.into_iter().enumerate().fold(String::new(), |mut acc, (index, player)| {
                    let _ = write!(acc, "{}. {player}<br>", index + 1);
                    acc
                })
            },
            Self::PlayBall => "\"PLAY BALL.\"".to_string(),
            Self::GameOver { message } => message.to_string(),
            Self::Recordkeeping { winning_team, losing_team, winning_score, losing_score } => {
                format!("{winning_team} defeated {losing_team}. Final score: {winning_score}-{losing_score}")
            }
            Self::InningStart { number, side, batting_team, automatic_runner, pitcher_status } => {
                let ordinal = match number {
                    0 => panic!("Should not have 0th innings"),
                    1 => "1st".to_string(),
                    2 => "2nd".to_string(),
                    3 => "3rd".to_string(),
                    4.. => format!("{number}th")
                };
                let pitcher_message = match pitcher_status {
                    StartOfInningPitcher::Same { emoji, name } => format!("{emoji} {name} pitching."),
                    StartOfInningPitcher::Different { leaving_pitcher, arriving_pitcher } => {
                        format!("{leaving_pitcher} is leaving the game. {arriving_pitcher} takes the mound.")
                    }
                };
                let automatic_runner = match automatic_runner {
                    Some(runner) => format!(" {runner} starts the inning on second base."),
                    None => String::new()
                };
                format!("Start of the {side} of the {ordinal}. {batting_team} batting.{automatic_runner} {pitcher_message}")
            },
            Self::NowBatting {batter, stats} => {
                let stats = match stats {
                    NowBattingStats::FirstPA =>  format!(" (1st PA of game)"),
                    NowBattingStats::Stats { stats } => {
                        format!(" ({})", stats.into_iter().map(BatterStat::unparse).collect::<Vec<_>>().join(", "))
                    }
                    NowBattingStats::NoStats => {
                        String::new()
                    }
                };
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
            Self::MoundVisit {team } => {
                format!("The {team} manager is making a mound visit.")
            },
            Self::PitcherRemains { remaining_pitcher } => {
                format!("{remaining_pitcher} remains in the game.")
            },
            Self::PitcherSwap { leaving_pitcher, arriving_pitcher } => {
                format!("{leaving_pitcher} is leaving the game. {arriving_pitcher} takes the mound.")
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
            Self::HomeRun { batter, fair_ball_type, destination, scores, grand_slam } => {
                let scores = once(String::new()).chain(scores.into_iter().map(|runner| format!("<strong>{runner} scores!</strong>")))
                    .collect::<Vec<String>>()
                    .join(" ");

                if !grand_slam {
                    format!("<strong>{batter} homers on a {fair_ball_type} to {destination}!</strong>{scores}")
                } else {
                    format!("<strong>{batter} hits a grand slam on a {fair_ball_type} to {destination}!</strong>{scores}")
                }
            }
            Self::CaughtOut { batter, fair_ball_type, caught_by: catcher, scores, advances, sacrifice, perfect } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let sacrifice = if sacrifice {"on a sacrifice fly "} else {""};
                let perfect = if perfect {" <strong>Perfect catch!</strong>"} else {""};
                
                format!("{batter} {fair_ball_type} out {sacrifice}to {catcher}.{perfect}{scores_and_advances}")
            }
            Self::GroundedOut { batter, fielders, scores, advances, perfect } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders(fielders);
                let perfect = if perfect {" <strong>Perfect catch!</strong>"} else {""};
                format!("{batter} grounds out{fielders}.{scores_and_advances}{perfect}")
            }
            Self::ForceOut { batter, fielders, fair_ball_type, out, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders_for_play(fielders);
                let fair_ball_type = fair_ball_type.verb_name();
                format!("{batter} {fair_ball_type} into a force out{fielders}. {out}{scores_and_advances}")
            }
            Self::ReachOnFieldersChoice { batter, fielders, result, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                match result {
                    FieldingAttempt::Out {out} => {
                        let fielders = unparse_fielders_for_play(fielders);
                        
                        format!("{batter} reaches on a fielder's choice out{fielders}. {out}{scores_and_advances}")
                    }
                    FieldingAttempt::Error { fielder, error } => {
                        let fielder_long = fielders.first().unwrap();
                        let error = error.uppercase();
                        format!("{batter} reaches on a fielder's choice, fielded by {fielder_long}.{scores_and_advances} {error} error by {fielder}.")
                    }
                }
            }
            Self::DoublePlayGrounded { batter, fielders, out_one, out_two, scores, advances, sacrifice } => {
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let sacrifice = if sacrifice {"sacrifice "} else {""};
                format!("{batter} grounded into a {sacrifice}double play{fielders}. {out_one} {out_two}{scores_and_advances}")
            }
            Self::DoublePlayCaught { batter, fair_ball_type, fielders, out_two, scores, advances } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!("{batter} {fair_ball_type} into a double play{fielders}. {out_two}{scores_and_advances}")
            }
            Self::ReachOnFieldingError { batter, fielder, error, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let error = error.lowercase();
                format!("{batter} reaches on a {error} error by {fielder}.{scores_and_advances}")
            }
            Self::WeatherDelivery { team, player, item_emoji, item } => {
                format!("{team} {player} received a {item_emoji} {item} Delivery.")
            },
            Self::WeatherDeliveryDiscard { item_emoji, item } => {
                format!("{item_emoji} {item} was discarded as no player had space.")
            }
        }
    }
}

fn unparse_fielders<S:Display>(fielders: Vec<PositionedPlayer<S>>) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!(" to {}", fielders.first().unwrap()),
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
        leaving_pitcher: PositionedPlayer<S>,
        arriving_pitcher: PositionedPlayer<S>,
    }
}

/// Either an Out or an Error - e.g. for a Fielder's Choice.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FieldingAttempt<S> {
    Out {
        out: RunnerOut<S>,
    },
    Error {
        fielder: S,
        error: FieldingErrorType
    }
}
impl<S:Display> Display for FieldingAttempt<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Out { out } => {
                write!(f, "{} out at {}.", out.runner, out.base)
            },
            Self::Error { fielder, error } => {
                let error = error.uppercase();
                write!(f, "{error} by {fielder}.")
            }
        }
    }
}

/// A team's emoji and name, which is how teams are usually presented in mmolb.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EmojiTeam<S> {
    pub emoji: S,
    pub name: S
}
impl<S: Display> Display for EmojiTeam<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
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
    pub base: BaseNameVariant,
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
            true => write!(f, "{} is caught stealing {}.", self.runner, self.base.to_base_str()),
            false => match self.base {
                Base::Home => write!(f, "<strong>{} steals {}!</strong>", self.runner, self.base.to_base_str()),
                _ => write!(f, "{} steals {}!", self.runner, self.base.to_base_str()),
            }
        }
    }
}
impl<S> TryFrom<BaseSteal<S>> for RunnerOut<S> {
    type Error = ();
    fn try_from(value: BaseSteal<S>) -> Result<Self, Self::Error> {
        if value.caught {
            Ok(RunnerOut { runner: value.runner, base: BaseNameVariant::basic_name(value.base) })
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