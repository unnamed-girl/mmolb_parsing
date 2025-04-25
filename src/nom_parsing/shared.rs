use std::{collections::HashSet, str::FromStr};

use nom::{branch::alt, bytes::complete::{tag, take_while}, character::complete::{space0, space1}, combinator::{cut, opt, recognize}, error::{context, ErrorKind, ParseError}, multi::{count, separated_list1}, sequence::{delimited, separated_pair, terminated}, AsChar, Parser};
use nom_language::error::VerboseError;

use crate::enums::{Base, FielderError, FoulType, HitDestination, HitType, Position, StrikeType};

pub(super) type Error<'a> = VerboseError<&'a str>;
pub(super) type IResult<'a, I, O> = nom::IResult<I, O, Error<'a>>;

#[derive(Clone, Debug)]
pub struct ParsingContext {
    pub player_names: HashSet<String>
}
impl ParsingContext {
    pub fn new() -> Self {
        Self {
            player_names: HashSet::new()
        }
    }
}

/// Discards \<strong>\</strong> tags and whitespace from around the child parser.
pub(super) fn bold<'a, E: ParseError<&'a str>, F: Parser<&'a str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'a str, Output =  O, Error = E> {
    delimited(stripped_tag("<strong>"), parser, stripped_tag("</strong>"))
}
/// Discards whitespace and a terminating full stop from around the child parser.
pub(super) fn sentence<'a, E: ParseError<&'a str>, F: Parser<&'a str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'a str, Output =  O, Error = E> {
    delimited(space0, parser, stripped_tag("."))
}

/// Discards whitespace and a terminating exclamation mark from around the child parser.
pub(super) fn exclamation<'a, E: ParseError<&'a str>, F: Parser<&'a str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'a str, Output =  O, Error = E> {
    delimited(space0, parser, stripped_tag("!"))
}

/// A single group of alphanumeric, discarding whitespace on either side
pub(super) fn word(s: &str) -> IResult<&str, &str> {
    delimited(space0, take_while(AsChar::is_alphanum), space0).parse(s)
}

/// As the tag combinator, but discards whitespace on either side.
pub(super) fn stripped_tag<'a, Error: ParseError<&'a str>>(tag_str: &'a str) -> impl Parser<&'a str, Output = &'a str, Error = Error> {
    delimited(space0, tag(tag_str), space0)
}

/// n groups of space-separated aphamuric characters, outputed as a single str. Discards whitespace on either side
pub(super) fn words<'a>(n: usize) -> impl Parser<&'a str, Output = &'a str, Error = Error<'a>> {
        delimited(space0, recognize((take_while(AsChar::is_alphanum), count((space1, take_while(AsChar::is_alphanum)), n-1))), space0)
}

/// A single player's name, obtained by matching the known player names in the context.
pub(super) fn player_name<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = &'a str, Error = Error<'a>> {
    move |i: &'a str| {
        for name in parsing_context.player_names.iter() {
            match stripped_tag(name.as_str()).parse(i) {
                r @ IResult::Ok(_) => return r,
                _ => ()
            }
        }
        IResult::Err(nom::Err::Error(VerboseError::from_error_kind(i, ErrorKind::Tag)))
    }
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

/// A strike type, e.g. "Swinging"
pub(super) fn strike_type(i: &str) -> IResult<&str, StrikeType> {
    word.map_res(StrikeType::try_from).parse(i)
}

/// A list of fielders involved in a catch, e.g. "P Niblet Hornsby to 1B Geo Kerr to 3B Joffrey Nishida"
pub(super) fn fielders<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<(Position, String)>, Error = Error<'a>> {
    terminated(separated_list1(stripped_tag("to"), fielder(parsing_context)),
            opt(stripped_tag("unassisted")))
}

/// Context name for parsing a fielder name.
pub const EXTRACT_FIELDER_NAME:&'static str = "fielder_name";

/// A position + a fielder's name.
///
/// It is possible that a fielder is a newly generated player (e.g. in case of Relegation.) - so failures to extract a name are irrecoverable failure.
/// (Only after position is successfully parsed, so this *should* only apply where there is actually a Relegation-related error).
/// Use const EXTRACT_FIELDER_NAME to match on the context of this failure.
pub(super) fn fielder<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = (Position, String), Error = Error<'a>> {
    (position, context(EXTRACT_FIELDER_NAME, cut(player_name(parsing_context))).map(String::from))
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
pub(super) fn out<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = (&'a str, Base), Error = Error<'a>> {
    terminated(separated_pair(player_name(parsing_context), stripped_tag("out at"), alt((base, base_slang))), opt(stripped_tag("base")))
}