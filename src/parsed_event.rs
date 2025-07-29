use std::{convert::Infallible, fmt::{Display, Write}, iter::once, str::FromStr};

use serde::{Serialize, Deserialize};
use strum::{EnumDiscriminants, EnumString, Display};
use thiserror::Error;

use crate::{enums::{Base, BaseNameVariant, BatterStat, Distance, EventType, FairBallDestination, FairBallType, FieldingErrorType, FoulType, GameOverMessage, HomeAway, ItemPrefix, ItemSuffix, ItemName, MoundVisitType, NowBattingStats, Place, StrikeType, TopBottom}, time::Breakpoints, Game, NotRecognized};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Error)]
pub enum GameEventParseError {
    #[error("event type {} not recognized", .0.0)]
    EventTypeNotRecognized(#[source] NotRecognized),
    #[error("failed parsing {event_type} event \"{message}\"")]
    FailedParsingMessage {
        event_type: EventType,
        message: String
    }
}

/// S is the string type used. S = &'output str is used by the parser,
/// but a mutable type is necessary when directly deserializing, because some players have escaped characters in their names
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
#[serde(tag = "event_type")]
pub enum ParsedEventMessage<S> {
    ParseError {
        error: GameEventParseError,
        message: S
    },
    KnownBug {
        bug: KnownBug<S>
    },
    // Season 0
    LiveNow {
        away_team: EmojiTeam<S>,
        home_team: EmojiTeam<S>,
        stadium: Option<S>
    },
    PitchingMatchup {
        away_team: EmojiTeam<S>,
        home_team: EmojiTeam<S>,
        home_pitcher: S,
        away_pitcher: S,
    },
    Lineup {
        side: HomeAway,
        players: Vec<PlacedPlayer<S>>
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
        team: EmojiTeam<S>,
        mound_visit_type: MoundVisitType
    },
    PitcherRemains {
        remaining_pitcher: PlacedPlayer<S>,
    },
    PitcherSwap {
        leaving_pitcher_emoji: Option<S>,
        leaving_pitcher: PlacedPlayer<S>,
        arriving_pitcher_emoji: Option<S>,
        arriving_pitcher_place: Option<Place>,
        arriving_pitcher_name: S,
    },

    // Pitch
    Ball { steals: Vec<BaseSteal<S>>, count:(u8, u8), cheer: Option<Cheer>, aurora_photos: Option<SnappedPhotos<S>>},
    Strike { strike: StrikeType, steals: Vec<BaseSteal<S>>, count:(u8, u8), cheer: Option<Cheer>},
    Foul { foul: FoulType, steals: Vec<BaseSteal<S>>, count:(u8, u8), cheer: Option<Cheer> },
    Walk { batter: S, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, cheer: Option<Cheer> },
    HitByPitch { batter: S, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, cheer: Option<Cheer> },
    FairBall {
        batter: S,
        fair_ball_type: FairBallType,
        destination: FairBallDestination,
        cheer: Option<Cheer>,
        ejection: Option<Ejection<S>>,
    },
    StrikeOut { foul: Option<FoulType>, batter: S, strike: StrikeType, steals: Vec<BaseSteal<S>>, cheer: Option<Cheer> },

    // Field
    BatterToBase { batter: S, distance: Distance, fair_ball_type: FairBallType, fielder: PlacedPlayer<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    HomeRun { batter: S, fair_ball_type: FairBallType, destination: FairBallDestination, scores: Vec<S>, grand_slam: bool },
    CaughtOut { batter: S, fair_ball_type: FairBallType, caught_by: PlacedPlayer<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, sacrifice: bool, perfect: bool },
    GroundedOut { batter: S, fielders: Vec<PlacedPlayer<S>>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, perfect: bool, ejection: Option<Ejection<S>> },
    ForceOut { batter: S, fielders: Vec<PlacedPlayer<S>>, fair_ball_type: FairBallType, out:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ReachOnFieldersChoice { batter: S, fielders: Vec<PlacedPlayer<S>>, result:FieldingAttempt<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    DoublePlayGrounded { batter: S, fielders: Vec<PlacedPlayer<S>>, out_one:RunnerOut<S>, out_two:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, sacrifice: bool },
    DoublePlayCaught { batter: S, fair_ball_type: FairBallType, fielders: Vec<PlacedPlayer<S>>, out_two:RunnerOut<S>, scores: Vec<S>, advances: Vec<RunnerAdvance<S>> },
    ReachOnFieldingError { batter: S, fielder:PlacedPlayer<S>, error: FieldingErrorType, scores: Vec<S>, advances: Vec<RunnerAdvance<S>>, ejection: Option<Ejection<S>> },

    // Season 1
    WeatherDelivery { delivery: Delivery<S> },
    FallingStar { player_name: S },
    FallingStarOutcome { deflection: Option<S>, player_name: S, outcome: FallingStarOutcome<S> },

    // Season 2
    WeatherShipment {
        deliveries: Vec<Delivery<S>>
    },
    WeatherSpecialDelivery {
        delivery: Delivery<S>
    },
    Balk {
        pitcher: S,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>
    },

    // Season 3,
    WeatherProsperity {
        home_income: u8,
        away_income: u8
    },

    // Season 4
    PhotoContest {
        winning_team: EmojiTeam<S>,
        winning_tokens: u8,
        winning_player: S,
        winning_score: u8,
        losing_team: EmojiTeam<S>,
        losing_tokens: u8,
        losing_player: S,
        losing_score: u8,
    }
}
impl<S: Display> ParsedEventMessage<S> {
    /// Recreate the event message this ParsedEvent was built out of.
    pub fn unparse(&self, game: &Game, event_index: Option<u16>) -> String {
        match self {
            Self::ParseError { message, .. } => message.to_string(),
            Self::LiveNow { away_team, home_team, stadium } => {
                match stadium {
                    Some(stadium) => format!("{} vs {} @ {}", away_team, home_team, stadium),
                    None => format!("{} @ {}", away_team, home_team),
                }
            },
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
                    StartOfInningPitcher::Different { leaving_emoji, leaving_pitcher, arriving_emoji, arriving_pitcher } => {
                        let leaving_emoji = leaving_emoji.as_ref().map(|e| format!("{e} ")).unwrap_or_default();
                        let arriving_emoji = arriving_emoji.as_ref().map(|e| format!("{e} ")).unwrap_or_default();
                        format!("{leaving_emoji}{leaving_pitcher} is leaving the game. {arriving_emoji}{arriving_pitcher} takes the mound.")
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
                    NowBattingStats::FirstPA =>  " (1st PA of game)".to_string(),
                    NowBattingStats::Stats(stats) => {
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
            Self::MoundVisit {team, mound_visit_type } => {
                match mound_visit_type {
                    MoundVisitType::MoundVisit => format!("The {team} manager is making a mound visit."),
                    MoundVisitType::PitchingChange => format!("The {team} manager is making a pitching change.")
                }
            },
            Self::PitcherRemains { remaining_pitcher } => {
                format!("{remaining_pitcher} remains in the game.")
            },
            Self::PitcherSwap { leaving_pitcher_emoji, leaving_pitcher, arriving_pitcher_emoji, arriving_pitcher_place, arriving_pitcher_name } => {
                let arriving_pitcher_place = arriving_pitcher_place.map(|place| format!("{place} ")).unwrap_or_default();
                let leaving_pitcher_emoji = leaving_pitcher_emoji.as_ref().map(|emoji| format!("{emoji} ")).unwrap_or_default();
                let arriving_pitcher_emoji = arriving_pitcher_emoji.as_ref().map(|emoji| format!("{emoji} ")).unwrap_or_default();

                format!("{leaving_pitcher_emoji}{leaving_pitcher} is leaving the game. {arriving_pitcher_emoji}{arriving_pitcher_place}{arriving_pitcher_name} takes the mound.")
            },

            Self::Ball { steals, count, cheer, aurora_photos } => {
                let steals = once(String::new()).chain(steals.iter().map(BaseSteal::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();

                let aurora_photos = aurora_photos.as_ref().map(|p| p.unparse()).unwrap_or_default();

                format!("{space}Ball. {}-{}.{steals}{cheer}{aurora_photos}", count.0, count.1,)
            },
            Self::Strike { strike, steals, count, cheer } => {
                let steals: Vec<String> = once(String::new()).chain(steals.into_iter().map(|steal| steal.to_string())).collect();
                let steals = steals.join(" ");
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();

                format!("{space}Strike, {strike}. {}-{}.{steals}{cheer}", count.0, count.1)
            }
            Self::Foul { foul, steals, count, cheer } => {
                let steals: Vec<String> = once(String::new()).chain(steals.into_iter().map(|steal| steal.to_string())).collect();
                let steals = steals.join(" ");
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();

                format!("{space}Foul {foul}. {}-{}.{steals}{cheer}", count.0, count.1)
            }
            Self::Walk { batter, scores, advances, cheer } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();

                format!("{space}Ball 4. {batter} walks.{scores_and_advances}{cheer}")
            }
            Self::HitByPitch { batter, scores, advances, cheer } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();

                format!("{space}{batter} was hit by the pitch and advances to first base.{scores_and_advances}{cheer}")
            }
            Self::FairBall { batter, fair_ball_type, destination, cheer, ejection } => {
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();

                format!("{space}{batter} hits a {fair_ball_type} to {destination}.{cheer}{ejection}")
            }
            Self::StrikeOut { foul, batter, strike, steals, cheer } => {
                let foul = match foul {
                    Some(foul) => format!("Foul {foul}. "),
                    None => String::new()
                };
                let steals: Vec<String> = once(String::new()).chain(steals.into_iter().map(|steal| steal.to_string())).collect();
                let steals = steals.join(" ");
                let space = old_space(game, event_index);

                let cheer = cheer.as_ref().map(|c| c.unparse(game, event_index)).unwrap_or_default();

                format!("{space}{foul}{batter} struck out {strike}.{steals}{cheer}")
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
                let sacrifice = if *sacrifice {"on a sacrifice fly "} else {""};
                let perfect = if *perfect {" <strong>Perfect catch!</strong>"} else {""};

                format!("{batter} {fair_ball_type} out {sacrifice}to {catcher}.{perfect}{scores_and_advances}")
            }
            Self::GroundedOut { batter, fielders, scores, advances, perfect, ejection } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders(fielders);
                let perfect = if *perfect {" <strong>Perfect catch!</strong>"} else {""};
                let ejection = if let Some(ej) = ejection { ej.unparse() } else { String::new() };
                format!("{batter} grounds out{fielders}.{scores_and_advances}{perfect}{ejection}")
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
                let sacrifice = if *sacrifice {"sacrifice "} else {""};
                format!("{batter} grounded into a {sacrifice}double play{fielders}. {out_one} {out_two}{scores_and_advances}")
            }
            Self::DoublePlayCaught { batter, fair_ball_type, fielders, out_two, scores, advances } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!("{batter} {fair_ball_type} into a double play{fielders}. {out_two}{scores_and_advances}")
            }
            Self::ReachOnFieldingError { batter, fielder, error, scores, advances, ejection } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let error = error.lowercase();
                let ejection = if let Some(e) = ejection { e.unparse() } else { String::new() };
                format!("{batter} reaches on a {error} error by {fielder}.{scores_and_advances}{ejection}")
            }
            Self::WeatherDelivery {delivery } => {
                delivery.unparse("Delivery")
            },
            Self::FallingStar { player_name } => {
                format!("<strong>🌠 {player_name} is hit by a Falling Star!</strong>")
            },
            Self::FallingStarOutcome { deflection, player_name, outcome } => {
                let deflection_msg = if let Some(deflected_off_player_name) = deflection {
                    format!("It deflected off {deflected_off_player_name} and struck {player_name}!</strong> <strong>")
                } else {
                    String::new()
                };

                let outcome_msg = match outcome {
                    FallingStarOutcome::Injury => format!("{player_name} was injured by the extreme force of the impact!"),
                    FallingStarOutcome::Retired(None) => format!("😇 {player_name} retired from MMOLB!"),
                    FallingStarOutcome::Retired(Some(replacement_player_name)) => {
                        format!("😇 {player_name} retired from MMOLB! {replacement_player_name} was called up to take their place.")
                    },
                    FallingStarOutcome::InfusionI => format!("{player_name} was infused with a glimmer of celestial energy!"),
                    FallingStarOutcome::InfusionII => format!("{player_name} began to glow brightly with celestial energy!"),
                    FallingStarOutcome::InfusionIII => format!("{player_name} was fully charged with an abundance of celestial energy!")
                };

                format!(" <strong>{deflection_msg}{outcome_msg}</strong>")
            },
            Self::WeatherShipment { deliveries } => {
                deliveries.iter().map(|d| d.unparse("Shipment")).collect::<Vec<String>>().join(" ")
            }
            Self::WeatherSpecialDelivery { delivery } => {
                delivery.unparse("Special Delivery")
            },
            Self::Balk { pitcher, scores, advances } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!("Balk. {pitcher} dropped the ball.{scores_and_advances}")
            },
            Self::KnownBug { bug } => format!("{bug}"),
            Self::WeatherProsperity { home_income, away_income } => {
                let home = (*home_income > 0).then_some(format!("{} {} are Prosperous! They earned {home_income} 🪙.", game.home_team_emoji, game.home_team_name)).unwrap_or_default();
                let away = (*away_income > 0).then_some(format!("{} {} are Prosperous! They earned {away_income} 🪙.", game.away_team_emoji, game.away_team_name)).unwrap_or_default();
                let gap = (*home_income > 0 && *away_income > 0).then_some(" ").unwrap_or_default();

                if Breakpoints::Season3PreSuperstarBreakUpdate.before(game.season, game.day.as_ref().ok().copied(), event_index) {
                    format!("{home}{gap}{away}")
                } else {
                    if home_income > away_income {
                        format!("{away}{gap}{home}")
                    } else {
                        format!("{home}{gap}{away}")
                    }
                }
            },
            Self::PhotoContest { winning_team, winning_tokens, winning_score, winning_player, losing_team, losing_tokens, losing_score, losing_player } => {
                let winning_emoji = &winning_team.emoji;
                let losing_emoji = &losing_team.emoji;
                format!("{winning_team} earned {winning_tokens} 🪙. {losing_team} earned {losing_tokens} 🪙.<br>Top scoring Photos:<br>{winning_emoji} {winning_player} - {winning_score} {losing_emoji} {losing_player} - {losing_score}")
            }
        }
    }
}

fn unparse_fielders<S:Display>(fielders: &Vec<PlacedPlayer<S>>) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!(" to {}", fielders.first().unwrap()),
        _ => format!(", {}", fielders.iter().map(PlacedPlayer::to_string).collect::<Vec<_>>().join(" to "))
    }
}

fn unparse_fielders_for_play<S:Display>(fielders: &Vec<PlacedPlayer<S>>) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!(", {} unassisted", fielders.first().unwrap()),
        _ => format!(", {}", fielders.iter().map(PlacedPlayer::to_string).collect::<Vec<_>>().join(" to "))
    }
}
fn unparse_scores_and_advances<S: Display>(scores: &Vec<S>, advances: &Vec<RunnerAdvance<S>>) -> String {
    once(String::new()).chain(scores.iter().map(|runner| format!("<strong>{runner} scores!</strong>"))
        .chain(advances.iter().map(|advance| advance.to_string())))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StartOfInningPitcher<S> {
    Same {emoji: S, name: S},
    Different {
        leaving_emoji: Option<S>,
        leaving_pitcher: PlacedPlayer<S>,
        arriving_emoji: Option<S>,
        arriving_pitcher: PlacedPlayer<S>,
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
pub struct PlacedPlayer<S> {
    pub name: S,
    pub place: Place
}
impl<S: Display> Display for PlacedPlayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.place, self.name)
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FallingStarOutcome<S> {
    Injury,
    Retired(Option<S>),
    InfusionI,
    InfusionII,
    InfusionIII
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ItemAffixes<S> {
    None,
    PrefixSuffix(Option<ItemPrefix>, Option<ItemSuffix>),
    RareName(S)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Item<S> {
    pub item_emoji: S,
    pub item: ItemName,
    pub affixes: ItemAffixes<S>
}

impl<S: Display> Display for Item<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Item { item_emoji, item, affixes} = self;

        match affixes {
            ItemAffixes::None => write!(f, "{item_emoji} {item}"),
            ItemAffixes::PrefixSuffix(prefix, suffix) => {
            let prefix = match prefix {
                Some(prefix) => format!("{prefix} "),
                None => String::new()
            };
            let suffix = match suffix {
                Some(suffix) => format!(" {suffix}"),
                None => String::new()
            };
                write!(f, "{item_emoji} {prefix}{item}{suffix}")
            },
            ItemAffixes::RareName(rare_name) => write!(f, "{item_emoji} {rare_name} {item}"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Delivery<S> {
    Successful {
        team: EmojiTeam<S>,
        player: Option<S>,
        item: Item<S>,
        discarded: Option<Item<S>>
    },
    NoSpace {
        item: Item<S>,
    }
}

impl<S: Display> Delivery<S> {
    pub fn unparse(&self, delivery_label: &str) -> String {
        match self {
            Self::Successful { team, player, item, discarded } => {
                let discarded = match discarded {
                    Some(discarded) => format!(" They discarded their {discarded}."),
                    None => String::new(),
                };

                let player = player.as_ref().map(|player| format!(" {player}")).unwrap_or_default();

                format!("{team}{player} received a {item} {delivery_label}.{discarded}")
            }
            Self::NoSpace { item } => {
                format!("{item} was discarded as no player had space.")
            }
        }
    }
}

fn old_space(game: &Game, event_index: Option<u16>) -> &'static str {
    if Breakpoints::S2D169.before(game.season, game.day.as_ref().copied().ok(), event_index) {
        " "
    } else {
        ""
    }
}

/// See individual variant documentation for an example of each bug, and the known properties of their events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
pub enum KnownBug<S> {
    /// https://mmolb.com/watch/6851bb34f419fdc04f9d0ed5 "Genevieve Hirose reaches on a fielder's choice out, 1B N. Kitagawa"
    ///
    /// Potentially fixed on S2D152
    ///
    /// Properties:
    /// - They are called fielders choice outs
    /// - They (so far) always occur when no runners are on base
    /// - They count as an out for the count
    /// - the batter does end up on base
    FirstBasemanChoosesAGhost {
        batter: S,
        first_baseman: S
    },
    /// https://mmolb.com/watch/68758c796154982c31f5d803?event=412 ""
    ///
    /// Potentially fixed as a side effect of the Season 3 Pre-Superstar Break Update, s3d112
    ///
    /// NPC teams cannot earn Tokens. Before s3d112, teams that got shutout didn't prosper.
    /// Thus, if an npc team shut out a player team, then an empty prosperity message would be displayed.
    ///
    /// Properties
    /// - It displays as an empty string
    /// - It is a prosperity event in which neither team earned any tokens.
    NoOneProspers
}

impl<S: Display> Display for KnownBug<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnownBug::FirstBasemanChoosesAGhost { batter, first_baseman } => {
                write!(f, "{batter} reaches on a fielder's choice out, 1B {first_baseman}")
            },
            KnownBug::NoOneProspers => {
                write!(f, "")
            }
        }
    }
}

fn _check(_: &str) -> Infallible {
    unreachable!("This is dead code that exists for a strum parse_err_fn")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, Display)]
#[strum(
    parse_err_fn = check,
    parse_err_ty = Infallible
)]
pub enum Cheer {
    #[strum(to_string = "A tremendous cheer fills the air")]
    ATremendousCheerFillsTheAir,
    #[strum(to_string = "A thunderous cheer echoes across the field")]
    AThunderousCheerEchoesAcrossTheField,
    #[strum(to_string = "Chants thunder around the stadium")]
    ChantsThunderAroundTheStadium,
    #[strum(to_string = "Everyone cheers at once")]
    EveryoneCheersAtOnce,
    #[strum(to_string = "Everyone is chanting the team's name")]
    EveryoneIsChantingTheTeamsName,
    #[strum(to_string = "The cheers drown out everything else")]
    TheCheersDrownOutEverythingElse,
    #[strum(to_string = "Cheers roll through the stadium")]
    CheersRollThroughTheStadium,
    #[strum(to_string = "Supporters pound on the railings")]
    SupportersPoundOnTheRailings,
    #[strum(to_string = "Hands clap and whistles pierce the air")]
    HandsClapAndWhistlesPierceTheAir,
    #[strum(to_string = "Home fans stomp their feet in unison")]
    HomeFansStompTheirFeetInUnison,
    #[strum(to_string = "No one is sitting anymore")]
    NoOneIsSittingAnymore,
    #[strum(to_string = "The hometown supporters give a roar")]
    TheHometownSupportersGiveARoar,
    #[strum(to_string = "The hometown section is deafening")]
    TheHomeTownSectionIsDeafening,
    #[strum(to_string = "The home faithful go berserk")]
    TheHomeFaithfulGoBerserk,
    #[strum(to_string = "Fans are losing their minds")]
    TheFansAreLosingTheirMinds,
    #[strum(to_string = "The stands explode in noise")]
    TheStandsExplodeInNoise,
    #[strum(to_string = "It's a wall of sound")]
    ItsAWallOfSound,
    #[strum(to_string = "The entire ballpark is rocking")]
    TheEntireBallparkIsRocking,
    #[strum(to_string = "The ballpark comes alive")]
    TheBallparkComesAlive,
    #[strum(to_string = "Excitement pours from the crowd")]
    ExcitementPoursFromTheCrowd,
    #[strum(to_string = "The home supporters can't contain themselves")]
    TheHomeSupportersCantContainThemselves,
    #[strum(to_string = "The crowd lets out a collective howl")]
    TheCrowdLetsOutACollectiveHowl,
    #[strum(to_string = "A rumble of excitement rolls through the stadium")]
    ARumbleOfExcitementRollsThroughTheStadium,
    #[strum(to_string = "Banners wave wildly")]
    BannersWaveWildly,
    #[strum(to_string = "The noise level skyrockets")]
    TheNoiseLevelSkyrockets,
    #[strum(to_string = "The energy in here is palpable")]
    TheEnergyInHereIsPalpable,
    #[strum(to_string = "The ballpark erupts in applause")]
    TheBallparkEruptsInApplause,
    #[strum(to_string = "The bleachers are booming")]
    TheBleachersAreBooming,
    #[strum(to_string = "Fans slap the walls in rhythm")]
    FansSlapTheWallsInRhythm,
    #[strum(to_string = "The hometown fans roar their support")]
    TheHometownFansRoarTheirSupport,
    #[strum(to_string = "Noise rains down from the seats")]
    NoiseRainsDownFromTheSeats,
    #[strum(to_string = "Fans wave their arms wildly")]
    FansWaveTheirArmsWildly,
    #[strum(to_string = "Energy pulses from the stands")]
    EnergyPulsesFromTheStands,
    #[strum(to_string = "The stands shake with noise")]
    TheStandsShakeWithNoise,
    #[strum(to_string = "The ballpark is in an uproar")]
    TheBallparkIsInAnUproar,
    #[strum(to_string = "The stadium swell with cheers")]
    TheStadiumSwellWithCheers,
    #[strum(to_string = "The stadium erupts")]
    TheStadiumErupts,
    #[strum(to_string = "Fans roar in support")]
    FansRoarInSupport,
    #[strum(to_string = "A chant grows louder and louder")]
    AChantGrowsLouderAndLouder,
    #[strum(to_string = "Fans shout encouragement")]
    FansShoutEncouragement,
    #[strum(to_string = "The stadium is buzzing")]
    TheStadiumIsBuzzing,
    #[strum(to_string = "The hometown faithful are loving this")]
    TheHometownFaithfulAreLovingThis,
    #[strum(to_string = "Fans whoop and holler")]
    FansWhoopAndHoller,
    #[strum(to_string = "The home crowd is fired up")]
    TheHomeCrowdIsFiredUp,
    #[strum(to_string = "It's pandemonium in the stands")]
    ItsPandemoniumInTheStands,
    #[strum(to_string = "The noise is overwhelming")]
    TheNoiseIsOverwhelming,
    #[strum(to_string = "Home supporters whistle and cheer")]
    HomeSupportersWhistleAndCheer,
    #[strum(to_string = "The home fans make themselves heard")]
    TheHomeFansMakeThemselvesHeard,
    #[strum(to_string = "The crowd rallies behind the team")]
    TheCrowdRalliesBehindTheTeam,
    #[strum(to_string = "The stands are rumbling")]
    TheStandsAreRumbling,
    #[strum(to_string = "It's pure pandemonium")]
    ItsPurePandemonium,
    #[strum(to_string = "The place is rocking")]
    ThePlaceIsRocking,
    #[strum(to_string = "A wave of cheers sweeps the stadium")]
    AWaveOfCheersSweepsTheStadium,
    #[strum(to_string = "The hometown fans roar")]
    TheHometownFansRoar,
    #[strum(to_string = "The fans thunder their approval")]
    TheFansThunderTheirApproval,
    #[strum(to_string = "Supporters yell at full volume")]
    SupportersYellAtFullVolume,
    #[strum(to_string = "The crowd goes absolutely bonkers")]
    TheCrowdGoesAbsolutelyBonkers,
    #[strum(to_string = "The cheering is relentless")]
    TheCheeringIsRelentless,
    #[strum(to_string = "Fans pump their fists in the air")]
    FansPumpTheirFistsInTheAir,
    #[strum(to_string = "The home crowd won't stop cheering")]
    TheHomeCrowdWontStopCheering,
    #[strum(to_string = "The crowd erupts into cheers")]
    TheCrowdEruptsIntoCheers,
    #[strum(to_string = "A roar builds from the seats")]
    ARoarBuildsFromTheSeats,
    #[strum(to_string = "Everyone in the stands is on their feet")]
    EveryoneInTheStandsIsOnTheirFeet,
    #[strum(to_string = "It's a roar from the rafters")]
    ItsARoarFromTheRafters,
    #[strum(to_string = "Chants rise from the bleachers")]
    ChantsRiseFromTheBleachers,
    #[strum(to_string = "A cheer rips through the park")]
    ACheerRipsThroughThePark,
    #[strum(to_string = "Everyone's clapping in rhythm")]
    EveryonesClappingInRhythm,
    #[strum(to_string = "The crowd is pumped up")]
    TheCrowdIsPumpedUp,
    #[strum(to_string = "The stadium swells with cheers")]
    TheStadiumSwellsWithCheers,
    #[strum(to_string = "You can barely hear the announcer")]
    YouCanBarelyHearTheAnnouncer,
    #[strum(to_string = "The fans are fired up")]
    TheFansAreFiredUp,
    #[strum(to_string = "The decibels rise to a frenzy")]
    TheDecibelsRiseToAFrenzy,
    #[strum(to_string = "The energy in the park surges")]
    TheEnergyInTheParkSurges,
    #[strum(to_string = "Fans yell themselves hoarse")]
    FansYellThemselvesHoarse,
    #[strum(to_string = "A mighty cheer erupts")]
    AMightyCheerErupts,
    #[strum(to_string = "The stands are electric")]
    TheStandsAreElectric,
    #[strum(to_string = "Fans jump and shout")]
    FansJumpAndShout,
    #[strum(to_string = "The cheers echo off the walls")]
    TheCheersEchoOffTheWalls,
    #[strum(to_string = "You can feel the stands vibrating")]
    YouCanFeelTheStandsVibrating,
    #[strum(to_string = "Noise levels spike")]
    NoiseLevelsSpike,
    #[strum(to_string = "Everyone is cheering at once")]
    EveryoneIsCheeringAtOnce,
    #[strum(to_string = "A huge cheer erupts")]
    AHugeCheerErupts,
    #[strum(to_string = "The noise just keeps building")]
    TheNoiseJustKeepsBuilding,
    #[strum(to_string = "The faithful are shouting at the top of their lungs")]
    TheFaithfulAreShoutingAtTheTopOfTheirLungs,
    #[strum(to_string = "They cheer like there's no tomorrow")]
    TheyCheerLikeTheresNoTomorrow,
    #[strum(to_string = "You can feel the hype building")]
    YouCanFeelTheHypeBuilding,
    #[strum(to_string = "The excitement is off the charts")]
    TheExcitementIsOffTheCharts,
    #[strum(to_string = "Excitement surges through the park")]
    ExcitementSurgesThroughThePark,
    #[strum(to_string = "Every fan is making noise")]
    EveryFanIsMakingNoise,
    #[strum(to_string = "The supporters fuel their team")]
    TheSupportersFuelTheirTeam,
    #[strum(to_string = "The park vibrates with noise")]
    TheParkVibratesWithNoise,
    #[strum(to_string = "The stands are shaking")]
    TheStandsAreShaking,
    #[strum(to_string = "The fans bellow encouragement")]
    TheFansBellowEncouragement,
    #[strum(to_string = "The stadium shakes with excitement")]
    TheStadiumShakesWithExcitement,
    #[strum(to_string = "Supporters wave their banners high")]
    SupportersWaveTheirBannersHigh,
    #[strum(to_string = "The faithful rise as one")]
    TheFaithfulRiseAsOne,
    #[strum(to_string = "Cheers cascade from every section")]
    CheersCascadeFromEverySection,
    #[strum(to_string = "The crowd belts out the team's chant")]
    TheCrowdBeltsOutTheTeamsChant,
    #[strum(to_string = "The fans scream for their heroes")]
    TheFansScreamForTheirHeroes,
    #[strum(to_string = "The crowd is ecstatic")]
    TheCrowdIsEcstatic,
    #[strum(to_string = "The crowd is pumped")]
    TheCrowdIsPumped,

    #[strum(default)]
    Unknown(String)
}

impl Cheer {
    pub fn new(value: &str) -> Self {
        let r = Cheer::from_str(value)
            .expect("This error type is infallible");

        if matches!(r, Cheer::Unknown(_)) {
            tracing::warn!("Failed to match cheer '{value}'");
        }

        r
    }

    pub fn unparse(&self, game: &Game, event_index: Option<u16>) -> String {
        if Breakpoints::CheersGetEmoji.before(game.season, game.day.as_ref().ok().copied(), event_index) {
            format!(" {self}!")
        } else {
            format!(" 📣 {self}!")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, Display)]
#[strum(
    parse_err_fn = check,
    parse_err_ty = Infallible
)]
pub enum EjectionReason {
    #[strum(to_string = "eating a hotdog")]
    EatingAHotdog,

    #[strum(default)]
    Unknown(String)
}

impl EjectionReason {
    pub fn new(value: &str) -> Self {
        let r = EjectionReason::from_str(value)
            .expect("This error type is infallible");

        if matches!(r, EjectionReason::Unknown(_)) {
            tracing::warn!("Failed to match ejection reason '{value}'");
        }

        r
    }

    pub fn unparse(&self) -> String {
        format!("{self}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SnappedPhotos<S> {
    pub away_team_emoji: S,
    pub away_player: PlacedPlayer<S>,
    pub home_team_emoji: S,
    pub home_player: PlacedPlayer<S>,
}

impl<S: Display> SnappedPhotos<S> {
    pub fn unparse(&self) -> String {
        format!(
            " The Geomagnetic Storms Intensify! {} {} and {} {} snapped photos of the aurora.",
            self.away_team_emoji, self.away_player, self.home_team_emoji, self.home_player,
        )
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ejection<S> {
    pub team: EmojiTeam<S>,
    pub ejected_player: PlacedPlayer<S>,
    pub reason: EjectionReason,
    pub replacement_player_name: S,
}

impl<S: Display> Ejection<S> {
    pub fn unparse(&self) -> String {
        format!(
            " 🤖 ROBO-UMP ejected {} {} for a Sportsmanship Violation ({}). Bench Player {} takes their place.",
            self.team,
            self.ejected_player,
            self.reason,
            self.replacement_player_name,
        )
    }
}
