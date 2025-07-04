use std::{fmt::Debug, str::FromStr};

use nom::{branch::alt, bytes::complete::{tag, take, take_till, take_until, take_until1, take_while}, character::complete::{one_of, space0, u8}, combinator::{all_consuming, opt, recognize, rest, value, verify}, error::{ErrorKind, ParseError}, multi::{count, many0, many1, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Input, Parser};
use nom_language::error::VerboseError;

use crate::{enums::{Base, BatterStat, FairBallDestination, FairBallType, NowBattingStats}, feed_event::{EmojilessItem, FeedDelivery}, parsed_event::{BaseSteal, Delivery, EmojiTeam, Item, PlacedPlayer, RunnerAdvance, RunnerOut}, time::Breakpoints, Game};

pub(super) type Error<'a> = VerboseError<&'a str>;
pub(super) type IResult<'a, I, O> = nom::IResult<I, O, Error<'a>>;
pub(super) trait MyParser<'output, T>: Parser<&'output str, Output = T, Error = Error<'output>> {}
impl<'output, T, P: Parser<&'output str, Output = T, Error = Error<'output>>> MyParser<'output, T> for P {}


/// Context necessary for parsing. The 'output lifetime is linked to ParsedEvents parsed in this context.
#[derive(Clone, Debug)]
pub struct ParsingContext<'output, 'parse> {
    pub game_id: &'parse str,
    pub game: &'output Game,
    pub event_index: Option<u16>
}
impl<'output, 'parse> ParsingContext<'output, 'parse> {
    pub fn new(game_id: &'parse str, game: &'output Game, event_index: Option<u16>) -> Self {
        Self {
            game_id,
            game,
            event_index
        }
    }
    pub(crate) fn before(&self, breakpoint: Breakpoints) -> bool {
        breakpoint.before(self.game.season, &self.game.day, self.event_index)
    }
    pub(crate) fn after(&self, breakpoint: Breakpoints) -> bool {
        breakpoint.after(self.game.season, &self.game.day, self.event_index)
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

pub(super) fn home_emoji_team<'output, 'parse>(parsing_context: &'parse ParsingContext<'output, 'parse>) -> impl Parser<&'output str, Output = EmojiTeam<&'output str>, Error = Error<'output>> + 'parse {
    move |i: &'output str| {
        separated_pair(emoji, tag(" "), tag(parsing_context.game.home_team_name.as_str()))
            .map(|(emoji, name)| EmojiTeam {emoji, name})
            .parse(i)
    }
}

pub(super) fn away_emoji_team<'output, 'parse>(parsing_context: &'parse ParsingContext<'output, 'parse>) -> impl Parser<&'output str, Output = EmojiTeam<&'output str>, Error = Error<'output>> + 'parse {
    move |i: &'output str| {
        separated_pair(emoji, tag(" "), tag(parsing_context.game.away_team_name.as_str()))
            .map(|(emoji, name)| EmojiTeam {emoji, name})
            .parse(i)
    }
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

pub(super) fn placed_player_eof(input: &str) -> IResult<&str, PlacedPlayer<&str>> {
    separated_pair(try_from_word, tag(" "), name_eof)
    .map(|(place, name)| PlacedPlayer { name, place })
    .parse(input)
}

pub(super) fn name_eof(input: &str) -> IResult<&str, &str> {
    verify(rest,  |name: &str| 
        name.input_len() >= 2 &&
        // Removed for now because of early season 1 bug where feed names didn't print their spaces
        // name.chars().any(|c| c == ' ') && // From the API, we know players have first/last name, so there should always be a space
        !name.chars().any(|c| [',', '(', ')', '<', '>', '\\'].contains(&c)) && // These characters should not be in names
        !['.', ' '].contains(&name.chars().nth(0).unwrap()) // Vulnerable to "X jr." style name 
    )
    .parse(input)
}

pub(super) fn sentence_eof<'output, E: ParseError<&'output str> + Debug, F: Parser<&'output str, Output = O, Error = E>, O: Debug>(mut parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    all_consuming(sentence(move |input: &'output str| {
        take(input.chars().count()-1).and_then(|i| parser.parse(i))
        .parse(input)
    }))
}


pub(super) fn emoji(input: &str) -> IResult<&str, &str> {
    verify(take_till(AsChar::is_space), |s: &str| {
        for c in s.chars() {
            if c.is_ascii() {
                return false
            }
        }
        true
    }).parse(input)
}

pub(super) fn emoji_team_eof(input: &str) -> IResult<&str, EmojiTeam<&str>> {
    separated_pair(emoji, tag(" "), name_eof)
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
        separated_list1(tag(", "), batter_stat).map(|stats| NowBattingStats::Stats { stats } )
    )).parse(input)
}

pub(super) fn item(input: &str) -> IResult<&str, Item<&str>> {
    (
        emoji,
        opt(preceded(tag(" "), try_from_word)),
        preceded(tag(" "), try_from_word),
        opt(preceded(tag(" of "), try_from_words_m_n(1,2)))
    ).map(|(item_emoji, prefix, item, suffix)| Item { item_emoji, prefix, item, suffix})
    .parse(input)
}

pub(super) fn emojiless_item(input: &str) -> IResult<&str, EmojilessItem> {
    (
        opt(terminated(try_from_word, tag(" "))),
        try_from_word,
        opt(preceded(tag(" of "), try_from_words_m_n(1,2)))
    ).map(|(prefix, item, suffix)| EmojilessItem { prefix, item, suffix})
    .parse(input)
}

pub(super) fn delivery<'parse, 'output>(parsing_context: &'parse ParsingContext<'output, 'parse>, label: &'output str) -> impl MyParser<'output, Delivery<&'output str>> + 'parse {
    let success = (
        alt((
            separated_pair(away_emoji_team(parsing_context), tag(" "), parse_terminated(" received a ")),
            separated_pair(home_emoji_team(parsing_context), tag(" "), parse_terminated(" received a ")),
        )),
        terminated(item, (tag(" "), tag(label), tag("."))),
        opt(delimited(tag(" They discarded their "), item, tag(".")))
    ).map(|((team, player), item, discarded)| Delivery::Successful {team, player, item, discarded} );

    let fail = terminated(item, tag(" was discarded as no player had space.")).map(|item| Delivery::NoSpace { item });

    alt((
        success,
        fail
    ))
}

pub(super) fn feed_delivery<'output>(label: &'output str) -> impl MyParser<'output, FeedDelivery<&'output str>> {
        (
            parse_terminated(" received a "),
            terminated(item, (tag(" "), tag(label), tag("."))),
            opt(delimited(tag(" They discarded their "), item, tag(".")))
        ).map(|(player, item, discarded)| FeedDelivery {player, item, discarded} )
}

#[cfg(test)]
mod test {
    use nom::Parser;
    use crate::{enums::{BaseNameVariant, FairBallType, TopBottom}, nom_parsing::shared::{emoji, out, parse_and, try_from_word, try_from_words_m_n}, parsed_event::RunnerOut};

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
}