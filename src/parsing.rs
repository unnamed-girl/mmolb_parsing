use nom_language::error::VerboseErrorKind;

use crate::{nom_parsing::{parse_event, ParsingContext, EXTRACT_PLAYER_NAME, EXTRACT_TEAM_NAME}, parsed_event::{ParsedEvent, StartOfInningPitcher}, Game};

/// Processes a game into a list of ParsedEvents.
/// Note that the game must live longer than the events, as zero copy parsing is used. 
pub fn process_events<'output>(game: &'output Game) -> Vec<ParsedEvent<&'output str>> {
    let mut result = Vec::new();
    let mut parsing_context = ParsingContext::new(game);

    for event in &game.event_log {
        let parsed_event = match parse_event(event, &parsing_context) {
            Ok(event) => Some(event),
            Err(err) => {
                #[cfg(debug_assertions)]
                {
                    if err.errors.iter().any(|err| matches!(err, (_, VerboseErrorKind::Context(EXTRACT_PLAYER_NAME)) | (_, VerboseErrorKind::Context(EXTRACT_TEAM_NAME)))) {
                        println!("{err}");
                        result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone() });
                    } else {
                        panic!("{err} {:?}", err.errors)
                    }
                }
                #[cfg(not(debug_assertions))] {
                    result.push(ParsedEvent::ParseError { event_type: event.event, message: event.message.clone() });
                }
                None
            }
        };
        if let Some(parsed_event) = parsed_event {
            #[cfg(debug_assertions)] {
                assert_eq!(event.message, parsed_event.clone().unparse(), "Raw should equal unparsed. {:?}", parsed_event);
            }    
            match &parsed_event {
                ParsedEvent::NowBatting { batter, .. } => {
                    parsing_context.player_names.insert(batter);
                },
                ParsedEvent::InningStart { batting_team_name, pitcher_status, .. } => {   
                    if let StartOfInningPitcher::Different { arriving_pitcher, .. } = pitcher_status {
                        parsing_context.player_names.insert(arriving_pitcher);
                    }
                    parsing_context.team_names.insert(batting_team_name);
                }
                ParsedEvent::PitchingMatchup { home_pitcher, away_pitcher, .. } => {
                    parsing_context.player_names.insert(home_pitcher);
                    parsing_context.player_names.insert(away_pitcher);
                },
                ParsedEvent::PitcherSwap { arriving_pitcher, .. } => {
                    parsing_context.player_names.insert(arriving_pitcher);
                },
                ParsedEvent::Lineup { players, .. } => {
                    parsing_context.player_names.extend( players.iter().map(|player| player.name));
                }
                ParsedEvent::LiveNow { away_team_name, home_team_name, .. } => {
                    parsing_context.team_names.insert(away_team_name);
                    parsing_context.team_names.insert(home_team_name);
                }
                _ => ()
            }
            result.push(parsed_event)
        };
    }
    result
}
