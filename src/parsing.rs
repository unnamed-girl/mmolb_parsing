use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{enums::{BallType, EventType, HitDestination, Position, Side}, game::{Event, Pitch}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParsedEvent {
    PitchingMatchup {
        home_pitcher: String,
        away_pitcher: String,
    },
    MoundVisit {
        team: String,
    },
    PitcherSwap {
        leaving_position: Position,
        leaving_pitcher: String,
        arriving_position: Position,
        arriving_pitcher: String,
    },
    GameOver,
    Steal(String, u8),
    RunnerAdvance { runner: String, base: u8 },
    Error { fielder: String, error: String },
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
        ball_type: BallType,
        destination: HitDestination
    },
    /// Includes home runs as base = 4.
    BatterToBase {batter: String, base:u8, fielder: Option<(Position, String)>},
    Out {player: String, fielders: Vec<(Position, String)>},
    Scores {player: String },
    Ball,
    FoulBall,
    Strike {strike_type: String},
    StruckOut,
    HitByPitch,
    InningEnd {
        number: u8,
        side: Side
    },
    PlayBall,
    NowBatting {
        batter: String,
        first_pa: bool
    }
}

pub fn process_events(events_log: &Vec<Event>) -> Vec<ParsedEvent> {
    // Field Outcomes:
    let homer= Regex::new(r"[^\.<>!,]+ homers on a [\S]+ [\S]+ to [^\.<>!,]+").unwrap();
    let grand_slam = Regex::new(r"[^\.<>!,]+ hits a grand slam on a [\S]+ [\S]+ to [^\.<>!,]+").unwrap();
    let double_play = Regex::new(r"[^\.<>!,]+ [\S]+ into a double play, [^\.<>!,]+. .*").unwrap();
    let multi_fielder_out = Regex::new(r"[^\.<>!,]+ [\S]+ out, [^\.<>!,]+").unwrap();
    let multi_fielder_two = Regex::new(r" [\S]+ out, ").unwrap();
    let single_fielder_out = Regex::new(r"[^\.<>!,]+ [\S]+ out to [\S]+ [^\.<>!,]+").unwrap();
    let single_fielder_out_split = Regex::new(r" [\S]+ out to ").unwrap();
    let reaches_on_choice = Regex::new(r"[^\.<>!,]+ reaches on a fielder's choice, fielded by [\S]+ [^\.<>!,]+").unwrap();
    let reaches_on_choice_split = Regex::new(r" reaches on a fielder's choice, fielded by ").unwrap();
    let reaches_on_error = Regex::new(r"[^\.<>!,]+ reaches on a [\S]+ error by [\S]+ [^\.<>!,]+").unwrap();
    let reaches_on_error_split = Regex::new(r" reaches on a [\S]+ error by ").unwrap();
    let successful_hit = Regex::new(r"[^\.<>!,]+ [\S]+ on a [^\.<>!,]+ to [\S]+ [^\.<>!,]+").unwrap();
    let successful_hit_find_base = Regex::new(r"[\S]+ on").unwrap();
    let successful_hit_split = Regex::new(r" [\S]+ on a [^\.<>!,]+ to ").unwrap();

    // Pitch Outcomes
    let pitch_hit = Regex::new(r"[^\.<>!, ][^\.<>!,]+ hits a [^\.<>!,]+ to [^\.<>!,]+").unwrap();
    let pitch_hit_split_one = Regex::new(r" hits a ").unwrap();
    let pitch_hit_split_two = Regex::new(r" to ").unwrap();

    let struck_out = Regex::new(r"[^\.<>!, ][^\.<>!,]+ struck out [^\.<>!,]+").unwrap();
    let struck_out_split = Regex::new(r" struck out ").unwrap();
    let hit_by_pitch = Regex::new(r"[^\.<>!, ][^\.<>!,]+ was hit by the pitch and advances to first base.").unwrap();

    // Things that sometimes get stuffed at the end of other events
    let base_steals = Regex::new(r"[^\.<>!, ][^\.<>!,]+ steals [\S]+ base").unwrap();
    let advances = Regex::new(r"[^\.<>!, ][^\.<>!,]+ to [\S]+ base").unwrap();
    let scores = Regex::new(r"[^\.<>!, ][^\.<>!,]+ scores!").unwrap();
    let errors = Regex::new(r"[^\.<>!, ][\S]+ error by [^\.<>!,]+").unwrap();

    let mut events = events_log.into_iter();
    let mut result = Vec::new();
    while let Some(event) = events.next() {
        match event.event {
            EventType::PitchingMatchup => {
                let mut iter = event.message.split(" vs. ").map(|side| {
                    let mut words = side.split(" ").collect::<Vec<_>>();
                    words.reverse();
                    let mut name = words.into_iter().take(2).collect::<Vec<_>>();
                    name.reverse();
                    name.join(" ")
                });
                let home_pitcher = iter.next().expect("Pitching matchup string parse to succeed");
                let away_pitcher = iter.next().expect("Pitching matchup string parse to succeed");
                result.push(ParsedEvent::PitchingMatchup { home_pitcher, away_pitcher })
            }
            EventType::AwayLineup => result.push(ParsedEvent::Lineup(Side::Away, parse_lineup(&event.message))),
            EventType::HomeLineup => result.push(ParsedEvent::Lineup(Side::Home, parse_lineup(&event.message))),
            EventType::Field => {
                if let Some(m) = homer.find(&event.message) {
                    // m = [batter name] homers on a [hit type] to [position]

                    let batter = m.as_str().split(" homers").next().unwrap().to_string();
                    result.push(ParsedEvent::BatterToBase { batter: batter.clone(), base: 4, fielder: None });
                    result.push(ParsedEvent::Scores { player: batter });
                } else if let Some(m) = grand_slam.find(&event.message) {
                    // m = [batter name] hits a grand slam on a [hit type] to [position]
                    let batter = m.as_str().split(" hits").next().unwrap().to_string();
                    result.push(ParsedEvent::BatterToBase { batter: batter.clone(), base: 4, fielder: None });
                    result.push(ParsedEvent::Scores { player: batter });
                } else if let Some(m) = double_play.find(&event.message) {
                    // m = [batter name] [hit type] into a double play, ([position] [fielder name] to [position] [fielder name]...). [Runner] out at [Base]. [Runner] out at [Base].
                    let mut iter = m.as_str().split(". ");
                    let fielders = iter.next().unwrap().split(", ").skip(1).next().unwrap().split(" to ").map(extract_position_and_name).collect::<Vec<_>>();
                    for out in iter {
                        let player = out.split(" ").take_while(|s| *s != "out").collect::<Vec<_>>().join(" ");
                    
                        result.push(ParsedEvent::Out { player, fielders: fielders.clone() });
                    }
                } else if let Some(m) = multi_fielder_out.find(&event.message) { 
                    // m = [batter name] [hit type] out, ([position] [fielder name] to [position] [fielder name]...) 
                    
                    let mut iter = multi_fielder_two.split(m.as_str());
                    let batter = iter.next().unwrap().to_string();
                    let fielders = iter.next().unwrap().split(" to ").map(extract_position_and_name).collect::<Vec<_>>();
                    result.push(ParsedEvent::Out { player: batter, fielders });
                } else if let Some(m) = single_fielder_out.find(&event.message) {
                    // m = [batter name] [hit type]s out to [position] [fielder name]
                    
                    let mut iter = single_fielder_out_split.split(m.as_str());
                    let batter = iter.next().unwrap().to_string();
                    let fielder = extract_position_and_name(iter.next().unwrap());
                    result.push(ParsedEvent::Out { player: batter, fielders: vec![fielder] });
                } else if let Some(m) = reaches_on_choice.find(&event.message) {
                    // m = [batter name] reaches on a [error_type] error by [position] [fielder name]
                    let mut iter = reaches_on_choice_split.split(m.as_str());
                    let batter = iter.next().unwrap().to_string();
                    let fielder = extract_position_and_name(iter.next().unwrap());
                    result.push(ParsedEvent::BatterToBase { batter, base: 1, fielder: Some(fielder) })
                }   else if let Some(m) = reaches_on_error.find(&event.message) {
                    // m = [batter name] reaches on a [error_type] error by [position] [fielder name]
                    let mut iter = reaches_on_error_split.split(m.as_str());
                    let batter = iter.next().unwrap().to_string();
                    let fielder = extract_position_and_name(iter.next().unwrap());
                    result.push(ParsedEvent::BatterToBase { batter, base: 1, fielder: Some(fielder) })
                } else if let Some(m) = successful_hit.find(&event.message) {
                    // m = [batter name] [singles/doubles/triples/homers on a [hit] [type] to [position] [fielder name]
                    let base = match successful_hit_find_base.find(m.as_str()).unwrap().as_str().strip_suffix(" on").unwrap() {
                        "singles" => 1,
                        "doubles" => 2,
                        "triples" => 3,
                        _ => panic!("Base not recognised")
                    };

                    let mut iter = successful_hit_split.split(m.as_str());
                    let batter = iter.next().unwrap().to_string();
                    let fielder = extract_position_and_name(iter.next().unwrap());
                    result.push(ParsedEvent::BatterToBase { batter, base, fielder: Some(fielder) });
                } else {
                    todo!("unrecognised field event: {}", event.message);
                }
            },
            EventType::Pitch => {
                result.push(ParsedEvent::Pitch(event.pitch.clone().unwrap()));
                if let Some(m) = pitch_hit.find(&event.message) {
                    let mut iter = pitch_hit_split_one.split(m.as_str());
                    let batter = iter.next().unwrap();

                    let mut iter =  pitch_hit_split_two.split(iter.next().unwrap());
                    let ball_type = iter.next().unwrap().try_into().unwrap();
                    let destination = iter.next().unwrap().try_into().unwrap();
                    result.push(ParsedEvent::Hit { ball_type, destination });
                } else if event.message.contains("Strike") {
                    let strike_type = event.message.strip_prefix(" ").unwrap().split(" ").skip(1).next().unwrap().strip_suffix(".").unwrap().to_string();
                    result.push(ParsedEvent::Strike { strike_type });
                } else if event.message.contains("Ball") {
                    result.push(ParsedEvent::Ball);
                } else if event.message.contains("Foul") {
                    result.push(ParsedEvent::FoulBall);
                } else if let Some(m) = struck_out.find(&event.message) {
                    let mut iter = struck_out_split.split(m.as_str());
                    let batter = iter.next().unwrap().to_string();
                    let strike_type = iter.next().unwrap().to_string();
                    result.push(ParsedEvent::Strike { strike_type });
                    result.push(ParsedEvent::StruckOut);
                } else if let Some(_) = hit_by_pitch.find(&event.message) {
                    result.push(ParsedEvent::HitByPitch);
                } else {
                    todo!("Can't parse pitch: {}", event.message)
                }
            },
            EventType::GameOver => result.push(ParsedEvent::GameOver),
            EventType::InningEnd => {
                let mut iter = event.message.split(" ").skip(3);
                let side = match iter.next().expect("InningEnd Parsing: side") {
                    "top" => Side::Home,
                    "bottom" => Side::Away,
                    _ => panic!("Not top or bottom") 
                };
                let mut iter = iter.skip(2);
                let number = iter.next().expect("InningEndParsing: number exists")
                    .chars().rev().skip(3).collect::<Vec<char>>().into_iter().rev().collect::<String>()
                    .parse().expect("InningEndParsing: number can be parsed");
                result.push(ParsedEvent::InningEnd { number, side });
            }
            EventType::InningStart => {
                let mut iter = event.message.split(" ")
                    .skip(3); // Start of the
                let side = match iter.next().expect("InningEnd Parsing: side") {
                    "top" => Side::Home,
                    "bottom" => Side::Away,
                    _ => panic!("Not top or bottom") 
                };
                let mut iter = iter
                    .skip(2); // of the
                let number = iter.next().expect("InningStartParsing: number exists")
                    .chars().rev().skip(3).collect::<Vec<char>>().into_iter().rev().collect::<String>() // Remove "st.", "nd." or "th." from the end
                    .parse().expect("InningStartParsing: number can be parsed");
                let mut batting_team = iter.by_ref().take_while(|s| *s != "batting.");
                let batting_emoji = batting_team.next().unwrap().to_string();
                let batting_team = batting_team.collect::<Vec<_>>().join(" ");

                if event.message.contains("takes the mound.") {
                    let remaining_message = iter.collect::<Vec<_>>().join(" ");
                    let (leaving_position, leaving_pitcher, arriving_position, arriving_pitcher) = pitcher_swap(&remaining_message);
                    
                    result.push(ParsedEvent::InningStart { number, side, batting_team, pitcher: None });
                    result.push(ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });
                } else {
                    let pitching_emoji = iter.next().unwrap().to_string();
                    let pitcher = iter.take_while(|s| *s != "pitching.")
                        .collect::<Vec<_>>().join(" ");
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
                } else {
                    let (leaving_position, leaving_pitcher, arriving_position, arriving_pitcher) = pitcher_swap(&event.message);   
                    result.push(ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });
                }
            }
            EventType::NowBatting => {
                let mut message = event.message.strip_prefix("Now batting: ").unwrap().split(" (");
                let batter = message.next().unwrap().to_string();
                let first_pa = Some("1st PA of game)") == message.next();
                result.push(ParsedEvent::NowBatting { batter, first_pa });
            }
            EventType::PlayBall => result.push(ParsedEvent::PlayBall),
            EventType::Recordkeeping => {
                let score = event.message.split(" ").last().unwrap();
                let mut iter = score.split("-");
                let home_score = iter.next().unwrap().parse().unwrap();
                let away_score = iter.next().unwrap().parse().unwrap();
                result.push(ParsedEvent::Recordkeeping { home_score, away_score });
            }
        };

        // -------------- Events that sometimes get stuffed at the end of another event ---------------------------
        let later_sentences = event.message.split(".").skip(1).collect::<Vec<_>>().join(".");
        for base_steal in base_steals.find_iter(&later_sentences) {
            let mut iter = base_steal.as_str().strip_suffix(" base").unwrap().split(" steals ");
            let runner = iter.next().unwrap().to_string();
            let base = match iter.next().unwrap() {
                "first" => 1,
                "second" => 2,
                "third" => 3,
                "home" => 4,
                _ => panic!("don't recognise base")
            };
            result.push(ParsedEvent::Steal(runner, base))
        }
        for score in scores.find_iter(&later_sentences) {
            let player = score.as_str().strip_suffix("scores!").unwrap().to_string();
            result.push(ParsedEvent::Scores { player });
        }
        for advances in advances.find_iter(&later_sentences) {
            let mut iter = advances.as_str().strip_suffix(" base").unwrap().split(" to ");
            let runner = iter.next().unwrap().to_string();
            let base = match iter.next().unwrap() {
                "first" => 1,
                "second" => 2,
                "third" => 3,
                "home" => 4,
                _ => panic!("don't recognise base")
            };
            result.push(ParsedEvent::RunnerAdvance { runner, base })
        }
        for error in errors.find_iter(&later_sentences) {
            let mut iter = error.as_str().split(" error by ");
            let error = iter.next().unwrap().to_string();
            let fielder = iter.next().unwrap().to_string();
            result.push(ParsedEvent::Error { fielder, error })
        }
    }
    result
}

fn pitcher_swap(message: &String) -> (Position, String, Position, String) {
    let mut iter = message.split(" ");
    let leaving_position = iter.next().unwrap().try_into().unwrap();
    let leaving_pitcher = iter.by_ref().take_while(|s| *s != "is").collect::<Vec<_>>().join(" ");
                    
    let mut iter = iter.skip_while(|s| *s != "game.").skip(1);
    // is leaving the game
    let arriving_position = iter.next().unwrap().try_into().unwrap();
    let arriving_pitcher = iter.take_while(|s| *s != "takes")
        .collect::<Vec<_>>().join(" ");
    (leaving_position, leaving_pitcher, arriving_position, arriving_pitcher)
}

fn parse_lineup(message: &str) -> Vec<(Position, String)> {
    message.strip_suffix("<br>").unwrap().split("<br>").map(|player| {
        let mut iter = player.split(" ");
        let _number = iter.next();
        extract_position_and_name(&iter.collect::<Vec<_>>().join(" "))
    }).collect()
}

fn extract_position_and_name(position_and_name: &str) -> (Position, String) {
    let mut iter = position_and_name.split(" ");
    let position = iter.next().unwrap().try_into().unwrap();
    let name = iter.collect::<Vec<_>>().join(" ");
    (position, name)
}