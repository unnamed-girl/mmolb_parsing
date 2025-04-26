use nom::{Finish, Parser};
use nom_language::error::{VerboseError, VerboseErrorKind};
use serde::{Deserialize, Serialize};

use crate::{enums::{Base, EventType, FielderError, FoulType, HitDestination, HitType, Position, Side, StrikeType}, game::{Event, Pitch}, nom_parsing::{parse_field_event, parse_inning_start_event, parse_lineup_event, parse_mound_visit, parse_pitch_event, parse_pitching_matchup_event, ParsingContext, EXTRACT_PLAYER_NAME}, Game};

/// A parsed event. the 'output lifetime is linked to the ParsingContext<'output> used to create this event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParsedEvent<'output> {
    PitchingMatchup {
        home_pitcher: &'output str,
        away_pitcher: &'output str,
    },
    MoundVisit {
        team: &'output str,
    },
    MoundVisitRefused,
    PitcherSwap {
        leaving_position: Position,
        leaving_pitcher: &'output str,
        arriving_position: Position,
        arriving_pitcher: &'output str,
    },
    GameOver,
    RunnerAdvance { runner: &'output str, base: Base, is_steal: bool },
    Error { fielder: &'output str, error: FielderError },
    Lineup(Side, Vec<(Position, &'output str)>),
    Recordkeeping {
        home_score: u8,
        away_score: u8,
    },
    LiveNow,
    InningStart {
        number: u8,
        side: Side,
        batting_team: &'output str,
        pitcher: Option<&'output str>,
    },
    Pitch(Pitch),
    Hit {
        hit_type: HitType,
        destination: HitDestination
    },
    /// Includes home runs as base = 4.
    BatterToBase {base: Base, fielder: Option<(Position, &'output str)>},
    Out {player: &'output str, fielders: Vec<(Position, &'output str)>, perfect_catch: bool},
    Scores {player: &'output str},
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
        batter: &'output str,
        first_pa: bool
    },
    ParseError {
        event_type: EventType,
        message: String,
        reason: String
    }
}

/// Processes a game into a list of ParsedEvents.
/// Note that the game must live longer than the events, as zero copy parsing is used. 
pub fn process_events<'output>(game: &'output Game) -> Vec<ParsedEvent<'output>> {
    let mut result = Vec::new();
    let mut parsing_context = ParsingContext::new(&game);

    for event in &game.event_log {
        let mut parse_event = || {
            let mut new_events = Vec::new();
            match event.event {
                EventType::PitchingMatchup => match parse_pitching_matchup_event(&parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => new_events.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::AwayLineup => match parse_lineup_event(Side::Away, &parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => new_events.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::HomeLineup => match parse_lineup_event(Side::Home, &parsing_context).parse(&event.message).finish() {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
            },
                EventType::Field => match parse_field_event(&parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => new_events.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::Pitch => match parse_pitch_event(&parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => new_events.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::GameOver => new_events.push(ParsedEvent::GameOver),
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
                    new_events.push(ParsedEvent::InningEnd { number, side });
                }
                EventType::InningStart => match parse_inning_start_event(&parsing_context).parse(&event.message).finish() {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::LiveNow => result.push(ParsedEvent::LiveNow),
                EventType::MoundVisit => match parse_mound_visit(&parsing_context).parse(&event.message).finish() {
                    Ok((_, events)) => new_events.push(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::NowBatting => {
                    let mut message = event.message.strip_prefix("Now batting: ")?.split(" (");
                    let batter = message.next()?;
                    let first_pa = Some("1st PA of game)") == message.next();
                    parsing_context.player_names.insert(batter);
                    new_events.push(ParsedEvent::NowBatting { batter, first_pa });
                }
                EventType::PlayBall => result.push(ParsedEvent::PlayBall),
                EventType::Recordkeeping => {
                    let score = event.message.split(" ").last()?;
                    let mut iter = score.split("-");
                    let home_score = iter.next()?.parse().ok()?;
                    let away_score = iter.next()?.parse().ok()?;
                    new_events.push(ParsedEvent::Recordkeeping { home_score, away_score });
                }
            };
            Some(new_events)
        };
        if let Some(events) = parse_event() {
            for event in events {
                match &event {
                    ParsedEvent::NowBatting { batter, .. } => {
                        parsing_context.player_names.insert(batter);
                    },
                    ParsedEvent::PitchingMatchup { home_pitcher, away_pitcher } => {
                        parsing_context.player_names.insert(home_pitcher);
                        parsing_context.player_names.insert(away_pitcher);
                    },
                    ParsedEvent::PitcherSwap { arriving_pitcher, .. } => {
                        parsing_context.player_names.insert(arriving_pitcher);
                    },
                    ParsedEvent::Lineup(_, lineup) => {
                        parsing_context.player_names.extend(lineup.iter().map(|(_, name)| *name));
                    }
                    _ => ()
                }
                result.push(event)
            }
        } else {
            panic!("Couldn't parse {event:?}")
        };
    }
    result
}

#[cfg(debug_assertions)]
fn handle_error(event:&Event, err: VerboseError<&str>, result: &mut Vec<ParsedEvent>) {
    use crate::nom_parsing::EXTRACT_TEAM_NAME;

    if err.errors.iter().any(|err| matches!(err, (_, VerboseErrorKind::Context(EXTRACT_PLAYER_NAME)) | (_, VerboseErrorKind::Context(EXTRACT_TEAM_NAME)))) {
        println!("{err}");
        result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone(), reason:EXTRACT_PLAYER_NAME.to_string() });
    } else {
        panic!("{err} {:?}", err.errors)
    }
    // if let Some((_, VerboseErrorKind::Context(EXTRACT_PLAYER_NAME))) = err.errors.last() {
    //     println!("{err} {:?}", err.errors);
    //     result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone(), reason:EXTRACT_PLAYER_NAME.to_string() });
    // } else {
    //     panic!("{err}")
    // }
}

#[cfg(not(debug_assertions))]
fn handle_error(event:&Event, err: VerboseError<&str>, result: &mut Vec<ParsedEvent>) {
    result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone(), reason: err.to_string() });
}