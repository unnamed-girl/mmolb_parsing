use nom::{branch::alt, bytes::complete::{tag, take_while}, character::complete::{i16, u8}, combinator::{fail, opt, verify}, error::context, multi::{many1, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use tracing::error;
use crate::{enums::{CelestialEnergyTier, FeedEventType}, feed_event::{AttributeChange, FeedEvent, FeedEventParseError, ParsedFeedEventText}, nom_parsing::shared::{emoji_team_eof, emojiless_item, feed_delivery, name_eof, parse_terminated, sentence_eof, try_from_word, try_from_words_m_n}, time::{Breakpoints, Timestamp}};

use super::shared::Error;

trait FeedEventParser<'output>: Parser<&'output str, Output = ParsedFeedEventText<&'output str>, Error = Error<'output>> {}
impl<'output, T: Parser<&'output str, Output = ParsedFeedEventText<&'output str>, Error = Error<'output>>> FeedEventParser<'output> for T {}


pub fn parse_feed_event<'output>(event: &'output FeedEvent) -> ParsedFeedEventText<&'output str> {
    let event_type = match &event.event_type {
        Ok(event_type) => event_type,
        Err(e) => {
            let error = FeedEventParseError::EventTypeNotRecognized(e.clone());
            return ParsedFeedEventText::ParseError { error, text: &event.text };
        }
    };

    let result = match event_type {
        FeedEventType::Game => game(event).parse(&event.text),
        FeedEventType::Augment => augment(event).parse(&event.text),
        FeedEventType::Release => release().parse(&event.text),
        FeedEventType::Season => fail().parse(event.text.as_str())
    };
    match result.finish() {
        Ok(("", output)) => output,
        Ok((leftover, _)) => {
            error!("{event_type} feed event parsed had leftover: {leftover} from {}", &event.text);
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            ParsedFeedEventText::ParseError { error, text: &event.text }
        }
        Err(_) => {
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            tracing::error!("Parse error: {}", error);
            ParsedFeedEventText::ParseError { error, text: &event.text }
        }
    }
}

fn game<'output>(event: &'output FeedEvent) -> impl FeedEventParser<'output> {
    context("Game Feed Event", alt((
        game_result(),
        feed_delivery("Delivery").map(|delivery| ParsedFeedEventText::Delivery { delivery } ),
        feed_delivery("Shipment").map(|delivery| ParsedFeedEventText::Shipment { delivery } ),
        feed_delivery("Special Delivery").map(|delivery| ParsedFeedEventText::SpecialDelivery { delivery } ),
        prosperous(),
        retirement(),
        injured_by_falling_star(event),
        infused_by_falling_star(),
    )))
}

fn augment<'output>(event: &'output FeedEvent) -> impl FeedEventParser<'output> {
    context("Augment Feed Event", alt((
        attribute_gain(),
        enchantment_s1a(),
        enchantment_s1b(),
        enchantment_s2(),
        enchantment_compensatory(),
        modification(),
        take_the_mound(),
        take_the_plate(),
        multiple_attribute_equal(event),
        single_attribute_equal(event),
        swap_places(),
        recompose(event)
    )))
}

fn release<'output>() -> impl FeedEventParser<'output> {
    context("Release Feed Event", 
        preceded(tag("Released by the "), sentence_eof(name_eof)).map(|team| ParsedFeedEventText::Released { team })
    )
}

fn injured_by_falling_star<'output>(event: &'output FeedEvent) -> impl FeedEventParser<'output> {
    |input|
        if event.after(Breakpoints::EternalBattle) {
            parse_terminated(" was injured by the extreme force of the impact!")
                .and_then(name_eof)
                .map(|player| ParsedFeedEventText::InjuredByFallingStar { player })
                .parse(input)
        } else {
            parse_terminated(" was hit by a Falling Star!")
                .and_then(name_eof)
                .map(|player| ParsedFeedEventText::InjuredByFallingStar { player })
                .parse(input)   
        }
}

fn infused_by_falling_star<'output>() -> impl FeedEventParser<'output> {
    alt((
        parse_terminated(" began to glow brightly with celestial energy!").and_then(name_eof).map(|player| (player, CelestialEnergyTier::BeganToGlow)),
        parse_terminated(" was infused with a glimmer of celestial energy!").and_then(name_eof).map(|player| (player, CelestialEnergyTier::Infused)),
        parse_terminated(" was fully charged with an abundance of celestial energy!").and_then(name_eof).map(|player| (player, CelestialEnergyTier::FullyCharged))
    ))
    .map(|(player, infusion_tier)| ParsedFeedEventText::InfusedByFallingStar { player, infusion_tier })
}

fn prosperous<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" are Prosperous! They earned ").and_then(emoji_team_eof),
        terminated(u8, tag(" 🪙."))
    ).map(|(team, income)| ParsedFeedEventText::Prosperous { team, income })
}

fn retirement<'output>() -> impl FeedEventParser<'output> {
    (
        preceded(tag("😇 "), parse_terminated(" retired from MMOLB!").and_then(name_eof)),
        opt(preceded(tag(" "), parse_terminated(" was called up to take their place.").and_then(name_eof)))
    ).map(|(original, new)| ParsedFeedEventText::Retirement { previous: original, new })
}

fn game_result<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" vs. ").and_then(emoji_team_eof),
        parse_terminated(" - ").and_then(emoji_team_eof),
        preceded(tag("FINAL "), separated_pair(u8, tag("-"), u8))
    ).map(|(away_team, home_team, (away_score, home_score))| 
        ParsedFeedEventText::GameResult { home_team, away_team, home_score, away_score }
    )
}

fn recompose<'output>(event: &'output FeedEvent) -> impl FeedEventParser<'output> {
    |input: &'output str|
    if event.timestamp > Timestamp::Season3RecomposeChange.timestamp() {
        (
            parse_terminated(" was Recomposed into "),
            sentence_eof(name_eof)
        ).map(|(original, new)| ParsedFeedEventText::Recomposed { previous: original, new })
        .parse(input)
    } else {
        (
            parse_terminated(" was Recomposed using "),
            sentence_eof(name_eof)
        ).map(|(original, new)| ParsedFeedEventText::Recomposed { previous: original, new })
        .parse(input)
    }
}

fn attribute_gain<'output>() -> impl FeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated(" gained +")),
            i16,
            delimited(tag(" "), try_from_word, tag("."))
        ).map(|(player_name, amount, attribute)| AttributeChange { player_name, amount, attribute })
    ).map(|changes| ParsedFeedEventText::AttributeChanges { changes })
}

fn single_attribute_equal<'output>(event: &'output FeedEvent) -> impl FeedEventParser<'output> {
    |input| if event.after(Breakpoints::Season3) {
        (
            parse_terminated("'s "),
            try_from_word,
            delimited(tag(" was set to their "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| ParsedFeedEventText::SingleAttributeEquals { player_name, changing_attribute, value_attribute })
        .parse(input)
    } else if event.after(Breakpoints::S1AttributeEqualChange) {
        (
            parse_terminated("'s "),
            try_from_word,
            delimited(tag(" became equal to their current base "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| ParsedFeedEventText::SingleAttributeEquals { player_name, changing_attribute, value_attribute })
        .parse(input)
    } else {
        (
            parse_terminated("'s "),
            try_from_word,
            delimited(tag(" was set to their "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| ParsedFeedEventText::SingleAttributeEquals { player_name, changing_attribute, value_attribute })
        .parse(input)
    }
}

fn multiple_attribute_equal<'output>(event: &'output FeedEvent) -> impl FeedEventParser<'output> {
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
        ).map(|(changing_attribute, value_attribute, players)| ParsedFeedEventText::MassAttributeEquals { players, changing_attribute, value_attribute })
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
            ParsedFeedEventText::MassAttributeEquals { players: players.into_iter().map(|(player, _, _)| (None, player)).collect(), changing_attribute, value_attribute }
        })
        .parse(input)
    }
}

fn enchantment_s1a<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated("'s "),
        emojiless_item,
        preceded(tag(" was enchanted with +"), u8),
        delimited(tag(" to "), try_from_word, tag("."))
    ).map(|(player_name, item, amount, attribute)| ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute })
}

fn enchantment_s1b<'output>() -> impl FeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        delimited(tag(" gained a +"), u8, tag(" ")),
        terminated(try_from_word, tag(" bonus."))
    ).map(|(player_name, item, amount, attribute)| ParsedFeedEventText::S1Enchantment { player_name, item, amount, attribute })
}

fn enchantment_s2<'output>() -> impl FeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
        delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag(".")),
    ).map(|(player_name, item, (amount, attribute), enchant_two)| ParsedFeedEventText::S2Enchantment { player_name, item, amount, attribute, enchant_two: Some(enchant_two), compensatory: false })
}

fn enchantment_compensatory<'output>() -> impl FeedEventParser<'output> {
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
    ).map(|(player_name, item, (amount, attribute, enchant_two))| ParsedFeedEventText::S2Enchantment { player_name, item, amount, attribute, enchant_two, compensatory: true })
}

fn modification<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" gained the "),
        terminated(try_from_words_m_n(1, 2), tag(" Modification.")),
    )
        .map(|(player_name, modification)| ParsedFeedEventText::Modification { player_name, modification })
}

fn take_the_mound<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" was moved to the mound. "),
        parse_terminated(" was sent to the lineup."),
    )
        .map(|(to_mound_player, to_lineup_player)| ParsedFeedEventText::TakeTheMound { to_mound_player, to_lineup_player })
}

fn take_the_plate<'output>() -> impl FeedEventParser<'output> {
    (
        parse_terminated(" was sent to the plate. "),
        parse_terminated(" was pulled from the lineup."),
    )
        .map(|(to_plate_player, from_lineup_player)| ParsedFeedEventText::TakeThePlate { to_plate_player, from_lineup_player })
}

fn swap_places<'output>() -> impl FeedEventParser<'output> {
    sentence_eof((
        parse_terminated(" swapped places with "),
        name_eof
    ))
    .map(|(player_one, player_two)| ParsedFeedEventText::SwapPlaces { player_one, player_two })
}

#[cfg(test)]
mod test {
    use nom::Parser;

    use crate::{enums::Attribute, feed_event::{AttributeChange, ParsedFeedEventText}, nom_parsing::parse_feed_event::{attribute_gain, game_result}, parsed_event::EmojiTeam};

    #[test]
    fn test_attribute_gain() {
        assert_eq!(
            Ok(ParsedFeedEventText::AttributeChanges { changes: vec![AttributeChange { player_name: "Nancy Bright", amount: 50, attribute: Attribute::Awareness}] }),
            attribute_gain().parse("Nancy Bright gained +50 Awareness.").map(|(_, o)| o).map_err(|e| e.to_string())
        );
    }

    #[test]
    fn test_game_result() {
        let s = "🦖 Peoria Monster Monster Monster vs. 📮 Akron Anteaters Pace Stick - FINAL 2-4";
        assert_eq!(
            Ok(ParsedFeedEventText::GameResult { away_team: EmojiTeam {emoji: "🦖", name: "Peoria Monster Monster Monster"}, home_team: EmojiTeam { emoji: "📮", name: "Akron Anteaters Pace Stick" }, away_score: 2, home_score: 4 }),
            game_result().parse(s).map(|(_, o)| o).map_err(|e| e.to_string())
        );
    }
}