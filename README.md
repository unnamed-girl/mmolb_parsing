Known parse errors:
- (season 0 issue) If a team's name changes between the start of game and the PitchingMatchup (the second event of a game), the parser is unable to parse that event. Team name changes at any other time should be fine.

# Library
## Game event parsing
`mmolb_parsing::Game` - can be deserialized from the mmolb api response.
- has an event_log field, a vec of events.

`mmolb_parsing::process_event`
- produce a `mmolb_parsing::ParsedEventMessage` from an event and a game.

## Feed parsing
New, will be very volatile for the next while.

`mmolb_parsing::team::Team` - can be deserialized from the mmolb api response
- has a feed field, `Vec<mmolb_parsing::feed_event::FeedEvent>`

`mmolb_parsing::feed_event::FeedEvent`
- has an event_type field, which can be cast to an `Option<mmolb_parsing::enums::FeedEventType>` with into_inner.
- has a text field, `mmolb_parsing::feed_event::FeedEventText` with a parse() method that takes an `mmolb_parsing::enums::FeedEventType`.

Alternatively `mmolb_parsing::feed_event::parse_feed_event` is provided
