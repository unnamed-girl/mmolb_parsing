use nom::{branch::alt, bytes::{complete::{take_till, take_until}, tag}, character::complete::{digit1, u8}, combinator::{all_consuming, cut, fail, opt, rest, value}, error::context, multi::{many0, many1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Finish, Parser};

use crate::{enums::{EventType, HitType, HomeAway}, game::Event, parsed_event::{Play, PositionedPlayer, StartOfInningPitcher}, ParsedEvent};

use super::{shared::{base_steal_sentence, bold, destination, distance, exclamation, fielders, fielding_error_type, foul_type, hit_type, hit_type_verb_name, ordinal_suffix, out_sentence, play_sentence, player_name, position, positioned_player, s_tag, score_update_sentence, scores_and_advances, scores_sentence, sentence, strike_type, strip, switch_pitcher_sentences, team_emoji_and_name, top_or_bottom, Error}, ParsingContext};

pub fn parse_event<'output, 'parse>(event: &'output Event, parsing_context: &'parse ParsingContext<'output>) -> Result<ParsedEvent<&'output str>, Error<'output>> {
    match event.event {
        EventType::PitchingMatchup => pitching_matchup(parsing_context).parse(&event.message),
        EventType::MoundVisit => mound_visit(parsing_context).parse(&event.message),
        EventType::GameOver => game_over(parsing_context).parse(&event.message),
        EventType::Field => field(parsing_context).parse(&event.message),
        EventType::HomeLineup => lineup(HomeAway::Home, parsing_context).parse(&event.message),
        EventType::Recordkeeping => record_keeping(parsing_context).parse(&event.message),
        EventType::LiveNow => live_now(parsing_context).parse(&event.message),
        EventType::InningStart => inning_start(parsing_context).parse(&event.message),
        EventType::Pitch => pitch(parsing_context).parse(&event.message),
        EventType::AwayLineup => lineup(HomeAway::Away, parsing_context).parse(&event.message),
        EventType::InningEnd => inning_end(parsing_context).parse(&event.message),
        EventType::PlayBall => play_ball(parsing_context).parse(&event.message),
        EventType::NowBatting => now_batting(parsing_context).parse(&event.message)
    }.finish().map(|(_, o)| o)
}

fn game_over<'output, 'parse>(_parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Game Over", all_consuming(
        tag("\"GAME OVER.\""))
    ).map(|_| ParsedEvent::GameOver)
}

fn record_keeping<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Record Keeping", all_consuming((
        sentence(separated_pair(
            team_emoji_and_name(parsing_context),
            s_tag("defeated"),
            team_emoji_and_name(parsing_context),
        )),
        preceded(s_tag("Final score:"), separated_pair(u8, s_tag("-"), u8))
    ).map(|(((winning_team_emoji, winning_team_name), (losing_team_emoji, losing_team_name)), (winning_score, losing_score))|
        ParsedEvent::Recordkeeping { winning_team_emoji, winning_team_name, losing_team_emoji, losing_team_name, winning_score, losing_score }
    )))
}

fn inning_end<'output, 'parse>(_parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Inning End", all_consuming(
        sentence((
            preceded(s_tag("End of the"), top_or_bottom),
            delimited(s_tag("of the"), u8, ordinal_suffix)
        )).map(|(side, number)| ParsedEvent::InningEnd { number, side })
    ))
}

fn play_ball<'output, 'parse>(_parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Play Ball", all_consuming(
        tag("\"PLAY BALL.\"")
    ).map(|_| ParsedEvent::PlayBall))
}

fn now_batting<'output, 'parse>(_parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Now Batting", all_consuming(alt((
        (
            delimited(s_tag("Now batting:"), take_until(" ("), s_tag("(")),
             terminated(take_until(")"), s_tag(")"))
        ).map(|(batter, stats)| ParsedEvent::NowBatting { batter, stats: Some(stats) }),
        preceded(s_tag("Now batting: "), rest)
            .map(|batter| ParsedEvent::NowBatting { batter, stats:None }),
    ))))
}

fn field<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    let batter_to_base = sentence((player_name(parsing_context), distance, delimited(s_tag("on a"), hit_type, s_tag("to")), positioned_player(parsing_context)))
    .and(scores_and_advances(parsing_context))
    .map(|((batter, distance, hit, fielder), (scores, advances))| {
        ParsedEvent::BatterToBase { batter, distance, hit, fielder, scores, advances }
    });

    let homers = bold(exclamation((player_name(parsing_context), delimited(s_tag("homers on a"), hit_type, s_tag("to")), destination)))
    .and(many0(scores_sentence(parsing_context)))
    .map(|((batter, hit, destination), scores)| ParsedEvent::Homer { batter, hit, destination, scores });
    
    let grand_slam = bold(exclamation((player_name(parsing_context), delimited(s_tag("hits a grand slam on a"), hit_type, s_tag("to")), destination)))
    .and(many0(scores_sentence(parsing_context)))
    .map(|((batter, hit, destination), scores)| ParsedEvent::GrandSlam { batter, hit, destination, scores });

    let caught_out = sentence((
        player_name(parsing_context),
        terminated(hit_type_verb_name, s_tag("out")),
        opt(s_tag("on a sacrifice fly")).map(|sacrifice| sacrifice.is_some()),
        preceded(s_tag("to"), positioned_player(parsing_context)),
    ))
    .and(scores_and_advances(parsing_context))
    .and(opt(bold(exclamation(s_tag("Perfect catch")))).map(|perfect| perfect.is_some()))
    .map(|(((batter, hit, sacrifice, catcher), (scores, advances)), perfect)| ParsedEvent::CaughtOut { batter, hit, catcher, sacrifice, scores, advances, perfect });

    let grounded_out = sentence(separated_pair(
        player_name(parsing_context),
        s_tag("grounds out"),
        alt((
            preceded(s_tag("to"), positioned_player(parsing_context)).map(|fielder| vec![fielder]),
            preceded(s_tag(","), fielders(parsing_context))
        ))
    ))
    .and(scores_and_advances(parsing_context))
    .map(|((batter, fielders), (scores, advances))| ParsedEvent::GroundedOut { batter, fielders, scores, advances });

    let forced_out = sentence((
        player_name(parsing_context),
        terminated(hit_type_verb_name, s_tag("into a force out,")),
        fielders(parsing_context)
    ))
    .and(out_sentence(parsing_context))
    .and(scores_and_advances(parsing_context))
    .map(|(((batter, hit, fielders), out), (scores, advances))| 
        ParsedEvent::ForceOut { batter, hit, fielders, out, scores, advances }
    );

    let reaches_on_fielders_choice_out =  sentence(separated_pair(player_name(parsing_context), s_tag("reaches on a fielder's choice out,"), fielders(parsing_context)))
    .and(out_sentence(parsing_context))
    .and(scores_and_advances(parsing_context))
    .map(|(((batter, fielders), out), (scores, advances))| {
        ParsedEvent::FieldersChoice { batter, fielders, play: Play::Out { out }, scores, advances }
    });

    let reaches_on_fielders_choice_error = sentence(separated_pair(player_name(parsing_context), s_tag("reaches on a fielder's choice, fielded by"), positioned_player(parsing_context)))
    .and(scores_and_advances(parsing_context))
    .and(sentence(separated_pair(fielding_error_type, s_tag("error by"), player_name(parsing_context))))
    .map(|(((batter, fielder), (scores, advances)), (error, error_fielder))| {
        ParsedEvent::FieldersChoice {batter,  fielders: vec![fielder], play: Play::Error { fielder: error_fielder, error }, scores, advances }
    });

    let reaches_on_error = sentence((player_name(parsing_context), delimited(s_tag("reaches on a"), fielding_error_type, s_tag("error by")), positioned_player(parsing_context)))
    .and(scores_and_advances(parsing_context))
    .map(|((batter, error, fielder), (scores, advances))| {
        ParsedEvent::FieldingError {batter, fielder, error, scores, advances }
    });

    let double_play = sentence((
        player_name(parsing_context),
        alt((value(HitType::GroundBall, s_tag("grounded")), hit_type_verb_name)),
        delimited(s_tag("into a"), opt(s_tag("sacrifice")).map(|s| s.is_some()), s_tag("double play,")),
        fielders(parsing_context)
    ))
    .and(cut(
        (
                many1(play_sentence(parsing_context)),
                scores_and_advances(parsing_context)
            )
    ))
    .map(|((batter, hit, sacrifice, fielders), (plays, (scores, advances)))| 
        ParsedEvent::DoublePlay {batter, hit, fielders, plays, scores, advances, sacrifice }
    );

    let fielding_outcomes = alt((
        batter_to_base,
        homers,
        grand_slam,
        caught_out,
        grounded_out,
        forced_out,
        reaches_on_fielders_choice_out,
        reaches_on_fielders_choice_error,
        reaches_on_error,
        double_play,
        fail()
    ));

    context("Field event", all_consuming(fielding_outcomes))
}

fn pitch<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    let hit = sentence((
        player_name(parsing_context), 
        delimited(s_tag("hits a"), hit_type, s_tag("to")),
        destination
    ))
    .map(|(batter, hit, destination)| ParsedEvent::Hit { batter, hit, destination });

    let struck_out = (
        opt(sentence(preceded(s_tag("Foul"), foul_type))),
        sentence(separated_pair(
            player_name(parsing_context), 
            s_tag("struck out"), 
            strike_type)
    ))
    .and(many0(base_steal_sentence(parsing_context)))
    .map(|((foul, (batter, strike)), steals)|
        ParsedEvent::StrikeOut { foul, batter, strike, steals }
    );

    let hit_by_pitch = sentence(terminated(player_name(parsing_context), s_tag("was hit by the pitch and advances to first base")))
    .and(scores_and_advances(parsing_context))
    .map(|(batter, (scores, advances))| ParsedEvent::HitByPitch { batter, scores, advances });

    let walks = preceded(
        sentence(s_tag("Ball 4")),
        sentence(terminated(player_name(parsing_context), s_tag("walks")))
    ).and(scores_and_advances(parsing_context))
    .map(|(batter, (scores, advances))| ParsedEvent::Walk { batter, scores, advances });
 
    let ball = preceded(sentence(s_tag("Ball")), score_update_sentence)
    .and(many0(base_steal_sentence(parsing_context)))
    .map(|(count, steals)| ParsedEvent::Ball { steals, count });

    let strike = sentence(preceded(s_tag("Strike,"), strike_type))
    .and(cut((score_update_sentence, many0(base_steal_sentence(parsing_context)))))
    .map(|(strike, (count, steals))| ParsedEvent::Strike { strike, steals, count });

    let foul = sentence(preceded(s_tag("Foul"), foul_type))
    .and(score_update_sentence)
    .and(many0(base_steal_sentence(parsing_context)))
    .map(|((foul, count), steals)| ParsedEvent::Foul { foul, steals, count });

    let pitch_options = alt((
        struck_out,
        walks,
        ball,
        strike,
        foul,
        hit,
        hit_by_pitch,
        fail()
    ));

    context("Pitch event", all_consuming(pitch_options))
}

/// Parse the home and away pitchers from the pitching matchup message.
fn pitching_matchup<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Pitching matchup", all_consuming(
        ( 
            team_emoji_and_name(parsing_context), 
            terminated(take_until(" vs. "),s_tag("vs.")) , 
            team_emoji_and_name(parsing_context), 
            rest
        ).map(|((away_team_emoji, away_team_name), away_pitcher, (home_team_emoji, home_team_name) , home_pitcher)| ParsedEvent::PitchingMatchup { home_team_emoji, home_team_name, home_pitcher, away_team_emoji, away_team_name, away_pitcher })
    ))
}

fn lineup<'output, 'parse>(side: HomeAway, _parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Lineup", all_consuming(
        many1(delimited(
            (digit1, s_tag(".")), 
            (position, take_until("<br>")),
             s_tag("<br>")
            ).map(|(position, player)| PositionedPlayer {position, name: player})
            )
    ).map(move |players| ParsedEvent::Lineup {side, players }))
}

fn inning_start<'output, 'parse>(_parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    let keep_pitcher = sentence(terminated((strip(take_till(AsChar::is_space)), take_until(" pitching")), s_tag("pitching")))
    .map(|(emoji, name)| StartOfInningPitcher::Same { emoji, name });

    let swap_pitcher = switch_pitcher_sentences
    .map(|((leaving_position, leaving_pitcher), (arriving_position, arriving_pitcher))| StartOfInningPitcher::Different { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });

    let start_inning = (
        sentence((
            preceded(s_tag("Start of the"), top_or_bottom), 
            delimited(s_tag("of the"), u8, ordinal_suffix))),
        sentence(terminated((strip(take_till(AsChar::is_space)), take_until(" batting")), s_tag("batting")))
    );

    let pitcher_status = alt ((
        keep_pitcher,
        swap_pitcher
    ));
    context("Inning Start", 
        all_consuming((start_inning, pitcher_status))
        .map(|(((side, number), (batting_emoji, batting_team)), pitcher_status)| 
            ParsedEvent::InningStart { number, side, batting_team_emoji: batting_emoji, batting_team_name: batting_team, pitcher_status }
        )
    )
}

fn mound_visit<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    let mound_visit_options = alt((
        sentence(preceded(s_tag("The"), terminated(team_emoji_and_name(parsing_context), s_tag("manager is making a mound visit"))))
        .map(|(emoji, team)| ParsedEvent::MoundVisit { emoji, team }),

        switch_pitcher_sentences.map(|((leaving_position, leaving_pitcher), (arriving_position, arriving_pitcher))| ParsedEvent::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher }),

        sentence(terminated(positioned_player(parsing_context), s_tag("remains in the game")))
        .map(|remaining_pitcher| ParsedEvent::PitcherRemains { remaining_pitcher }),
    ));
    
    context("Mound visit", all_consuming(mound_visit_options))
}

fn live_now<'output, 'parse>(_parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEvent<&'output str>, Error = Error<'output>> + 'parse {
    context("Live now", all_consuming(
            separated_pair((strip(take_till(AsChar::is_space)), take_until(" @ ")), s_tag("@"), (strip(take_till(AsChar::is_space)), rest))
        ).map(|((away_team_emoji, away_team_name), (home_team_emoji, home_team_name))| ParsedEvent::LiveNow { away_team_name, away_team_emoji, home_team_name, home_team_emoji })
    )
}
