use nom::{branch::alt, bytes::complete::tag, character::complete::{i16, u8}, combinator::{cond, fail, opt}, error::context, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use nom::character::complete::u32;
use crate::{enums::{CelestialEnergyTier, FeedEventType, ModificationType}, feed_event::{FeedEvent, FeedEventParseError, FeedFallingStarOutcome}, nom_parsing::shared::{emojiless_item, feed_delivery, name_eof, parse_terminated, sentence_eof, try_from_word}, player_feed::ParsedPlayerFeedEventText, time::{Breakpoints, Timestamp}};
use crate::feed_event::{GreaterAugment, PlayerGreaterAugment};
use super::shared::{door_prize, falling_star, feed_event_contained, feed_event_door_prize, feed_event_equipped_door_prize, feed_event_party, feed_event_wither, grow, player_moved, player_positions_swapped, player_relegated, purified, Error, IResult};


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
        FeedEventType::Election => election(event).parse(&event.text),
        FeedEventType::Roster => roster(event).parse(event.text.as_str()),
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
        feed_event_door_prize.map(|prize| ParsedPlayerFeedEventText::DoorPrize { prize }),
        feed_event_equipped_door_prize.map(|prize| ParsedPlayerFeedEventText ::DoorPrize { prize }),
        falling_star(event).map(|(player_name, outcome)| ParsedPlayerFeedEventText::FallingStarOutcome { player_name, outcome }),
        retirement(true),
        feed_event_wither.map(|player_name| ParsedPlayerFeedEventText::CorruptedByWither { player_name }),
        feed_event_party.map(|party| ParsedPlayerFeedEventText::Party { party }),
        feed_event_contained.map(|(contained_player_name, container_player_name)| ParsedPlayerFeedEventText::PlayerContained { contained_player_name, container_player_name }),
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
        purified.map(|(player_name, outcome)| ParsedPlayerFeedEventText::Purified { player_name, outcome }),
        player_positions_swapped.map(|swap| ParsedPlayerFeedEventText::PlayerPositionsSwapped { swap }),
        grow.map(|grow| ParsedPlayerFeedEventText::PlayerGrow { grow }),
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
        seasonal_durability_loss,
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

fn seasonal_durability_loss(input: &str) -> IResult<&str, ParsedPlayerFeedEventText<&str>> {
    alt((
        seasonal_durability_loss_happened,
        seasonal_durability_loss_blocked,
    ))
        .parse(input)
}

fn seasonal_durability_loss_happened(input: &str) -> IResult<&str, ParsedPlayerFeedEventText<&str>> {
    // This may need more intelligent parsing if " lost " is ever a player name substring
    let (input, player_name) = parse_terminated(" lost ").parse(input)?;
    let (input, durability_lost) = u32.parse(input)?;
    let (input, _) = tag(" durability for playing in Season ").parse(input)?;
    let (input, season) = u32.parse(input)?;
    let (input, _) = tag(".").parse(input)?;

    Ok((input, ParsedPlayerFeedEventText::SeasonalDurabilityLoss { player_name, durability_lost: Some(durability_lost), season }))
}

fn seasonal_durability_loss_blocked(input: &str) -> IResult<&str, ParsedPlayerFeedEventText<&str>> {
    let (input, player_name) = parse_terminated("'s Prolific Greater Boon resisted Durability loss for Season ").parse(input)?;
    let (input, season) = u32.parse(input)?;
    let (input, _) = tag(".").parse(input)?;

    Ok((input, ParsedPlayerFeedEventText::SeasonalDurabilityLoss { player_name, durability_lost: None, season }))
}

fn election<'output>(_event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    context("Election Feed Event", alt((
        player_greater_augment_result,
        player_retracted_greater_augment_result,
        player_retroactive_greater_augment_result,
    )))
}

fn player_greater_augment_result(input: &str) -> IResult<&str, ParsedPlayerFeedEventText<&str>> {
    let (input, (player_name, greater_augment)) = alt((
        parse_terminated(" gained +10 to all Defense Attributes.").map(|p| (p, PlayerGreaterAugment::Plating)),
        terminated((parse_terminated(" gained +75 "), try_from_word), tag(".")).map(|(p, attribute)| (p, PlayerGreaterAugment::Headliners { attribute })),
        terminated((parse_terminated(" gained +50 "), try_from_word), tag(".")).map(|(p, attribute)| (p, PlayerGreaterAugment::StartSmall { attribute })),
    )).parse(input)?;

    Ok((input, ParsedPlayerFeedEventText::GreaterAugment { player_name, greater_augment }))
}

// This is from when the s7 greater augments accidentally hit the demoted greater league players
// instead of the newly promoted players, and then Danny retroactively corrected them.
// The attribute numbers given here were accidentally in the 0-1 scale instead of the 0-100 scale
fn player_retracted_greater_augment_result(input: &str) -> IResult<&str, ParsedPlayerFeedEventText<&str>> {
    let (input, (player_name, greater_augment)) = alt((
        parse_terminated(" lost 0.1 from all Defense Attributes.").map(|p| (p, PlayerGreaterAugment::Plating)),
        terminated((parse_terminated(" lost 0.75 from "), try_from_word), tag(".")).map(|(p, attribute)| (p, PlayerGreaterAugment::Headliners { attribute })),
        terminated((parse_terminated(" lost 0.5 from "), try_from_word), tag(".")).map(|(p, attribute)| (p, PlayerGreaterAugment::StartSmall { attribute })),
    )).parse(input)?;

    Ok((input, ParsedPlayerFeedEventText::RetractedGreaterAugment { player_name, greater_augment }))
}

fn player_retroactive_greater_augment_result(input: &str) -> IResult<&str, ParsedPlayerFeedEventText<&str>> {
    let (input, (player_name, greater_augment)) = alt((
        parse_terminated(" gained +0.1 to all Defense Attributes.").map(|p| (p, PlayerGreaterAugment::Plating)),
        terminated((parse_terminated(" gained +0.75 to "), try_from_word), tag(".")).map(|(p, attribute)| (p, PlayerGreaterAugment::Headliners { attribute })),
        terminated((parse_terminated(" gained +0.5 to "), try_from_word), tag(".")).map(|(p, attribute)| (p, PlayerGreaterAugment::StartSmall { attribute })),
    )).parse(input)?;

    Ok((input, ParsedPlayerFeedEventText::RetroactiveGreaterAugment { player_name, greater_augment }))
}

fn roster<'output>(_event: &'output FeedEvent) -> impl PlayerFeedEventParser<'output> {
    context("Roster Feed Event", alt((
        player_relegated.map(|player_name| ParsedPlayerFeedEventText::PlayerRelegated { player_name }),
        player_moved.map(|(team_emoji, player_name)| ParsedPlayerFeedEventText::PlayerMoved { team_emoji, player_name }),
    )))
}