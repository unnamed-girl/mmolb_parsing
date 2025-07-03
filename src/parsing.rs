use crate::{game::Event, nom_parsing::{parse_event, ParsingContext}, parsed_event::ParsedEventMessage, Game};
use tracing::error;

/// Convenience method to call process_event for every event in a game
pub fn process_game<'output, 'parse>(game: &'output Game, game_id: &'parse str) -> Vec<ParsedEventMessage<&'output str>> {
    let mut result = Vec::new();

    for event in &game.event_log {
        result.push(process_event(event, game, game_id))
    }
    result
}

/// Processes an event into a ParsedEventMessage. Zero-copy parsing, the strings in the returned ParsedEventMessage are references to the strings in event and game.
pub fn process_event<'output, 'parse>(event: &'output Event, game: &'output Game, game_id: &'parse str) -> ParsedEventMessage<&'output str> {
    let parsing_context = ParsingContext::new(game_id, game, event.index);
    let parsed_event_message = match parse_event(event, &parsing_context) {
        Ok(event) => event,
        Err(e) => {
            error!("{game_id} s{}d{}i{:?} Parse error: for {:?}: {e}", game.season, game.day, event.index, &event.event);
            ParsedEventMessage::ParseError { event_type: event.event.to_string(), message: event.message.clone() }
        }
    };
    parsed_event_message
}