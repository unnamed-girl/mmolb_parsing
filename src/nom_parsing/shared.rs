use std::{collections::HashSet, fmt::Debug, str::FromStr};

use nom::{branch::alt, bytes::complete::{tag, take_while}, character::complete::{multispace0, space1}, combinator::{cut, opt, recognize}, error::{context, ErrorKind, ParseError}, multi::{count, separated_list1}, sequence::{delimited, separated_pair, terminated}, AsChar, Compare, CompareResult, Parser};
use nom_language::error::VerboseError;

use crate::{enums::{Base, FielderError, FoulType, HitDestination, HitType, Position, Side, StrikeType}, Game};

pub(super) type Error<'a> = VerboseError<&'a str>;
pub(super) type IResult<'a, I, O> = nom::IResult<I, O, Error<'a>>;

/// Context necessary for parsing. The 'output lifetime is linked to ParsedEvents parsed in this context.
#[derive(Clone, Debug)]
pub struct ParsingContext<'output> {
    pub player_names: HashSet<&'output str>,
    pub game: &'output Game
}
impl<'output> ParsingContext<'output> {
    pub fn new(game: &'output Game) -> Self {
        Self {
            player_names: HashSet::new(),
            game
        }
    }
}

pub(super) fn debugger<'output, E: ParseError<&'output str> + Debug, F: Parser<&'output str, Output = O, Error = E>, O: Debug>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    let mut r = parser;
    move |i| {
        match r.parse(i) {
            r @ Err(_) => return dbg!(r),
            o @ _ => return o 
        }
    }
}

pub(super) fn strip<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    delimited(multispace0, parser, multispace0)
}

/// Discards \<strong>\</strong> tags and whitespace from around the child parser.
pub(super) fn bold<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    strip(delimited(tag("<strong>"), parser, tag("</strong>")))
}
/// Discards whitespace and a terminating full stop from around the child parser.
pub(super) fn sentence<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    strip(terminated(parser, tag(".")))
}

/// Discards whitespace and a terminating exclamation mark from around the child parser.
pub(super) fn exclamation<'output, E: ParseError<&'output str>, F: Parser<&'output str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'output str, Output =  O, Error = E> {
    strip(terminated(parser, tag("!")))
}

/// A single group of alphanumeric, discarding whitespace on either side
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

/// A single player's name, obtained by matching the known player names in the context.
pub(super) fn player_name<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = &'output str, Error = Error<'output>> + 'parse {
    strip(move |i: &'output str| {
        for name in parsing_context.player_names.iter() {
            let name_len = name.len();
            match i.compare(*name) {
                CompareResult::Ok => {
                    return Ok((&i[name_len..], &i[..name_len]))
                },
                _ => (),
            }
        }
        IResult::Err(nom::Err::Error(VerboseError::from_error_kind(i, ErrorKind::Tag)))
    })
}

/// A type of hit, e.g. "ground ball"
pub(super) fn hit_type(i: &str) -> IResult<&str, HitType> {
    alt((
        word.map_res(|a| HitType::try_from(a)),
        words(2).map_res(|a| HitType::try_from(a)),
    )).parse(i)
}

/// Verb names for hit types, e.g. "pops"
pub(super) fn hit_type_verb_name(i: &str) -> IResult<&str, HitType> {
    word.map_opt(|word| match word {
        "flies" => Some(HitType::FlyBall),
        "grounds" => Some(HitType::GroundBall),
        "grounded" => Some(HitType::GroundBall),
        "lines" => Some(HitType::LineDrive),
        "pops" => Some(HitType::Popup),
        _ => None
    }).parse(i)
}

/// A destination for a hit, e.g. "the shortstop"
pub(super) fn destination(i: &str) -> IResult<&str, HitDestination> {
    words(2).map_res(|a| HitDestination::try_from(a))
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
pub(super) fn top_or_bottom(i: &str) -> IResult<&str, Side> {
    word.map_opt(|s| match s {
        "top" => Some(Side::Away),
        "bottom" => Some(Side::Home),
        _ => None
    }).parse(i)
}

/// A strike type, e.g. "Swinging"
pub(super) fn strike_type(i: &str) -> IResult<&str, StrikeType> {
    word.map_res(StrikeType::try_from).parse(i)
}

/// A list of fielders involved in a catch, e.g. "P Niblet Hornsby to 1B Geo Kerr to 3B Joffrey Nishida"
pub(super) fn fielders<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<(Position, &'output str)>, Error = Error<'output>> + 'parse{
    terminated(separated_list1(s_tag("to"), position_and_name(parsing_context)),
            opt(s_tag("unassisted")))
}


pub const EXTRACT_TEAM_NAME:&'static str = "team_name";

/// A team's name and emoji - if the emoji is found, assume this is the right path and Fail if team name not found.
pub(super) fn team_emoji_and_name<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = (&'output str, &'output str), Error = Error<'output>> + 'parse {
    (team_emoji(parsing_context), context(EXTRACT_TEAM_NAME, cut(team_name(parsing_context))))
}
/// A team's emoji
pub(super) fn team_emoji<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = &'output str, Error = Error<'output>> + 'parse {
    alt((
        s_tag(&parsing_context.game.home_team_emoji),
        s_tag(&parsing_context.game.away_team_emoji)
    ))
}


/// A team's name
pub(super) fn team_name<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = &'output str, Error = Error<'output>> + 'parse {
    alt((
        s_tag(&parsing_context.game.home_team_name),
        s_tag(&parsing_context.game.away_team_name)

    ))
}

/// Context name for parsing a fielder name.
pub const EXTRACT_PLAYER_NAME:&'static str = "player_name";

/// A position + a player's name.
///
/// It is possible that a player is a newly generated player (e.g. in case of Relegation.) - so failures to extract a name are irrecoverable failure.
/// (Only after position is successfully parsed, so this *should* only apply where there is actually a Relegation-related error).
/// Use const EXTRACT_FIELDER_NAME to match on the context of this failure.
pub(super) fn position_and_name<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = (Position, &'output str), Error = Error<'output>> + 'parse {
    (position, context(EXTRACT_PLAYER_NAME, cut(player_name(parsing_context))))
}

/// A distance a batter runs, e.g. "singles"
pub(super) fn batter_run_distance(i: &str) -> IResult<&str, Base> {
    word.map_opt(|w| match w {
        "singles" => Some(Base::First),
        "doubles" => Some(Base::Second),
        "triples" => Some(Base::Third),
        _ => None
    }).parse(i)
}

/// A base, e.g. "first". Case insensitive
pub(super) fn base(i: &str) -> IResult<&str, Base> {
    word.map_res(Base::try_from).parse(i)
}

/// Sometimes bases get called e.g. "1B" instead.
pub(super) fn base_slang(i: &str) -> IResult<&str, Base> {
    word.map_opt(|w| match w {
        "1B" => Some(Base::First),
        "2B" => Some(Base::Second),
        "3B" => Some(Base::Third),
        _ => None
    }).parse(i)
}

/// A type fielding error e.g. "throwing". Case insensitive
pub(super) fn fielding_error_type(i: &str) -> IResult<&str, FielderError> {
    word.map_res(FielderError::from_str).parse(i)
}

/// A single instance of an out, e.g. "Franklin Shoebill out at home"
pub(super) fn out<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = (&'output str, Base), Error = Error<'output>> + 'parse {
    terminated(separated_pair(player_name(parsing_context), s_tag("out at"), alt((base, base_slang))), opt(s_tag("base")))
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