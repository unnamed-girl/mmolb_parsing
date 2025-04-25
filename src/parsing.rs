use nom::{Finish, Parser};
use nom_language::error::VerboseErrorKind;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{enums::{Base, EventType, FielderError, FoulType, HitDestination, HitType, Position, Side, StrikeType}, game::{Event, Pitch}, nom_parsing::{field_outcomes, ParsingContext, EXTRACT_FIELDER_NAME}};

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

pub struct MmolbRegexes {
    // Pitch Outcomes
    pitch_hit : Regex,
    struck_out : Regex,
    hit_by_pitch : Regex,

    walks : Regex,
    strike : Regex,
    foul : Regex,
    ball : Regex,

    // Things that sometimes get stuffed at the end of other events
    base_steals : Regex,
    caught_stealing : Regex,
    advances : Regex,
    scores : Regex,
    errors : Regex,
}
impl MmolbRegexes {
    pub fn new() -> Self {
        let name = r"([A-Z]\.|[^ \.!<>]+) ([^ \.!<>]+ ){0,2}?[^ \.!<>]+"; // Names can have up to four words. Names cannot have full stops/exclamation marks/gt/lt, with the exception of when the first name is an initial.
        let raw_position = r"\S\S?";
        
        let fielders = format!(r"(?<fielders>{raw_position} {name} unassisted|({raw_position} {name} to )+?{raw_position} {name})");
        let batter = format!(r"(?<batter>{name})");
        let runner = format!(r"(?<runner>{name})");
        let fielder = format!(r"(?<fielder>{name})");
        let hit_type = r"(?<hit_type>[^ \.!<>]+|[^ \.!<>]+ [^ \.!<>]+)";
        let destination = r"(?<destination>[^ \.!<>]+ [^ \.!<>]+)";
        let base = r"(?<base>[^ \.!<>]+)";
        let error = r"(?<error>[^ \.!<>]+)";
        let strike_type = r"(?<strike_type>[^ \.!<>]+)";
        let foul = r"(?<foul_type>[^ \.!<>]+)";
        let position = format!("(?<position>{raw_position})");
    
        // Pitch Outcomes
        let pitch_hit = Regex::new(&format!(r"^ {batter} hits a {hit_type} to {destination}\.")).unwrap();
        let struck_out = Regex::new(&format!(r"^ (Foul {foul}. )?{batter} struck out {strike_type}\.")).unwrap();
        let hit_by_pitch = Regex::new(&format!(r"^ {batter} was hit by the pitch and advances to first base\.")).unwrap();
    
        let walks = Regex::new(&format!(r"^ Ball 4. {batter} walks.")).unwrap();
        let strike = Regex::new(&format!(r"^ Strike, {strike_type}. \d-\d.")).unwrap();
        let foul = Regex::new(&format!(r"^ Foul {foul}. \d-\d.")).unwrap();
        let ball = Regex::new(r"^ Ball. \d-\d.").unwrap();
    
        // Things that sometimes get stuffed at the end of other events
        let base_steals = Regex::new(&format!(r"(>|!|\.) {runner} steals {base} base")).unwrap();
        let caught_stealing = Regex::new(&format!(r"(>|!|\.) {runner} is caught stealing {base} base")).unwrap();
        let advances = Regex::new(&format!(r"(>|!|\.) {runner} to {base} base")).unwrap();
        let scores = Regex::new(&format!(r"(>|!|\.) <strong>{runner} scores!</strong")).unwrap();
        let errors = Regex::new(&format!(r"(>|!|\.) {error} error by {fielder}")).unwrap();

        Self {pitch_hit, struck_out, hit_by_pitch, walks, strike, foul, ball, base_steals, caught_stealing, advances, scores, errors }
    }
}

pub fn process_events(events_log: &Vec<Event>, regexes: &MmolbRegexes) -> Vec<ParsedEvent> {
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
                    match field_outcomes(&parsing_context).parse(&event.message).finish() {
                        Ok((_, events)) => result.extend(events),
                        Err(err) => {
                            if let Some((_, VerboseErrorKind::Context(EXTRACT_FIELDER_NAME))) = err.errors.last() {
                                println!("{err}");
                                result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone(), reason:EXTRACT_FIELDER_NAME.to_string() });
                            } else {
                                panic!("{err}")
                            }
                        }
                    }
                },
                EventType::Pitch => {
                    result.push(ParsedEvent::Pitch(event.pitch.clone()?));
                    if let Some(captures) = regexes.pitch_hit.captures(&event.message) {
                        // let batter = captures.name("batter").unwrap().as_str().to_string();
                        let destination = captures.name("destination").unwrap().as_str().try_into().ok()?;
                        let hit_type = captures.name("hit_type").unwrap().as_str().try_into().ok()?;
                        result.push(ParsedEvent::Hit { hit_type, destination });
                    } else if let Some(captures) = regexes.strike.captures(&event.message) {
                        let strike_type = captures.name("strike_type").unwrap().as_str().try_into().ok()?;
                        result.push(ParsedEvent::Strike { strike_type });
                    } else if let Some(_) = regexes.walks.captures(&event.message) {
                        // let batter = captures.name("batter").unwrap().as_str().to_string();
                        result.push(ParsedEvent::Ball);
                        result.push(ParsedEvent::Walk);
                    } else if let Some(_) = regexes.ball.captures(&event.message) {
                        result.push(ParsedEvent::Ball);
                    } else if let Some(captures) = regexes.foul.captures(&event.message) {
                        let foul_type = captures.name("foul_type").unwrap().as_str().try_into().ok()?;
                        result.push(ParsedEvent::Foul { foul_type });
                    } else if let Some(captures) = regexes.struck_out.captures(&event.message) {
                        let batter = captures.name("batter").unwrap().as_str().to_string();
                        let strike_type = captures.name("strike_type").unwrap().as_str().try_into().ok()?;
                        result.push(ParsedEvent::Strike { strike_type });
                        result.push(ParsedEvent::Out { player:batter, fielders: Vec::new(), perfect_catch: false});
                    } else if let Some(_) = regexes.hit_by_pitch.captures(&event.message) {
                        result.push(ParsedEvent::HitByPitch);
                    } else {
                        todo!("Can't parse pitch: {}", event.message)
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
                    let batting_emoji = batting_team.next()?.to_string();
                    let batting_team = batting_team.collect::<Vec<_>>().join(" ");

                    if event.message.contains("takes the mound.") {
                        let remaining_message = iter.collect::<Vec<_>>().join(" ");
                        let (leaving_position, leaving_pitcher, arriving_position, arriving_pitcher) = pitcher_swap(&remaining_message)?;
                        
                        parsing_context.player_names.insert(arriving_pitcher.clone());

                        result.push(ParsedEvent::InningStart { number, side, batting_team, pitcher: None });
                        result.push(ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });
                    } else {
                        let pitching_emoji = iter.next()?.to_string();
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

            // -------------- Events that sometimes get stuffed at the end of another event ---------------------------
            for captures in regexes.base_steals.captures_iter(&event.message) {
                let runner = captures.name("runner").unwrap().as_str().to_string();
                let base = captures.name("base").unwrap().as_str().try_into().ok()?;
                result.push(ParsedEvent::RunnerAdvance{ runner, base, is_steal: true })
            }
            for captures in regexes.caught_stealing.captures_iter(&event.message) {
                let runner = captures.name("runner").unwrap().as_str().to_string();
                // let base = captures.name("base").unwrap().as_str().try_into().ok()?;
                result.push(ParsedEvent::Out { player: runner, fielders: Vec::new(), perfect_catch: false });
            }
            for captures in regexes.scores.captures_iter(&event.message) {
                let player = captures.name("runner").unwrap().as_str().to_string();
                result.push(ParsedEvent::Scores { player });
            }
            for captures in regexes.advances.captures_iter(&event.message) {
                let runner = captures.name("runner").unwrap().as_str().to_string();
                // Overlap between advances and a hit to a base.
                if !runner.contains("hits a") {
                    let base = captures.name("base").unwrap().as_str().try_into().ok()?;
                    result.push(ParsedEvent::RunnerAdvance { runner, base, is_steal: false })
                }
            }
            for captures in regexes.errors.captures_iter(&event.message) {
                let fielder = captures.name("fielder").unwrap().as_str().to_string();
                let error = captures.name("error").unwrap().as_str().try_into().ok()?;
                result.push(ParsedEvent::Error { fielder, error })
            }

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

fn extract_fielders(fielders_group: &str) -> Option<Vec<(Position, String)>> {
    if fielders_group.contains(" to ") {
        fielders_group.split(" to ").map(extract_position_and_name).collect()
    } else {
        Some(vec![extract_position_and_name(fielders_group.strip_suffix("unassisted")?)?])
    }
}