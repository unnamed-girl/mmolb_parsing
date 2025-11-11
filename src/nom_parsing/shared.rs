use std::{fmt::Debug, str::FromStr};
use std::fmt::{Display, Formatter};
use nom::{branch::alt, bytes::complete::{tag, take, take_till, take_until, take_until1, take_while}, character::complete::{one_of, space0, u8, u16}, combinator::{all_consuming, fail, opt, recognize, rest, value, verify}, error::{ErrorKind, ParseError}, multi::{count, many0, many1, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Input, Parser};
use nom::bytes::complete::is_not;
use nom_language::error::VerboseError;

use crate::{enums::{Base, BatterStat, Day, FairBallDestination, FairBallType, HomeAway, NowBattingStats, Place}, feed_event::{EmojilessItem, FeedDelivery, FeedEvent}, game::Event, parsed_event::{BaseSteal, Cheer, Delivery, DoorPrize, Ejection, EjectionReason, EmojiTeam, Item, ItemAffixes, PlacedPlayer, Prize, RunnerAdvance, RunnerOut, SnappedPhotos, ViolationType}, player, time::{Breakpoints, Time}, Game};
use crate::enums::Attribute;
use crate::parsed_event::{EjectionReplacement, ItemEquip, ItemPrize, WitherStruggle};
use crate::player::{Deserialize, Serialize};

pub(super) type Error<'a> = VerboseError<&'a str>;
pub(super) type IResult<'a, I, O> = nom::IResult<I, O, Error<'a>>;
pub(super) trait MyParser<'output, T>: Parser<&'output str, Output = T, Error = Error<'output>> {}
impl<'output, T, P: Parser<&'output str, Output = T, Error = Error<'output>>> MyParser<'output, T> for P {}


/// Context necessary for parsing. The 'output lifetime is linked to ParsedEvents parsed in this context.
#[derive(Clone, Debug)]
pub struct ParsingContext<'parse> {
    pub game_id: &'parse str,
    pub event_log: &'parse [Event],
    pub event_index: Option<u16>,
    pub home_emoji_team: EmojiTeam<&'parse str>,
    pub away_emoji_team: EmojiTeam<&'parse str>,
    pub season: u32,
    pub day: Option<Day>
}
impl<'parse> ParsingContext<'parse> {
    pub fn new(game_id: &'parse str, game: &'parse Game, event_index: Option<u16>) -> Self {
        Self {
            game_id,
            event_index,
            event_log: &game.event_log,
            home_emoji_team: EmojiTeam { emoji: &game.home_team_emoji, name: &game.home_team_name },
            away_emoji_team: EmojiTeam { emoji: &game.away_team_emoji, name: &game.away_team_name },
            season: game.season,
            day: game.day.as_ref().copied().ok()
        }
    }

    /// Whether this event is before the given time
    pub(crate) fn before(&self, time: impl Into<Time>) -> bool {
        time.into().before(self.season, self.day, self.event_index)
    }

    /// Whether this event is after the given time
    pub(crate) fn after(&self, time: impl Into<Time>) -> bool {
        time.into().after(self.season, self.day, self.event_index)
    }
}

impl FeedEvent {
    pub(crate) fn after(&self, time: impl Into<Time>) -> bool {
        time.into().after(self.season as u32, self.day.as_ref().ok().copied(), None)
    }

    #[allow(dead_code)]
    pub(crate) fn before(&self, time: impl Into<Time>) -> bool {
        time.into().before(self.season as u32, self.day.as_ref().ok().copied(), None)
    }
}

impl<'parse> EmojiTeam<&'parse str> {
    pub(super) fn parser<'output, 'a>(&'a self) -> impl MyParser<'output, EmojiTeam<&'output str>> + 'parse {
        let emoji = self.emoji;
        let name = self.name;
        move |input: &'output str| {
            separated_pair(tag(emoji), tag(" "), tag(name))
                .map(|(emoji, name)| EmojiTeam {emoji, name})
                .parse(input)
        }
    }
}

#[allow(dead_code)]
pub(super) fn debugger<'output, E: ParseError<&'output str> + Debug, F: Parser<&'output str, Output = O, Error = E>, O: Debug>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    let mut r = parser;
    move |i| {
        match r.parse(i) {
            r @ Err(_) => {
                tracing::error!("{r:?}");
                r
            },
            o => o
        }
    }
}

/// Discards \<strong>\</strong> tags and whitespace from around the child parser.
pub(super) fn bold<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    delimited((space0, tag("<strong>")), parser, tag("</strong>"))
}
/// Discards whitespace and a terminating full stop from around the child parser.
pub(super) fn sentence<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    delimited(space0, parser, tag("."))
}

/// Discards whitespace and a terminating exclamation mark from around the child parser.
pub(super) fn exclamation<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    delimited(space0, parser, tag("!"))
}

/// Takes until it sees punctuation or a space.
pub(super) fn word(s: &str) -> IResult<&str, &str> {
    take_while(|char| ![',', '.', ' ', '!', '<', '>', ':', ';'].contains(&char)).parse(s)
}

/// n groups of space-separated words. Will get stuck on punctuation
pub(super) fn words<'output>(n: usize) -> impl Parser<&'output str, Output = &'output str, Error = Error<'output>> {
    recognize((word, count((one_of(" "), word), n-1)))
        .map(str::trim)
}

/// Verb names for fair ball types, e.g. "pops"
pub(super) fn fair_ball_type_verb_name(i: &str) -> IResult<&str, FairBallType> {
    word.map_opt(|word| match word {
        "grounds" => Some(FairBallType::GroundBall),
        "flies" => Some(FairBallType::FlyBall),
        "lines" => Some(FairBallType::LineDrive),
        "pops" => Some(FairBallType::Popup),
        _ => None
    }).parse(i)
}
/// Verb names for fly ball types, e.g. "pops"
pub(super) fn fly_ball_type_verb_name(i: &str) -> IResult<&str, FairBallType> {
    word.map_opt(|word| match word {
        "flies" => Some(FairBallType::FlyBall),
        "lines" => Some(FairBallType::LineDrive),
        "pops" => Some(FairBallType::Popup),
        _ => None
    }).parse(i)
}

/// A destination for a fair ball, e.g. "the shortstop"
pub(super) fn destination(i: &str) -> IResult<&str, FairBallDestination> {
    words(2).map_res(FairBallDestination::try_from)
        .parse(i)
}

pub(super) fn try_from_word<'output, T:FromStr>(i: &'output str) -> IResult<'output,&'output str, T> {
    word.map_res(T::from_str).parse(i)
}

/// n > m
pub(super) fn try_from_words_m_n<'output, T:FromStr>(m: usize, n:usize) -> impl MyParser<'output, T> {
    move |input: &'output str| {
        for i in (m..=n).rev() {
            match words(i).map_res(T::from_str).parse(input) {
                Ok(o) => return Ok(o),
                Err(_) => ()
            }
        }
        Err(nom::Err::Error(VerboseError::<&str>::from_error_kind(input, ErrorKind::MapRes)))
    }
}

/// A list of fielders involved in a catch, e.g. "P Niblet Hornsby to 1B Geo Kerr to 3B Joffrey Nishida"
pub(super) fn fielders_eof(input: &str) -> IResult<&str, Vec<PlacedPlayer<&str>>> {
    all_consuming(alt((
        (
            many1(parse_terminated(" to ").and_then(placed_player_eof)),
            placed_player_eof
        ).map(|(mut fielders, last)| {
            fielders.push(last);
            fielders
        }),
        parse_terminated(" unassisted").and_then(placed_player_eof).map(|fielder| vec![fielder])
    )))
    .parse(input)
}

/// A single instance of an out, e.g. "Franklin Shoebill out at home"
pub(super) fn out(input: &str) -> IResult<&str, RunnerOut<&str>> {
    (
        parse_terminated(" out at ").and_then(name_eof),
        try_from_words_m_n(1,2)
    )
    .map(|(player, base)| RunnerOut { runner: player, base })
    .parse(input)
}

/// A single instance of a runner scoring, e.g. "<bold>Franklin Shoebill scores!</bold>"
pub(super) fn scores_sentence(input: &str) -> IResult<&str, &str> {
    bold(exclamation(parse_terminated(" scores").and_then(name_eof)))
    .parse(input)
}

// A single instance of a runner advancing, e.g. "Franklin shoebill to third base."
pub fn runner_advance_sentence(input: &str) -> IResult<&str, RunnerAdvance<&str>> {
    sentence((parse_terminated(" to ").and_then(name_eof), terminated(try_from_word, tag(" base"))))
    .map(|(runner, base)| RunnerAdvance {runner, base})
    .parse(input)
}

/// The suffix of an ordinal, e.g. the "th" of 4th
pub(super) fn ordinal_suffix(i: &str) -> IResult<&str, &str> {
    alt((
        tag("th"),
        tag("rd"),
        tag("nd"),
        tag("st"),
    )).parse(i)
}

/// Any number of runners scoring followed by any number of runners advancing
pub(super) fn scores_and_advances(input: &str) -> IResult<&str, (Vec<&str>, Vec<RunnerAdvance<&str>>)> {
    (
        many0(scores_sentence),
        many0(runner_advance_sentence)
    )
    .parse(input)
}

pub(super) fn base_steal_sentence(input: &str) -> IResult<&str, BaseSteal<&str>> {
    let home_steal = bold(exclamation(parse_terminated(" steals home")))
    .map(|runner| BaseSteal { runner, base:Base::Home, caught:false });

    let successful_steal = exclamation((parse_terminated(" steals "), terminated(try_from_word, tag(" base"))))
    .map(|(runner, base)| BaseSteal {runner, base, caught: false });

    let caught_stealing_home = sentence(parse_terminated(" is caught stealing home"))
    .map(|runner| BaseSteal {runner, base:Base::Home, caught: true });

    let caught_stealing = sentence((parse_terminated(" is caught stealing "), terminated(try_from_word, tag(" base"))))
    .map(|(runner, base)| BaseSteal {runner, base, caught: true });

    alt((
        caught_stealing,
        successful_steal,
        caught_stealing_home,
        home_steal,
    )).parse(input)
}

pub(super) fn score_update(i: &str) -> IResult<&str, (u8, u8)> {
    separated_pair(u8, tag("-"), u8)
    .parse(i)
}

/// Splits the first sentence out of the input, passes it into the `sentence` parser and then passes the remainder into the `rest` parser.
/// Sentences split full stop boundaries, but may contain full stops - this implementation uses backtracking,
/// splitting at each full stop until it finds a split that satisfies both parsers.
///
/// Fails if it can't find a split point that satisfies both parsers.
///
/// E.g. "BATTER flies out to SS M. Lastname. FIELDER to second." would attempt to split at character 0, 25, 35 and 54.
pub(super) fn all_consuming_sentence_and<'output, F: Parser<&'output str, Output = O, Error = Error<'output>>, F2: Parser<&'output str, Output = O2, Error = Error<'output>>, O, O2>(mut sentence: F, mut rest: F2) -> impl Parser<&'output str, Output = (O, O2), Error = Error<'output>> {
    move |input| {
        let mut i = 0;
        loop {
            i += 1;
            match preceded(space0, recognize(count((take_until("."), tag(".")), i))).parse(input) {
                IResult::Ok((remainder, in_sentence)) => {
                    if let Ok(("", o)) = sentence.parse(&in_sentence[..in_sentence.len()-1]) { // Cut out full stop
                        if let Ok(("", o_2)) = rest.parse(remainder) {
                            return Ok(("", (o, o_2)))
                        }
                    }
                }
                IResult::Err(_) => return IResult::Err(nom::Err::Error(VerboseError::from_error_kind(input, ErrorKind::Tag)))
            }

            if i >= 10 {
                return IResult::Err(nom::Err::Error(VerboseError::from_error_kind(input, ErrorKind::Tag)))
            }
        }
    }
}


/// Keeps searching for the delimiter until it finds an instance immediately followed by a valid input to the child parser.
/// Returns everything up to the delimiter and the output of the child parser.
pub fn parse_and<'output, F, O>(
    mut f: F,
    delimiter: &'output str,
  ) -> impl Parser<&'output str, Output = (&'output str, O), Error = Error<'output>>
  where
    F: Parser<&'output str, Output = O, Error = Error<'output>>,
{
    move |input: &'output str| {
        let mut i = 1usize;
        let delimiter_len = delimiter.input_len();

        loop {
            let (remainder, parsed) = recognize(count((take_until(delimiter), tag(delimiter)), i)).parse(input)?;
            if let Ok((remainder, o)) = f.parse(remainder) {
                return Ok((remainder, (&parsed[..parsed.input_len()-delimiter_len], o))) // parsed ends in the delimiter so parsed.input_len() - delimiter_len is always >=0.
            }
            i += 1;
        }
    }

}

// Taken from Fed
/// Parse until tag is found, then discard that tag.
pub(super) fn parse_terminated(tag_content: &str) -> impl Fn(&str) -> IResult<&str, &str> + '_ {
    move |input| {
        // There's an "and Friends" name now
        if tag_content == " and " {
            let (new_input, prefix) = opt(parse_terminated(" and Friends and ")).parse(input)?;
            if let Some(prefix) = prefix {
                // Extend val by the length of " and Friends"
                let name_len = prefix.len() + " and Friends".len();
                let full_match = &input[..name_len];
                return Ok((new_input, full_match));
            }
        }

        let (input, parsed_value) = if tag_content == "." {
            alt((
                // The Kaj Statter Jr. rule
                verify(recognize(terminated(take_until1(".."), tag("."))), |s: &str| !s.contains('\n')),
                verify(take_until1(tag_content), |s: &str| !s.contains('\n')),
            )).parse(input)
        } else {
            verify(take_until1(tag_content), |s: &str| !s.contains('\n')).parse(input)
        }?;
        let (input, _) = tag(tag_content).parse(input)?;

        Ok((input, parsed_value))
    }
}

// This is for use in place of parse_terminated when the only remaining text in the string is ".",
// and so you can't use parse_terminated because that would improperly cut off names with periods
// like "Kaj Statter Jr."
pub(super) fn parse_until_period_eof(input: &str) -> IResult<&str, &str> {
    let (input, replacement_name_with_dot) = is_not("\n").parse(input)?;
    let replacement_name = replacement_name_with_dot.strip_suffix(".")
        .ok_or_else(|| fail::<&str, &str, _>().parse(input).unwrap_err())?;

    Ok((input, replacement_name))
}

// Same idea as parse_until_period_eof
pub(super) fn parse_until_exclamation_point_eof(input: &str) -> IResult<&str, &str> {
    let (input, replacement_name_with_dot) = is_not("\n").parse(input)?;
    let replacement_name = replacement_name_with_dot.strip_suffix("!")
        .ok_or_else(|| fail::<&str, &str, _>().parse(input).unwrap_err())?;

    Ok((input, replacement_name))
}


pub(super) fn placed_player_eof(input: &str) -> IResult<&str, PlacedPlayer<&str>> {
    separated_pair(try_from_word, tag(" "), name_eof)
    .map(|(place, name)| PlacedPlayer { name, place })
    .parse(input)
}

pub(super) fn name_eof(input: &str) -> IResult<&str, &str> {
    verify(rest,  |name: &str|
        name.input_len() >= 2 &&
        !["Dr"].contains(&name) &&
        // ignoring 0-length words, all words are 2 characters long and contain an ascii character, except Stanley Demir I
        // and the 7 in the team name "Organiz. Nazionale Combattenti 7 Zombie Deer Revolution"
        (name == "Stanley Demir I" || name.split_whitespace().all(|word| word.len() == 0 || (word.len() >= 2 && word.chars().any(|i| i.is_ascii())) || word.parse::<usize>().is_ok())) &&
        name.chars().any(|c| c == ' ') &&
        // Removed for now because of early season 1 bug where feed names didn't print their spaces
        // name.chars().any(|c| c == ' ') && // From the API, we know players have first/last name, so there should always be a space
        !name.chars().any(|c| [',', '(', ')', '<', '>', '\\', '\u{FE0F}'].contains(&c)) && // These characters should not be in names
        !['.', ' '].contains(&name.chars().nth(0).unwrap()) && // Names shouldn't start with these, and this catches some common logic errors (e.g. forgetting to parse the space before the name)
        ![' '].contains(&name.chars().last().unwrap()) && // Names shouldn't end with these, and this catches some common logic errors (e.g. forgetting to parse the space after the name)
        // Cleanest fix for https://mmolb.com/watch/68aab8a3318f19d301830b7c?event=316
        // "to 2B" etc. are exceedingly unlikely as a prefix to name
        name.strip_prefix("to ").map(|rest| try_from_word::<Place>(rest).is_err()).unwrap_or(true) 
    )
    .parse(input)
}

pub(super) fn conjoined_name_eof(input: &str) -> IResult<&str, &str> {
    Ok(("", input))
}

pub(super) fn sentence_eof<'output, E: ParseError<&'output str> + Debug, F: Parser<&'output str, Output = O, Error = E>, O: Debug>(mut parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    all_consuming(sentence(move |input: &'output str| {
        take(input.chars().count()-1).and_then(|i| parser.parse(i))
        .parse(input)
    }))
}


pub(super) fn emoji(input: &str) -> IResult<&str, &str> {
    verify(take_till(AsChar::is_space), |s: &str| {
        !s.is_ascii() && s.chars().all(|c| !['!', '.', '<', '>'].contains(&c))
    }).parse(input)
}

pub(super) fn emoji_team_eof(input: &str) -> IResult<&str, EmojiTeam<&str>> {
    separated_pair(emoji, tag(" "), name_eof)
    .map(|(emoji, name)| EmojiTeam { emoji, name })
    .parse(input)
}

pub(super) fn emoji_team_eof_maybe_no_space(input: &str) -> IResult<&str, EmojiTeam<&str>> {
    separated_pair(emoji, tag(" "), conjoined_name_eof)
    .map(|(emoji, name)| EmojiTeam { emoji, name })
    .parse(input)
}

pub(super) fn batter_stat(input: &str) -> IResult<&str, BatterStat> {
    alt((
        terminated(u8, tag(" 1B")).map(BatterStat::FirstBases),
        terminated(u8, tag(" 2B")).map(BatterStat::SecondBases),
        terminated(u8, tag(" 3B")).map(BatterStat::ThirdBases),
        terminated(u8, tag(" LO")).map(BatterStat::LineOuts),
        terminated(u8, tag(" SO")).map(BatterStat::StrikeOuts),
        terminated(u8, tag(" FO")).map(BatterStat::ForceOuts),
        terminated(u8, tag(" HR")).map(BatterStat::HomeRuns),
        terminated(u8, tag(" FC")).map(BatterStat::FieldersChoices),
        terminated(u8, tag(" SF")).map(BatterStat::SacrificeFlies),
        terminated(u8, tag(" F")).map(BatterStat::Fouls),
        terminated(u8, tag(" BB")).map(BatterStat::BaseOnBalls),
        terminated(u8, tag(" FC")).map(BatterStat::SacrificeFlies),
        terminated(u8, tag(" HBP")).map(BatterStat::HitByPitchs),
        terminated(u8, tag(" GIDP")).map(BatterStat::GroundIntoDoublePlays),
        terminated(u8, tag(" CDP")).map(BatterStat::CaughtDoublePlays),
        terminated(u8, tag(" PO")).map(BatterStat::PopOuts),
        terminated(u8, tag(" GO")).map(BatterStat::GroundOuts),
        separated_pair(u8, tag(" for "), u8).map(|(hits, at_bats)| BatterStat::HitsForAtBats { hits, at_bats }),
    ))
    .parse(input)
}

/// Doesn't include the brackets. e.g. "1st PA of game" or "1 for 2, 1 1B, 1 FO"
pub(super) fn now_batting_stats(input: &str) -> IResult<&str, NowBattingStats> {
    alt ((
        value(NowBattingStats::FirstPA, tag("1st PA of game")),
        separated_list1(tag(", "), batter_stat).map(|stats| NowBattingStats::Stats(stats) )
    )).parse(input)
}

pub(super) fn item(input: &str) -> IResult<&str, Item<&str>> {
    alt((
        verify((
            emoji,
            opt(preceded(tag(" "), try_from_word)),
            preceded(tag(" "), try_from_words_m_n(1,3)),
            opt(preceded(tag(" "), try_from_words_m_n(2,3)))),
            |(_, prefix, _, suffix)| prefix.is_some() || suffix.is_some()
        ).map(|(item_emoji, prefix, item, suffix)| Item { item_emoji, item, affixes: ItemAffixes::PrefixSuffix(prefix, suffix)}),
        (
            emoji,
            preceded(tag(" "), try_from_words_m_n(1,3)),
        ).map(|(item_emoji, item, )| Item { item_emoji, item, affixes: ItemAffixes::None}),
        (
            emoji,
            preceded(tag(" "), parse_and(fail_once(try_from_words_m_n(1,3)), " ")) // fail_once janky fix for rarenames being two words.
        ).map(|(item_emoji, (rare_name, item))| Item { item_emoji, item, affixes: ItemAffixes::RareName(rare_name)})
    ))
    .parse(input)
}

pub(super) fn emojiless_item(input: &str) -> IResult<&str, EmojilessItem> {
    (
        opt(terminated(try_from_word, tag(" "))),
        try_from_word,
        opt(preceded(tag(" "), try_from_words_m_n(2,3)))
    ).map(|(prefix, item, suffix)| EmojilessItem { prefix, item, suffix})
    .parse(input)
}

pub(super) fn delivery<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>, label: &'parse str) -> impl MyParser<'output, Delivery<&'output str>> + 'parse {
    let receive_text = received_text(parsing_context.season, parsing_context.day, parsing_context.event_index);
    let discard_text = discarded_text(parsing_context.season, parsing_context.day, parsing_context.event_index);
    let success = (
        alt(( // Alt needs the later context to distinguish "Buffalo Buffalo" and "Buffalo Buffalo Buffalo"
            terminated(parsing_context.away_emoji_team.parser(), tag(receive_text)).map(|team| (team, None)),
            (parsing_context.away_emoji_team.parser(), preceded(tag(" "), parse_terminated(receive_text).map(Some))),
            terminated(parsing_context.home_emoji_team.parser(), tag(receive_text)).map(|team| (team, None)),
            (parsing_context.home_emoji_team.parser(), preceded(tag(" "), parse_terminated(receive_text).map(Some))),
        )),
        terminated(item, (tag(" "), tag(label), tag("."))),
        opt(delimited(tag(discard_text), item, tag(".")))
    ).map(|((team, player), item, discarded)| Delivery::Successful {team, player, item, discarded} );

    let discard_text = parsing_context.after(Breakpoints::Season5TenseChange).then_some(" is discarded as no player has space.").unwrap_or(" was discarded as no player had space.");
    let fail = terminated(item, tag(discard_text)).map(|item| Delivery::NoSpace { item });

    alt((
        success,
        fail
    ))
}

pub(super) fn feed_delivery(label: &str) -> impl MyParser<FeedDelivery<&str>> {
    move |input| {
        let (input, (player, equipped)) = alt((
            parse_terminated(" received a ").map(|n| (n, false)),
            parse_terminated(" receives a ").map(|n| (n, false)),
            parse_terminated(" equips ").map(|n| (n, true)),
        )).parse(input)?;
        let (input, item) = item.parse(input)?;
        let (input, _) = tag(if equipped { " from " } else { " " }).parse(input)?;
        let (input, _) = tag(label).parse(input)?;
        let (input, discarded) = opt(discarded_item()).parse(input)?;
        let (input, _) = tag(".").parse(input)?;

        Ok((input, FeedDelivery { player, item, discarded, equipped }))
    }
}

pub(super) fn discarded_item<'output>() -> impl MyParser<'output, Item<&'output str>> {
    |input| {
        let (input, _) = alt((tag(". They discarded their "), tag(". They discard their "))).parse(input)?;
        let (input, item) = item.parse(input)?;
        Ok((input, item))
    }
}

pub(super) fn cheer<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, Cheer> + 'parse {
    |input| {
        if parsing_context.before(Breakpoints::Season3) {
            tracing::warn!("Cheer before season 3");
            fail().parse(input)
        } else if parsing_context.before(Breakpoints::CheersGetEmoji) {
            parse_terminated("!").map(Cheer::new).parse(input)
        } else {
            preceded(
                tag("📣 "),
                parse_terminated("!").map(Cheer::new)
            ).parse(input)
        }
    }
}

pub(super) fn aurora_players<'parse, 'output: 'parse>(first: EmojiTeam<&'parse str>, second: EmojiTeam<&'parse str>) -> impl MyParser<'output, (&'output str, PlacedPlayer<&'output str>, &'output str, PlacedPlayer<&'output str>)> + 'parse {
    move |input| {
        let (input, first_team_emoji) = tag(first.emoji).parse(input)?;
        let (input, _) = tag(" ").parse(input)?;
        let (input, first_player) = parse_terminated(" and ").and_then(placed_player_eof).parse(input)?;

        let (input, second_team_emoji) = tag(second.emoji).parse(input)?;
        let (input, _) = tag(" ").parse(input)?;
        let (input, second_player) = parse_terminated(" snapped photos of the aurora.").and_then(placed_player_eof).parse(input)?;

        Ok((input, (
            first_team_emoji,
            first_player,
            second_team_emoji,
            second_player,
        )))
    }
}

pub(super) fn aurora<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, SnappedPhotos<&'output str>> + 'parse {
    |input| {
        let (input, _) = tag("The Geomagnetic Storms Intensify! ").parse(input)?;

        // Note: if you try to expose which of home and away is first, consider that it's
        // technically possible for two teams with the same name and emoji to play each other
        let (input, (
            first_team_emoji,
            first_player,
            second_team_emoji,
            second_player,
        )) = alt((
            aurora_players(parsing_context.home_emoji_team, parsing_context.away_emoji_team),
            aurora_players(parsing_context.away_emoji_team, parsing_context.home_emoji_team),
        )).parse(input)?;

        Ok((input, SnappedPhotos {
            first_team_emoji,
            first_player,
            second_team_emoji,
            second_player,
        }))
    }
}

// TODO Delete the leading space from this and instead add it in all the
//   places this is used as a child parser
pub(super) fn ejection<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, Ejection<&'output str>> + 'parse {
    |input| {
        let (input, _) = tag(" 🤖 ROBO-UMP ejected ").parse(input)?;

        let (input, output)  = ejection_tail(parsing_context).parse(input)?;

        // ejection_tail intentionally doesn't consume the period
        let (input, _) = tag(".").parse(input)?;

        Ok((input, output))
    }
}

// This is an ejection when the leading " 🤖 ROBO-UMP ejected " has already
// been consumed, e.g. by a parse_terminated
pub(super) fn ejection_tail<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, Ejection<&'output str>> + 'parse {
    |input| {
        let (input, team) = alt((
            parsing_context.away_emoji_team.parser(),
            parsing_context.home_emoji_team.parser(),
        )).parse(input)?;

        let (input, _) = tag(" ").parse(input)?;

        // " for a " is borderline but I still think probably OK to assume will never be part of a name
        let (input, ejected_player) = parse_terminated(" for a ").and_then(placed_player_eof).parse(input)?;

        let (input, violation_type) = parse_terminated(" Violation (").map(ViolationType::new).parse(input)?;

        let (input, (reason, replacement)) = alt((
              (
                  parse_terminated("). Bench Player ").map(EjectionReason::new),
                  parse_terminated(" takes their place").map(|player_name| EjectionReplacement::BenchPlayer { player_name }),
              ),
              (
                  terminated(parse_terminated("). "), terminated(tag(team.emoji), tag(" "))).map(EjectionReason::new),
                  parse_terminated(" takes the mound").and_then(placed_player_eof).map(|player| EjectionReplacement::RosterPlayer { player }),
              ),
        )).parse(input)?;

        Ok((input, Ejection {
            team,
            ejected_player,
            violation_type,
            reason,
            replacement,
        }))
    }
}

pub(super) fn door_prizes(input: &str) -> IResult<&str, Vec<DoorPrize<&str>>> {
    many0(preceded(tag("<br>"), door_prize)).parse(input)
}

pub(super) fn wither_s6<'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl Fn(&str) -> IResult<&str, WitherStruggle<&str>> + use<'parse> {
    move |input| {
        // Wither is a suffix so always has a space separating it from the main event body
        let (input, _) = tag(" ").parse(input)?;
        let (input, team_emoji) = either_team_emoji(parsing_context).parse(input)?;
        let (input, _) = tag(" ").parse(input)?;
        let (input, target_str) = parse_terminated(" struggles against the 🥀 Wither.").parse(input)?;

        let (_, target) = placed_player_eof(target_str)?;

        Ok((input, WitherStruggle { team_emoji, target, source_name: None }))
    }
}

pub(super) fn wither_s7<'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl Fn(&str) -> IResult<&str, WitherStruggle<&str>> + use<'parse> {
    move |input| {
        // Wither is a suffix so always has a space separating it from the main event body
        let (input, _) = tag(" ").parse(input)?;
        let (input, source_name) = parse_terminated(" is trying to spread the 🥀 Wither to ").parse(input)?;
        let (input, team_emoji) = either_team_emoji(parsing_context).parse(input)?;
        let (input, _) = tag(" ").parse(input)?;
        // Please danny don't let player names include exclamation points
        let (input, target_str) = parse_terminated("!").parse(input)?;

        let (_, target) = placed_player_eof(target_str)?;

        Ok((input, WitherStruggle { team_emoji, target, source_name: Some(source_name) }))
    }
}

pub(super) fn wither<'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl Fn(&str) -> IResult<&str, WitherStruggle<&str>> + use<'parse> {
    |input| alt((wither_s6(parsing_context), wither_s7(parsing_context))).parse(input)
}

pub(super) fn equipped_item(input: &str) -> IResult<&str, ItemPrize<&str>> {
    let (input, player_name) = parse_terminated(" equips ").parse(input)?;
    let (input, item) = item.parse(input)?;
    let (input, _) = tag(" from the Door Prize").parse(input)?;
    let (input, discarded_item) = opt(discarded_item()).parse(input)?;

    Ok((input, ItemPrize {
        item,
        equip: ItemEquip::Equipped {
            player_name,
            discarded_item,
        }
    }))
}

// This is when the item from the door prize was discarded, not for when the player's
// previous item was discarded after they equipped that one
pub(super) fn door_prize_item_discarded(input: &str) -> IResult<&str, ItemPrize<&str>> {
    let (input, item) = item.parse(input)?;
    let (input, _) = tag(" is discarded; nobody can use it").parse(input)?;

    Ok((input, ItemPrize {
        item,
        equip: ItemEquip::Discarded
    }))
}

pub(super) fn equipped_prize<'output>(input: &'output str) -> IResult<'output, &'output str, Prize<&'output str>> {
    alt((
        terminated(u16, tag(" 🪙")).map(Prize::Tokens),
        separated_list1(tag(". "), alt((equipped_item, door_prize_item_discarded))).map(Prize::Items)
    )).parse(input)
}


pub(super) fn prize<'output>(input: &'output str) -> IResult<'output, &'output str, Prize<&'output str>> {
    alt((
        terminated(u16, tag(" 🪙")).map(Prize::Tokens),
        separated_list1(tag(", "), item).map(|items| {
            Prize::Items(
                items
                    .into_iter()
                    .map(|item| ItemPrize { item, equip: ItemEquip::None })
                    .collect()
            )
        })
    )).parse(input)
}

pub(super) fn door_prize<'output>(input: &'output str) -> IResult<'output, &'output str, DoorPrize<&'output str>> {
    let not_win = |input: &'output str| {
        let (input, player) = parse_terminated(" didn't win a Door Prize.").parse(input)?;
        Ok((input, DoorPrize { player, prize: None }))
    };
    let win = |input: &'output str| {
        let (input, player) = parse_terminated(" won a Door Prize: ").parse(input)?;
        let (input, prize) = prize.parse(input)?;
        let (input, _) = tag(".").parse(input)?;
        Ok((input, DoorPrize { player, prize: Some(prize) }))
    };
    let win_and_equip = |input: &'output str| {
        let (input, player) = parse_terminated(" won a Door Prize! ").parse(input)?;
        let (input, prize) = equipped_prize.parse(input)?;
        let (input, _) = tag(".").parse(input)?;
        Ok((input, DoorPrize { player, prize: Some(prize) }))
    };
    let (input, _) = tag("🥳 ").parse(input)?;

    alt((
        not_win,
        win,
        win_and_equip,
    )).parse(input)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FeedEventParty<S> {
    pub player_name: S,
    pub amount_gained: u8,
    pub attribute: Attribute,
    // As of this writing, the Prolific boon is the only way to not lose durability
    pub durability_lost: Option<u8>,
}

impl<S: Display> Display for FeedEventParty<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} is Partying! {} gained +{} {} and ",
            self.player_name,
            self.player_name,
            self.amount_gained,
            self.attribute,
        )?;

        match self.durability_lost {
            None => write!(f, "their Prolific Greater Boon resisted Durability loss."),
            Some(durability_lost) => write!(f, "lost {durability_lost} Durability."),
        }
    }
}

pub(super) fn feed_event_party(input: &str) -> IResult<&str, FeedEventParty<&str>> {
    let (input, player_name) = parse_terminated(" is Partying! ").parse(input)?;
    let (input, _) = tag(player_name).parse(input)?;
    let (input, _) = tag(" gained +").parse(input)?;
    let (input, amount_gained) = u8.parse(input)?;
    let (input, _) = tag(" ").parse(input)?;
    let (input, attribute) = try_from_word.parse(input)?;

    let lost_durability = |input| {
        let (input, _) = tag(" and lost ").parse(input)?;
        let (input, durability_lost) = u8.parse(input)?;
        let (input, _) = tag(" Durability.").parse(input)?;

        Ok((input, durability_lost))
    };

    let (input, durability_lost) = alt((
        lost_durability.map(Some),
        tag(" and their Prolific Greater Boon resisted Durability loss.").map(|_| None),
    )).parse(input)?;

    Ok((input, FeedEventParty {
        player_name,
        amount_gained,
        attribute,
        durability_lost,
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeedEventDoorPrize<S> {
    pub player_name: S,
    pub prize: Prize<S>,
}

impl<S: Display> Display for FeedEventDoorPrize<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let punct = match &self.prize {
            Prize::Items(i) if i.iter().any(|prize| !prize.equip.is_none()) => "!",
            _ => ":",
        };
        write!(f, "{} won a Door Prize{punct} {}.", self.player_name, self.prize.unparse())
    }
}

pub(super) fn feed_event_door_prize(input: &str) -> IResult<&str, FeedEventDoorPrize<&str>> {
    let (input, player_name) = parse_terminated(" won a Door Prize: ").parse(input)?;
    let (input, prize) = prize.parse(input)?;
    let (input, _) = tag(".").parse(input)?;

    Ok((input, FeedEventDoorPrize {
        player_name,
        prize,
    }))
}

pub(super) fn feed_event_equipped_door_prize(input: &str) -> IResult<&str, FeedEventDoorPrize<&str>> {
    let (input, player_name) = parse_terminated(" won a Door Prize! ").parse(input)?;
    let (input, prize) = equipped_prize.parse(input)?;
    let (input, _) = tag(".").parse(input)?;

    Ok((input, FeedEventDoorPrize {
        player_name,
        prize,
    }))
}

pub(super) fn team_emoji<'parse, 'output, 'a>(side: HomeAway, parsing_context: &'a ParsingContext<'parse>) -> impl MyParser<'output, &'output str> + 'parse {
    let home_team_emoji = parsing_context.home_emoji_team.emoji;
    let away_team_emoji = parsing_context.away_emoji_team.emoji;
    move |input| match side {
        HomeAway::Home => tag(home_team_emoji).parse(input),
        HomeAway::Away => tag(away_team_emoji).parse(input),
    }
}

pub(super) fn either_team_emoji<'parse, 'output, 'a>(parsing_context: &'a ParsingContext<'parse>) -> impl MyParser<'output, &'output str> + 'parse {
    // Most of the time when a team's emoji appears on its own we can't
    // know what team it is in case they have the same emoji
    alt((
        team_emoji(HomeAway::Away, parsing_context),
        team_emoji(HomeAway::Home, parsing_context),
    ))
}

pub(super) fn fail_once<'output, F, O>(
    mut f: F,
) -> impl Parser<&'output str, Output = O, Error = Error<'output>>
where F: Parser<&'output str, Output = O, Error = Error<'output>>,
{
    let mut failed = false;
    move |input: &'output str| {
        if failed {
            f.parse(input)
        } else {
            failed = true;
            fail().parse(input)
        }
    }
}

pub fn strike_out_text(season: u32, day: Option<Day>, event_index: Option<u16>) -> &'static str {
    Breakpoints::Season5TenseChange.after(season, day, event_index).then_some(" strikes out ").unwrap_or(" struck out ")
}

pub fn hit_by_pitch_text(season: u32, day: Option<Day>, event_index: Option<u16>) -> &'static str {
    Breakpoints::Season5TenseChange.after(season, day, event_index).then_some(" is hit by the pitch and advances to first base").unwrap_or(" was hit by the pitch and advances to first base")
}

pub fn received_text(season: u32, day: Option<Day>, event_index: Option<u16>) -> &'static str {
    Breakpoints::Season5TenseChange.after(season, day, event_index).then_some(" receives a ").unwrap_or(" received a ")
}

pub fn discarded_text(season: u32, day: Option<Day>, event_index: Option<u16>) -> &'static str {
    Breakpoints::Season5TenseChange.after(season, day, event_index).then_some(" They discard their ").unwrap_or(" They discarded their ")
}

pub fn was_is_text(season: u32, day: Option<Day>, event_index: Option<u16>) -> &'static str {
    Breakpoints::Season5TenseChange.after(season, day, event_index).then_some("was").unwrap_or("is")
}

#[cfg(test)]
mod test {
    use nom::Parser;
    use crate::{enums::{BaseNameVariant, Day, FairBallType, TopBottom}, nom_parsing::{shared::{delivery, emoji, out, parse_and, try_from_word, try_from_words_m_n}, ParsingContext}, parsed_event::{EmojiTeam, RunnerOut}};

    #[test]
    fn test_parse_and() {
        assert_eq!(Ok((" wow", ("hi hi", TopBottom::Top))), parse_and(try_from_word::<TopBottom>, " ").parse("hi hi top wow"));
        assert!(parse_and(try_from_word::<TopBottom>, " ").parse("top wow").is_err());
    }

    #[test]
    fn test_try_from_words() {
        assert_eq!(Ok((" blah", FairBallType::LineDrive)), try_from_words_m_n(1,2).parse("line drive blah"));
    }

    #[test]
    fn test_out() {
        assert_eq!(Ok(("", RunnerOut {runner: "Dolorenine Lomidze", base :BaseNameVariant::ThirdBase})), out("Dolorenine Lomidze out at third base"));
    }

    #[test]
    fn test_emoji() {
        assert_eq!(Ok(("", "\u{26be}")), emoji("\u{26be}"));
    }

    #[test]
    fn whale_bones() {
        let text = "🏴󠁧󠁢󠁷󠁬󠁳󠁿 Llanfairpwllgwyngyll Whale Bones received a 🧢 Artistic Gloves Cap Special Delivery.";

        let mut parser = delivery(&ParsingContext { game_id: "", event_log: &[], event_index: None, home_emoji_team: EmojiTeam { emoji: "", name: "" }, away_emoji_team: EmojiTeam { emoji: "🏴󠁧󠁢󠁷󠁬󠁳󠁿", name: "Llanfairpwllgwyngyll Whale Bones" }, season: 3, day: Some(Day::Day(166)) }, "Special Delivery");

        parser.parse(text).unwrap();
    }
}
