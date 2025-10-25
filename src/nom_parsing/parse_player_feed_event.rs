use nom::{branch::alt, bytes::complete::tag, character::complete::{i16, u8}, combinator::{cond, fail, opt}, error::context, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use crate::{enums::{CelestialEnergyTier, FeedEventType, ModificationType}, feed_event::{FeedEvent, FeedEventParseError, FeedFallingStarOutcome}, nom_parsing::shared::{emojiless_item, feed_delivery, name_eof, parse_terminated, sentence_eof, try_from_word}, player_feed::ParsedPlayerFeedEventText, time::{Breakpoints, Timestamp}};

use super::shared::Error;


trait PlayerFeedEventParser<'output>: Parser<&'output str, Output = ParsedPlayerFeedEventText<&'output str>, Error = Error<'output>> {}
impl<'output, T: Parser<&'output str, Output = ParsedPlayerFeedEventText<&'output str>, Error = Error<'output>>> PlayerFeedEventParser<'output> for T {}


pub fn parse_player_feed_event<'output>(event: &'output FeedEvent) -> ParsedPlayerFeedEventText<&'output str> {
    let event_type = match &event.event_type {
        Ok(event_type) => event_type,
        Err(e) => {
            let error = FeedEventParseError::EventTypeNotRecognized(e.clone());
            return ParsedPlayerFeedEventText::ParseError { error, text: &event.text };
        }
    };

    let result = match event_type {
        FeedEventType::Game => game(event).parse(&event.text),
        FeedEventType::Augment => augment(event).parse(&event.text),
        FeedEventType::Release => release(event).parse(&event.text),
        FeedEventType::Season => season(event).parse(event.text.as_str()),
        // TODO More descriptive error message
        FeedEventType::Lottery => fail().parse(event.text.as_str()),
        FeedEventType::Maintenance => fail().parse(event.text.as_str()),
    };
    match result.finish() {
        Ok(("", output)) => output,
        Ok((leftover, _)) => {
            tracing::error!("{event_type} feed event parsed had leftover: {leftover} from {}", &event.text);
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            ParsedPlayerFeedEventText::ParseError { error, text: &event.text }
        }
        Err(e) => {
            let error = FeedEventParseError::FailedParsingText { event_type: *event_type, text: event.text.clone() };
            tracing::error!("Parse error: {e:?}");
            ParsedPlayerFeedEventText::ParseError { error, text: &event.text }
        }
    }
}

fn game<'output>(event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    context("Game Feed Event", alt((
        feed_delivery("Delivery").map(|delivery| ParsedPlayerFeedEventText::Delivery { delivery } ),
        feed_delivery("Shipment").map(|delivery| ParsedPlayerFeedEventText::Shipment { delivery } ),
        feed_delivery("Special Delivery").map(|delivery| ParsedPlayerFeedEventText::SpecialDelivery { delivery } ),
        injured_by_falling_star(event),
        infused_by_falling_star(),
        deflected_falling_star_harmlessly(),
        retirement(true),
        fail(),
    )))
}

fn augment<'output>(event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    context("Augment Feed Event", alt((
        attribute_gain(),
        modification(),
        enchantment_s1a(),
        enchantment_s1b(),
        enchantment_s2(),
        enchantment_compensatory(),
        attribute_equal(event),
        recompose(event),
        take_the_mound(),
        take_the_plate(),
        swap_places(),
        fail(),
    )))
}

fn release<'output>(_event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    context("Release Feed Event", alt((
        preceded(tag("Released by the "), sentence_eof(name_eof)).map(|team| ParsedPlayerFeedEventText::Released { team }),
    )))
}

fn season<'output>(_event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    context("Season Feed Event", alt((
        retirement(false),
    )))
}

fn attribute_gain<'output>() -> impl PlayerFeedEventParser<'output> {
    (
        preceded(opt(tag(" ")), parse_terminated(" gained +")),
        i16,
        delimited(tag(" "), try_from_word, tag("."))
    ).map(|(player_name, amount, attribute)| ParsedPlayerFeedEventText::AttributeChanges { player_name, amount, attribute })
}

fn attribute_equal<'output>(event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    |input| if event.after(Breakpoints::Season3) {
        (
            parse_terminated("'s "),
            try_from_word,
            delimited(tag(" was set to their "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| ParsedPlayerFeedEventText::AttributeEquals { player_name, changing_attribute, value_attribute })
        .parse(input)
    } else if event.after(Breakpoints::S1AttributeEqualChange) {
        (
            parse_terminated("'s "),
            try_from_word,
            delimited(tag(" became equal to their current base "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| ParsedPlayerFeedEventText::AttributeEquals { player_name, changing_attribute, value_attribute })
        .parse(input)
    } else {
        (
            parse_terminated("'s "),
            try_from_word,
            delimited(tag(" was set to their "), try_from_word, tag("."))
        ).map(|(player_name, changing_attribute, value_attribute)| ParsedPlayerFeedEventText::AttributeEquals { player_name, changing_attribute, value_attribute })
        .parse(input)
    }
}

fn injured_by_falling_star<'output>(event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    |input|
        if event.after(Breakpoints::EternalBattle) {
            parse_terminated(" was injured by the extreme force of the impact!")
                .and_then(name_eof)
                .map(|player_name| ParsedPlayerFeedEventText::FallingStarOutcome { player_name, outcome: FeedFallingStarOutcome::Injury })
                .parse(input)
        } else {
            parse_terminated(" was hit by a Falling Star!")
                .and_then(name_eof)
                .map(|player_name| ParsedPlayerFeedEventText::FallingStarOutcome { player_name, outcome: FeedFallingStarOutcome::Injury })
                .parse(input)
        }
}

fn infused_by_falling_star<'output>() -> impl PlayerFeedEventParser<'output> {
    alt((
        parse_terminated(" began to glow brightly with celestial energy!").and_then(name_eof).map(|player| (player, CelestialEnergyTier::BeganToGlow)),
        parse_terminated(" was infused with a glimmer of celestial energy!").and_then(name_eof).map(|player| (player, CelestialEnergyTier::Infused)),
        parse_terminated(" was fully charged with an abundance of celestial energy!").and_then(name_eof).map(|player| (player, CelestialEnergyTier::FullyCharged))
    ))
    .map(|(player_name, infusion_tier)| ParsedPlayerFeedEventText::FallingStarOutcome { player_name, outcome: FeedFallingStarOutcome::Infusion(infusion_tier) })
}

fn deflected_falling_star_harmlessly<'output>() -> impl PlayerFeedEventParser<'output> {
    preceded(
        tag("It deflected off "),
        parse_terminated(" harmlessly.").and_then(name_eof)
    )
    .map(|player_name| ParsedPlayerFeedEventText::FallingStarOutcome { player_name, outcome: FeedFallingStarOutcome::DeflectedHarmlessly })
}

fn recompose<'output>(event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    |input: &'output str|
    if event.timestamp > Timestamp::Season3RecomposeChange.timestamp() {
        (
            parse_terminated(" was Recomposed into "),
            sentence_eof(name_eof)
        ).map(|(original, new)| ParsedPlayerFeedEventText::Recomposed { previous: original, new })
        .parse(input)
    } else {
        (
            parse_terminated(" was Recomposed using "),
            sentence_eof(name_eof)
        ).map(|(original, new)| ParsedPlayerFeedEventText::Recomposed { previous: original, new })
        .parse(input)
    }
}

fn enchantment_s1a<'output>() -> impl PlayerFeedEventParser<'output> {
    (
        parse_terminated("'s "),
        emojiless_item,
        preceded(tag(" was enchanted with +"), u8),
        delimited(tag(" to "), try_from_word, tag("."))
    ).map(|(player_name, item, amount, attribute)| ParsedPlayerFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two: None, compensatory: false })
}

fn enchantment_s1b<'output>() -> impl PlayerFeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        delimited(tag(" gained a +"), u8, tag(" ")),
        terminated(try_from_word, tag(" bonus."))
    ).map(|(player_name, item, amount, attribute)| ParsedPlayerFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two: None, compensatory: false })
}

fn enchantment_s2<'output>() -> impl PlayerFeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
        delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag(".")),
    ).map(|(player_name, item, (amount, attribute), enchant_two)| ParsedPlayerFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two: Some(enchant_two), compensatory: false })
}

fn enchantment_compensatory<'output>() -> impl PlayerFeedEventParser<'output> {
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
    ).map(|(player_name, item, (amount, attribute, enchant_two))| ParsedPlayerFeedEventText::Enchantment { player_name, item, amount, attribute, enchant_two, compensatory: true })
}

fn take_the_mound<'output>() -> impl PlayerFeedEventParser<'output> {
    (
        parse_terminated(" was moved to the mound. "),
        parse_terminated(" was sent to the lineup."),
    )
        .map(|(to_mound_player, to_lineup_player)| ParsedPlayerFeedEventText::TakeTheMound { to_mound_player, to_lineup_player })
}

fn take_the_plate<'output>() -> impl PlayerFeedEventParser<'output> {
    (
        parse_terminated(" was sent to the plate. "),
        parse_terminated(" was pulled from the lineup."),
    )
        .map(|(to_plate_player, from_lineup_player)| ParsedPlayerFeedEventText::TakeThePlate { to_plate_player, from_lineup_player })
}

fn swap_places<'output>() -> impl PlayerFeedEventParser<'output> {
    sentence_eof((
        parse_terminated(" swapped places with "),
        name_eof
    ))
    .map(|(player_one, player_two)| ParsedPlayerFeedEventText::SwapPlaces { player_one, player_two })
}

fn modification<'output>() -> impl PlayerFeedEventParser<'output> {
    |input| {
        if let Ok((input, player_name)) = (parse_terminated(" lost the ")).parse(input) {
            let (_, player_name) = name_eof(player_name)?;
            let (input, lost_modification) = parse_terminated(" Modification. ").map(ModificationType::new).parse(input)?;
            let (input, _) = (tag(player_name), tag(" gained the ")).parse(input)?;
            let (input, modification) = parse_terminated(" Modification.").map(ModificationType::new).parse(input)?;
            Ok((input, ParsedPlayerFeedEventText::Modification { player_name, modification, lost_modification: Some(lost_modification) }))
        } else {
            let (input, (player_name, modification)) = (   
                parse_terminated(" gained the "),
                parse_terminated(" Modification.").map(ModificationType::new),
            )
            .parse(input)?;

            Ok((input, ParsedPlayerFeedEventText::Modification { player_name, modification, lost_modification: None }))
        }
    }
}

fn retirement<'output>(emoji: bool) -> impl PlayerFeedEventParser<'output> {
    (
        preceded(cond(emoji, tag("😇 ")), parse_terminated(" retired from MMOLB!").and_then(name_eof)),
        opt(preceded(tag(" "), parse_terminated(" was called up to take their place.").and_then(name_eof)))
    ).map(|(original, new)| ParsedPlayerFeedEventText::Retirement { previous: original, new })
}