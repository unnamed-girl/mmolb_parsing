use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::{
    convert::Infallible,
    fmt::{Display, Write},
    iter::once,
    str::FromStr,
};
use strum::{Display, EnumDiscriminants, EnumString, IntoStaticStr};
use thiserror::Error;

use crate::enums::{Attribute, FoodName};
use crate::nom_parsing::shared::{discarded_text, received_text};
use crate::UnparsingContext;
use crate::{
    enums::{
        Base, BaseNameVariant, BatterStat, Distance, EventType, FairBallDestination, FairBallType,
        FieldingErrorType, FoulType, GameOverMessage, HomeAway, ItemName, ItemPrefix, ItemSuffix,
        MoundVisitType, NowBattingStats, Place, StrikeType, TopBottom,
    },
    nom_parsing::shared::{hit_by_pitch_text, strike_out_text},
    time::Breakpoints,
    NotRecognized,
};

pub use crate::nom_parsing::shared::GrowAttributeChange;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Error)]
pub enum GameEventParseError {
    #[error("event type {} not recognized", .0.0)]
    EventTypeNotRecognized(#[source] NotRecognized),
    #[error("failed parsing {event_type} event \"{message}\"")]
    FailedParsingMessage {
        event_type: EventType,
        message: String,
    },
}

/// S is the string type used. S = &'output str is used by the parser,
/// but a mutable type is necessary when directly deserializing, because some players have escaped characters in their names
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
#[serde(tag = "event_type")]
pub enum ParsedEventMessage<S> {
    ParseError {
        error: GameEventParseError,
        message: S,
    },
    KnownBug {
        bug: KnownBug<S>,
    },
    // Season 0
    LiveNow {
        away_team: EmojiTeam<S>,
        home_team: EmojiTeam<S>,
        stadium: Option<S>,
    },
    PitchingMatchup {
        away_team: EmojiTeam<S>,
        home_team: EmojiTeam<S>,
        home_pitcher: S,
        away_pitcher: S,
    },
    Lineup {
        side: HomeAway,
        players: Vec<PlacedPlayer<S>>,
    },
    PlayBall,
    GameOver {
        message: GameOverMessage,
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
        /// This message doesn't display properly for the superstar game.
        pitcher_status: Option<StartOfInningPitcher<S>>,
    },
    NowBatting {
        batter: S,
        stats: NowBattingStats,
    },
    InningEnd {
        number: u8,
        side: TopBottom,
    },

    // Mound visits
    MoundVisit {
        team: EmojiTeam<S>,
        mound_visit_type: MoundVisitType,
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
    Ball {
        steals: Vec<BaseSteal<S>>,
        count: (u8, u8),
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        ejection: Option<Ejection<S>>,
        door_prizes: Vec<DoorPrize<S>>,
        wither: Option<WitherStruggle<S>>,
        efflorescence: Vec<Efflorescence<S>>,
    },
    Strike {
        strike: StrikeType,
        steals: Vec<BaseSteal<S>>,
        count: (u8, u8),
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        ejection: Option<Ejection<S>>,
        door_prizes: Vec<DoorPrize<S>>,
        wither: Option<WitherStruggle<S>>,
        efflorescence: Vec<Efflorescence<S>>,
    },
    Foul {
        foul: FoulType,
        steals: Vec<BaseSteal<S>>,
        count: (u8, u8),
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        door_prizes: Vec<DoorPrize<S>>,
        wither: Option<WitherStruggle<S>>,
        efflorescence: Vec<Efflorescence<S>>,
    },
    Walk {
        batter: S,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        ejection: Option<Ejection<S>>,
        wither: Option<WitherStruggle<S>>,
    },
    HitByPitch {
        batter: S,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        ejection: Option<Ejection<S>>,
        door_prizes: Vec<DoorPrize<S>>,
        wither: Option<WitherStruggle<S>>,
        efflorescence: Vec<Efflorescence<S>>,
    },
    FairBall {
        batter: S,
        fair_ball_type: FairBallType,
        destination: FairBallDestination,
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        door_prizes: Vec<DoorPrize<S>>,
        efflorescence: Vec<Efflorescence<S>>,
    },
    StrikeOut {
        foul: Option<FoulType>,
        batter: S,
        strike: StrikeType,
        steals: Vec<BaseSteal<S>>,
        cheer: Option<Cheer>,
        aurora_photos: Option<SnappedPhotos<S>>,
        ejection: Option<Ejection<S>>,
        wither: Option<WitherStruggle<S>>,
    },

    // Field
    BatterToBase {
        batter: S,
        distance: Distance,
        fair_ball_type: FairBallType,
        fielder: PlacedPlayer<S>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        ejection: Option<Ejection<S>>,
    },
    HomeRun {
        batter: S,
        fair_ball_type: FairBallType,
        destination: FairBallDestination,
        scores: Vec<S>,
        grand_slam: bool,
        ejection: Option<Ejection<S>>,
    },
    CaughtOut {
        batter: S,
        fair_ball_type: FairBallType,
        caught_by: PlacedPlayer<S>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        sacrifice: bool,
        perfect: bool,
        ejection: Option<Ejection<S>>,
    },
    GroundedOut {
        batter: S,
        fielders: Vec<PlacedPlayer<S>>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        amazing: bool,
        ejection: Option<Ejection<S>>,
    },
    ForceOut {
        batter: S,
        fielders: Vec<PlacedPlayer<S>>,
        fair_ball_type: FairBallType,
        out: RunnerOut<S>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        ejection: Option<Ejection<S>>,
    },
    ReachOnFieldersChoice {
        batter: S,
        fielders: Vec<PlacedPlayer<S>>,
        result: FieldingAttempt<S>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        ejection: Option<Ejection<S>>,
    },
    DoublePlayGrounded {
        batter: S,
        fielders: Vec<PlacedPlayer<S>>,
        out_one: RunnerOut<S>,
        out_two: RunnerOut<S>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        sacrifice: bool,
        ejection: Option<Ejection<S>>,
    },
    DoublePlayCaught {
        batter: S,
        fair_ball_type: FairBallType,
        fielders: Vec<PlacedPlayer<S>>,
        out_two: RunnerOut<S>,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        ejection: Option<Ejection<S>>,
    },
    ReachOnFieldingError {
        batter: S,
        fielder: PlacedPlayer<S>,
        error: FieldingErrorType,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
        ejection: Option<Ejection<S>>,
    },

    // Season 1
    WeatherDelivery {
        delivery: Delivery<S>,
    },
    FallingStar {
        player_name: S,
    },
    FallingStarOutcome {
        deflection: Option<S>,
        player_name: S,
        outcome: FallingStarOutcome<S>,
    },

    // Season 2
    WeatherShipment {
        deliveries: Vec<Delivery<S>>,
    },
    WeatherSpecialDelivery {
        delivery: Delivery<S>,
    },
    Balk {
        pitcher: S,
        scores: Vec<S>,
        advances: Vec<RunnerAdvance<S>>,
    },

    // Season 3,
    WeatherProsperity {
        home_income: u32,
        away_income: u32,
    },

    // Season 4
    PhotoContest {
        winning_team: EmojiTeam<S>,
        winning_tokens: u32,
        winning_player: S,
        winning_score: u16,
        losing_team: EmojiTeam<S>,
        losing_tokens: u32,
        losing_player: S,
        losing_score: u16,
    },

    // Season 5
    Party {
        pitcher_name: S,
        pitcher_amount_gained: u8,
        pitcher_attribute: Attribute,
        batter_name: S,
        batter_amount_gained: u8,
        batter_attribute: Attribute,
        durability_loss: PartyDurabilityLoss<S>,
    },
    WeatherReflection {
        team: EmojiTeam<S>,
    },
    WeatherWither {
        team_emoji: S,
        player: PlacedPlayer<S>,
        corrupted: WitherResult,
        contained: ContainResult<S>,
    },

    // Season 6
    LinealBeltTransfer {
        claimed_by: EmojiTeam<S>,
        claimed_from: EmojiTeam<S>,
    },

    // Season 9
    WeatherConsumption(WeatherConsumptionEvents<S>),
}
impl<S: Display> ParsedEventMessage<S> {
    /// Recreate the event message this ParsedEvent was built out of.
    pub fn unparse<'a>(
        &self,
        context: impl Into<UnparsingContext<'a>>,
        event_index: Option<u16>,
    ) -> String {
        let context = context.into();
        match self {
            Self::ParseError { message, .. } => message.to_string(),
            Self::LiveNow {
                away_team,
                home_team,
                stadium,
            } => match stadium {
                Some(stadium) => format!("{} vs {} @ {}", away_team, home_team, stadium),
                None => format!("{} @ {}", away_team, home_team),
            },
            Self::PitchingMatchup {
                away_team,
                home_team,
                home_pitcher,
                away_pitcher,
            } => format!("{away_team} {away_pitcher} vs. {home_team} {home_pitcher}"),
            Self::Lineup { side: _, players } => {
                players
                    .iter()
                    .enumerate()
                    .fold(String::new(), |mut acc, (index, player)| {
                        let _ = write!(acc, "{}. {player}<br>", index + 1);
                        acc
                    })
            }
            Self::PlayBall => "\"PLAY BALL.\"".to_string(),
            Self::GameOver { message } => message.to_string(),
            Self::Recordkeeping {
                winning_team,
                losing_team,
                winning_score,
                losing_score,
            } => {
                format!("{winning_team} defeated {losing_team}. Final score: {winning_score}-{losing_score}")
            }
            Self::InningStart {
                number,
                side,
                batting_team,
                automatic_runner,
                pitcher_status,
            } => {
                let ordinal = match number {
                    0 => panic!("Should not have 0th innings"),
                    1 => "1st".to_string(),
                    2 => "2nd".to_string(),
                    3 => "3rd".to_string(),
                    4.. => format!("{number}th"),
                };
                let pitcher_message = match pitcher_status {
                    Some(StartOfInningPitcher::Same { emoji, name }) => {
                        format!(" {emoji} {name} pitching.")
                    }
                    Some(StartOfInningPitcher::Different {
                        leaving_emoji,
                        leaving_pitcher,
                        arriving_emoji,
                        arriving_pitcher,
                    }) => {
                        let leaving_emoji = leaving_emoji
                            .as_ref()
                            .map(|e| format!("{e} "))
                            .unwrap_or_default();
                        let arriving_emoji = arriving_emoji
                            .as_ref()
                            .map(|e| format!("{e} "))
                            .unwrap_or_default();
                        format!(" {leaving_emoji}{leaving_pitcher} is leaving the game. {arriving_emoji}{arriving_pitcher} takes the mound.")
                    }
                    None => String::new(),
                };
                let automatic_runner = match automatic_runner {
                    Some(runner) => format!(" {runner} starts the inning on second base."),
                    None => String::new(),
                };
                format!("Start of the {side} of the {ordinal}. {batting_team} batting.{automatic_runner}{pitcher_message}")
            }
            Self::NowBatting { batter, stats } => {
                let stats = match stats {
                    NowBattingStats::FirstPA => " (1st PA of game)".to_string(),
                    NowBattingStats::Stats(stats) => {
                        format!(
                            " ({})",
                            stats
                                .iter()
                                .map(BatterStat::unparse)
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                    NowBattingStats::NoStats => String::new(),
                };
                format!("Now batting: {batter}{stats}")
            }
            Self::InningEnd { number, side } => {
                let ordinal = match number {
                    0 => panic!("Should not have 0th innings"),
                    11 => "11th".to_string(),
                    12 => "12th".to_string(),
                    13 => "13th".to_string(),
                    _ => match number % 10 {
                        1 => format!("{number}st"),
                        2 => format!("{number}nd"),
                        3 => format!("{number}rd"),
                        _ => format!("{number}th")
                    },
                };
                format!("End of the {side} of the {ordinal}.")
            }
            Self::MoundVisit {
                team,
                mound_visit_type,
            } => match mound_visit_type {
                MoundVisitType::MoundVisit => {
                    format!("The {team} manager is making a mound visit.")
                }
                MoundVisitType::PitchingChange => {
                    format!("The {team} manager is making a pitching change.")
                }
            },
            Self::PitcherRemains { remaining_pitcher } => {
                format!("{remaining_pitcher} remains in the game.")
            }
            Self::PitcherSwap {
                leaving_pitcher_emoji,
                leaving_pitcher,
                arriving_pitcher_emoji,
                arriving_pitcher_place,
                arriving_pitcher_name,
            } => {
                let arriving_pitcher_place = arriving_pitcher_place
                    .map(|place| format!("{place} "))
                    .unwrap_or_default();
                let leaving_pitcher_emoji = leaving_pitcher_emoji
                    .as_ref()
                    .map(|emoji| format!("{emoji} "))
                    .unwrap_or_default();
                let arriving_pitcher_emoji = arriving_pitcher_emoji
                    .as_ref()
                    .map(|emoji| format!("{emoji} "))
                    .unwrap_or_default();

                format!("{leaving_pitcher_emoji}{leaving_pitcher} is leaving the game. {arriving_pitcher_emoji}{arriving_pitcher_place}{arriving_pitcher_name} takes the mound.")
            }

            Self::Ball {
                steals,
                count,
                cheer,
                aurora_photos,
                ejection,
                door_prizes,
                wither,
                efflorescence,
            } => {
                let steals = once(String::new())
                    .chain(steals.iter().map(BaseSteal::to_string))
                    .collect::<Vec<String>>()
                    .join(" ");
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();
                let door_prizes = once(String::new())
                    .chain(door_prizes.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>");
                let wither = wither
                    .as_ref()
                    .map_or_else(String::new, |wither| format!(" {}", wither));
                let efflorescence = once(String::new())
                    .chain(efflorescence.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>🌹 ");

                format!("{space}Ball. {}-{}.{steals}{aurora_photos}{ejection}{cheer}{door_prizes}{wither}{efflorescence}", count.0, count.1)
            }
            Self::Strike {
                strike,
                steals,
                count,
                cheer,
                aurora_photos,
                ejection,
                door_prizes,
                wither,
                efflorescence,
            } => {
                let steals: Vec<String> = once(String::new())
                    .chain(steals.iter().map(|steal| steal.to_string()))
                    .collect();
                let steals = steals.join(" ");
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();
                let door_prizes = once(String::new())
                    .chain(door_prizes.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>");
                let wither = wither
                    .as_ref()
                    .map_or_else(String::new, |wither| format!(" {}", wither));
                let efflorescence = once(String::new())
                    .chain(efflorescence.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>🌹 ");

                format!("{space}Strike, {strike}. {}-{}.{steals}{aurora_photos}{ejection}{cheer}{door_prizes}{wither}{efflorescence}", count.0, count.1)
            }
            Self::Foul {
                foul,
                steals,
                count,
                cheer,
                aurora_photos,
                door_prizes,
                wither,
                efflorescence,
            } => {
                let steals: Vec<String> = once(String::new())
                    .chain(steals.iter().map(|steal| steal.to_string()))
                    .collect();
                let steals = steals.join(" ");
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let door_prizes = once(String::new())
                    .chain(door_prizes.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>");
                let wither = wither
                    .as_ref()
                    .map_or_else(String::new, |wither| format!(" {}", wither));
                let efflorescence = once(String::new())
                    .chain(efflorescence.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>🌹 ");

                format!("{space}Foul {foul}. {}-{}.{steals}{aurora_photos}{cheer}{door_prizes}{wither}{efflorescence}", count.0, count.1)
            }
            Self::Walk {
                batter,
                scores,
                advances,
                cheer,
                aurora_photos,
                ejection,
                wither,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();
                let wither = wither
                    .as_ref()
                    .map_or_else(String::new, |wither| format!(" {}", wither));

                // Proof cheer is before ejection: https://mmolb.com/watch/6887e503f142e23550fc1254?event=369
                format!("{space}Ball 4. {batter} walks.{scores_and_advances}{aurora_photos}{cheer}{ejection}{wither}")
            }
            Self::HitByPitch {
                batter,
                scores,
                advances,
                cheer,
                aurora_photos,
                ejection,
                door_prizes,
                wither,
                efflorescence,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();
                let door_prizes = once(String::new())
                    .chain(door_prizes.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>");
                let hbp_text = hit_by_pitch_text(context.season, context.day, event_index);
                let wither = wither
                    .as_ref()
                    .map_or_else(String::new, |wither| format!(" {}", wither));
                let efflorescence = once(String::new())
                    .chain(efflorescence.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>🌹 ");
                format!("{space}{batter}{hbp_text}.{scores_and_advances}{aurora_photos}{cheer}{ejection}{door_prizes}{wither}{efflorescence}")
            }
            Self::FairBall {
                batter,
                fair_ball_type,
                destination,
                cheer,
                aurora_photos,
                door_prizes,
                efflorescence,
            } => {
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let door_prizes = once(String::new())
                    .chain(door_prizes.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>");
                let efflorescence = once(String::new())
                    .chain(efflorescence.iter().map(|d| d.unparse()))
                    .collect::<Vec<_>>()
                    .join("<br>🌹 ");

                format!("{space}{batter} hits a {fair_ball_type} to {destination}.{aurora_photos}{cheer}{door_prizes}{efflorescence}")
            }
            Self::StrikeOut {
                foul,
                batter,
                strike,
                steals,
                cheer,
                aurora_photos,
                ejection,
                wither,
            } => {
                let foul = match foul {
                    Some(foul) => format!("Foul {foul}. "),
                    None => String::new(),
                };
                let steals: Vec<String> = once(String::new())
                    .chain(steals.iter().map(|steal| steal.to_string()))
                    .collect();
                let steals = steals.join(" ");
                let space = old_space(context, event_index);

                let cheer = cheer
                    .as_ref()
                    .map(|c| c.unparse(context, event_index))
                    .unwrap_or_default();
                let aurora_photos = aurora_photos
                    .as_ref()
                    .map(|p| p.unparse())
                    .unwrap_or_default();
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();
                let wither = wither
                    .as_ref()
                    .map_or_else(String::new, |wither| format!(" {}", wither));

                // I do have proof that cheer is before ejection at least on this event
                // (game 6887e4f9f142e23550fc1134 event 265)
                let strike_out_text = strike_out_text(context.season, context.day, event_index);
                format!("{space}{foul}{batter}{strike_out_text}{strike}.{steals}{aurora_photos}{cheer}{ejection}{wither}")
            }
            Self::BatterToBase {
                batter,
                distance,
                fair_ball_type,
                fielder,
                scores,
                advances,
                ejection,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();
                format!("{batter} {distance} on a {fair_ball_type} to {fielder}.{scores_and_advances}{ejection}")
            }
            Self::HomeRun {
                batter,
                fair_ball_type,
                destination,
                scores,
                grand_slam,
                ejection,
            } => {
                let scores = once(String::new())
                    .chain(
                        scores
                            .iter()
                            .map(|runner| format!("<strong>{runner} scores!</strong>")),
                    )
                    .collect::<Vec<String>>()
                    .join(" ");
                let ejection = ejection.as_ref().map(|e| e.unparse()).unwrap_or_default();

                if !grand_slam {
                    format!("<strong>{batter} homers on a {fair_ball_type} to {destination}!</strong>{scores}{ejection}")
                } else {
                    format!("<strong>{batter} hits a grand slam on a {fair_ball_type} to {destination}!</strong>{scores}{ejection}")
                }
            }
            Self::CaughtOut {
                batter,
                fair_ball_type,
                caught_by: catcher,
                scores,
                advances,
                ejection,
                sacrifice,
                perfect,
            } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let sacrifice = if *sacrifice {
                    "on a sacrifice fly "
                } else {
                    ""
                };
                let perfect = if *perfect {
                    " <strong>Perfect catch!</strong>"
                } else {
                    ""
                };
                let ejection = if let Some(ej) = ejection {
                    ej.unparse()
                } else {
                    String::new()
                };

                format!("{batter} {fair_ball_type} out {sacrifice}to {catcher}.{scores_and_advances}{perfect}{ejection}")
            }
            Self::GroundedOut {
                batter,
                fielders,
                scores,
                advances,
                amazing,
                ejection,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders(fielders);
                let perfect = if *amazing {
                    if context.season < 5 {
                        " <strong>Perfect catch!</strong>"
                    } else {
                        " <strong>Amazing throw!</strong>"
                    }
                } else {
                    ""
                };
                let ejection = if let Some(ej) = ejection {
                    ej.unparse()
                } else {
                    String::new()
                };
                format!("{batter} grounds out{fielders}.{scores_and_advances}{perfect}{ejection}")
            }
            Self::ForceOut {
                batter,
                fielders,
                fair_ball_type,
                out,
                scores,
                advances,
                ejection,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let fielders = unparse_fielders_for_play(fielders);
                let fair_ball_type = fair_ball_type.verb_name();
                let ejection = if let Some(ej) = ejection {
                    ej.unparse()
                } else {
                    String::new()
                };
                format!("{batter} {fair_ball_type} into a force out{fielders}. {out}{scores_and_advances}{ejection}")
            }
            Self::ReachOnFieldersChoice {
                batter,
                fielders,
                result,
                scores,
                advances,
                ejection,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let ejection = if let Some(ej) = ejection {
                    ej.unparse()
                } else {
                    String::new()
                };
                match result {
                    FieldingAttempt::Out { out } => {
                        let fielders = unparse_fielders_for_play(fielders);

                        format!("{batter} reaches on a fielder's choice out{fielders}. {out}{scores_and_advances}{ejection}")
                    }
                    FieldingAttempt::Error { fielder, error } => {
                        let fielder_long = fielders.first().unwrap();
                        let error = error.uppercase();
                        format!("{batter} reaches on a fielder's choice, fielded by {fielder_long}.{scores_and_advances} {error} error by {fielder}.{ejection}")
                    }
                }
            }
            Self::DoublePlayGrounded {
                batter,
                fielders,
                out_one,
                out_two,
                scores,
                advances,
                sacrifice,
                ejection,
            } => {
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let sacrifice = if *sacrifice { "sacrifice " } else { "" };

                let ejection = if let Some(ej) = ejection {
                    ej.unparse()
                } else {
                    String::new()
                };

                let verb = if Breakpoints::Season5TenseChange.before(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    "grounded"
                } else {
                    "grounds"
                };

                format!("{batter} {verb} into a {sacrifice}double play{fielders}. {out_one} {out_two}{scores_and_advances}{ejection}")
            }
            Self::DoublePlayCaught {
                batter,
                fair_ball_type,
                fielders,
                out_two,
                scores,
                advances,
                ejection,
            } => {
                let fair_ball_type = fair_ball_type.verb_name();
                let fielders = unparse_fielders_for_play(fielders);
                let scores_and_advances = unparse_scores_and_advances(scores, advances);

                let ejection = if let Some(ej) = ejection {
                    ej.unparse()
                } else {
                    String::new()
                };

                format!("{batter} {fair_ball_type} into a double play{fielders}. {out_two}{scores_and_advances}{ejection}")
            }
            Self::ReachOnFieldingError {
                batter,
                fielder,
                error,
                scores,
                advances,
                ejection,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                let error = error.lowercase();
                let ejection = if let Some(e) = ejection {
                    e.unparse()
                } else {
                    String::new()
                };
                format!("{batter} reaches on a {error} error by {fielder}.{scores_and_advances}{ejection}")
            }
            Self::WeatherDelivery { delivery } => {
                delivery.unparse(context, event_index, "Delivery")
            }
            Self::FallingStar { player_name } => {
                format!("<strong>🌠 {player_name} is hit by a Falling Star!</strong>")
            }
            Self::FallingStarOutcome {
                deflection,
                player_name,
                outcome,
            } => {
                let deflection_msg = if let Some(deflected_off_player_name) = deflection {
                    format!("It deflected off {deflected_off_player_name} and struck {player_name}!</strong> <strong>")
                } else {
                    String::new()
                };

                let outcome_msg =
                    outcome.unparse(context, event_index, player_name.to_string().as_str());

                format!(" <strong>{deflection_msg}{outcome_msg}</strong>")
            }
            Self::WeatherShipment { deliveries } => deliveries
                .iter()
                .map(|d| d.unparse(context, event_index, "Shipment"))
                .collect::<Vec<String>>()
                .join(" "),
            Self::WeatherSpecialDelivery { delivery } => {
                delivery.unparse(context, event_index, "Special Delivery")
            }
            Self::Balk {
                pitcher,
                scores,
                advances,
            } => {
                let scores_and_advances = unparse_scores_and_advances(scores, advances);
                format!("Balk. {pitcher} dropped the ball.{scores_and_advances}")
            }
            Self::KnownBug { bug } => format!("{bug}"),
            Self::WeatherProsperity {
                home_income,
                away_income,
            } => {
                let earn = if Breakpoints::Season5TenseChange.before(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    "earned"
                } else {
                    "earn"
                };

                let home = (*home_income > 0)
                    .then_some(format!(
                        "{} {} are Prosperous! They {earn} {home_income} 🪙.",
                        context.home_emoji_team.emoji, context.home_emoji_team.name
                    ))
                    .unwrap_or_default();
                let away = (*away_income > 0)
                    .then_some(format!(
                        "{} {} are Prosperous! They {earn} {away_income} 🪙.",
                        context.away_emoji_team.emoji, context.away_emoji_team.name
                    ))
                    .unwrap_or_default();
                let gap = if *home_income > 0 && *away_income > 0 {
                    " "
                } else {
                    Default::default()
                };

                if Breakpoints::Season3PreSuperstarBreakUpdate.before(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    format!("{home}{gap}{away}")
                } else if home_income > away_income {
                    format!("{away}{gap}{home}")
                } else {
                    format!("{home}{gap}{away}")
                }
            }
            Self::PhotoContest {
                winning_team,
                winning_tokens,
                winning_score,
                winning_player,
                losing_team,
                losing_tokens,
                losing_score,
                losing_player,
            } => {
                let winning_emoji = &winning_team.emoji;
                let losing_emoji = &losing_team.emoji;

                let earn = if Breakpoints::Season5TenseChange.before(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    "earned"
                } else {
                    "earn"
                };

                format!("{winning_team} {earn} {winning_tokens} 🪙. {losing_team} {earn} {losing_tokens} 🪙.<br>Top scoring Photos:<br>{winning_emoji} {winning_player} - {winning_score} {losing_emoji} {losing_player} - {losing_score}")
            }
            Self::Party {
                pitcher_name,
                pitcher_amount_gained,
                pitcher_attribute,
                batter_name,
                batter_amount_gained,
                batter_attribute,
                durability_loss,
            } => {
                format!(
                    "<strong>🥳 {pitcher_name} and {batter_name} are Partying!</strong> \
                    {pitcher_name} gained +{pitcher_amount_gained} {pitcher_attribute}. \
                    {batter_name} gained +{batter_amount_gained} {batter_attribute}. \
                    {durability_loss}"
                )
            }
            Self::WeatherReflection { team } => {
                format!("🪞 The reflection shatters. {team} received a Fragment of Reflection.")
            }
            Self::WeatherWither {
                team_emoji,
                player,
                corrupted,
                contained,
            } => {
                let delim = match contained {
                    ContainResult::NoContain => ".",
                    ContainResult::SuccessfulContain { .. } => {
                        if Breakpoints::Season7SuccessfulContainPeriodFix.before(
                            context.season,
                            context.day,
                            event_index,
                        ) {
                            "."
                        } else {
                            ""
                        }
                    }
                    ContainResult::FailedContain { .. } => ".",
                };

                match corrupted {
                    WitherResult::Resisted => {
                        // This one actually changed from present to past tense. Probably an accident.
                        let resist = if Breakpoints::Season7WitherTenseChange.before(
                            context.season,
                            context.day,
                            event_index,
                        ) {
                            "resists"
                        } else {
                            "resisted"
                        };

                        format!("{team_emoji} {player} {resist} the effects of the 🥀 Wither{delim}{contained}")
                    }
                    WitherResult::ResistedEffloresced => {
                        format!("{team_emoji} {player} resisted the effects of the 🥀 Wither while Effloresced{delim}{contained}")
                    }
                    WitherResult::ResistedImmune => {
                        format!("{team_emoji} {player} resisted the effects of the 🥀 Wither with 🦠 Immunity{delim}{contained}")
                    }
                    WitherResult::Corrupted => {
                        format!("{team_emoji} {player} was Corrupted by the 🥀 Wither{delim}{contained}")
                    }
                }
            }
            Self::LinealBeltTransfer {
                claimed_by,
                claimed_from,
            } => {
                if context.before(event_index, Breakpoints::Season10) {
                    format!("➰ {claimed_by} claimed the Lineal Belt from {claimed_from}!")
                } else {
                    format!("{claimed_by} claimed the ➰ Lineal Belt from {claimed_from}!")
                }
            }
            Self::WeatherConsumption(weather_consumption) => weather_consumption.unparse(),
        }
    }
}

fn unparse_fielders<S: Display>(fielders: &[PlacedPlayer<S>]) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!(" to {}", fielders.first().unwrap()),
        _ => format!(
            ", {}",
            fielders
                .iter()
                .map(PlacedPlayer::to_string)
                .collect::<Vec<_>>()
                .join(" to ")
        ),
    }
}

fn unparse_fielders_for_play<S: Display>(fielders: &[PlacedPlayer<S>]) -> String {
    match fielders.len() {
        0 => panic!("0-fielders"),
        1 => format!(", {} unassisted", fielders.first().unwrap()),
        _ => format!(
            ", {}",
            fielders
                .iter()
                .map(PlacedPlayer::to_string)
                .collect::<Vec<_>>()
                .join(" to ")
        ),
    }
}
fn unparse_scores_and_advances<S: Display>(scores: &[S], advances: &[RunnerAdvance<S>]) -> String {
    once(String::new())
        .chain(
            scores
                .iter()
                .map(|runner| format!("<strong>{runner} scores!</strong>"))
                .chain(advances.iter().map(|advance| advance.to_string())),
        )
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
pub enum StartOfInningPitcher<S> {
    Same {
        emoji: S,
        name: S,
    },
    Different {
        leaving_emoji: Option<S>,
        leaving_pitcher: PlacedPlayer<S>,
        arriving_emoji: Option<S>,
        arriving_pitcher: PlacedPlayer<S>,
    },
}

/// Either an Out or an Error - e.g. for a Fielder's Choice.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
pub enum FieldingAttempt<S> {
    Out {
        out: RunnerOut<S>,
    },
    Error {
        fielder: S,
        error: FieldingErrorType,
    },
}
impl<S: Display> Display for FieldingAttempt<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Out { out } => {
                write!(f, "{} out at {}.", out.runner, out.base)
            }
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
    pub name: S,
}
impl<S: Display> Display for EmojiTeam<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
    }
}

impl<S: AsRef<str>> EmojiTeam<S> {
    pub fn as_ref(&self) -> EmojiTeam<&str> {
        EmojiTeam {
            emoji: self.emoji.as_ref(),
            name: self.name.as_ref(),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EmojiPlayer<S> {
    pub emoji: S,
    pub name: S,
}
impl<S: Display> Display for EmojiPlayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
    }
}

impl<S: AsRef<str>> EmojiPlayer<S> {
    pub fn as_ref(&self) -> EmojiPlayer<&str> {
        EmojiPlayer {
            emoji: self.emoji.as_ref(),
            name: self.name.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PlacedPlayer<S> {
    pub name: S,
    pub place: Place,
}
impl<S: Display> Display for PlacedPlayer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.place, self.name)
    }
}

impl<S: AsRef<str>> PlacedPlayer<S> {
    pub fn as_ref(&self) -> PlacedPlayer<&str> {
        PlacedPlayer {
            name: self.name.as_ref(),
            place: self.place,
        }
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
    pub base: Base,
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
    pub caught: bool,
}
impl<S: Display> Display for BaseSteal<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.caught {
            true => write!(
                f,
                "{} is caught stealing {}.",
                self.runner,
                self.base.to_base_str()
            ),
            false => match self.base {
                Base::Home => write!(
                    f,
                    "<strong>{} steals {}!</strong>",
                    self.runner,
                    self.base.to_base_str()
                ),
                _ => write!(f, "{} steals {}!", self.runner, self.base.to_base_str()),
            },
        }
    }
}
impl<S> TryFrom<BaseSteal<S>> for RunnerOut<S> {
    type Error = ();
    fn try_from(value: BaseSteal<S>) -> Result<Self, Self::Error> {
        if value.caught {
            Ok(RunnerOut {
                runner: value.runner,
                base: BaseNameVariant::basic_name(value.base),
            })
        } else {
            Err(())
        }
    }
}
impl<S> TryFrom<BaseSteal<S>> for RunnerAdvance<S> {
    type Error = ();
    fn try_from(value: BaseSteal<S>) -> Result<Self, Self::Error> {
        if !value.caught {
            Ok(RunnerAdvance {
                runner: value.runner,
                base: value.base,
            })
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
pub enum FallingStarOutcome<S> {
    Injury,
    Retired(Option<S>),
    InfusionI,
    InfusionII,
    InfusionIII,
    DeflectedHarmlessly,
}

impl<S: Display> FallingStarOutcome<S> {
    pub fn unparse<'a>(
        &self,
        context: impl Into<UnparsingContext<'a>>,
        event_index: Option<u16>,
        player_name: &str,
    ) -> String {
        let context = context.into();
        let was_is =
            if Breakpoints::Season5TenseChange.after(context.season, context.day, event_index) {
                "is"
            } else {
                "was"
            };

        match self {
            FallingStarOutcome::Injury => {
                format!("{player_name} {was_is} injured by the extreme force of the impact!")
            }
            FallingStarOutcome::Retired(None) => format!("😇 {player_name} retired from MMOLB!"),
            FallingStarOutcome::Retired(Some(replacement_player_name)) => {
                format!("😇 {player_name} retired from MMOLB! {replacement_player_name} {was_is} called up to take their place.")
            }
            FallingStarOutcome::InfusionI => {
                format!("{player_name} {was_is} infused with a glimmer of celestial energy!")
            }
            FallingStarOutcome::InfusionII => {
                let began_begins = if Breakpoints::Season5TenseChange.after(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    "begins"
                } else {
                    "began"
                };

                format!("{player_name} {began_begins} to glow brightly with celestial energy!")
            }
            FallingStarOutcome::InfusionIII => format!(
                "{player_name} {was_is} fully charged with an abundance of celestial energy!"
            ),
            FallingStarOutcome::DeflectedHarmlessly => {
                let deflected_deflects = if Breakpoints::Season5TenseChange.after(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    "deflects"
                } else {
                    "deflected"
                };

                format!("It {deflected_deflects} off {player_name} harmlessly.")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemAffixes<S> {
    None,
    PrefixSuffix(Vec<ItemPrefix>, Vec<ItemSuffix>),
    RareName(S),
}

impl<S: AsRef<str>> ItemAffixes<S> {
    pub fn to_ref(&self) -> ItemAffixes<&str> {
        match self {
            ItemAffixes::RareName(s) => ItemAffixes::RareName(s.as_ref()),
            ItemAffixes::PrefixSuffix(prefix, suffix) => {
                ItemAffixes::PrefixSuffix(prefix.clone(), suffix.clone())
            }
            ItemAffixes::None => ItemAffixes::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item<S> {
    pub item_emoji: S,
    pub item: ItemName,
    pub affixes: ItemAffixes<S>,
}

impl<S: AsRef<str>> Item<S> {
    pub fn to_ref(&self) -> Item<&str> {
        Item {
            item_emoji: self.item_emoji.as_ref(),
            item: self.item,
            affixes: self.affixes.to_ref(),
        }
    }
}

impl<S: Display> Display for Item<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Item {
            item_emoji,
            item,
            affixes,
        } = self;

        match affixes {
            ItemAffixes::None => write!(f, "{item_emoji} {item}"),
            ItemAffixes::PrefixSuffix(prefix, suffix) => {
                let prefix = prefix.iter().map(|p| format!("{p} ")).collect::<String>();
                let suffix = suffix.iter().map(|s| format!(" {s}")).collect::<String>();
                write!(f, "{item_emoji} {prefix}{item}{suffix}")
            }
            ItemAffixes::RareName(rare_name) => write!(f, "{item_emoji} {rare_name} {item}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Delivery<S> {
    Successful {
        team: EmojiTeam<S>,
        player: Option<S>,
        item: Item<S>,
        equipped: bool,
        discarded: Option<Item<S>>,
    },
    NoSpace {
        item: Item<S>,
    },
}

impl<S: Display> Delivery<S> {
    pub fn unparse<'a>(
        &self,
        context: impl Into<UnparsingContext<'a>>,
        event_index: Option<u16>,
        delivery_label: &str,
    ) -> String {
        let context = context.into();
        match self {
            Self::Successful {
                team,
                player,
                item,
                equipped,
                discarded,
            } => {
                let received_text = if *equipped {
                    " equips "
                } else {
                    received_text(context.season, context.day, event_index)
                };
                let discarded_text = discarded_text(context.season, context.day, event_index);

                let discarded = match discarded {
                    Some(discarded) => format!("{discarded_text}{discarded}."),
                    None => String::new(),
                };

                let player = player
                    .as_ref()
                    .map(|player| format!(" {player}"))
                    .unwrap_or_default();

                // There's this extra word "from" in the equipped message
                let from_text = if *equipped { "from " } else { "" };

                format!(
                    "{team}{player}{received_text}{item} {from_text}{delivery_label}.{discarded}"
                )
            }
            Self::NoSpace { item } => {
                let discard_text = if Breakpoints::Season8ItemDiscardedMessageChange.after(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    " is discarded as no player can use it."
                } else if Breakpoints::Season5TenseChange.after(
                    context.season,
                    context.day,
                    event_index,
                ) {
                    " is discarded as no player has space."
                } else {
                    " was discarded as no player had space."
                };

                format!("{item}{discard_text}")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WitherResult {
    Resisted,
    ResistedEffloresced,
    ResistedImmune,
    Corrupted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ContainResult<S> {
    NoContain,
    SuccessfulContain {
        contained_player_name: S,
        replacement_player_name: S,
    },
    FailedContain {
        target_player_name: S,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PartyDurabilityLoss<S> {
    Both(u8),
    OneProtected {
        protected_player_name: S,
        unprotected_player_name: S,
        durability_loss: u8,
    },
}

impl<S: Display> Display for PartyDurabilityLoss<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PartyDurabilityLoss::Both(durability_loss) => {
                write!(f, "Both players lose {durability_loss} Durability.")
            }
            PartyDurabilityLoss::OneProtected {
                protected_player_name,
                unprotected_player_name,
                durability_loss,
            } => {
                write!(
                    f,
                    "{unprotected_player_name} loses {durability_loss} Durability, but \
                    {protected_player_name}'s Prolific Greater Boon protects them from harm."
                )
            }
        }
    }
}

impl<S: Display> Display for ContainResult<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainResult::NoContain => Ok(()),
            ContainResult::SuccessfulContain {
                replacement_player_name,
                contained_player_name,
            } => {
                write!(f, ", and Contained {contained_player_name}. They were replaced by {replacement_player_name}.")
            }
            ContainResult::FailedContain { target_player_name } => {
                write!(
                    f,
                    ", and tried to Contain {target_player_name}, but they wouldn't budge."
                )
            }
        }
    }
}

fn old_space(context: UnparsingContext, event_index: Option<u16>) -> &'static str {
    if Breakpoints::S2D169.before(context.season, context.day, event_index) {
        " "
    } else {
        ""
    }
}

/// See individual variant documentation for an example of each bug, and the known properties of their events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
#[strum_discriminants(derive(Display))]
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
    FirstBasemanChoosesAGhost { batter: S, first_baseman: S },
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
    NoOneProspers,
}

impl<S: Display> Display for KnownBug<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnownBug::FirstBasemanChoosesAGhost {
                batter,
                first_baseman,
            } => {
                write!(
                    f,
                    "{batter} reaches on a fielder's choice out, 1B {first_baseman}"
                )
            }
            KnownBug::NoOneProspers => {
                write!(f, "")
            }
        }
    }
}

fn _check(_: &str) -> Infallible {
    unreachable!("This is dead code that exists for a strum parse_err_fn")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, IntoStaticStr, Display)]
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
    Unknown(String),
}

impl Cheer {
    pub fn new(value: &str) -> Self {
        let r = Cheer::from_str(value).expect("This error type is infallible");

        if matches!(r, Cheer::Unknown(_)) {
            tracing::warn!("Failed to match cheer '{value}'");
        }

        r
    }

    pub fn unparse<'a>(
        &self,
        context: impl Into<UnparsingContext<'a>>,
        event_index: Option<u16>,
    ) -> String {
        let context = context.into();
        if Breakpoints::CheersGetEmoji.before(context.season, context.day, event_index) {
            format!(" {self}!")
        } else {
            format!(" 📣 {self}!")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, IntoStaticStr, Display)]
#[strum(
    parse_err_fn = check,
    parse_err_ty = Infallible
)]
pub enum EjectionReason {
    // Sportsmanship violations
    #[strum(to_string = "eating a hotdog")]
    EatingAHotdog,
    #[strum(to_string = "spitting")]
    Spitting,
    #[strum(to_string = "looking at them the wrong way")]
    LookingAtThemTheWrongWay,
    #[strum(to_string = "whispering something to another player")]
    WhisperingSomethingToAnotherPlayer,
    #[strum(to_string = "dancing")]
    Dancing,
    #[strum(to_string = "not looking excited enough")]
    NotLookingExcitedEnough,
    #[strum(to_string = "picking their nose")]
    PickingTheirNose,
    #[strum(to_string = "drinking beer")]
    DrinkingBeer,
    #[strum(to_string = "taking a phone call")]
    TakingAPhoneCall,
    #[strum(to_string = "using a foreign substance")]
    UsingAForeignSubstance,
    #[strum(to_string = "eating nachos")]
    EatingNachos,
    #[strum(to_string = "chewing gum too loud")]
    ChewingGumTooLoud,
    #[strum(to_string = "texting during play")]
    TextingDuringPlay,

    // Uniform violations
    #[strum(to_string = "hat worn at improper rotational value")]
    HatWornAtImproperRotationalValue,
    #[strum(to_string = "mismatched socks")]
    MismatchedSocks,
    #[strum(to_string = "wrinkled shirt")]
    WrinkledShirt,
    #[strum(to_string = "shoe untied")]
    ShoeUntied,

    // Communication violations
    #[strum(to_string = "making weird hand signals")]
    MakingWeirdHandSignals,
    #[strum(to_string = "laughing")]
    Laughing,
    #[strum(to_string = "something they said earlier in the locker room")]
    SomethingTheySaidEarlierInTheLockerRoom,
    #[strum(to_string = "telling a bad joke")]
    TellingABadJoke,
    #[strum(to_string = "winking at someone in the crowd")]
    WinkingAtSomeoneInTheCrowd,
    #[strum(to_string = "saying a bad word")]
    SayingABadWord,
    #[strum(to_string = "humming")]
    Humming,

    #[strum(default)]
    Unknown(String),
}

impl EjectionReason {
    pub fn new(value: &str) -> Self {
        let r = EjectionReason::from_str(value).expect("This error type is infallible");

        if matches!(r, EjectionReason::Unknown(_)) {
            tracing::warn!("Failed to match ejection reason '{value}'");
        }

        r
    }

    pub fn unparse(&self) -> String {
        format!("{self}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, IntoStaticStr, Display)]
#[strum(
    parse_err_fn = check,
    parse_err_ty = Infallible
)]
pub enum ViolationType {
    Sportsmanship,
    Uniform,
    Communication,

    #[strum(default)]
    Unknown(String),
}

impl ViolationType {
    pub fn new(value: &str) -> Self {
        let r = ViolationType::from_str(value).expect("This error type is infallible");

        if matches!(r, ViolationType::Unknown(_)) {
            tracing::warn!("Failed to match violation type '{value}'");
        }

        r
    }

    pub fn unparse(&self) -> String {
        format!("{self}")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct SnappedPhotos<S> {
    pub first_team_emoji: S,
    pub first_player: PlacedPlayer<S>,
    pub second_team_emoji: S,
    pub second_player: PlacedPlayer<S>,
}

impl<S: Display> SnappedPhotos<S> {
    pub fn unparse(&self) -> String {
        format!(
            " The Geomagnetic Storms Intensify! {} {} and {} {} snapped photos of the aurora.",
            self.first_team_emoji, self.first_player, self.second_team_emoji, self.second_player,
        )
    }
}

impl<S: AsRef<str>> SnappedPhotos<S> {
    pub fn as_ref(&self) -> SnappedPhotos<&str> {
        SnappedPhotos {
            first_team_emoji: self.first_team_emoji.as_ref(),
            first_player: self.first_player.as_ref(),
            second_team_emoji: self.second_team_emoji.as_ref(),
            second_player: self.second_player.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
pub enum EjectionReplacement<S> {
    BenchPlayer { player_name: S },
    RosterPlayer { player: PlacedPlayer<S> },
}

impl<S: AsRef<str>> EjectionReplacement<S> {
    pub fn as_ref(&self) -> EjectionReplacement<&str> {
        match self {
            EjectionReplacement::BenchPlayer { player_name } => EjectionReplacement::BenchPlayer {
                player_name: player_name.as_ref(),
            },
            EjectionReplacement::RosterPlayer { player } => EjectionReplacement::RosterPlayer {
                player: player.as_ref(),
            },
        }
    }
}

impl<S> EjectionReplacement<S> {
    pub fn player_name(&self) -> &S {
        match self {
            EjectionReplacement::BenchPlayer { player_name } => player_name,
            EjectionReplacement::RosterPlayer { player } => &player.name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Ejection<S> {
    Ejection {
        team: EmojiTeam<S>,
        ejected_player: PlacedPlayer<S>,
        violation_type: ViolationType,
        reason: EjectionReason,
        replacement: EjectionReplacement<S>,
    },
    FailedEjection {
        player_names: [S; 2],
    },
}

impl<S: Display> Ejection<S> {
    pub fn unparse(&self) -> String {
        match self {
            Ejection::Ejection { team, ejected_player, violation_type, reason, replacement } => {
                match replacement {
                    EjectionReplacement::BenchPlayer { player_name } => format!(
                        " 🤖 ROBO-UMP ejected {} {} for a {} Violation ({}). Bench Player {} takes their place.",
                        team,
                        ejected_player,
                        violation_type,
                        reason,
                        player_name,
                    ),
                    EjectionReplacement::RosterPlayer { player } => format!(
                        " 🤖 ROBO-UMP ejected {} {} for a {} Violation ({}). {} {} takes the mound.",
                        team,
                        ejected_player,
                        violation_type,
                        reason,
                        team.emoji,
                        player,
                    ),
                }
            }
            Ejection::FailedEjection { player_names: [n1, n2] } => format!(
                " 🤖 ROBO-UMP attempted an ejection, but {}, {} would not budge.", n1, n2,
            )
        }
    }
}

impl<S: AsRef<str>> Ejection<S> {
    // This does clone violation_type and reason, which may (rarely) hold strings.
    // Perhaps it should be named something other than as_ref?
    pub fn as_ref(&self) -> Ejection<&str> {
        match self {
            Ejection::Ejection {
                team,
                ejected_player,
                violation_type,
                reason,
                replacement,
            } => Ejection::Ejection {
                team: team.as_ref(),
                ejected_player: ejected_player.as_ref(),
                violation_type: violation_type.clone(),
                reason: reason.clone(),
                replacement: replacement.as_ref(),
            },
            Ejection::FailedEjection {
                player_names: [n1, n2],
            } => Ejection::FailedEjection {
                player_names: [n1.as_ref(), n2.as_ref()],
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemEquip<S> {
    None,
    Discarded,
    Equipped {
        player_name: S,
        discarded_item: Option<Item<S>>,
    },
}

impl<S> ItemEquip<S> {
    pub fn is_none(&self) -> bool {
        matches!(self, ItemEquip::None)
    }
}

impl<S: AsRef<str>> ItemEquip<S> {
    pub fn to_ref(&self) -> ItemEquip<&str> {
        match self {
            ItemEquip::None => ItemEquip::None,
            ItemEquip::Discarded => ItemEquip::Discarded,
            ItemEquip::Equipped {
                player_name,
                discarded_item,
            } => ItemEquip::Equipped {
                player_name: player_name.as_ref(),
                discarded_item: discarded_item.as_ref().map(|i| i.to_ref()),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ItemPrize<S> {
    pub item: Item<S>,
    pub equip: ItemEquip<S>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Prize<S> {
    Tokens(u16),
    Items(Vec<ItemPrize<S>>),
}

impl<S: Display> Prize<S> {
    pub fn unparse(&self) -> String {
        match self {
            Prize::Tokens(tokens) => format!("{tokens} 🪙"),
            Prize::Items(items) => {
                let has_equip = items.iter().any(|i| !i.equip.is_none());

                items
                    .iter()
                    .map(|prize| match &prize.equip {
                        ItemEquip::None => prize.item.to_string(),
                        ItemEquip::Discarded => {
                            format!("{} is discarded; nobody can use it", prize.item)
                        }
                        ItemEquip::Equipped {
                            player_name,
                            discarded_item,
                        } => {
                            let discard = match discarded_item {
                                None => String::new(),
                                Some(discarded_item) => {
                                    format!(". They discard their {discarded_item}")
                                }
                            };
                            format!(
                                "{player_name} equips {} from the Door Prize{discard}",
                                prize.item
                            )
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(if has_equip { ". " } else { ", " })
            }
        }
    }
}

impl<S: AsRef<str>> Prize<S> {
    pub fn to_ref(&self) -> Prize<&str> {
        match self {
            Prize::Items(items) => Prize::Items(
                items
                    .iter()
                    .map(|prize| ItemPrize {
                        item: prize.item.to_ref(),
                        equip: prize.equip.to_ref(),
                    })
                    .collect(),
            ),
            Prize::Tokens(t) => Prize::Tokens(*t),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DoorPrize<S> {
    pub player: S,
    /// None when they don't win.
    pub prize: Option<Prize<S>>,
}

impl<S: Display> DoorPrize<S> {
    pub fn unparse(&self) -> String {
        match &self.prize {
            Some(prize) => {
                let punct = match prize {
                    Prize::Items(items) if items.iter().any(|item| !item.equip.is_none()) => "!",
                    _ => ":",
                };
                format!(
                    "🥳 {} won a Door Prize{punct} {}.",
                    self.player,
                    prize.unparse()
                )
            }
            None => format!("🥳 {} didn't win a Door Prize.", self.player),
        }
    }
}

impl<S: AsRef<str>> DoorPrize<S> {
    pub fn to_ref(&self) -> DoorPrize<&str> {
        DoorPrize {
            player: self.player.as_ref(),
            prize: self.prize.as_ref().map(Prize::to_ref),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WitherStruggle<S> {
    pub team_emoji: S,
    pub target: PlacedPlayer<S>,
    // Source was not named in s6
    pub source_name: Option<S>,
}

impl<S: Display> Display for WitherStruggle<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.source_name {
            None => write!(
                f,
                "{} {} struggles against the 🥀 Wither.",
                self.team_emoji, self.target
            ),
            Some(source_name) => write!(
                f,
                "{source_name} is trying to spread the 🥀 Wither to {} {}!",
                self.team_emoji, self.target
            ),
        }
    }
}

impl<S: AsRef<str>> WitherStruggle<S> {
    pub fn to_ref(&self) -> WitherStruggle<&str> {
        WitherStruggle {
            team_emoji: self.team_emoji.as_ref(),
            source_name: self.source_name.as_ref().map(AsRef::as_ref),
            target: self.target.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EfflorescenceOutcome {
    Grow([GrowAttributeChange; 2]),
    Effloresce,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Efflorescence<S> {
    pub player: S,
    pub outcome: EfflorescenceOutcome,
}

impl<S: Display> Efflorescence<S> {
    pub fn unparse(&self) -> String {
        match &self.outcome {
            EfflorescenceOutcome::Grow(changes) => {
                let mut s = String::new();
                write!(s, "{} grew: ", self.player).unwrap();
                for (i, change) in changes.iter().enumerate() {
                    let prefix = if i == 0 { "" } else { ", " };
                    write!(s, "{prefix}{:+} {}", change.amount, change.attribute).unwrap();
                }
                write!(s, ".").unwrap();
                s
            }
            EfflorescenceOutcome::Effloresce => {
                format!(
                    "{} Effloresced, shedding their Corrupted Modification.",
                    self.player
                )
            }
        }
    }
}

impl<S: AsRef<str>> Efflorescence<S> {
    pub fn to_ref(&self) -> Efflorescence<&str> {
        Efflorescence {
            player: self.player.as_ref(),
            outcome: self.outcome,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct EmojiFood<S> {
    pub food_emoji: S,
    pub food: FoodName,
}

impl<S: AsRef<str>> EmojiFood<S> {
    pub fn as_ref(&self) -> EmojiFood<&str> {
        EmojiFood {
            food_emoji: self.food_emoji.as_ref(),
            food: self.food,
        }
    }
}

impl<S: Display> EmojiFood<S> {
    pub fn unparse(&self) -> String {
        format!("{} {}", self.food_emoji, self.food)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeatherConsumptionEvents<S> {
    StartContest {
        batting_team_player: EmojiPlayer<S>,
        pitching_team_player: EmojiPlayer<S>,
        emoji_food: EmojiFood<S>,
    },
    Consumes {
        batting_team_player: EmojiPlayer<S>,
        batting_team_progress: u32,
        pitching_team_player: EmojiPlayer<S>,
        pitching_team_progress: u32,
        food_emoji: Option<S>,
        food: FoodName,
        batting_team_score: u32,
        pitching_team_score: u32,
    },
    EndContest {
        winning_score: u32,
        food_emoji: Option<S>,
        food: FoodName,
        winning_player: EmojiPlayer<S>,
        winning_team: EmojiTeam<S>,
        winning_tokens: u32,
        winning_prize: Item<S>,
        losing_team: EmojiTeam<S>,
        losing_tokens: u32,
    },
    EndContestTie {
        final_score: u32,
        food_emoji: Option<S>,
        food: FoodName,
        batting_team: EmojiTeam<S>,
        batting_team_tokens: u32,
        batting_team_prize: Item<S>,
        pitching_team: EmojiTeam<S>,
        pitching_team_tokens: u32,
        pitching_team_prize: Item<S>,
    },
}

impl<S: Display> WeatherConsumptionEvents<S> {
    pub fn unparse(&self) -> String {
        match &self {
            WeatherConsumptionEvents::StartContest {
                batting_team_player,
                pitching_team_player,
                emoji_food: food,
            } => {
                format!("🍽️ Consumption Contest! {batting_team_player} vs. {pitching_team_player}.<br>Who can Consume the most {food}? Go!", food = food.unparse())
            }
            WeatherConsumptionEvents::Consumes {
                batting_team_player,
                batting_team_progress,
                pitching_team_player,
                pitching_team_progress,
                food_emoji,
                food,
                batting_team_score,
                pitching_team_score,
            } => {
                let food = if let Some(emoji) = food_emoji {
                    format!("{emoji} {food}")
                } else {
                    food.to_string()
                };
                format!("{batting_team_player} Consumes {batting_team_progress} {food}!<br>{pitching_team_player} Consumes {pitching_team_progress} {food}!<br>Score: {batting_team_score} - {pitching_team_score}")
            }
            WeatherConsumptionEvents::EndContest {
                winning_score,
                food_emoji,
                food,
                winning_player,
                winning_team,
                winning_prize,
                winning_tokens,
                losing_team,
                losing_tokens,
            } => {
                let food = if let Some(emoji) = food_emoji {
                    format!("{emoji} {food}")
                } else {
                    food.to_string()
                };
                format!("The winner with {winning_score} {food} Consumed is {winning_player}!<br>{winning_team} receives 🪙 {winning_tokens}, and win a {winning_prize}.<br>{losing_team} receives 🪙 {losing_tokens}.")
            }
            WeatherConsumptionEvents::EndContestTie {
                final_score,
                food_emoji,
                food,
                batting_team,
                batting_team_tokens,
                batting_team_prize,
                pitching_team,
                pitching_team_tokens,
                pitching_team_prize,
            } => {
                let food = if let Some(emoji) = food_emoji {
                    format!("{emoji} {food}")
                } else {
                    food.to_string()
                };
                format!("The Consumption Contest ends in a tie at {final_score} {food} Consumed!<br>{batting_team} receives 🪙 {batting_team_tokens} and a {batting_team_prize}.<br>{pitching_team} receives 🪙 {pitching_team_tokens} and a {pitching_team_prize}.")
            }
        }
    }
}

impl<S: AsRef<str>> WeatherConsumptionEvents<S> {
    pub fn to_ref(&self) -> WeatherConsumptionEvents<&str> {
        match &self {
            WeatherConsumptionEvents::StartContest {
                batting_team_player,
                pitching_team_player,
                emoji_food: food,
            } => WeatherConsumptionEvents::StartContest {
                batting_team_player: batting_team_player.as_ref(),
                pitching_team_player: pitching_team_player.as_ref(),
                emoji_food: food.as_ref(),
            },
            WeatherConsumptionEvents::Consumes {
                batting_team_player,
                batting_team_progress,
                pitching_team_player,
                pitching_team_progress,
                food_emoji,
                food,
                batting_team_score,
                pitching_team_score,
            } => WeatherConsumptionEvents::Consumes {
                batting_team_player: batting_team_player.as_ref(),
                batting_team_progress: *batting_team_progress,
                pitching_team_player: pitching_team_player.as_ref(),
                pitching_team_progress: *pitching_team_progress,
                food_emoji: food_emoji.as_ref().map(|s| s.as_ref()),
                food: *food,
                batting_team_score: *batting_team_score,
                pitching_team_score: *pitching_team_score,
            },
            WeatherConsumptionEvents::EndContest {
                winning_score,
                food_emoji,
                food,
                winning_player,
                winning_team,
                winning_tokens,
                winning_prize,
                losing_team,
                losing_tokens,
            } => WeatherConsumptionEvents::EndContest {
                winning_score: *winning_score,
                food_emoji: food_emoji.as_ref().map(|s| s.as_ref()),
                food: *food,
                winning_player: winning_player.as_ref(),
                winning_team: winning_team.as_ref(),
                winning_tokens: *winning_tokens,
                winning_prize: winning_prize.to_ref(),
                losing_team: losing_team.as_ref(),
                losing_tokens: *losing_tokens,
            },
            WeatherConsumptionEvents::EndContestTie {
                final_score,
                food_emoji,
                food,
                batting_team,
                batting_team_tokens,
                batting_team_prize,
                pitching_team,
                pitching_team_tokens,
                pitching_team_prize,
            } => WeatherConsumptionEvents::EndContestTie {
                final_score: *final_score,
                food_emoji: food_emoji.as_ref().map(|s| s.as_ref()),
                food: *food,
                batting_team: batting_team.as_ref(),
                batting_team_tokens: *batting_team_tokens,
                batting_team_prize: batting_team_prize.to_ref(),
                pitching_team: pitching_team.as_ref(),
                pitching_team_tokens: *pitching_team_tokens,
                pitching_team_prize: pitching_team_prize.to_ref(),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;

    use serde::Deserialize;

    use crate::{process_game, utils::no_tracing_errs, Game};

    //https://freecashe.ws/api/chron/v0/entities?kind=game&id=6851bb34f419fdc04f9d0ed5,685b744530d8d1ac659c30de,68611cb61e65f5fb52cb618f,68611cb61e65f5fb52cb61d6,68799d0621c82ae41451ca4f,68782f7d206bc4d2a2003b05,6879f14e21c82ae41451e785,6893c2899361d52a6890a9f0
    #[test]
    fn first_baseman_chooses_a_ghost() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        pub struct FreeCashewResponse {
            pub items: Vec<GameEntity>,
        }

        #[derive(Deserialize)]
        pub struct GameEntity {
            pub data: Game,
            pub entity_id: String,
        }

        let no_tracing_errors = no_tracing_errs();

        let f = File::open("test_data/fbcag.json")?;
        let response: FreeCashewResponse = serde_json::from_reader(f)?;

        for entity in response.items {
            process_game(&entity.data, &entity.entity_id);
        }

        drop(no_tracing_errors);
        Ok(())
    }
}
