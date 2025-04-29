use nom::{Finish, Parser};
use nom_language::error::{VerboseError, VerboseErrorKind};
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use crate::{
    enums::{
        Base, Distance, EventType, FielderError, FlyballType, FoulType, HitDestination, HitType,
        Position, Side, StrikeType,
    },
    game::{Event, Pitch},
    nom_parsing::{
        parse_field_event, parse_inning_start_event, parse_lineup_event, parse_mound_visit,
        parse_pitch_event, parse_pitching_matchup_event, ParsingContext, EXTRACT_PLAYER_NAME,
    },
    Game,
};

/// S is the string type used. S = &'output str is used by the parser,
/// but a mutable type is necessary when directly deserializing, because some players have escaped characters in their names
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumDiscriminants)]
pub enum ParsedEvent<S> {
    ParseError {
        event_type: EventType,
        message: String,
    },

    // One off events
    LiveNow,
    PitchingMatchup {
        home_pitcher: S,
        away_pitcher: S,
    },
    Lineup {
        side: Side,
        players: Vec<(Position, S)>,
    },
    PlayBall,
    GameOver,
    Recordkeeping {
        home_score: u8,
        away_score: u8,
    },

    // inningTiming
    InningStart {
        number: u8,
        side: Side,
        batting_team: S,
        /// If none this is followed by a pitcher swap
        pitcher: Option<S>,
    },
    NowBatting {
        batter: S,
        first_pa: bool,
    },
    InningEnd {
        number: u8,
        side: Side,
    },

    // Mound visits
    MoundVisit {
        team: S,
    },
    MoundVisitRefused,
    PitcherSwap {
        leaving_position: Position,
        leaving_pitcher: S,
        arriving_position: Position,
        arriving_pitcher: S,
    },

    // Pitch
    Pitch {
        pitch: Pitch,
    },
    Ball,
    Strike {
        strike: StrikeType,
    },
    Foul {
        foul: FoulType,
    },
    Walk {
        batter: S,
    },
    HitByPitch {
        batter: S,
    },
    Hit {
        batter: S,
        hit: HitType,
        destination: HitDestination,
    },
    StrikeOut {
        batter: S,
    },
    /// Stealing home scores
    Steal {
        runner: S,
        base: Base,
    },
    CaughtStealing {
        runner: S,
        base: Base,
    },

    // Field
    /// Scores if home run
    BatterToBase {
        batter: S,
        distance: Distance,
        fielder: Option<(Position, S)>,
    },
    CaughtOut {
        batter: S,
        fly: FlyballType,
        catcher: (Position, S),
        sacrifice: bool,
        perfect: bool,
    },
    GroundedOut {
        batter: S,
        runner: S,
        fielders: Vec<(Position, S)>,
        base: Base,
        sacrifice: bool,
    },
    /// Advancing to home scores
    Advance {
        runner: S,
        base: Base,
    },
    FieldingError {
        fielder: S,
        error: FielderError,
    },
}
impl<S> ParsedEvent<S> {
    pub fn out(&self) -> Option<&S> {
        match self {
            ParsedEvent::GroundedOut { runner, .. } => Some(runner),
            ParsedEvent::StrikeOut { batter } => Some(batter),
            ParsedEvent::CaughtOut { batter, .. } => Some(batter),
            ParsedEvent::CaughtStealing { runner, .. } => Some(runner),
            _ => None,
        }
    }
    pub fn scores(&self) -> Option<&S> {
        match self {
            ParsedEvent::Advance {
                runner,
                base: Base::Home,
            } => Some(runner),
            ParsedEvent::Steal {
                runner,
                base: Base::Home,
            } => Some(runner),
            ParsedEvent::BatterToBase {
                batter,
                distance: Distance::HomeRun,
                ..
            } => Some(batter),
            _ => None,
        }
    }
}

/// Processes a game into a list of ParsedEvents.
/// Note that the game must live longer than the events, as zero copy parsing is used.
pub fn process_events<'output>(game: &'output Game) -> Vec<ParsedEvent<&'output str>> {
    let mut result = Vec::new();
    let mut parsing_context = ParsingContext::new(&game);

    for event in &game.event_log {
        let mut parse_event = || {
            let mut new_events = Vec::new();
            match event.event {
                EventType::PitchingMatchup => match parse_pitching_matchup_event(&parsing_context)
                    .parse(&event.message)
                    .finish()
                {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::AwayLineup => match parse_lineup_event(Side::Away, &parsing_context)
                    .parse(&event.message)
                    .finish()
                {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::HomeLineup => match parse_lineup_event(Side::Home, &parsing_context)
                    .parse(&event.message)
                    .finish()
                {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::Field => match parse_field_event(&parsing_context)
                    .parse(&event.message)
                    .finish()
                {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::Pitch => {
                    new_events.push(ParsedEvent::Pitch {
                        pitch: event.pitch.clone().expect("Pitch to have a pitch"),
                    });
                    match parse_pitch_event(&parsing_context)
                        .parse(&event.message)
                        .finish()
                    {
                        Ok((_, events)) => new_events.extend(events),
                        Err(err) => handle_error(&event, err, &mut result),
                    }
                }
                EventType::GameOver => new_events.push(ParsedEvent::GameOver),
                EventType::InningEnd => {
                    let mut iter = event.message.split(" ").skip(3);
                    let side = match iter.next()? {
                        "top" => Side::Home,
                        "bottom" => Side::Away,
                        _ => return None,
                    };
                    let mut iter = iter.skip(2);
                    let number = iter
                        .next()?
                        .chars()
                        .rev()
                        .skip(3)
                        .collect::<Vec<char>>()
                        .into_iter()
                        .rev()
                        .collect::<String>()
                        .parse()
                        .ok()?;
                    new_events.push(ParsedEvent::InningEnd { number, side });
                }
                EventType::InningStart => match parse_inning_start_event(&parsing_context)
                    .parse(&event.message)
                    .finish()
                {
                    Ok((_, events)) => new_events.extend(events),
                    Err(err) => handle_error(&event, err, &mut result),
                },
                EventType::LiveNow => result.push(ParsedEvent::LiveNow),
                EventType::MoundVisit => match parse_mound_visit(&parsing_context)
                    .parse(&event.message)
                    .finish()
                {
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
                    new_events.push(ParsedEvent::Recordkeeping {
                        home_score,
                        away_score,
                    });
                }
            };
            Some(new_events)
        };
        if let Some(events) = parse_event() {
            for event in events {
                match &event {
                    ParsedEvent::NowBatting { batter, .. } => {
                        parsing_context.player_names.insert(batter);
                    }
                    ParsedEvent::PitchingMatchup {
                        home_pitcher,
                        away_pitcher,
                    } => {
                        parsing_context.player_names.insert(home_pitcher);
                        parsing_context.player_names.insert(away_pitcher);
                    }
                    ParsedEvent::PitcherSwap {
                        arriving_pitcher, ..
                    } => {
                        parsing_context.player_names.insert(arriving_pitcher);
                    }
                    ParsedEvent::Lineup { players, .. } => {
                        parsing_context
                            .player_names
                            .extend(players.iter().map(|(_, name)| *name));
                    }
                    _ => (),
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
fn handle_error(event: &Event, err: VerboseError<&str>, result: &mut Vec<ParsedEvent<&str>>) {
    use crate::nom_parsing::EXTRACT_TEAM_NAME;

    if err.errors.iter().any(|err| {
        matches!(
            err,
            (_, VerboseErrorKind::Context(EXTRACT_PLAYER_NAME))
                | (_, VerboseErrorKind::Context(EXTRACT_TEAM_NAME))
        )
    }) {
        println!("{err}");
        result.push(ParsedEvent::ParseError {
            event_type: event.event,
            message: event.message.clone(),
        });
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
fn handle_error(event: &Event, err: VerboseError<&str>, result: &mut Vec<ParsedEvent<&str>>) {
    result.push(ParsedEvent::ParseError {
        event_type: event.event,
        message: event.message.clone(),
    });
}
