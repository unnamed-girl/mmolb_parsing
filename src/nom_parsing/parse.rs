use std::iter::once;

use nom::{branch::alt, combinator::{all_consuming, fail, opt, value}, error::context, multi::{many0, many1}, sequence::{delimited, preceded, separated_pair, terminated}, Parser};

use crate::{enums::Base, ParsedEvent};

use super::{shared::{base, batter_run_distance, bold, destination, exclamation, fielder, fielders, fielding_error_type, foul_type, hit_type, hit_type_verb_name, out, player_name, sentence, strike_type, stripped_tag, word, Error}, ParsingContext};

/// Parses the full message for a Field event.
pub fn parse_field_event<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<ParsedEvent>, Error = Error<'a>> {
    // Main body of event
    let successful_hit = sentence((player_name(parsing_context), batter_run_distance, delimited(stripped_tag("on a"), hit_type, stripped_tag("to")), fielder(parsing_context)))
    .map(|(_batter, base, _hit_type, fielder)| {vec![ParsedEvent::BatterToBase { base, fielder: Some(fielder) }]});

    let homers = bold(exclamation((player_name(parsing_context), preceded(alt((stripped_tag("homers on a"), stripped_tag("hits a grand slam on a"))), hit_type), preceded(stripped_tag("to"), destination))))
        .map(|(batter, _hit_type, _destination)| vec![ParsedEvent::BatterToBase { base: Base::Home, fielder: None }, ParsedEvent::Scores { player: batter.to_string() }]);

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
    let throwing_error = sentence(separated_pair(fielding_error_type, stripped_tag("error by"), player_name(parsing_context)))
    .map(|(error, fielder)| vec![ParsedEvent::Error { fielder: fielder.to_string(), error }]);

    let extras = alt((
        runner_advance(parsing_context),
        scores(parsing_context),
        throwing_error
    ));
    
    context("Field event", all_consuming((fielding_options, many0(extras)))
        .map(|(events, extra)| events.into_iter().chain(extra.into_iter().flatten()).collect()))
}

fn scores<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<ParsedEvent>, Error = Error<'a>> {
    bold(exclamation(terminated(player_name(parsing_context), stripped_tag("scores"))))
    .map(|runner| vec![ParsedEvent::RunnerAdvance { runner: runner.to_string(), base: Base::Home, is_steal: false }, ParsedEvent::Scores { player: runner.to_string() }])
}

fn runner_advance<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<ParsedEvent>, Error = Error<'a>> {
    sentence((player_name(parsing_context), delimited(stripped_tag("to"), base, stripped_tag("base"))))
    .map(|(runner, base)| vec![ParsedEvent::RunnerAdvance { runner: runner.to_string(), base, is_steal: false }])
}

pub fn parse_pitch_event<'a>(parsing_context: &'a ParsingContext) -> impl Parser<&'a str, Output = Vec<ParsedEvent>, Error = Error<'a>> {
    let hit = sentence((player_name(parsing_context), delimited(stripped_tag("hits a"), hit_type, stripped_tag("to")), destination))
    .map(|(_batter, hit_type, destination)| vec![ParsedEvent::Hit { hit_type, destination }]);

    let struck_out = (opt(sentence(preceded(stripped_tag("Foul"), foul_type))), sentence(separated_pair(player_name(parsing_context), stripped_tag("struck out"), strike_type)))
    .map(|(_foul, (batter, strike_type))| vec![ParsedEvent::Strike { strike_type }, ParsedEvent::Out { player: batter.to_string(), fielders: Vec::new(), perfect_catch: false }]);

    let hit_by_pitch = sentence(terminated(player_name(parsing_context), stripped_tag("was hit by the pitch and advances to first base")))
    .map(|_batter| vec![ParsedEvent::HitByPitch]);

    let walks = preceded(sentence(stripped_tag("Ball 4")), sentence(terminated(player_name(parsing_context), stripped_tag("walks"))))
    .map(|_batter| vec![ParsedEvent::Ball, ParsedEvent::Walk]);

    let boring_pitch_outcomes = terminated(alt((
        sentence(stripped_tag("Ball")).map(|_| vec![ParsedEvent::Ball]),
        sentence(preceded(stripped_tag("Strike,"), strike_type)).map(|strike_type| vec![ParsedEvent::Strike { strike_type }]),
        sentence(preceded(stripped_tag("Foul"), foul_type)).map(|foul_type| vec![ParsedEvent::Foul { foul_type }]),
    )), sentence(separated_pair(word, stripped_tag("-"), word)));

    let pitch_options = alt((
        struck_out,
        walks,
        boring_pitch_outcomes,
        hit,
        hit_by_pitch,
        fail()
    ));

    let home_steal = bold(exclamation(terminated(player_name(parsing_context), stripped_tag("steals home"))))
    .map(|runner| vec![ParsedEvent::RunnerAdvance { runner: runner.to_string(), base:Base::Home, is_steal: true }, ParsedEvent::Scores { player: runner.to_string() }]);

    let successful_steal = exclamation((player_name(parsing_context), delimited(stripped_tag("steals"), base, stripped_tag("base"))))
    .map(|(runner, base)| vec![ParsedEvent::RunnerAdvance { runner: runner.to_string(), base, is_steal: true }]);

    let caught_stealing = sentence((player_name(parsing_context), delimited(stripped_tag("is caught stealing"), base, opt(stripped_tag("base")))))
    .map(|(runner, _base)| vec![ParsedEvent::Out { player: runner.to_string(), fielders: Vec::new(), perfect_catch: false }]);

    let extras = alt((
        home_steal,
        successful_steal,
        caught_stealing,
        runner_advance(parsing_context),
        scores(parsing_context)
    ));

    context("Pitch event", all_consuming((pitch_options, many0(extras)))
        .map(|(events, extra)| events.into_iter().chain(extra.into_iter().flatten()).collect()))
}