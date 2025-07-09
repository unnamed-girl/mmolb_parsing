use crate::{game::Event, nom_parsing::{parse_event, ParsingContext}, parsed_event::{GameEventParseError, ParsedEventMessage}, Game};
use tracing::error;

/// Convenience method to call process_event for every event in a game
pub fn process_game<'output, 'parse>(game: &'output Game, game_id: &'parse str) -> Vec<Result<ParsedEventMessage<&'output str>, GameEventParseError>> {
    let mut result = Vec::new();

    for event in &game.event_log {
        result.push(process_event(event, game, game_id))
    }
    result
}

/// Processes an event into a ParsedEventMessage. Zero-copy parsing, the strings in the returned ParsedEventMessage are references to the strings in event and game.
pub fn process_event<'output, 'parse>(event: &'output Event, game: &'output Game, game_id: &'parse str) -> Result<ParsedEventMessage<&'output str>, GameEventParseError> {
    let parsing_context = ParsingContext::new(game_id, game, event.index);
    let parsed_event_message = parse_event(event, &parsing_context);
    if let Err(e) = &parsed_event_message {
        error!("Parse error for {:?}: {e}", &event.event);
    }
    parsed_event_message
}


#[cfg(test)]
mod test {
    use std::{error::Error, fs::File, io::Read};

    

    use crate::{process_game, utils::no_tracing_errs, Game, ParsedEventMessage};

    #[test]
    fn livingston() -> Result<(), Box<dyn Error>> {
        let no_tracing_errors = no_tracing_errs();

        let f = File::open("test_data/livingston_game.json")?;
        let game:Game = serde_json::from_reader(f)?;

        let mut buf = String::new();
        let mut f = File::open("test_data/livingston_game.ron")?;
        f.read_to_string(&mut buf)?;

        let actual_events: Vec<ParsedEventMessage<String>> = buf.lines().map(|line| ron::from_str(line)).collect::<Result<Vec<_>, _>>()?;

        assert_eq!(game.event_log.len(), actual_events.len(), "Event count should match");

        let parsed_events = process_game(&game, "68474b55452606ed6b72dbe8");

        for (event_a, event_b) in parsed_events.iter().zip(actual_events.iter()) {
            let event_a = serde_json::to_value(event_a)?;
            let event_b = serde_json::to_value(event_b)?;
            let diff = serde_json_diff::values(event_a, event_b);
            assert!(diff.is_none(), "{diff:?}");
        }

        drop(no_tracing_errors);
        Ok(())
    }
}