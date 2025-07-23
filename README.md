As mmolb is constantly changing, this library is very volatile. Commits beginning with a version number include a change to an api.
Struct fields and enum variants are often added - but non_exhaustive is not being used, because associated projects like mmoldb rely on exhaustively covering all currently known variants so that the compiler warns them when new variants appear.

# Library
## Game event parsing
`mmolb_parsing::Game` - can be deserialized from the mmolb api response.
- has an event_log field, a vec of events.

`mmolb_parsing::process_event`
- produce a `mmolb_parsing::ParsedEventMessage` from an event and a game.

## Feed parsing
New, will be very volatile for the next while.

`mmolb_parsing::team::Team` - can be deserialized from the mmolb api response
- has a feed field, `AddedLater<Vec<mmolb_parsing::feed_event::FeedEvent>>` (use unwrap_or_default or similar to handle teams deleted before the feed was added)

`mmolb_parsing::team::Player` - can be deserialized from the mmolb api response
- has a feed field, `AddedLater<Vec<mmolb_parsing::feed_event::FeedEvent>>` (use unwrap_or_default or similar to handle players deleted before the feed was added)

`mmolb_parsing::feed_event::FeedEvent`
- has an event_type field, which can be cast to an `Option<mmolb_parsing::enums::FeedEventType>` with into_inner.
- has a text field, `mmolb_parsing::feed_event::FeedEventText` with a parse() method that takes an `mmolb_parsing::enums::FeedEventType`.

Alternatively `mmolb_parsing::feed_event::parse_feed_event` is provided
