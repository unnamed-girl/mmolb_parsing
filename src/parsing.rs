use crate::{nom_parsing::{parse_event, ParsingContext}, parsed_event::ParsedEventMessage, Game};

/// Processes a game into a list of ParsedEvents.
/// Note that the game must live longer than the events, as zero copy parsing is used. 
pub fn process_events(game: &Game) -> Vec<ParsedEventMessage<&str>> {
    let mut result = Vec::new();
    let mut parsing_context = ParsingContext::new(game);

    for event in &game.event_log {
        let parsed_event = match parse_event(event, &parsing_context) {
            Ok(event) => Some(event),
            Err(err) => {
                #[cfg(debug_assertions)]
                {
                    panic!("{err} {:?}", err.errors)
                }
                #[cfg(not(debug_assertions))] {
                    result.push(ParsedEventMessage::ParseError { event_type: event.event, message: event.message.clone() });
                }
                None
            }
        };
        if let Some(parsed_event) = parsed_event {
            #[cfg(debug_assertions)] {
                assert_eq!(event.message, parsed_event.clone().unparse(), "Raw should equal unparsed. {:?}", parsed_event);
            }    
            match &parsed_event {
                ParsedEventMessage::LiveNow { away_team_name, home_team_name, .. } => {
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
