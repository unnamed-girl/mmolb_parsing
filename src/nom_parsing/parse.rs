use std::iter::once;

use nom::{branch::alt, bytes::complete::take_until, character::complete::digit1, combinator::{all_consuming, fail, opt, rest}, error::context, multi::{many0, many1}, sequence::{delimited, preceded, separated_pair, terminated}, Parser};

use crate::{enums::{Base, Distance, FoulType, Position, Side}, ParsedEvent};

use super::{shared::{base, batter_run_distance, bold, destination, exclamation, fielders, fielding_error_type, fly_type_verb_name, foul_type, hit_type, hit_type_verb_name, ordinal_suffix, out, player_name, position, position_and_name, s_tag, sentence, strike_type, team_emoji, team_emoji_and_name, top_or_bottom, word, Error, IResult}, ParsingContext};

/// Parses the full message for a Field event.
pub fn parse_field_event<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    // Main body of event
    let successful_hit = sentence((player_name(parsing_context), batter_run_distance, delimited(s_tag("on a"), hit_type, s_tag("to")), position_and_name(parsing_context)))
    .map(|(batter, distance, _hit_type, fielder)| {vec![ParsedEvent::BatterToBase { batter, distance, fielder: Some(fielder) }]});

    let homers = bold(exclamation((player_name(parsing_context), delimited(alt((s_tag("homers on a"), s_tag("hits a grand slam on a"))), hit_type, s_tag("to")), destination)))
    .map(|(batter, _hit_type, _destination)| vec![ParsedEvent::BatterToBase { batter, distance: Distance::HomeRun, fielder: None }]);

    let caught_out = sentence((
        player_name(parsing_context),
        terminated(fly_type_verb_name, s_tag("out")),
        opt(s_tag("on a sacrifice fly")).map(|sacrifice| sacrifice.is_some()),
        preceded(s_tag("to"), position_and_name(parsing_context)),
    )).and(opt(bold(exclamation(s_tag("Perfect catch")))).map(|perfect| perfect.is_some()))
    .map(|((batter, fly_type, sacrifice, catcher), perfect)| vec![ParsedEvent::CaughtOut { batter, fly: fly_type, catcher, sacrifice, perfect }]);

    let grounded_out = sentence(separated_pair(
        player_name(parsing_context),
        s_tag("grounds out"),
        alt((
            preceded(s_tag("to"), position_and_name(parsing_context)).map(|fielder| vec![fielder]),
            preceded(s_tag(","), fielders(parsing_context))
        ))
    ))
    .map(|(batter, fielders)| vec![ParsedEvent::GroundedOut { batter, runner: batter, fielders, base: Base::First, sacrifice: false}]);

    let forced_out = sentence((
        player_name(parsing_context),
        terminated(hit_type_verb_name, s_tag("into a force out,")),
        fielders(parsing_context)
    ))
    .and(many1(sentence(out(parsing_context))))
    .map(|((batter, _hit_type, fielders), outs)| 
        outs.into_iter().map(|(runner, base)| ParsedEvent::GroundedOut { batter, runner, fielders: fielders.clone(), base, sacrifice: false })
        .collect::<Vec<ParsedEvent<&'output str>>>()
    );

    let reaches_on_fielders_choice_out =  sentence(separated_pair(player_name(parsing_context), s_tag("reaches on a fielder's choice out,"), fielders(parsing_context)))
    .and(many1(sentence(out(parsing_context))))
    .map(|((batter, fielders), outs)| {
        once(ParsedEvent::BatterToBase { batter, distance: Distance::Single, fielder: None })
        .chain(outs.into_iter().map(|(runner, base)| ParsedEvent::GroundedOut { batter, runner, fielders: fielders.clone(), base, sacrifice: false }))
        .collect::<Vec<ParsedEvent<&'output str>>>()
    });

    let reaches_on_fielders_choice = sentence(separated_pair(player_name(parsing_context), s_tag("reaches on a fielder's choice, fielded by"), position_and_name(parsing_context)))
    .map(|(batter, fielder)| {
        vec![ParsedEvent::BatterToBase { batter, distance: Distance::Single, fielder: Some(fielder) }]
    });

    let reaches_on_error = sentence((player_name(parsing_context), delimited(s_tag("reaches on a"), fielding_error_type, s_tag("error by")), position_and_name(parsing_context)))
    .map(|(batter, error, (position, fielder))| {
        vec![ParsedEvent::FieldingError { fielder, error }, ParsedEvent::BatterToBase { batter, distance: Distance::Single, fielder: Some((position, fielder)) }]
    });

    let double_play = sentence((
        player_name(parsing_context),
        hit_type_verb_name,
        delimited(s_tag("into a"), opt(s_tag("sacrifice")).map(|s| s.is_some()), s_tag("double play,")),
        fielders(parsing_context)
    ))
    .and(many1(sentence(out(parsing_context))))
    .map(|((batter, _hit_type, sacrifice, fielders), outs)| 
        outs.into_iter().map(|(runner, base)| ParsedEvent::GroundedOut { batter, runner, fielders: fielders.clone(), base, sacrifice })
        .collect::<Vec<ParsedEvent<&'output str>>>()
    );

    let fielding_options = alt((
        successful_hit,
        homers,
        caught_out,
        grounded_out,
        forced_out,
        reaches_on_fielders_choice_out,
        reaches_on_fielders_choice,
        reaches_on_error,
        double_play,
        fail()
    ));


    // Additional sentences
    let throwing_error = sentence(separated_pair(fielding_error_type, s_tag("error by"), player_name(parsing_context)))
    .map(|(error, fielder)| vec![ParsedEvent::FieldingError { fielder, error }]);

    let extras = alt((
        runner_advance(parsing_context),
        scores(parsing_context),
        throwing_error
    ));
    
    context("Field event", all_consuming((fielding_options, many0(extras)))
        .map(|(events, extra)| events.into_iter().chain(extra.into_iter().flatten()).collect()))
}

fn scores<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    bold(exclamation(terminated(player_name(parsing_context), s_tag("scores"))))
    .map(|runner| vec![ParsedEvent::Advance { runner: runner, base: Base::Home }])
}

fn runner_advance<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>)-> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    sentence((player_name(parsing_context), delimited(s_tag("to"), base, s_tag("base"))))
    .map(|(runner, base)| vec![ParsedEvent::Advance { runner, base }])
}

pub fn parse_pitch_event<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    let hit = sentence((
        player_name(parsing_context), 
        delimited(s_tag("hits a"), hit_type, s_tag("to")),
        destination
    ))
    .map(|(batter, hit, destination)| vec![ParsedEvent::Hit { batter, hit, destination }]);

    let struck_out = (
        opt(sentence(s_tag("Foul tip"))).map(|s| s.is_some()),
        sentence(separated_pair(
            player_name(parsing_context), 
            s_tag("struck out"), 
            strike_type)
    ))
    .map(|(foul_tip, (batter, strike))| {
        if foul_tip {
            vec![ParsedEvent::Foul { foul: FoulType::Tip }, ParsedEvent::StrikeOut { batter }]
        } else {
            vec![ParsedEvent::Strike { strike }, ParsedEvent::StrikeOut { batter }]
        }
    });

    let hit_by_pitch = sentence(terminated(player_name(parsing_context), s_tag("was hit by the pitch and advances to first base")))
    .map(|batter| vec![ParsedEvent::HitByPitch {batter}]);

    let walks = preceded(
        sentence(s_tag("Ball 4")),
        sentence(terminated(player_name(parsing_context), s_tag("walks")))
    )
    .map(|batter| vec![ParsedEvent::Ball, ParsedEvent::Walk {batter}]);

    let boring_pitch_outcomes = terminated(alt((
        sentence(s_tag("Ball")).map(|_| vec![ParsedEvent::Ball]),
        sentence(preceded(s_tag("Strike,"), strike_type)).map(|strike| vec![ParsedEvent::Strike { strike }]),
        sentence(preceded(s_tag("Foul"), foul_type)).map(|foul| vec![ParsedEvent::Foul { foul }]),
    )), sentence(separated_pair(word, s_tag("-"), word)));

    let pitch_options = alt((
        struck_out,
        walks,
        boring_pitch_outcomes,
        hit,
        hit_by_pitch,
        fail()
    ));

    let home_steal = bold(exclamation(terminated(player_name(parsing_context), s_tag("steals home"))))
    .map(|runner| vec![ParsedEvent::Steal { runner: runner, base:Base::Home }]);

    let successful_steal = exclamation((player_name(parsing_context), delimited(s_tag("steals"), base, s_tag("base"))))
    .map(|(runner, base)| vec![ParsedEvent::Steal { runner: runner, base }]);

    let caught_stealing = sentence((player_name(parsing_context), delimited(s_tag("is caught stealing"), base, opt(s_tag("base")))))
    .map(|(runner, base)| vec![ParsedEvent::CaughtStealing { runner, base }]);

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

/// Parse the home and away pitchers from the pitching matchup message.
pub fn parse_pitching_matchup_event<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    context("Parse pitching matchup", all_consuming(
        separated_pair( 
            preceded(team_emoji_and_name(parsing_context), 
                    take_until(" vs. ")), 
            s_tag("vs."), 
            preceded(team_emoji_and_name(parsing_context), 
                    rest)
        ).map(|(away_pitcher, home_pitcher)| vec![ParsedEvent::PitchingMatchup { home_pitcher, away_pitcher }])
    ))
}

pub fn parse_lineup_event<'output, 'parse>(side: Side, _parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    context("Parse lineup", all_consuming(
        many1(delimited((digit1, s_tag(".")), (position, take_until("<br>")), s_tag("<br>")))
    ).map(move |players| vec![ParsedEvent::Lineup {side, players }]))
}

pub fn parse_inning_start_event<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = Vec<ParsedEvent<&'output str>>, Error = Error<'output>> + 'parse {
    let keep_pitcher = sentence(delimited(team_emoji(parsing_context), take_until(" pitching"), s_tag("pitching")));
    
    context("Inning Start", all_consuming(
        alt((
            (start_inning(parsing_context), keep_pitcher).map(|(((side, number), batting_team), pitcher)| vec![ParsedEvent::InningStart { number, side, batting_team, pitcher: Some(pitcher) }]),
            (start_inning(parsing_context), switch_pitcher_sentences).map(|(((side, number), batting_team), ((leaving_position, leaving_pitcher), (arriving_position, arriving_pitcher)))| vec![ParsedEvent::InningStart { number, side, batting_team, pitcher: None }, ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher }]),
        ))
    ))
}

fn switch_pitcher_sentences<'output>(i: &'output str) -> IResult<'output, &'output str, ((Position, &'output str), (Position, &'output str))> {
    (
        sentence(terminated((position, take_until(" is leaving the game")), s_tag("is leaving the game"))), 
        sentence(terminated((position, take_until(" takes the mound")), s_tag("takes the mound")))
    )
    .parse(i)
}

fn start_inning<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ((Side, u8), &'output str), Error = Error<'output>> + 'parse {
    (
        sentence((
            preceded(s_tag("Start of the"), top_or_bottom), 
            delimited(s_tag("of the"), digit1.map_res(str::parse::<u8>), ordinal_suffix))),
        sentence(delimited(team_emoji(parsing_context), take_until(" batting"), s_tag("batting")))
    )
}

pub fn parse_mound_visit<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    let mound_visit_options = alt((
        sentence(preceded(s_tag("The"), terminated(team_emoji_and_name(parsing_context), s_tag("manager is making a mound visit"))))
        .map(|(_emoji, team)| ParsedEvent::MoundVisit { team }),

        switch_pitcher_sentences.map(|((leaving_position, leaving_pitcher), (arriving_position, arriving_pitcher))| ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher }),

        sentence(terminated(position_and_name(parsing_context), s_tag("remains in the game")))
        .map(|_player| ParsedEvent::MoundVisitRefused),
    ));
    
    context("Mound visit", all_consuming(mound_visit_options))
}