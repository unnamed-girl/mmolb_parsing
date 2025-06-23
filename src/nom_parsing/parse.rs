use std::str::FromStr;

use nom::{branch::alt, bytes::{complete::{take_till, take_until}, tag}, character::complete::{digit1, u8}, combinator::{all_consuming, cut, fail, opt, rest}, error::context, multi::{many0, many1}, sequence::{delimited, preceded, separated_pair, terminated}, AsChar, Finish, Parser};

use crate::{enums::{EventType, GameOverMessage, HomeAway, MaybeRecognized, NowBattingStats}, game::Event, nom_parsing::shared::{emoji, try_from_word}, parsed_event::{FieldingAttempt, PositionedPlayer, StartOfInningPitcher}, ParsedEventMessage};

use super::{shared::{all_consuming_sentence_and, base_steal_sentence, bold, destination, emoji_team_eof, exclamation, fair_ball_type, fair_ball_type_verb_name, fielders_eof, fielding_error_type, fly_ball_type_verb_name, name_eof, now_batting_stats, ordinal_suffix, out, parse_and, parse_terminated, positioned_player_eof, s_tag, score_update_sentence, scores_and_advances, scores_sentence, sentence, sentence_eof, strip, switch_pitcher_sentences, emoji_team, Error}, ParsingContext};

trait GameEventParser<'output>: Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>> {}
impl<'output, T: Parser<&'output str, Output = ParsedEventMessage<&'output str>, Error = Error<'output>>> GameEventParser<'output> for T {}

pub fn parse_event<'output>(event: &'output Event, parsing_context: &ParsingContext<'output>) -> Result<ParsedEventMessage<&'output str>, Error<'output>> {
    let event_type = match &event.event {
        MaybeRecognized::Recognized(event_type) => event_type,
        MaybeRecognized::NotRecognized(event_type) => return Ok(ParsedEventMessage::ParseError { event_type: event_type.clone(), message: event.message.clone() })
    };
    
    match event_type {
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
        EventType::NowBatting => now_batting().parse(&event.message),
        EventType::WeatherDelivery => weather_delivery(parsing_context).parse(&event.message),
    }.finish().map(|(_, o)| o)
}

fn weather_delivery<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl GameEventParser<'output> + 'parse {
    let weather_delivery_success = ( 
            emoji_team(parsing_context),
            parse_terminated(" received a "),
            strip(take_till(AsChar::is_space)),
            terminated(try_from_word, tag("Delivery.")),
        ).map(|(team, player, item_emoji, item)| ParsedEventMessage::WeatherDelivery { team, player, item_emoji, item });

    let weather_delviery_discard = (
        emoji,
        terminated(try_from_word, tag("was discarded as no player had space."))
    ).map(|(item_emoji,  item)| ParsedEventMessage::WeatherDeliveryDiscard { item_emoji, item });

    context("Weather Delivery", all_consuming(alt((
        weather_delivery_success,
        weather_delviery_discard,
    ))))
}
fn game_over<'output>() -> impl GameEventParser<'output> {
    context("Game Over", all_consuming(
        rest.map_res(GameOverMessage::from_str)
    )).map(|message| ParsedEventMessage::GameOver { message })
}

fn record_keeping<'output>() -> impl GameEventParser<'output> {
    context("Record Keeping", all_consuming((
        all_consuming_sentence_and((
            parse_terminated(" defeated ").and_then(emoji_team_eof),
            emoji_team_eof
        ),
        preceded(s_tag("Final score:"), separated_pair(u8, s_tag("-"), u8))
        )
    ).map(|((winning_team, losing_team), (winning_score, losing_score))|
        ParsedEventMessage::Recordkeeping { winning_team, losing_team, winning_score, losing_score }
    )))
}

fn inning_end<'output>() -> impl GameEventParser<'output> {
    context("Inning End", all_consuming(
        sentence((
            preceded(s_tag("End of the"), try_from_word),
            delimited(s_tag("of the"), u8, ordinal_suffix)
        )).map(|(side, number)| ParsedEventMessage::InningEnd { number, side })
    ))
}

fn play_ball<'output>() -> impl GameEventParser<'output> {
    context("Play Ball", all_consuming(
        tag("\"PLAY BALL.\"")
    ).map(|_| ParsedEventMessage::PlayBall))
}

fn now_batting<'output>() -> impl GameEventParser<'output> {
    context("Now Batting", all_consuming(alt((
        (
            preceded(tag("Now batting: "), parse_terminated(" (")), 
            terminated(now_batting_stats, tag(")"))
        ).map(|(batter, stats)| ParsedEventMessage::NowBatting { batter, stats }),
        preceded(s_tag("Now batting: "), name_eof)
            .map(|batter| ParsedEventMessage::NowBatting { batter, stats:NowBattingStats::NoStats }),
    ))))
}

fn field<'output>() -> impl GameEventParser<'output> {
    let batter_to_base = all_consuming_sentence_and(
        (parse_and(try_from_word, " "), preceded(s_tag("on a"), fair_ball_type), preceded(s_tag("to"), positioned_player_eof)),
    scores_and_advances
    )
    .map(|(((batter, distance), fair_ball_type, fielder), (scores, advances))| {
        ParsedEventMessage::BatterToBase { batter, distance, fair_ball_type, fielder, scores, advances }
    });

    let homers = bold(exclamation((parse_terminated(" homers on a "), fair_ball_type, preceded(s_tag("to"), destination))))
    .and(many0(scores_sentence))
    .map(|((batter, fair_ball_type, destination), scores)| ParsedEventMessage::HomeRun { batter, fair_ball_type, destination, scores, grand_slam: false });
    
    let grand_slam = bold(exclamation((parse_terminated(" hits a grand slam on a "), fair_ball_type, preceded(s_tag("to"), destination))))
    .and(many0(scores_sentence))
    .map(|((batter, fair_ball_type, destination), scores)| ParsedEventMessage::HomeRun { batter, fair_ball_type, destination, scores, grand_slam: true });

    let caught_out = all_consuming_sentence_and(
        (
            terminated(parse_and(fly_ball_type_verb_name, " "), s_tag("out")),
            opt(s_tag("on a sacrifice fly")).map(|sacrifice| sacrifice.is_some()),
            preceded(s_tag("to"), positioned_player_eof),
        ),
        (scores_and_advances, opt(bold(exclamation(s_tag("Perfect catch")))).map(|perfect| perfect.is_some()))
    )
    .map(|(((batter, fair_ball_type), sacrifice, catcher), ((scores, advances), perfect))| ParsedEventMessage::CaughtOut { batter, fair_ball_type, caught_by: catcher, sacrifice, scores, advances, perfect });

    let grounded_out = all_consuming_sentence_and(
        (
            parse_terminated(" grounds out"),
            alt((
                preceded(s_tag("to"), positioned_player_eof).map(|fielder| vec![fielder]),
                preceded(s_tag(","), fielders_eof)
            ))
        ),
        (
            scores_and_advances,
            opt(bold(exclamation(s_tag("Perfect catch")))).map(|perfect| perfect.is_some())
        )
    )
    .map(|((batter, fielders), ((scores, advances), perfect))| ParsedEventMessage::GroundedOut { batter, fielders, scores, advances, perfect });

    let forced_out = all_consuming_sentence_and(
        (
            parse_and(fair_ball_type_verb_name, " "),
            preceded(s_tag("into a force out,"), fielders_eof)
        ),
        (sentence(out), scores_and_advances)
    )
    .map(|(((batter, fair_ball_type), fielders), (out, (scores, advances)))| 
        ParsedEventMessage::ForceOut { batter, fair_ball_type, fielders, out, scores, advances }
    );

    let reaches_on_fielders_choice_out = all_consuming_sentence_and(
        (parse_terminated(" reaches on a fielder's choice out, "), fielders_eof),
        (sentence(out), scores_and_advances)
    )
    .map(|((batter, fielders), (out, (scores, advances)))| {
        ParsedEventMessage::ReachOnFieldersChoice { batter, fielders, result: FieldingAttempt::Out { out }, scores, advances }
    });

    let reaches_on_fielders_choice_error = all_consuming_sentence_and(
        (parse_terminated(" reaches on a fielder's choice, fielded by "), positioned_player_eof),
        (scores_and_advances, sentence_eof(separated_pair(fielding_error_type, s_tag("error by"), name_eof)))
    )
    .map(|((batter, fielder), ((scores, advances), (error, error_fielder)))| {
        ParsedEventMessage::ReachOnFieldersChoice {batter,  fielders: vec![fielder], result: FieldingAttempt::Error { fielder: error_fielder, error }, scores, advances }
    });

    let reaches_on_error = all_consuming_sentence_and(
        (parse_terminated(" reaches on a "), terminated(fielding_error_type, s_tag("error by")), positioned_player_eof),
        scores_and_advances
    )
    .map(|((batter, error, fielder), (scores, advances))| {
        ParsedEventMessage::ReachOnFieldingError {batter, fielder, error, scores, advances }
    });

    let double_play_grounded = all_consuming_sentence_and(
        (
            parse_terminated(" grounded "),
            delimited(s_tag("into a"), opt(s_tag("sacrifice")).map(|s| s.is_some()), s_tag("double play,")),
            fielders_eof
        ),
        (sentence(out), sentence(out),scores_and_advances)
    )
    .map(|((batter, sacrifice, fielders), (out_one, out_two, (scores, advances)))| 
        ParsedEventMessage::DoublePlayGrounded { batter, fielders, out_one, out_two, scores, advances, sacrifice }
    );

    let double_play_caught = all_consuming_sentence_and(
        (terminated(parse_and( fair_ball_type_verb_name, " "), s_tag("into a double play,")),fielders_eof),
        (sentence(out), scores_and_advances)
    )
    .map(|(((batter, fair_ball_type), fielders), (out_two, (scores, advances)))| 
        ParsedEventMessage::DoublePlayCaught { batter, fair_ball_type, fielders, out_two, scores, advances }
    );

    let fielding_outcomes = alt((
        batter_to_base,
        homers,
        grand_slam,
        grounded_out,
        caught_out,
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

fn pitch<'output>() -> impl GameEventParser<'output> {
    let fair_ball = sentence((
        parse_terminated(" hits a "), 
        fair_ball_type,
        preceded(s_tag("to"), destination)
    ))
    .map(|(batter, fair_ball_type, destination)| ParsedEventMessage::FairBall { batter, fair_ball_type, destination });

    let struck_out = (
        opt(sentence(preceded(s_tag("Foul"), try_from_word))),
        sentence((
            parse_terminated(" struck out "), 
            try_from_word)
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

    let strike = sentence(preceded(s_tag("Strike,"), try_from_word))
    .and(cut((score_update_sentence, many0(base_steal_sentence))))
    .map(|(strike, (count, steals))| ParsedEventMessage::Strike { strike, steals, count });

    let foul = sentence(preceded(s_tag("Foul"), try_from_word))
    .and(score_update_sentence)
    .and(many0(base_steal_sentence))
    .map(|((foul, count), steals)| ParsedEventMessage::Foul { foul, steals, count });

    let pitch_options = alt((
        struck_out,
        walks,
        ball,
        strike,
        foul,
        fair_ball,
        hit_by_pitch,
        fail()
    ));

    context("Pitch event", all_consuming(pitch_options))
}

/// Parse the home and away pitchers from the pitching matchup message.
fn pitching_matchup<'output, 'parse>(parsing_context: &'parse ParsingContext<'output>) -> impl GameEventParser<'output> + 'parse {
    context("Pitching matchup", all_consuming(
        ( 
            emoji_team(parsing_context), 
            terminated(take_until(" vs. "),s_tag("vs.")) , 
            emoji_team(parsing_context), 
            name_eof
        ).map(|(away_team, away_pitcher, home_team , home_pitcher)| ParsedEventMessage::PitchingMatchup { home_team, home_pitcher, away_team, away_pitcher })
    ))
}

fn lineup<'output>(side: HomeAway) -> impl GameEventParser<'output> {
    context("Lineup", all_consuming(
        many1(delimited(
            (digit1, s_tag(".")), 
            (try_from_word, take_until("<br>")),
             s_tag("<br>")
            ).map(|(position, player)| PositionedPlayer {position, name: player})
            )
    ).map(move |players| ParsedEventMessage::Lineup {side, players }))
}

fn inning_start<'output>() -> impl GameEventParser<'output> {
    let keep_pitcher = sentence((strip(take_till(AsChar::is_space)), parse_terminated(" pitching")))
    .map(|(emoji, name)| StartOfInningPitcher::Same { emoji, name });

    let swap_pitcher = switch_pitcher_sentences
    .map(|(leaving_pitcher, arriving_pitcher)| StartOfInningPitcher::Different { leaving_pitcher, arriving_pitcher });

    let start_inning = (
        sentence((
            preceded(s_tag("Start of the"), try_from_word), 
            delimited(s_tag("of the"), u8, ordinal_suffix))),
        sentence(strip(parse_terminated(" batting").and_then(emoji_team_eof)))
    );

    let automatic_runner = sentence(
        parse_terminated(" starts the inning on second base")
    );

    let pitcher_status = alt ((
        keep_pitcher,
        swap_pitcher
    ));
    context("Inning Start", 
        all_consuming((start_inning, opt(automatic_runner), pitcher_status))
        .map(|(((side, number), batting_team), automatic_runner, pitcher_status)| 
            ParsedEventMessage::InningStart { number, side, batting_team, automatic_runner, pitcher_status }
        )
    )
}

fn mound_visit<'output>() -> impl GameEventParser<'output> {
    let mound_visit_options = alt((
        preceded(tag("The "), parse_terminated(" manager is making a mound visit.").and_then(emoji_team_eof))
        .map(|team| ParsedEventMessage::MoundVisit { team }),

        switch_pitcher_sentences.map(|(leaving_pitcher, arriving_pitcher)| ParsedEventMessage::PitcherSwap { leaving_pitcher, arriving_pitcher }),

        sentence(parse_terminated(" remains in the game").and_then(positioned_player_eof))
        .map(|remaining_pitcher| ParsedEventMessage::PitcherRemains { remaining_pitcher }),
    ));
    
    context("Mound visit", all_consuming(mound_visit_options))
}

fn live_now<'output>() -> impl GameEventParser<'output> {
    context("Live now", all_consuming(
            (parse_terminated(" @ ").and_then(emoji_team_eof), emoji_team_eof)
        ).map(|(away_team, home_team)| ParsedEventMessage::LiveNow { away_team, home_team })
    )
}
