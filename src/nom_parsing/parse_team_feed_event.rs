use nom::{branch::alt, combinator::fail, error::context, Finish, Parser};
use crate::{enums::FeedEventType, feed_event::{FeedEvent, FeedEventParseError}, team_feed::ParsedTeamFeedEventText};

use super::shared::Error;


trait TeamFeedEventParser<'output>: Parser<&'output str, Output = ParsedTeamFeedEventText<&'output str>, Error = Error<'output>> {}
impl<'output, T: Parser<&'output str, Output = ParsedTeamFeedEventText<&'output str>, Error = Error<'output>>> TeamFeedEventParser<'output> for T {}


pub fn parse_team_feed_event<'output>(event: &'output FeedEvent) -> ParsedTeamFeedEventText<&'output str> {
    let event_type = match &event.event_type {
        Ok(event_type) => event_type,
        Err(e) => {
            let error = FeedEventParseError::EventTypeNotRecognized(e.clone());
            return ParsedTeamFeedEventText::ParseError { error, text: &event.text };
        }
    };

    let result = match event_type {
        FeedEventType::Game => game(event).parse(&event.text),
        FeedEventType::Augment => augment(event).parse(&event.text),
        FeedEventType::Release => release(event).parse(&event.text),
        FeedEventType::Season => season(event).parse(event.text.as_str())
    };
    match result.finish() {
        Ok(("", output)) => output,
        Ok((leftover, _)) => {
            tracing::error!("{event_type} feed event parsed had leftover: {leftover} from {}", &event.text);
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            ParsedTeamFeedEventText::ParseError { error, text: &event.text }
        }
        Err(e) => {
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            tracing::error!("Parse error: {e:?}");
            ParsedTeamFeedEventText::ParseError { error, text: &event.text }
        }
    }
}

fn game<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Game Feed Event", alt((
        fail(),
    )))
}

fn augment<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Augment Feed Event", alt((
        fail(),
    )))
}

fn release<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Release Feed Event", alt((
        fail(),
    )))
}

fn season<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Season Feed Event", alt((
        fail(),
    )))
}
