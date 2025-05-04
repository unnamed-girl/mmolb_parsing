use nom::{branch::alt, bytes::{complete::{take_till, take_until}, tag}, character::complete::{digit1, u8}, combinator::{all_consuming, cut, fail, opt, rest, value}, error::context, multi::{many0, many1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Finish, Parser};

use crate::{enums::{EventType, HitType, HomeAway}, game::Event, parsed_event::{Play, PositionedPlayer, StartOfInningPitcher}, ParsedEventMessage};

use super::{shared::{all_consuming_sentence_and, base_steal_sentence, bold, destination, distance, emoji_and_name_eof, exclamation, fielders_eof, fielding_error_type, foul_type, hit_type, hit_type_verb_name, name_eof, ordinal_suffix, out, parse_terminated, parse_and, play_eof, position, positioned_player_eof, s_tag, score_update_sentence, scores_and_advances, scores_sentence, sentence, sentence_eof, strike_type, strip, switch_pitcher_sentences, team_emoji_and_name, top_or_bottom, Error}, ParsingContext};

pub fn parse_event<'output>(event: &'output Event, parsing_context: &ParsingContext<'output>) -> Result<ParsedEventMessage<&'output str>, Error<'output>> {
    match event.event {
        EventType::PitchingMatchup => pitching_matchup(parsing_context).parse(&event.message),
        EventType::MoundVisit => mound_visit().parse(&event.message),
        EventType::GameOver => game_over().parse(&event.message),
        EventType::Field => field().parse(&event.message),
        EventType::HomeLineup => lineup(HomeAway::Home).parse(&event.message),
        EventType::Recordkeeping => record_keeping().parse(&event.message),
        EventType::LiveNow => live_now().parse(&event.message),
        EventType::InningStart => inning_start().parse(&event.message),
        EventType::Pitch => pitch().parse(&event.message),
        EventType::AwayLineup => lineup(HomeAway::Away).parse(&event.message),
        EventType::InningEnd => inning_end().parse(&event.message),
        EventType::PlayBall => play_ball().parse(&event.message),
        EventType::NowBatting => now_batting().parse(&event.message)
    }.finish().map(|(_, o)| o)
}

fn game_over<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Game Over", all_consuming(
        tag("\"GAME OVER.\""))
    ).map(|_| ParsedEventMessage::GameOver)
}

fn record_keeping<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Record Keeping", all_consuming((
        all_consuming_sentence_and((
            parse_terminated(" defeated ").and_then(emoji_and_name_eof),
            emoji_and_name_eof
        ),
        preceded(s_tag("Final score:"), separated_pair(u8, s_tag("-"), u8))
        )
    ).map(|(((winning_team_emoji, winning_team_name), (losing_team_emoji, losing_team_name)), (winning_score, losing_score))|
        ParsedEventMessage::Recordkeeping { winning_team_emoji, winning_team_name, losing_team_emoji, losing_team_name, winning_score, losing_score }
    )))
}

fn inning_end<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Inning End", all_consuming(
        sentence((
            preceded(s_tag("End of the"), top_or_bottom),
            delimited(s_tag("of the"), u8, ordinal_suffix)
        )).map(|(side, number)| ParsedEventMessage::InningEnd { number, side })
    ))
}

fn play_ball<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Play Ball", all_consuming(
        tag("\"PLAY BALL.\"")
    ).map(|_| ParsedEventMessage::PlayBall))
}

fn now_batting<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Now Batting", all_consuming(alt((
        (
            delimited(s_tag("Now batting:"), take_until(" ("), s_tag("(")),
             terminated(take_until(")"), s_tag(")"))
        ).map(|(batter, stats)| ParsedEventMessage::NowBatting { batter, stats: Some(stats) }),
        preceded(s_tag("Now batting: "), rest)
            .map(|batter| ParsedEventMessage::NowBatting { batter, stats:None }),
    ))))
}

fn field<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    let batter_to_base = all_consuming_sentence_and(
        (parse_and(distance, " "), preceded(s_tag("on a"), hit_type), preceded(s_tag("to"), positioned_player_eof)),
    scores_and_advances
    )
    .map(|(((batter, distance), hit, fielder), (scores, advances))| {
        ParsedEventMessage::BatterToBase { batter, distance, hit, fielder, scores, advances }
    });

    let homers = bold(exclamation((parse_terminated(" homers on a "), hit_type, preceded(s_tag("to"), destination))))
    .and(many0(scores_sentence))
    .map(|((batter, hit, destination), scores)| ParsedEventMessage::HomeRun { batter, hit, destination, scores });
    
    let grand_slam = bold(exclamation((parse_terminated(" hits a grand slam on a "), hit_type, preceded(s_tag("to"), destination))))
    .and(many0(scores_sentence))
    .map(|((batter, hit, destination), scores)| ParsedEventMessage::GrandSlam { batter, hit, destination, scores });

    let caught_out = all_consuming_sentence_and(
        (
            terminated(parse_and(hit_type_verb_name, " "), s_tag("out")),
            opt(s_tag("on a sacrifice fly")).map(|sacrifice| sacrifice.is_some()),
            preceded(s_tag("to"), positioned_player_eof),
        ),
        (scores_and_advances, opt(bold(exclamation(s_tag("Perfect catch")))).map(|perfect| perfect.is_some()))
    )
    .map(|(((batter, hit), sacrifice, catcher), ((scores, advances), perfect))| ParsedEventMessage::CaughtOut { batter, hit, catcher, sacrifice, scores, advances, perfect });

    let grounded_out = all_consuming_sentence_and(
        (
            parse_terminated(" grounds out"),
            alt((
                preceded(s_tag("to"), positioned_player_eof).map(|fielder| vec![fielder]),
                preceded(s_tag(","), fielders_eof)
            ))
        ),
        scores_and_advances
    )
    .map(|((batter, fielders), (scores, advances))| ParsedEventMessage::GroundedOut { batter, fielders, scores, advances });

    let forced_out = all_consuming_sentence_and(
        (
            parse_and(hit_type_verb_name, " "),
            preceded(s_tag("into a force out,"), fielders_eof)
        ),
        (sentence(out), scores_and_advances)
    )
    .map(|(((batter, hit), fielders), (out, (scores, advances)))| 
        ParsedEventMessage::ForceOut { batter, hit, fielders, out, scores, advances }
    );

    let reaches_on_fielders_choice_out = all_consuming_sentence_and(
        (parse_terminated(" reaches on a fielder's choice out, "), fielders_eof),
        (sentence(out), scores_and_advances)
    )
    .map(|((batter, fielders), (out, (scores, advances)))| {
        ParsedEventMessage::ReachOnFieldersChoice { batter, fielders, play: Play::Out { out }, scores, advances }
    });

    let reaches_on_fielders_choice_error = all_consuming_sentence_and(
        (parse_terminated(" reaches on a fielder's choice, fielded by "), positioned_player_eof),
        (scores_and_advances, sentence_eof(separated_pair(fielding_error_type, s_tag("error by"), name_eof)))
    )
    .map(|((batter, fielder), ((scores, advances), (error, error_fielder)))| {
        ParsedEventMessage::ReachOnFieldersChoice {batter,  fielders: vec![fielder], play: Play::Error { fielder: error_fielder, error }, scores, advances }
    });

    let reaches_on_error = all_consuming_sentence_and(
        (parse_terminated(" reaches on a "), terminated(fielding_error_type, s_tag("error by")), positioned_player_eof),
        scores_and_advances
    )
    .map(|((batter, error, fielder), (scores, advances))| {
        ParsedEventMessage::ReachOnFieldingError {batter, fielder, error, scores, advances }
    });

    let double_play_grounded = all_consuming_sentence_and((
        parse_terminated(" grounded "),
        delimited(s_tag("into a"), opt(s_tag("sacrifice")).map(|s| s.is_some()), s_tag("double play,")),
        fielders_eof
    ),
    all_consuming_sentence_and(
        play_eof,
        all_consuming_sentence_and(play_eof,scores_and_advances),
    ))
    .map(|((batter, sacrifice, fielders), (play_one, (play_two, (scores, advances))))| 
        ParsedEventMessage::DoublePlayGrounded { batter, fielders, play_one, play_two, scores, advances, sacrifice }
    );

    let double_play_caught = all_consuming_sentence_and((
        terminated(parse_and( hit_type_verb_name, " "), s_tag("into a double play,")),fielders_eof
    ),
    all_consuming_sentence_and(
        play_eof,
        scores_and_advances
    ))
    .map(|(((batter, hit), fielders), (play, (scores, advances)))| 
        ParsedEventMessage::DoublePlayCaught { batter, hit, fielders, play, scores, advances }
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
        double_play_grounded,
        double_play_caught,
        fail()
    ));

    context("Field event", all_consuming(fielding_outcomes))
}

fn pitch<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    let hit = sentence((
        parse_terminated(" hits a "), 
        hit_type,
        preceded(s_tag("to"), destination)
    ))
    .map(|(batter, hit, destination)| ParsedEventMessage::FairBall { batter, hit, destination });

    let struck_out = (
        opt(sentence(preceded(s_tag("Foul"), foul_type))),
        sentence((
            parse_terminated(" struck out "), 
            strike_type)
    ))
    .and(many0(base_steal_sentence))
    .map(|((foul, (batter, strike)), steals)|
        ParsedEventMessage::StrikeOut { foul, batter, strike, steals }
    );

    let hit_by_pitch = sentence(parse_terminated(" was hit by the pitch and advances to first base"))
    .and(scores_and_advances)
    .map(|(batter, (scores, advances))| ParsedEventMessage::HitByPitch { batter, scores, advances });

    let walks = preceded(
        sentence(s_tag("Ball 4")),
        sentence(parse_terminated(" walks"))
    ).and(scores_and_advances)
    .map(|(batter, (scores, advances))| ParsedEventMessage::Walk { batter, scores, advances });
 
    let ball = preceded(s_tag("Ball."), score_update_sentence)
    .and(many0(base_steal_sentence))
    .map(|(count, steals)| ParsedEventMessage::Ball { steals, count });

    let strike = sentence(preceded(s_tag("Strike,"), strike_type))
    .and(cut((score_update_sentence, many0(base_steal_sentence))))
    .map(|(strike, (count, steals))| ParsedEventMessage::Strike { strike, steals, count });

    let foul = sentence(preceded(s_tag("Foul"), foul_type))
    .and(score_update_sentence)
    .and(many0(base_steal_sentence))
    .map(|((foul, count), steals)| ParsedEventMessage::Foul { foul, steals, count });

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
fn pitching_matchup<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> + 'parse {
    context("Pitching matchup", all_consuming(
        ( 
            team_emoji_and_name(parsing_context), 
            terminated(take_until(" vs. "),s_tag("vs.")) , 
            team_emoji_and_name(parsing_context), 
            rest
        ).map(|((away_team_emoji, away_team_name), away_pitcher, (home_team_emoji, home_team_name) , home_pitcher)| ParsedEventMessage::PitchingMatchup { home_team_emoji, home_team_name, home_pitcher, away_team_emoji, away_team_name, away_pitcher })
    ))
}

fn lineup<'output>(side: HomeAway) -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Lineup", all_consuming(
        many1(delimited(
            (digit1, s_tag(".")), 
            (position, take_until("<br>")),
             s_tag("<br>")
            ).map(|(position, player)| PositionedPlayer {position, name: player})
            )
    ).map(move |players| ParsedEventMessage::Lineup {side, players }))
}

fn inning_start<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    let keep_pitcher = sentence((strip(take_till(AsChar::is_space)), parse_terminated(" pitching")))
    .map(|(emoji, name)| StartOfInningPitcher::Same { emoji, name });

    let swap_pitcher = switch_pitcher_sentences
    .map(|((leaving_position, leaving_pitcher), (arriving_position, arriving_pitcher))| StartOfInningPitcher::Different { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher });

    let start_inning = (
        sentence((
            preceded(s_tag("Start of the"), top_or_bottom), 
            delimited(s_tag("of the"), u8, ordinal_suffix))),
        sentence((strip(take_till(AsChar::is_space)), parse_terminated(" batting")))
    );

    let pitcher_status = alt ((
        keep_pitcher,
        swap_pitcher
    ));
    context("Inning Start", 
        all_consuming((start_inning, pitcher_status))
        .map(|(((side, number), (batting_emoji, batting_team)), pitcher_status)| 
            ParsedEventMessage::InningStart { number, side, batting_team_emoji: batting_emoji, batting_team_name: batting_team, pitcher_status }
        )
    )
}

fn mound_visit<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    let mound_visit_options = alt((
        sentence(preceded(s_tag("The"), parse_terminated(" manager is making a mound visit").and_then((strip(take_till(AsChar::is_space)), name_eof))))
        .map(|(emoji, team)| ParsedEventMessage::MoundVisit { emoji, team }),

        switch_pitcher_sentences.map(|((leaving_position, leaving_pitcher), (arriving_position, arriving_pitcher))| ParsedEventMessage::PitcherSwap { leaving_position, leaving_pitcher, arriving_position, arriving_pitcher }),

        sentence(parse_terminated(" remains in the game").and_then(positioned_player_eof))
        .map(|remaining_pitcher| ParsedEventMessage::PitcherRemains { remaining_pitcher }),
    ));
    
    context("Mound visit", all_consuming(mound_visit_options))
}

fn live_now<'output>() -> impl Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {
    context("Live now", all_consuming(
            separated_pair((strip(take_till(AsChar::is_space)), take_until(" @ ")), s_tag("@"), (strip(take_till(AsChar::is_space)), rest))
        ).map(|((away_team_emoji, away_team_name), (home_team_emoji, home_team_name))| ParsedEventMessage::LiveNow { away_team_name, away_team_emoji, home_team_name, home_team_emoji })
    )
}
