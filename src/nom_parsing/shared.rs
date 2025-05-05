use std::{fmt::Debug, str::FromStr};

use nom::{branch::alt, bytes::complete::{tag, take, take_till, take_until, take_until1, take_while}, character::complete::{multispace0, space0, space1, u8}, combinator::{all_consuming, recognize, rest, verify}, error::{ErrorKind, ParseError}, multi::{count, many0, many1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Compare, CompareResult, Input, Parser};
use nom_language::error::VerboseError;

use crate::{enums::{Base, BaseNameVariants, Distance, FieldingErrorType, FoulType, FairBallDestination, FairBallType, Position, StrikeType, TopBottom}, parsed_event::{BaseSteal, Play, PositionedPlayer, RunnerAdvance, RunnerOut}, Game};

pub(super) type Error<'a> = VerboseError<&'a str>;
pub(super) type IResult<'a, I, O> = nom::IResult<I, O, Error<'a>>;

/// Context necessary for parsing. The 'output lifetime is linked to ParsedEvents parsed in this context.
#[derive(Clone, Debug)]
pub struct ParsingContext<'output> {
    pub game: &'output Game
}
impl<'output> ParsingContext<'output> {
    pub fn new(game: &'output Game) -> Self {
        Self {
            game
        }
    }
}

#[allow(dead_code)]
pub(super) fn debugger<'output, E: ParseError<&'output str> + Debug, F: Parser<&'output str, Output = O, Error = E>, O: Debug>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    let mut r = parser;
    move |i| {
        match r.parse(i) {
            r @ Err(_) => dbg!(r),
            o => o 
        }
    }
}

/// Discare whitespace around a child parser
pub(super) fn strip<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    delimited(multispace0, parser, multispace0)
}

/// Discards \<strong>\</strong> tags and whitespace from around the child parser.
pub(super) fn bold<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    strip(delimited(tag("<strong>"), parser, tag("</strong>")))
}
/// Discards whitespace and a terminating full stop from around the child parser.
pub(super) fn sentence<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    delimited(space0, parser, tag("."))
}

/// Discards whitespace and a terminating exclamation mark from around the child parser.
pub(super) fn exclamation<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    strip(terminated(parser, tag("!")))
}

/// A single group of alphanumeric characters, discarding whitespace on either side
pub(super) fn word(s: &str) -> IResult<&str, &str> {
    strip(take_while(AsChar::is_alphanum)).parse(s)
}

/// As the tag combinator, but discards whitespace on either side.
pub(super) fn s_tag<'output, Error: ParseError<&'output str> + Debug>(tag_str: &'output str) -> impl Parser<&'output str, Output = &'output str, Error = Error> {
    strip(tag(tag_str))
}

/// n groups of space-separated aphamuric characters, outputed as a single str. Discards whitespace on either side
pub(super) fn words<'output>(n: usize) -> impl Parser<&'output str, Output = &'output str, Error = Error<'output>> {
    strip(recognize((take_while(AsChar::is_alphanum), count((space1, take_while(AsChar::is_alphanum)), n-1))))
}

/// A type of fair ball, e.g. "ground ball"
pub(super) fn fair_ball_type(i: &str) -> IResult<&str, FairBallType> {
    alt((
        words(2).map_res(FairBallType::try_from),
        word.map_res(FairBallType::try_from),
    )).parse(i)
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

/// A destination for a fair ball, e.g. "the shortstop"
pub(super) fn destination(i: &str) -> IResult<&str, FairBallDestination> {
    words(2).map_res(FairBallDestination::try_from)
        .parse(i)
}

/// The acronym for a player's position, e.g. "SP"
pub(super) fn position(i: &str) -> IResult<&str, Position> {
    word.map_res(Position::try_from).parse(i)
}

/// A foul type, e.g. "Ball"
pub(super) fn foul_type(i: &str) -> IResult<&str, FoulType> {
    word.map_res(FoulType::try_from).parse(i)
}

/// Top or bottom of the inning, e.g. "top"
pub(super) fn top_or_bottom(i: &str) -> IResult<&str, TopBottom> {
    word.map_res(TopBottom::try_from).parse(i)
}

/// A strike type, e.g. "Swinging"
pub(super) fn strike_type(i: &str) -> IResult<&str, StrikeType> {
    word.map_res(StrikeType::try_from).parse(i)
}

/// A list of fielders involved in a catch, e.g. "P Niblet Hornsby to 1B Geo Kerr to 3B Joffrey Nishida"
pub(super) fn fielders_eof(input: &str) -> IResult<&str, Vec<PositionedPlayer<&str>>> {
    alt((
        (
            many1(parse_terminated(" to ").and_then(positioned_player_eof)),
            rest.and_then(positioned_player_eof)
        ).map(|(mut fielders, last)| {
            fielders.push(last);
            fielders
        }),
        parse_terminated(" unassisted").and_then(positioned_player_eof).map(|fielder| vec![fielder])
    ))
    .parse(input)
}

/// A team's emoji and name, e.g. "\ud83d\udc2f Antioch Royal Tigers".
pub(super) fn team_emoji_and_name<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = (&'output str, &'output str), Error = Error<'output>> + 'parse {
    (strip(take_till(AsChar::is_space)), team_name(parsing_context))
}

/// A single team's name, obtained by matching the known team names in the context. e.g. "Antioch Royal Tigers"
pub(super) fn team_name<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = &'output str, Error = Error<'output>> + 'parse {
    strip(move |i: &'output str| {
        for name in [parsing_context.game.home_team_name.as_str(), parsing_context.game.away_team_name.as_str()] {
            let name_len = name.input_len();
            if i.compare(name) == CompareResult::Ok {
                return Ok((&i[name_len..], &i[..name_len]))
            }
        }
        IResult::Err(nom::Err::Error(VerboseError::from_error_kind(i, ErrorKind::Tag)))
    })
}

/// A distance a batter runs, e.g. "singles"
pub(super) fn distance(i: &str) -> IResult<&str, Distance> {
    word.map_res(Distance::from_str).parse(i)
}

/// A base, e.g. "first". Case insensitive
pub(super) fn base(i: &str) -> IResult<&str, Base> {
    word.map_res(Base::try_from).parse(i)
}

/// Sometimes bases get called e.g. "1B" instead.
pub(super) fn base_name_variants(i: &str) -> IResult<&str, BaseNameVariants> {
    alt((
        words(2).map_res(BaseNameVariants::try_from),
        word.map_res(BaseNameVariants::try_from),
    )).parse(i)
}

/// A type fielding error e.g. "throwing". Case insensitive
pub(super) fn fielding_error_type(i: &str) -> IResult<&str, FieldingErrorType> {
    word.map_res(FieldingErrorType::from_str).parse(i)
}

/// A single instance of an out, e.g. "Franklin Shoebill out at home"
pub(super) fn out(input: &str) -> IResult<&str, RunnerOut<&str>> {
    (
        parse_terminated(" out at "),
        base_name_variants
    )
    .map(|(player, base)| RunnerOut { runner: player, base })
    .parse(input)
}

/// A single instance of a runner scoring, e.g. "<bold>Franklin Shoebill scores!</bold>"
pub(super) fn scores_sentence(input: &str) -> IResult<&str, &str> {
    bold(exclamation(parse_terminated(" scores")))
    .parse(input)
}

// A single instance of a runner advancing, e.g. "Franklin shoebill to third base."
pub fn runner_advance_sentence(input: &str) -> IResult<&str, RunnerAdvance<&str>> {
    sentence((parse_terminated(" to "), terminated(base, s_tag("base"))))
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

/// A play from a double play (either an out or an error). Will consume all characters given to it.
pub(super) fn play_eof(input: &str) -> IResult<&str, Play<&str>> {
    alt((
        out.map(|out| Play::Out { out }),
        separated_pair(fielding_error_type, s_tag("error by"), name_eof)
            .map(|(error, fielder)| Play::Error { fielder, error })
    ))
    .parse(input)
}

pub(super) fn base_steal_sentence(input: &str) -> IResult<&str, BaseSteal<&str>> {
    let home_steal = bold(exclamation(parse_terminated(" steals home")))
    .map(|runner| BaseSteal { runner, base:Base::Home, caught:false });

    let successful_steal = exclamation((parse_terminated(" steals "), terminated(base, s_tag("base"))))
    .map(|(runner, base)| BaseSteal {runner, base, caught: false });

    let caught_stealing_home = sentence(parse_terminated(" is caught stealing home"))
    .map(|runner| BaseSteal {runner, base:Base::Home, caught: true });

    let caught_stealing = sentence((parse_terminated(" is caught stealing "), terminated(base, s_tag("base"))))
    .map(|(runner, base)| BaseSteal {runner, base, caught: true });

    alt((
        caught_stealing,
        successful_steal,
        caught_stealing_home,
        home_steal,
    )).parse(input)
}

pub(super) fn score_update_sentence(i: &str) -> IResult<&str, (u8, u8)> {
    sentence(strip(separated_pair(u8, s_tag("-"), u8)))
    .parse(i)
}

pub(super) fn switch_pitcher_sentences(i: &str) -> IResult<&str, ((Position, &str), (Position, &str))> {
    (
        terminated((position, take_until(" is leaving the game")), s_tag("is leaving the game.")),
        terminated((position, take_until(" takes the mound")), s_tag("takes the mound."))
    )
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
                panic!("infinite depth oh no")
            }
        }
    }
}

pub fn parse_and<'output, F, O>(
    mut f: F,
    delimiter: &'output str,
  ) -> impl Parser<&'output str, Output = (&'output str, O), Error = Error<'output>>
  where
    F: Parser<&'output str, Output = O, Error = Error<'output>>,
{
    move |input: &'output str| {
        let mut i = 0usize;
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

pub(super) fn positioned_player_eof(input: &str) -> IResult<&str, PositionedPlayer<&str>> {
    (position, name_eof)
    .map(|(position, name)| PositionedPlayer { name, position })
    .parse(input)
}

pub(super) fn name_eof(input: &str) -> IResult<&str, &str> {
    verify(rest,  |name: &str| !name.contains(","))
    .parse(input)
}

pub(super) fn sentence_eof<'output, E: ParseError<&'output str> + Debug, F: Parser<&'output str, Output = O, Error = E>, O: Debug>(mut parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    all_consuming(sentence(move |input: &'output str| {
        take(input.chars().count()-1).and_then(|i| parser.parse(i))
        .parse(input)
    }))
}

pub(super) fn emoji_and_name_eof(input: &str) -> IResult<&str, (&str, &str)> {
    verify(rest,  |name: &str| !name.contains(","))
    .and_then((strip(take_till(AsChar::is_space)), rest))
    .parse(input)
}
