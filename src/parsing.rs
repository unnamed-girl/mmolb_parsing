use crate::{
    game::Event,
    nom_parsing::{parse_event, ParsingContext},
    parsed_event::ParsedEventMessage,
    Game,
};

/// Convenience method to call process_event for every event in a game
pub fn process_game<'output, 'parse: 'output>(
    game: &'output Game,
    game_id: &'parse str,
) -> Vec<ParsedEventMessage<&'output str>> {
    let mut result = Vec::new();

    for event in &game.event_log {
        result.push(process_event(event, game, game_id))
    }
    result
}

/// Processes an event into a ParsedEventMessage. Zero-copy parsing, the strings in the returned ParsedEventMessage are references to the strings in event and game.
pub fn process_event<'output, 'parse: 'output>(
    event: &'output Event,
    game: &'output Game,
    game_id: &'parse str,
) -> ParsedEventMessage<&'output str> {
    let parsing_context = ParsingContext::new(game_id, game, event.index);
    let parsed_event_message = parse_event(event, &parsing_context);
    parsed_event_message
}

#[cfg(test)]
mod test {
    use std::{error::Error, fs::File, io::Read};

    use crate::{process_game, utils::no_tracing_errs, Game, ParsedEventMessage};

    #[test]
    fn livingston() -> Result<(), Box<dyn Error>> {
        let no_tracing_errors = no_tracing_errs();

        let f = File::open("test_data/livingston_game.json").unwrap();
        let game: Game = serde_json::from_reader(f).unwrap();

        let mut buf = String::new();
        let mut f = File::open("test_data/livingston_game_result.json").unwrap();
        f.read_to_string(&mut buf).unwrap();

        let actual_events: Vec<ParsedEventMessage<String>> = buf
            .lines()
            .enumerate()
            .map(|(i, line)| serde_json::from_str(line).map_err(|e| (i, e)))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(
            game.event_log.len(),
            actual_events.len(),
            "Event count should match"
        );

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
