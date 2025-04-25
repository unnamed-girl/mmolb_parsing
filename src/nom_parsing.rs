use std::{collections::HashSet, iter::once, str::FromStr};

use nom::{branch::alt, bytes::complete::{tag, take_while}, character::complete::{space0, space1}, combinator::{all_consuming, cut, fail, opt, recognize, value}, error::{context, ErrorKind, ParseError}, multi::{count, many0, many1, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Parser};
use nom_language::error::VerboseError;

use crate::{enums::{Base, FielderError, HitDestination, HitType, Position}, ParsedEvent};

type Error<'a> = VerboseError<&'a str>;
type IResult<'a, I, O> = nom::IResult<I, O, Error<'a>>;
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
fn bold<'a, E: ParseError<&'a str>, F: Parser<&'a str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'a str, Output =  O, Error = E> {
    delimited(stripped_tag("<strong>"), parser, stripped_tag("</strong>"))
}
/// Discards whitespace and a terminating full stop from around the child parser.
fn sentence<'a, E: ParseError<&'a str>, F: Parser<&'a str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'a str, Output =  O, Error = E> {
    delimited(space0, parser, stripped_tag("."))
}

/// Discards whitespace and a terminating exclamation mark from around the child parser.
fn exclamation<'a, E: ParseError<&'a str>, F: Parser<&'a str, Output = O, Error = E>, O>(parser: F) -> impl Parser<&'a str, Output =  O, Error = E> {
    delimited(space0, parser, stripped_tag("!"))
}

/// A single group of alphanumeric, discarding whitespace on either side
fn word(s: &str) -> IResult<&str, &str> {
    delimited(space0, take_while(AsChar::is_alphanum), space0).parse(s)
}

/// As the tag combinator, but discards whitespace on either side.
fn stripped_tag<'a, Error: ParseError<&'a str>>(tag_str: &'a str) -> impl Parser<&'a str, Output = &'a str, Error = Error> {
    delimited(space0, tag(tag_str), space0)
}

/// n groups of space-separated aphamuric characters, outputed as a single str. Discards whitespace on either side
fn words<'a>(n: usize) -> impl Parser<&'a str, Output = &'a str, Error = Error<'a>> {
        delimited(space0, recognize(count((space1, take_while(AsChar::is_alphanum)), n)), space0)
}

/// A type of hit, e.g. "ground ball"
fn hit_type(i: &str) -> IResult<&str, HitType> {
    alt((
        word.map_res(|a| HitType::try_from(a)),
        words(2).map_res(|a| HitType::try_from(a)),
    )).parse(i)
}

/// Verb names for hit types, e.g. "pops"
fn hit_type_verb_name(i: &str) -> IResult<&str, HitType> {
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
fn destination(i: &str) -> IResult<&str, HitDestination> {
    words(2).map_res(|a| HitDestination::try_from(a))
        .parse(i)
}

/// A single player's name, obtained by matching the known player names in the context.
fn player_name<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = &'a str, Error = Error<'a>> {
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

/// The acronym for a player's position, e.g. "SP"
fn position(i: &str) -> IResult<&str, Position> {
    word.map_res(Position::try_from).parse(i)
}

/// A list of fielders involved in a catch, e.g. "P Niblet Hornsby to 1B Geo Kerr to 3B Joffrey Nishida"
fn fielders<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<(Position, String)>, Error = Error<'a>> {
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
fn fielder<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = (Position, String), Error = Error<'a>> {
    (position, context(EXTRACT_FIELDER_NAME, cut(player_name(parsing_context))).map(String::from))
}

/// A distance a batter runs, e.g. "singles"
fn batter_run_distance(i: &str) -> IResult<&str, Base> {
    word.map_opt(|w| match w {
        "singles" => Some(Base::First),
        "doubles" => Some(Base::Second),
        "triples" => Some(Base::Third),
        _ => None
    }).parse(i)
}

/// A base, e.g. "first". Case insensitive
fn base(i: &str) -> IResult<&str, Base> {
    word.map_res(Base::try_from).parse(i)
}

/// Sometimes bases get called e.g. "1B" instead.
fn base_slang(i: &str) -> IResult<&str, Base> {
    word.map_opt(|w| match w {
        "1B" => Some(Base::First),
        "2B" => Some(Base::Second),
        "3B" => Some(Base::Third),
        _ => None
    }).parse(i)
}

/// A type fielding error e.g. "throwing". Case insensitive
fn fielding_error_type(i: &str) -> IResult<&str, FielderError> {
    word.map_res(FielderError::from_str).parse(i)
}

/// A single instance of an out, e.g. "Franklin Shoebill out at home"
fn out<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = (&'a str, Base), Error = Error<'a>> {
    terminated(separated_pair(player_name(parsing_context), stripped_tag("out at"), alt((base, base_slang))), opt(stripped_tag("base")))
}

/// Parses the full message for a Field event.
pub fn field_outcomes<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<ParsedEvent>, Error = Error<'a>> {
    // Main body of event
    let successful_hit = sentence((player_name(parsing_context), batter_run_distance, delimited(stripped_tag("on a"), hit_type, stripped_tag("to")), fielder(parsing_context)))
    .map(|(_batter, base, _, fielder)| {vec![ParsedEvent::BatterToBase { base, fielder: Some(fielder) }]});

    let homers = bold(exclamation((player_name(parsing_context), preceded(alt((stripped_tag("homers on a"), stripped_tag("hits a grand slam on a"))), hit_type), preceded(stripped_tag("to"), destination))))
        .map(|(_, _, _)| vec![ParsedEvent::BatterToBase { base: Base::Home, fielder: None }]);

    let batter_out = sentence((
        player_name(parsing_context),
        terminated(hit_type_verb_name, stripped_tag("out")),
        opt(stripped_tag("on a sacrifice fly")).map(|sacrifice| sacrifice.is_some()),
        alt((
            preceded(stripped_tag("to"), fielder(parsing_context)).map(|fielder| vec![fielder]),
            preceded(stripped_tag(","), fielders(parsing_context))
        ))
    ))
    .and(opt(bold(exclamation(stripped_tag("Perfect catch")))).map(|perfect| perfect.is_some()))
    .map(|((batter, _hit_type, _sacrifice_play, fielders), perfect)|
        vec![ParsedEvent::Out { player: batter.to_string(), fielders, perfect_catch: perfect }]
    );

    let forced_out = sentence((player_name(parsing_context), hit_type_verb_name, stripped_tag("into a force out,"), fielders(parsing_context)))
    .and(many1(sentence(out(parsing_context))))
    .map(|((_, _, _, fielders), outs)| outs.into_iter().map(|(player, _)| ParsedEvent::Out { player: player.to_string(), fielders: fielders.clone(), perfect_catch: false }).collect());

    let reaches_on_fielders_choice_out =  sentence(separated_pair(player_name(parsing_context), stripped_tag("reaches on a fielder's choice out,"), fielders(parsing_context)))
    .and(many1(sentence(out(parsing_context))))
    .map(|((_batter, fielders), outs)| {
        once(ParsedEvent::BatterToBase { base: Base::First, fielder: None }).chain(outs.into_iter().map(|(player, _)| ParsedEvent::Out { player: player.to_string(), fielders: fielders.clone(), perfect_catch: false })).collect()
    });

    let reaches_on_fielders_choice = sentence(separated_pair(player_name(parsing_context), stripped_tag("reaches on a fielder's choice, fielded by"), fielder(parsing_context)))
    .map(|(_batter, fielder)| {
        vec![ParsedEvent::BatterToBase { base: Base::First, fielder: Some(fielder) }]
    });

    let reaches_on_error = sentence((player_name(parsing_context), delimited(stripped_tag("reaches on a"), fielding_error_type, stripped_tag("error by")), fielder(parsing_context)))
    .map(|(_batter, error, fielder)| {
        vec![ParsedEvent::Error { fielder: fielder.1.clone(), error }, ParsedEvent::BatterToBase { base: Base::First, fielder: Some(fielder) }]
    });

    let double_play = sentence((
        player_name(parsing_context),
        hit_type_verb_name,
        alt((
            value(true, stripped_tag("into a sacrifice double play,")),
            value(false, stripped_tag("into a double play,")),
        )),
        fielders(parsing_context)
    ))
    .and(many1(sentence(out(parsing_context))))
    .map(|((_batter, _hit_type, _sacrifice, fielders), outs)| outs.into_iter().map(|(player, _)| ParsedEvent::Out { player: player.to_string(), fielders: fielders.clone(), perfect_catch: false }).collect());

    let fielding_options = alt((
        successful_hit,
        homers,
        batter_out,
        forced_out,
        reaches_on_fielders_choice_out,
        reaches_on_fielders_choice,
        reaches_on_error,
        double_play,
        fail()
    ));


    // Additional sentences
    let runner_advance = sentence((player_name(parsing_context), delimited(stripped_tag("to"), base, stripped_tag("base"))))
    .map(|(runner, base)| vec![ParsedEvent::RunnerAdvance { runner: runner.to_string(), base, is_steal: false }]);

    let scores = bold(exclamation(terminated(player_name(parsing_context), stripped_tag("scores"))))
    .map(|runner| vec![ParsedEvent::Scores { player: runner.to_string() }]);

    let throwing_error = sentence(separated_pair(fielding_error_type, stripped_tag("error by"), player_name(parsing_context)))
    .map(|(error, fielder)| vec![ParsedEvent::Error { fielder: fielder.to_string(), error }]);

    let extras = alt((
        runner_advance,
        scores,
        throwing_error
    ));
    
    all_consuming((fielding_options, many0(extras)))
        .map(|(events, extra)| events.into_iter().chain(extra.into_iter().flatten()).collect())
}