use nom::{branch::alt, bytes::complete::tag, character::complete::{i16, u8, u32}, combinator::{cond, fail, opt}, error::context, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use nom::bytes::complete::take_while;
use nom::combinator::{eof, verify};
use nom::multi::{many1, separated_list1};
use nom::number::double;
use crate::{enums::{CelestialEnergyTier, FeedEventType, ModificationType}, feed_event::{FeedEvent, FeedEventParseError, FeedFallingStarOutcome}, nom_parsing::shared::{emojiless_item, feed_delivery, name_eof, parse_terminated, sentence_eof, try_from_word}, team_feed::ParsedTeamFeedEventText, time::{Breakpoints, Timestamp}};
use crate::enums::{BenchSlot, FullSlot, Slot};
use crate::feed_event::{AttributeChange, BenchImmuneModGranted, GrowAttributeChange};
use crate::parsed_event::{EmojiPlayer, EmojiTeam};
use crate::team_feed::PurifiedOutcome;
use super::shared::{emoji, emoji_team_eof, emoji_team_eof_maybe_no_space, feed_event_door_prize, feed_event_equipped_door_prize, feed_event_party, parse_until_period_eof, team_emoji, Error, IResult};


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
        FeedEventType::Maintenance => maintenance().parse(event.text.as_str()),
        FeedEventType::Roster => roster().parse(event.text.as_str()),
        FeedEventType::Election => election().parse(event.text.as_str()),
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
        feed_event_equipped_door_prize.map(|prize| ParsedTeamFeedEventText::DoorPrize { prize }),
        prosperous(),
        retirement(true),
        wither(),
        contained(),
        fail(),
    )))
}

fn augment(event: &FeedEvent) -> impl TeamFeedEventParser {
    context("Augment Feed Event", alt((
        attribute_gain(),
        modification(),
        enchantment_s1a(),
        enchantment_s1b(),
        enchantment_s2(),
        enchantment_compensatory(),
        multiple_attribute_equal(event),
        recompose(event),
        take_the_mound(),
        take_the_plate(),
        swap_places(),
        purified(),
        player_positions_swapped(),
        grow(),
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

fn wither<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, player_name) = parse_terminated(" was Corrupted by the 🥀 Wither.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::CorruptedByWither { player_name }))
    }
}

fn purified<'output>() -> impl TeamFeedEventParser<'output> {
    alt((purified_with_payout(), purified_without_payout()))
}

fn purified_with_payout<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, player_name) = parse_terminated(" was Purified of 🫀 Corruption and earned ").parse(input)?;
        let (input, payment) = u32.parse(input)?;
        let (input, _) = tag(" 🪙.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::Purified { player_name, outcome: PurifiedOutcome::Payment(payment) }))
    }
}

fn purified_without_payout<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, player_name) = parse_terminated(" was Purified of 🫀 Corruption.").parse(input)?;

        let (input, no_corruption) = opt((tag(" "), tag(player_name), tag(" had no Corruption to remove."))).parse(input)?;

        Ok((input, ParsedTeamFeedEventText::Purified { player_name, outcome: if no_corruption.is_some() { PurifiedOutcome::NoCorruption } else { PurifiedOutcome::None } }))
    }
}

fn prosperous<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, team_emoji_str) = parse_terminated(" are Prosperous! They ").parse(input)?;
        let (_, team) = emoji_team_eof.parse(team_emoji_str)?;
        let (input, _) = alt((tag("earned "), tag("earn "))).parse(input)?;
        let (input, income) = u32.parse(input)?;
        let (input, _) = tag(" 🪙.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::Prosperous { team, income }))
    }
}

fn contained<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, contained_player_name) = parse_terminated(" was contained by ").parse(input)?;
        let (input, container_player_name) = parse_terminated(" during the 🥀 Wither.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::PlayerContained { contained_player_name, container_player_name }))
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
        donated_to_lottery(),
        won_lottery(),
    )))
}

fn donated_to_lottery<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, _) = tag("The ")(input)?;
        let (input, team_name) = parse_terminated(" donated ")(input)?;
        let (input, amount) = u32.parse(input)?;
        let (input, _) = tag(" 🪙 to the ")(input)?;
        let (input, league_name) = parse_terminated(" Lottery.")(input)?;

        Ok((input, ParsedTeamFeedEventText::DonatedToLottery { team_name, amount, league_name }))
    }
}

fn won_lottery<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, _) = tag("Won ")(input)?;
        let (input, amount) = u32.parse(input)?;
        let (input, _) = tag(" 🪙 from the ")(input)?;
        let (input, league_name) = parse_terminated(" Lottery!")(input)?;

        Ok((input, ParsedTeamFeedEventText::WonLottery { amount, league_name }))
    }
}

fn maintenance<'output>() -> impl TeamFeedEventParser<'output> {
    context("Maintenance Feed Event", alt((
        tag("The team's name was reset in accordance with site policy.").map(|_| ParsedTeamFeedEventText::NameChanged),
    )))
}

fn roster<'output>() -> impl TeamFeedEventParser<'output> {
    context("Roster Feed Event", alt((
        player_moved(),
        player_relegated(),
    )))
}

fn player_moved<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, team_emoji) = emoji.parse(input)?;
        let (input, _) = tag(" ").parse(input)?;
        let (input, player_name) = parse_terminated(" was moved to the Bench.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::PlayerMoved { team_emoji, player_name }))
    }
}

fn player_relegated<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        // This might be team emoji, not sure
        let (input, _) = tag("🧳 ").parse(input)?;
        let (input, player_name) = parse_terminated(" was relegated to the Even Lesser League.").parse(input)?;

        Ok((input, ParsedTeamFeedEventText::PlayerRelegated { player_name }))
    }
}

fn player_positions_swapped<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        // I am not willing to bet on " and " being a reliable name separator, and we can
        // parse names reliably later in the message. So we're going to parse the combination
        // of names as a single unit here and then verify it after.
        let (input, anded_names) = parse_terminated(" swapped positions: ").parse(input)?;
        let (input, first_player_name) = parse_terminated(" moved to ").parse(input)?;
        let (input, first_player_new_slot) = full_slot.parse(input)?;
        let (input, _) = tag(", ").parse(input)?;
        let (input, second_player_name) = parse_terminated(" moved to ").parse(input)?;
        let (input, second_player_new_slot) = full_slot.parse(input)?;
        let (input, _) = tag(".").parse(input)?;

        // Verify that anded_names matches what's expected
        let (anded_names, _) = tag(first_player_name).parse(anded_names)?;
        let (anded_names, _) = tag(" and ").parse(anded_names)?;
        let (anded_names, _) = tag(second_player_name).parse(anded_names)?;
        let (_, _) = eof.parse(anded_names)?;

        Ok((input, ParsedTeamFeedEventText::PlayerPositionsSwapped {
            first_player_name,
            first_player_new_slot,
            second_player_name,
            second_player_new_slot,
        }))
    }
}

fn election<'output>() -> impl TeamFeedEventParser<'output> {
    context("Election Feed Event", alt((
        callup,
    )))
}

fn callup(input: &str) -> IResult<&str, ParsedTeamFeedEventText<&str>> {
    // First, look ahead for an easier-to-parse version of the team name
    // `input` is intentionally second here, and yes that is weird
    // Oh and to add to the weird, I'm including the leading space so it can
    // be more conveniently used as a tag in the next step
    let (rest, input) = parse_terminated(" joined the").parse(input)?;
    let (rest, lesser_team_name_with_space) = parse_until_period_eof.parse(rest)?;
    let lesser_team_name = &lesser_team_name_with_space[1..];

    let (input, lesser_team_emoji) = parse_terminated(lesser_team_name_with_space).parse(input)?;
    let (input, _) = tag(" ").parse(input)?;
    let (input, slot) = active_slot.parse(input)?;
    let (input, _) = tag(" ").parse(input)?;
    let (input, promoted_player_name) = parse_terminated(" was called up to replace ").parse(input)?;
    let (input, greater_team_emoji_name) = parse_terminated(&format!(" {slot} ")).parse(input)?;
    let (_, greater_league_team) = emoji_team_eof.parse(greater_team_emoji_name)?;

    // At this point, `input` should only contain "{player_name}. {player_name}".
    // That's hard to parse, but luckily we can just do character counting math
    // (and then verify the result)
    // It's safe to treat the string as a byte sequence here, since the name
    // should be the same number of bytes in both instances
    let name_length = input.len() / 2 - 1;
    let demoted_player_name = &input[..name_length];

    // Now that we (think we) know the name, actually parse it to make sure
    // everything is as we expect
    let (input, _) = tag(demoted_player_name).parse(input)?;
    let (input, _) = tag(". ").parse(input)?;
    let (input, _) = tag(demoted_player_name).parse(input)?;
    let (_, _) = eof.parse(input)?;

    // The weird way we did parsing means that `rest` is the right thing to return here
    Ok((rest, ParsedTeamFeedEventText::Callup {
        lesser_league_team: EmojiTeam {
            emoji: lesser_team_emoji,
            name: lesser_team_name,
        },
        greater_league_team,
        slot,
        promoted_player_name,
        demoted_player_name,
    }))
}

fn grow_attribute_change(input: &str) -> IResult<&str, GrowAttributeChange> {
    let (input, amount) = double().parse(input)?;
    let (input, _) = tag(" ")(input)?;
    let (input, attribute) = try_from_word.parse(input)?;

    Ok((input, GrowAttributeChange {
        attribute,
        amount,
    }))
}

fn grow<'output>() -> impl TeamFeedEventParser<'output> {
    |input| {
        let (input, player_name) = parse_terminated("'s Corruption grew: ").parse(input)?;

        // Decided to do this manually vs. with combinators because it's only 3 entries
        let (input, change_1) = grow_attribute_change.parse(input)?;
        let (input, _) = tag(", ")(input)?;
        let (input, change_2) = grow_attribute_change.parse(input)?;
        let (input, _) = tag(", ")(input)?;
        let (input, change_3) = grow_attribute_change.parse(input)?;

        let (input, immovable_granted) = alt((
            (tag(". "), tag(player_name), tag(" could not gain Immovable while on the Bench."))
                .map(|_| BenchImmuneModGranted::BenchPlayerImmune),
            // TODO Add "Yes" case
            tag(".").map(|_| BenchImmuneModGranted::No)
        )).parse(input)?;

        Ok((input, ParsedTeamFeedEventText::PlayerGrown {
            player_name,
            attribute_changes: [change_1, change_2, change_3],
            immovable_granted,
        }))
    }
}

fn bench_slot(input: &str) -> IResult<&str, BenchSlot> {
    alt((
        preceded(tag("Bench Batter "), u8).map(|num| BenchSlot::Batter(num)),
        preceded(tag("Bench Pitcher "), u8).map(|num| BenchSlot::Pitcher(num)),
    )).parse(input)
}


// TODO Dedup this
fn active_slot(input: &str) -> IResult<&str, Slot> {
    alt((
        tag("C").map(|_| Slot::Catcher),
        tag("1B").map(|_| Slot::FirstBaseman),
        tag("2B").map(|_| Slot::SecondBaseman),
        tag("3B").map(|_| Slot::ThirdBaseman),
        tag("LF").map(|_| Slot::LeftField),
        tag("CF").map(|_| Slot::CenterField),
        tag("RF").map(|_| Slot::RightField),
        tag("SS").map(|_| Slot::ShortStop),
        tag("DH").map(|_| Slot::DesignatedHitter),
        preceded(tag("SP"), u8).map(|i| Slot::StartingPitcher(i)),
        preceded(tag("RP"), u8).map(|i| Slot::ReliefPitcher(i)),
        tag("CL").map(|_| Slot::Closer),
    )).parse(input)
}

fn full_slot(input: &str) -> IResult<&str, FullSlot> {
    alt((
        bench_slot.map(|s| FullSlot::Bench(s)),
        active_slot.map(|s| FullSlot::Active(s)),
    )).parse(input)
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
    |input| {
        let text = if event.after(Breakpoints::Season5TenseChange) {
            " is injured by the extreme force of the impact!"
        } else if event.after(Breakpoints::EternalBattle) {
            " was injured by the extreme force of the impact!"
        } else {
            " was hit by a Falling Star!"
        };

        parse_terminated(text)
            .and_then(name_eof)
            .map(|team_name| ParsedTeamFeedEventText::FallingStarOutcome { team_name, outcome: FeedFallingStarOutcome::Injury })
            .parse(input)
    }
}

fn infused_by_falling_star<'output>() -> impl TeamFeedEventParser<'output> {
    alt((
        parse_terminated(" began to glow brightly with celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::BeganToGlow)),
        parse_terminated(" begins to glow brightly with celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::BeganToGlow)),
        parse_terminated(" was infused with a glimmer of celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::Infused)),
        parse_terminated(" is infused with a glimmer of celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::Infused)),
        parse_terminated(" was fully charged with an abundance of celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::FullyCharged)),
        parse_terminated(" is fully charged with an abundance of celestial energy!").and_then(name_eof).map(|team| (team, CelestialEnergyTier::FullyCharged)),
    ))
    .map(|(team_name, infusion_tier)| ParsedTeamFeedEventText::FallingStarOutcome { team_name, outcome: FeedFallingStarOutcome::Infusion(infusion_tier) })
}

fn deflected_falling_star_harmlessly<'output>() -> impl TeamFeedEventParser<'output> {
    preceded(
        alt((tag("It deflected off "), tag("It deflects off "))),
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