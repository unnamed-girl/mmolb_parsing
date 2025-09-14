use nom::{branch::alt, bytes::complete::{tag, take_while}, character::complete::{u16, u8}, combinator::{all_consuming, fail, opt, verify}, error::context, multi::{many1, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use crate::{enums::FeedEventType, feed_event::{AttributeChange, FeedEvent, FeedEventParseError}, nom_parsing::shared::{emoji_team_eof, emojiless_item, feed_delivery, name_eof, parse_terminated, try_from_word}, team_feed::ParsedTeamFeedEventText, time::Breakpoints};

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
        game_result(),
        feed_delivery("Delivery").map(|delivery| ParsedTeamFeedEventText::Delivery { delivery } ),
        feed_delivery("Shipment").map(|delivery| ParsedTeamFeedEventText::Shipment { delivery } ),
        feed_delivery("Special Delivery").map(|delivery| ParsedTeamFeedEventText::SpecialDelivery { delivery } ),
        photo_contest(),
        prosperous(),
        fail(),
    )))
}

fn augment<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Augment Feed Event", alt((
        enchantment_s1a(),
        enchantment_s1b(),
        enchantment_s2(),
        enchantment_compensatory(),
        attribute_gain(),
        multiple_attribute_equal(event),
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

fn game_result<'output>() -> impl TeamFeedEventParser<'output> {
    (
        parse_terminated(" vs. ").and_then(emoji_team_eof),
        parse_terminated(" - ").and_then(emoji_team_eof),
        preceded(tag("FINAL "), separated_pair(u8, tag("-"), u8))
    ).map(|(away_team, home_team, (away_score, home_score))| 
        ParsedTeamFeedEventText::GameResult { home_team, away_team, home_score, away_score }
    )
}

fn photo_contest<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, tokens_earned) = preceded(tag("Earned "), u16).parse(input)?;
        let (input, _) = tag(" 🪙 in the Photo Contest.").parse(input)?;
        Ok((input, ParsedTeamFeedEventText::PhotoContest { tokens_earned }))
    }
}

fn prosperous<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, team) = parse_terminated(" are Prosperous! They earned ").parse(input)?;
        let (input, tokens_earned) = u16(input)?;
        let (input, _) = tag(" 🪙.").parse(input)?;

        let (_, team) = emoji_team_eof.parse(team)?;
        Ok((input, ParsedTeamFeedEventText::Prosperous { team, tokens_earned }))
    }
}

fn enchantment_s1a<'output>() -> impl TeamFeedEventParser<'output> {
    (
        parse_terminated("'s "),
        emojiless_item,
        preceded(tag(" was enchanted with +"), u8),
        delimited(tag(" to "), try_from_word, tag("."))
    ).map(|(player_name, item, amount, attribute)| ParsedTeamFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two: None, compensatory: false })
}

fn enchantment_s1b<'output>() -> impl TeamFeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        delimited(tag(" gained a +"), u8, tag(" ")),
        terminated(try_from_word, tag(" bonus."))
    ).map(|(player_name, item, amount, attribute)| ParsedTeamFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two: None, compensatory: false })
}

fn enchantment_s2<'output>() -> impl TeamFeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
        delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag(".")),
    ).map(|(player_name, item, (amount, attribute), enchant_two)| ParsedTeamFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two: Some(enchant_two), compensatory: false })
}

fn enchantment_compensatory<'output>() -> impl TeamFeedEventParser<'output> {
    (
        preceded(tag("The Compensatory Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        alt((
            (
                preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
                delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag("."))
            ).map(|((amount, attribute), second)| (amount, attribute, Some(second))),
            (
                delimited(tag(" gained a +"), separated_pair(u8, tag(" "), try_from_word), tag(" bonus."))
                .map(|(amount, attribute)| (amount, attribute, None))
            )
        ))
    ).map(|(player_name, item, (amount, attribute, enchant_two))| ParsedTeamFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two, compensatory: true })
}

fn attribute_gain<'output>() -> impl TeamFeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated(" gained +")),
            u8,
            delimited(tag(" "), try_from_word, tag("."))
        ).map(|(player_name, amount, attribute)| AttributeChange { player_name, amount: amount as i16, attribute })
    ).map(|changes| ParsedTeamFeedEventText::AttributeChanges { changes })
}

fn multiple_attribute_equal<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    |input| if event.after(Breakpoints::Season3) {
        (
            delimited(tag("Batters' "), try_from_word, tag(" was set to their ")),
            terminated(try_from_word, tag(". Lineup:")),
            separated_list1(
                tag(","),
                (
                    delimited(tag(" "), u8, tag(". ")),
                    terminated(try_from_word, tag(" ")),
                    take_while(|c| c != ',').and_then(name_eof)
                ).map(|(_, slot, name)| (Some(slot), name))
            )
        ).map(|(changing_attribute, value_attribute, players)| ParsedTeamFeedEventText::AttributeEquals { players, changing_attribute, value_attribute })
        .parse(input)
    } else {
        let f = |input| {
            if event.after(Breakpoints::S1AttributeEqualChange) {
                (
                    parse_terminated("'s "),
                    try_from_word,
                    delimited(tag(" became equal to their current base "), try_from_word, tag("."))
                ).parse(input)
            } else {
                (
                    parse_terminated("'s "),
                    try_from_word,
                    delimited(tag(" became equal to their base "), try_from_word, tag("."))
                ).parse(input)
            }
        };

        verify(
            separated_list1(tag(" "), f).map(|players| {
                let (_, changing_attribute, value_attribute) = players.first().expect("separated_list1 is never empty");
                (*changing_attribute, *value_attribute, players)
            }),
            |(changing_attribute, value_attribute, players)| players.iter().all(|(_, changing, value)| changing == changing_attribute && value == value_attribute)
        ).map(|(changing_attribute, value_attribute, players)| {
            ParsedTeamFeedEventText::AttributeEquals { players: players.into_iter().map(|(player, _, _)| (None, player)).collect(), changing_attribute, value_attribute }
        })
        .parse(input)
    }
}