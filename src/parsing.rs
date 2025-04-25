use nom::{Finish, Parser};
use nom_language::error::{VerboseError, VerboseErrorKind};
use serde::{Deserialize, Serialize};

use crate::{enums::{Base, EventType, FielderError, FoulType, HitDestination, HitType, Position, Side, StrikeType}, game::{Event, Pitch}, nom_parsing::{parse_field_event, parse_pitch_event, ParsingContext, EXTRACT_FIELDER_NAME}};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParsedEvent {
    PitchingMatchup {
        home_pitcher: String,
        away_pitcher: String,
    },
    MoundVisit {
        team: String,
    },
    MoundVisitRefused,
    PitcherSwap {
        leaving_position: Position,
        leaving_pitcher: String,
        arriving_position: Position,
        arriving_pitcher: String,
    },
    GameOver,
    RunnerAdvance { runner: String, base: Base, is_steal: bool },
    Error { fielder: String, error: FielderError },
    Lineup(Side, Vec<(Position, String)>),
    Recordkeeping {
        home_score: u8,
        away_score: u8,
    },
    LiveNow,
    InningStart {
        number: u8,
        side: Side,
        batting_team: String,
        pitcher: Option<String>,
    },
    Pitch(Pitch),
    Hit {
        hit_type: HitType,
        destination: HitDestination
    },
    /// Includes home runs as base = 4.
    BatterToBase {base: Base, fielder: Option<(Position, String)>},
    Out {player: String, fielders: Vec<(Position, String)>, perfect_catch: bool},
    Scores {player: String},
    Walk,
    Ball,
    Foul {foul_type: FoulType},
    Strike {strike_type: StrikeType},
    HitByPitch,
    InningEnd {
        number: u8,
        side: Side
    },
    PlayBall,
    NowBatting {
        batter: String,
        first_pa: bool
    },
    ParseError {
        event_type: EventType,
        message: String,
        reason: String
    }
}

pub fn process_events(events_log: &Vec<Event>) -> Vec<ParsedEvent> {
    let mut events = events_log.into_iter();
    let mut result = Vec::new();
    let mut parsing_context = ParsingContext::new();

    while let Some(event) = events.next() {
        let mut parse_event = || {
            match event.event {
                EventType::PitchingMatchup => {
                    let mut iter = event.message.split(" vs. ").map(|side| {
                        let mut words = side.split(" ").collect::<Vec<_>>();
                        words.reverse();
                        let mut name = words.into_iter().take(2).collect::<Vec<_>>();
                        name.reverse();
                        name.join(" ")
                    });
                    let home_pitcher = iter.next()?;
                    let away_pitcher = iter.next()?;
                    parsing_context.player_names.insert(home_pitcher.clone());
                    parsing_context.player_names.insert(away_pitcher.clone());
                    result.push(ParsedEvent::PitchingMatchup { home_pitcher, away_pitcher })
                }
                EventType::AwayLineup => {
                    let players = parse_lineup(&event.message)?;
                    parsing_context.player_names.extend(players.iter().map(|(_, name)| name.clone()));
                    result.push(ParsedEvent::Lineup(Side::Away, players))
                },
                EventType::HomeLineup => {
                    let players = parse_lineup(&event.message)?;
                    parsing_context.player_names.extend(players.iter().map(|(_, name)| name.clone()));
                    result.push(ParsedEvent::Lineup(Side::Home, players))
                },
                EventType::Field => {
                    match parse_field_event(&parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => result.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                    }
                },
                EventType::Pitch => {
                    match parse_pitch_event(&parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => result.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                    }
                },
                EventType::GameOver => result.push(ParsedEvent::GameOver),
                EventType::InningEnd => {
                    let mut iter = event.message.split(" ").skip(3);
                    let side = match iter.next()? {
                        "top" => Side::Home,
                        "bottom" => Side::Away,
                        _ => return None 
                    };
                    let mut iter = iter.skip(2);
                    let number = iter.next()?
                        .chars().rev().skip(3).collect::<Vec<char>>().into_iter().rev().collect::<String>()
                        .parse().ok()?;
                    result.push(ParsedEvent::InningEnd { number, side });
                }
                EventType::InningStart => {
                    let mut iter = event.message.split(" ")
                        .skip(3); // Start of the
                    let side = match iter.next()? {
                        "top" => Side::Home,
                        "bottom" => Side::Away,
                        _ => return None, 
                    };
                    let mut iter = iter
                        .skip(2); // of the
                    let number = iter.next()?
                        .chars().rev().skip(3).collect::<Vec<char>>().into_iter().rev().collect::<String>() // Remove "st.", "nd." or "th." from the end
                        .parse().ok()?;
                    let mut batting_team = iter.by_ref().take_while(|s| *s != "batting.");
                    let _batting_emoji = batting_team.next()?.to_string();
                    let batting_team = batting_team.collect::<Vec<_>>().join(" ");

                    if event.message.contains("takes the mound.") {
                        let remaining_message = iter.collect::<Vec<_>>().join(" ");
                        let (leaving_position, leaving_pitcher, arriving_position, arriving_pitcher) = pitcher_swap(&remaining_message)?;
                        
                        parsing_context.player_names.insert(arriving_pitcher.clone());

                        result.push(ParsedEvent::InningStart { number, side, batting_team, pitcher: None });
                        result.push(ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });
                    } else {
                        let _pitching_emoji = iter.next()?.to_string();
                        let pitcher = iter.take_while(|s| *s != "pitching.")
                            .collect::<Vec<_>>().join(" ");
                        parsing_context.player_names.insert(pitcher.clone());
                        result.push(ParsedEvent::InningStart { number, side, batting_team, pitcher: Some(pitcher) });
                    }
                }
                EventType::LiveNow => result.push(ParsedEvent::LiveNow),
                EventType::MoundVisit => {
                    if event.message.contains("manager") {
                        let iter = event.message.split(" ")
                        .skip(2); // The
                        let team = iter.take_while(|s| *s != "manager").collect::<Vec<_>>().join(" ");
                        result.push(ParsedEvent::MoundVisit { team });
                    } else if event.message.contains("remains in the game") {
                        result.push(ParsedEvent::MoundVisitRefused);
                    } else {
                        let (leaving_position, leaving_pitcher, arriving_position, arriving_pitcher) = pitcher_swap(&event.message)?;   
                        parsing_context.player_names.insert(arriving_pitcher.clone());
                        result.push(ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });
                    }
                }
                EventType::NowBatting => {
                    let mut message = event.message.strip_prefix("Now batting: ")?.split(" (");
                    let batter = message.next()?.to_string();
                    let first_pa = Some("1st PA of game)") == message.next();
                    parsing_context.player_names.insert(batter.clone());
                    result.push(ParsedEvent::NowBatting { batter, first_pa });
                }
                EventType::PlayBall => result.push(ParsedEvent::PlayBall),
                EventType::Recordkeeping => {
                    let score = event.message.split(" ").last()?;
                    let mut iter = score.split("-");
                    let home_score = iter.next()?.parse().ok()?;
                    let away_score = iter.next()?.parse().ok()?;
                    result.push(ParsedEvent::Recordkeeping { home_score, away_score });
                }
            };
            Some(())
        };
        if None == parse_event() {
            panic!("Couldn't parse {event:?}")
        };
    }
    result
}

fn pitcher_swap(message: &String) -> Option<(Position, String, Position, String)> {
    let mut iter = message.split(" ");
    let leaving_position = iter.next()?.try_into().ok()?;
    let leaving_pitcher = iter.by_ref().take_while(|s| *s != "is").collect::<Vec<_>>().join(" ");
                    
    let mut iter = iter.skip_while(|s| *s != "game.").skip(1);
    // is leaving the game
    let arriving_position = iter.next()?.try_into().ok()?;
    let arriving_pitcher = iter.take_while(|s| *s != "takes")
        .collect::<Vec<_>>().join(" ");
    Some((leaving_position, leaving_pitcher, arriving_position, arriving_pitcher))
}

fn parse_lineup(message: &str) -> Option<Vec<(Position, String)>> {
    message.strip_suffix("<br>")?.split("<br>").map(|player| {
        let mut iter = player.split(" ");
        let _number = iter.next();
        extract_position_and_name(&iter.collect::<Vec<_>>().join(" "))
    }).collect()
}

fn extract_position_and_name(position_and_name: &str) -> Option<(Position, String)> {
    let mut iter = position_and_name.split(" ");
    let position = iter.next()?.try_into().ok()?;
    let name = iter.collect::<Vec<_>>().join(" ");
    Some((position, name))
}

#[cfg(debug_assertions)]
fn handle_error(event:&Event, err: VerboseError<&str>, result: &mut Vec<ParsedEvent>) {
    if err.errors.iter().any(|err| matches!(err, (_, VerboseErrorKind::Context(EXTRACT_FIELDER_NAME)))) {
        println!("{err}");
        result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone(), reason:EXTRACT_FIELDER_NAME.to_string() });
    } else {
        panic!("{err}")
    }
}

#[cfg(not(debug_assertions))]
fn handle_error(event:&Event, err: VerboseError<&str>, result: &mut Vec<ParsedEvent>) {
    println!("{err}");
    result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone(), reason: err.to_string() });
}