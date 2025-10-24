use nom::{branch::alt, bytes::complete::tag, character::complete::{i16, u8, u32}, combinator::{cond, fail, opt}, error::context, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use nom::bytes::complete::take_while;
use nom::combinator::verify;
use nom::multi::{many1, separated_list1};
use crate::{enums::{CelestialEnergyTier, FeedEventType, ModificationType}, feed_event::{FeedEvent, FeedEventParseError, FeedFallingStarOutcome}, nom_parsing::shared::{emojiless_item, feed_delivery, name_eof, parse_terminated, sentence_eof, try_from_word}, team_feed::ParsedTeamFeedEventText, time::{Breakpoints, Timestamp}};
use crate::feed_event::{AttributeChange, ParsedFeedEventText};
use crate::parsed_event::EmojiPlayer;
use super::shared::{emoji, emoji_team_eof, emoji_team_eof_maybe_no_space, feed_event_door_prize, feed_event_party, Error};


trait TeamFeedEventParser<'output>: Parser<&'output str, Output = ParsedTeamFeedEventText<&'output str>, Error = Error<'output>> {}
impl<'output, T: Parser<&'output str, Output = ParsedTeamFeedEventText<&'output str>, Error = Error<'output>>> TeamFeedEventParser<'output> for T {}


pub fn parse_team_feed_event(event: &FeedEvent) -> ParsedTeamFeedEventText<&str> {
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
        FeedEventType::Season => season(event).parse(event.text.as_str()),
        FeedEventType::Lottery => lottery().parse(event.text.as_str()),
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

fn game(event: &FeedEvent) -> impl TeamFeedEventParser {
    context("Game Feed Event", alt((
        game_result(),
        feed_delivery("Delivery").map(|delivery| ParsedTeamFeedEventText::Delivery { delivery } ),
        feed_delivery("Shipment").map(|delivery| ParsedTeamFeedEventText::Shipment { delivery } ),
        feed_delivery("Special Delivery").map(|delivery| ParsedTeamFeedEventText::SpecialDelivery { delivery } ),
        photo_contest(),
        injured_by_falling_star(event),
        infused_by_falling_star(),
        deflected_falling_star_harmlessly(),
        feed_event_party.map(|party| ParsedTeamFeedEventText::Party { party }),
        feed_event_door_prize.map(|prize| ParsedTeamFeedEventText::DoorPrize { prize }),
        prosperous(),
        // retirement(true),
        wither(),
        fail(),
    )))
}

fn augment(event: &FeedEvent) -> impl TeamFeedEventParser {
    context("Augment Feed Event", alt((
        attribute_gain(),
        modification(),
        enchantment_s1a(),
        // enchantment_s1b(),
        // enchantment_s2(),
        enchantment_compensatory(),
        multiple_attribute_equal(event),
        recompose(event),
        // take_the_mound(),
        take_the_plate(),
        swap_places(),
        purified(),
        fail(),
    )))
}

fn game_result<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, away_team) = parse_terminated(" vs. ").and_then(emoji_team_eof_maybe_no_space).parse(input)?;
        let (input, home_team) = parse_terminated(" - FINAL ").and_then(emoji_team_eof).parse(input)?;
        let (input, away_score) = u8.parse(input)?;
        let (input, _) = tag("-").parse(input)?;
        let (input, home_score) = u8.parse(input)?;

        Ok((input, ParsedTeamFeedEventText::GameResult {
            home_team,
            away_team,
            home_score,
            away_score,
        }))
    }
}

fn photo_contest<'output>() -> impl TeamFeedEventParser<'output> {
    alt((
        photo_contest_without_name(),
        photo_contest_with_name()
    ))
}

fn photo_contest_without_name<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, _) = tag("Earned ").parse(input)?;
        let (input, earned_coins) = u32.parse(input)?;
        let (input, _) = tag(" 🪙 in the Photo Contest.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::PhotoContest { player: None, earned_coins }))
    }
}

fn photo_contest_with_name<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, emoji) = emoji.parse(input)?;
        let (input, _) = tag(" ").parse(input)?;
        let (input, name) = parse_terminated(" won ").parse(input)?;
        let (input, earned_coins) = u32.parse(input)?;
        let (input, _) = tag(" 🪙 in a Photo Contest.").parse(input)?;

        let player = Some(EmojiPlayer { emoji, name });
        Ok((input, ParsedTeamFeedEventText::PhotoContest { player, earned_coins }))
    }
}

fn prosperous<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, player_name) = parse_terminated(" was Corrupted by the 🥀 Wither.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::CorruptedByWither { player_name }))
    }
}

fn purified<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, player_name) = parse_terminated(" was Purified of 🫀 Corruption and earned ").parse(input)?;
        let (input, payment) = u32.parse(input)?;
        let (input, _) = tag(" 🪙.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::Purified { player_name, payment }))
    }
}

fn wither<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, team_emoji_str) = parse_terminated(" are Prosperous! They ").parse(input)?;
        let (input, _) = alt((tag("earned "), tag("earn "))).parse(input)?;
        let (_, team) = emoji_team_eof.parse(team_emoji_str)?;
        let (input, income) = u8.parse(input)?;
        let (input, _) = tag(" 🪙.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::Prosperous { team, income }))
    }
}

fn release<'output>(_event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Release Feed Event", alt((
        preceded(tag("Released by the "), sentence_eof(name_eof)).map(|team| ParsedTeamFeedEventText::Released { team }),
    )))
}

fn season<'output>(_event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    context("Season Feed Event", alt((
        retirement(false),
    )))
}

fn lottery<'output>() -> impl TeamFeedEventParser<'output> {
    context("Lottery Feed Event", alt((
        lottery_event(),
    )))
}

fn lottery_event<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, _) = tag("The ")(input)?;
        let (input, team_name) = parse_terminated(" donated ")(input)?;
        let (input, amount) = u32.parse(input)?;
        let (input, _) = tag(" 🪙 to the ")(input)?;
        let (input, league_name) = parse_terminated(" Lottery.")(input)?;

        Ok((input, ParsedTeamFeedEventText::DonatedToLottery { team_name, amount, league_name }))
    }
}

fn attribute_gain<'output>() -> impl TeamFeedEventParser<'output> {
    many1(
        (
            preceded(opt(tag(" ")), parse_terminated(" gained +")),
            i16,
            delimited(tag(" "), try_from_word, tag("."))
        ).map(|(player_name, amount, attribute)| AttributeChange { player_name, amount, attribute })
    ).map(|changes| ParsedTeamFeedEventText::AttributeChanges { changes })
}

fn multiple_attribute_equal(event: &FeedEvent) -> impl TeamFeedEventParser {
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
        ).map(|(changing_attribute, value_attribute, players)| ParsedTeamFeedEventText::MassAttributeEquals { players, changing_attribute, value_attribute })
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
            ParsedTeamFeedEventText::MassAttributeEquals { players: players.into_iter().map(|(player, _, _)| (None, player)).collect(), changing_attribute, value_attribute }
        })
            .parse(input)
    }
}

// TODO Dedup all falling star functions between team and player
fn injured_by_falling_star<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    |input|
        if event.after(Breakpoints::EternalBattle) {
            parse_terminated(" was injured by the extreme force of the impact!")
                .and_then(name_eof)
                .map(|team_name| ParsedTeamFeedEventText::FallingStarOutcome { team_name, outcome: FeedFallingStarOutcome::Injury })
                .parse(input)
        } else {
            parse_terminated(" was hit by a Falling Star!")
                .and_then(name_eof)
                .map(|team_name| ParsedTeamFeedEventText::FallingStarOutcome { team_name, outcome: FeedFallingStarOutcome::Injury })
                .parse(input)
        }
}

fn infused_by_falling_star<'output>() -> impl TeamFeedEventParser<'output> {
    alt((
        parse_terminated(" began to glow brightly with celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::BeganToGlow)),
        parse_terminated(" was infused with a glimmer of celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::Infused)),
        parse_terminated(" was fully charged with an abundance of celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::FullyCharged))
    ))
    .map(|(team_name, infusion_tier)| ParsedTeamFeedEventText::FallingStarOutcome { team_name, outcome: FeedFallingStarOutcome::Infusion(infusion_tier) })
}

fn deflected_falling_star_harmlessly<'output>() -> impl TeamFeedEventParser<'output> {
    preceded(
        tag("It deflected off "),
        parse_terminated(" harmlessly.").and_then(name_eof)
    )
    .map(|team_name| ParsedTeamFeedEventText::FallingStarOutcome { team_name, outcome: FeedFallingStarOutcome::DeflectedHarmlessly })
}

fn recompose<'output>(event: &'output FeedEvent) -> impl TeamFeedEventParser<'output> {
    |input: &'output str|
    if event.timestamp > Timestamp::Season3RecomposeChange.timestamp() {
        (
            parse_terminated(" was Recomposed into "),
            sentence_eof(name_eof)
        ).map(|(original, new)| ParsedTeamFeedEventText::Recomposed { previous: original, new })
        .parse(input)
    } else {
        (
            parse_terminated(" was Recomposed using "),
            sentence_eof(name_eof)
        ).map(|(original, new)| ParsedTeamFeedEventText::Recomposed { previous: original, new })
        .parse(input)
    }
}

fn enchantment_s1a<'output>() -> impl TeamFeedEventParser<'output> {
    (
        parse_terminated("'s "),
        emojiless_item,
        preceded(tag(" was enchanted with +"), u8),
        delimited(tag(" to "), try_from_word, tag("."))
    ).map(|(team_name, item, amount, attribute)| ParsedTeamFeedEventText::Enchantment { team_name, item, amount, attribute, enchant_two: None, compensatory: false })
}

fn enchantment_s1b<'output>() -> impl TeamFeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        delimited(tag(" gained a +"), u8, tag(" ")),
        terminated(try_from_word, tag(" bonus."))
    ).map(|(team_name, item, amount, attribute)| ParsedTeamFeedEventText::Enchantment { team_name, item, amount, attribute, enchant_two: None, compensatory: false })
}

fn enchantment_s2<'output>() -> impl TeamFeedEventParser<'output> {
    (
        preceded(tag("The Item Enchantment was a success! "), parse_terminated("'s ")),
        emojiless_item,
        preceded((tag(" was enchanted with "), opt(tag("a ")) , tag("+")), separated_pair(u8, tag(" "), try_from_word)),
        delimited(tag(" and +"), separated_pair(u8, tag(" "), try_from_word), tag(".")),
    ).map(|(team_name, item, (amount, attribute), enchant_two)| ParsedTeamFeedEventText::Enchantment { team_name, item, amount, attribute, enchant_two: Some(enchant_two), compensatory: false })
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
    ).map(|(team_name, item, (amount, attribute, enchant_two))| ParsedTeamFeedEventText::Enchantment { team_name, item, amount, attribute, enchant_two, compensatory: true })
}

fn take_the_mound<'output>() -> impl TeamFeedEventParser<'output> {
    (
        parse_terminated(" was moved to the mound. "),
        parse_terminated(" was sent to the lineup."),
    )
        .map(|(to_mound_team, to_lineup_team)| ParsedTeamFeedEventText::TakeTheMound { to_mound_team, to_lineup_team })
}

fn take_the_plate<'output>() -> impl TeamFeedEventParser<'output> {
    (
        parse_terminated(" was sent to the plate. "),
        parse_terminated(" was pulled from the lineup."),
    )
        .map(|(to_plate_team, from_lineup_team)| ParsedTeamFeedEventText::TakeThePlate { to_plate_team, from_lineup_team })
}

fn swap_places<'output>() -> impl TeamFeedEventParser<'output> {
    sentence_eof((
        parse_terminated(" swapped places with "),
        name_eof
    ))
    .map(|(team_one, team_two)| ParsedTeamFeedEventText::SwapPlaces { team_one, team_two })
}

fn modification<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        if let Ok((input, team_name)) = (parse_terminated(" lost the ")).parse(input) {
            let (_, team_name) = name_eof(team_name)?;
            let (input, lost_modification) = parse_terminated(" Modification. ").map(ModificationType::new).parse(input)?;
            let (input, _) = (tag(team_name), tag(" gained the ")).parse(input)?;
            let (input, modification) = parse_terminated(" Modification.").map(ModificationType::new).parse(input)?;
            Ok((input, ParsedTeamFeedEventText::Modification { team_name, modification, lost_modification: Some(lost_modification) }))
        } else {
            let (input, (team_name, modification)) = (   
                parse_terminated(" gained the "),
                parse_terminated(" Modification.").map(ModificationType::new),
            )
            .parse(input)?;

            Ok((input, ParsedTeamFeedEventText::Modification { team_name, modification, lost_modification: None }))
        }
    }
}

fn retirement<'output>(emoji: bool) -> impl TeamFeedEventParser<'output> {
    (
        preceded(cond(emoji, tag("😇 ")), parse_terminated(" retired from MMOLB!").and_then(name_eof)),
        opt(preceded(tag(" "), parse_terminated(" was called up to take their place.").and_then(name_eof)))
    ).map(|(original, new)| ParsedTeamFeedEventText::Retirement { previous: original, new })
}