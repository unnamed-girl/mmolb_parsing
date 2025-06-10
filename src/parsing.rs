use crate::{game::Event, nom_parsing::{parse_event, ParsingContext}, parsed_event::ParsedEventMessage, Game};

/// Convenience method to call process_event for every event in a game
pub fn process_game(game: &Game) -> Vec<ParsedEventMessage<&str>> {
    let mut result = Vec::new();

    for event in &game.event_log {
        result.push(process_event(event, game))
    }
    result
}

/// Processes an event into a ParsedEventMessage. Zero-copy parsing, the strings in the returned ParsedEventMessage are references to the strings in event and game.
pub fn process_event<'output>(event: &'output Event, game: &'output Game) -> ParsedEventMessage<&'output str> {
    let parsing_context = ParsingContext::new(game);
    let parsed_event_message = match parse_event(event, &parsing_context) {
        Ok(event) => event,
        Err(err) => {
            #[cfg(feature = "panic_on_parse_error")]
            {
                panic!("{err} {:?}", err.errors)
            }
            #[cfg(not(feature = "panic_on_parse_error"))] {
                ParsedEventMessage::ParseError { event_type: event.event.to_string(), message: event.message.clone() }
            }
        }
    };
    #[cfg(feature = "panic_on_parse_error")] {
        assert_eq!(event.message, parsed_event_message.clone().unparse(), "Raw should equal unparsed. {:?}", parsed_event_message);
    }
    parsed_event_message
}