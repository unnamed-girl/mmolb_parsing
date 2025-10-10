use crate::{nom_parsing::shared::{door_prizes, ejection_tail, hit_by_pitch_text, strike_out_text}, time::is_superstar_game};
use std::str::FromStr;

use nom::{branch::alt, bytes::complete::{tag, take_until}, character::complete::{digit1, u8, u16}, combinator::{all_consuming, cut, fail, opt, rest, value, verify}, error::context, multi::{many0, many1, separated_list1}, sequence::{delimited, preceded, separated_pair, terminated}, Finish, Parser};
use nom::sequence::pair;
use phf::phf_map;

use crate::{enums::{EventType, GameOverMessage, HomeAway, MoundVisitType, NowBattingStats}, game::Event, nom_parsing::shared::{aurora, cheer, ejection, delivery, team_emoji, try_from_word, try_from_words_m_n, MyParser}, parsed_event::{EmojiTeam, FallingStarOutcome, FieldingAttempt, GameEventParseError, KnownBug, StartOfInningPitcher}, time::Breakpoints, ParsedEventMessage};

use super::{shared::{all_consuming_sentence_and, base_steal_sentence, bold, destination, emoji_team_eof, exclamation, fair_ball_type_verb_name, fielders_eof, fly_ball_type_verb_name, name_eof, now_batting_stats, ordinal_suffix, out, parse_and, parse_terminated, placed_player_eof, score_update, scores_and_advances, scores_sentence, sentence, sentence_eof}, ParsingContext};

const OVERRIDES: phf::Map<&'static str, phf::Map<u16, ParsedEventMessage<&'static str>>> = phf_map!();

pub fn parse_event<'parse, 'output: 'parse>(event: &'output Event, parsing_context: &ParsingContext<'parse>) -> ParsedEventMessage<&'output str> {
    if let Some(game_overrides) = OVERRIDES.get(parsing_context.game_id) {
        let event_index = parsing_context.event_index.unwrap_or_else(||
            parsing_context.event_log.iter().enumerate()
                .find(|(_, e)| e.message == event.message)
                .map(|(i, _)| i as u16)
                .expect("Overrides to be correct")
        );

        if let Some(event) = game_overrides.get(&event_index) {
            return event.clone();
        }
    }

    let event_type = match &event.event {
        Ok(event_type) => event_type,
        Err(event_type) => {
            tracing::error!("Event type {event_type} not recognized: {}", event.message);
            let error = GameEventParseError::EventTypeNotRecognized(event_type.clone());
            return ParsedEventMessage::ParseError { error, message: &event.message }
        }
    };

    match event_type {
        EventType::PitchingMatchup => pitching_matchup(parsing_context).parse(&event.message),
        EventType::MoundVisit => mound_visit(event, parsing_context).parse(&event.message),
        EventType::GameOver => game_over().parse(&event.message),
        EventType::Field => field(parsing_context).parse(&event.message),
        EventType::HomeLineup => lineup(HomeAway::Home).parse(&event.message),
        EventType::Recordkeeping => record_keeping().parse(&event.message),
        EventType::LiveNow => live_now(parsing_context).parse(&event.message),
        EventType::InningStart => inning_start(event, parsing_context).parse(&event.message),
        EventType::Pitch => pitch(parsing_context).parse(&event.message),
        EventType::AwayLineup => lineup(HomeAway::Away).parse(&event.message),
        EventType::InningEnd => inning_end().parse(&event.message),
        EventType::PlayBall => play_ball().parse(&event.message),
        EventType::NowBatting => now_batting().parse(&event.message),
        EventType::WeatherDelivery => weather_delivery(parsing_context).parse(&event.message),
        EventType::FallingStar => falling_star().parse(&event.message),
        EventType::Weather => weather().parse(&event.message),
        EventType::HrcLiveNow |
        EventType::HrcPitchingMatchup |
        EventType::HrcBattingMatchup |
        EventType::HrcPlayBall |
        EventType::HrcChange => fail().parse(event.message.as_str()),
        EventType::WeatherShipment => weather_shipment(parsing_context).parse(&event.message),
        EventType::WeatherSpecialDelivery => special_delivery(parsing_context).parse(&event.message),
        EventType::WeatherProsperity => weather_prosperity(parsing_context).parse(&event.message),
        EventType::Balk => balk().parse(&event.message),
        EventType::PhotoContest => photo_contest(parsing_context).parse(event.message.as_str()),
        EventType::Party => party(parsing_context).parse(event.message.as_str()),
        EventType::WeatherReflection => weather_reflection(parsing_context).parse(&event.message)
    }.finish().map(|(_, o)| o)
    .unwrap_or_else(move |_| {
            let error = GameEventParseError::FailedParsingMessage { event_type: *event_type, message: event.message.clone() };
            tracing::error!("Parse error: {}", error);
            ParsedEventMessage::ParseError { error, message: &event.message }
        }
    )
}
fn photo_contest<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let team = |team: EmojiTeam<&'parse str>| (terminated(team.parser(), tag(" earned ")), terminated(u8, tag(" 🪙.")));
    let player = |emoji: &'parse str| (terminated(tag(emoji), tag(" ")), parse_terminated(" - "), u16);

    context("Photo Contest", all_consuming(verify((
        alt((
            separated_pair(team(parsing_context.away_emoji_team), tag(" "), team(parsing_context.home_emoji_team)),
            separated_pair(team(parsing_context.home_emoji_team), tag(" "), team(parsing_context.away_emoji_team)),
        )),
        preceded(
            tag("<br>Top scoring Photos:<br>"),
            alt((
                separated_pair(player(parsing_context.home_emoji_team.emoji), tag(" "), player(parsing_context.away_emoji_team.emoji)),
                separated_pair(player(parsing_context.away_emoji_team.emoji), tag(" "), player(parsing_context.home_emoji_team.emoji)),
            ))
        )),
    |(((winning_team, _), (losing_team, _)),
        ((winning_emoji, _, _), (losing_emoji, _, _)))|
        winning_team.emoji == *winning_emoji && losing_team.emoji == *losing_emoji
    )))
    .map(|(((winning_team, winning_tokens), (losing_team, losing_tokens)),
        ((_, winning_player, winning_score), (_, losing_player, losing_score)))|
        ParsedEventMessage::PhotoContest { winning_team, winning_tokens, winning_player, winning_score, losing_team, losing_tokens, losing_player, losing_score }
    )
}
fn party<'parse, 'output: 'parse>(_parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    context("Party", all_consuming(verify(
        preceded(
            tag("<strong>🥳 "),
            (
                parse_terminated(" and "),
                parse_terminated(" are Partying!</strong> "),
                parse_terminated(" gained +"),
                terminated(u8, tag(" ")),
                terminated(try_from_word, tag(". ")), // attribute
                parse_terminated(" gained +"),
                terminated(u8, tag(" ")),
                terminated(try_from_word, tag(". Both players lose 3 Durability.")), // attribute
            ),
        ),
    |(pitcher_name_1, batter_name_1, pitcher_name_2, _, _, batter_name_2, _, _)|
        pitcher_name_1 == pitcher_name_2 && batter_name_1 == batter_name_2
    )))
    .map(|(_, _, pitcher_name, pitcher_amount_gained, pitcher_attribute, batter_name, batter_amount_gained, batter_attribute)|
        ParsedEventMessage::Party { pitcher_name, pitcher_amount_gained, pitcher_attribute, batter_name, batter_amount_gained, batter_attribute }
    )
}

fn balk<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Balk", all_consuming((
        preceded(tag("Balk. "), parse_terminated(" dropped the ball.")),
        scores_and_advances
    ))).map(|(pitcher, (scores, advances))| ParsedEventMessage::Balk { pitcher, scores, advances })
}

fn special_delivery<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let special_delivery = delivery(parsing_context, "Special Delivery")
        .map(|delivery| ParsedEventMessage::WeatherSpecialDelivery { delivery });
    context("Weather Shipment", all_consuming(
        special_delivery,
    ))
}

fn weather_shipment<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let weather_shipment = separated_list1(tag(" "),
        delivery(parsing_context, "Shipment")
    ).map(|deliveries| ParsedEventMessage::WeatherShipment { deliveries });
    context("Weather Shipment", all_consuming(
        weather_shipment,
    ))
}

fn weather_delivery<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    context("Weather Delivery", all_consuming(
        delivery(parsing_context, "Delivery").map(|delivery| ParsedEventMessage::WeatherDelivery { delivery })
    ))
}

fn weather_prosperity<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let prosperous = |t: EmojiTeam<&'parse str>| move |input: &'output str| delimited((t.parser(), tag(" are Prosperous! They earned ")), u8, tag(" 🪙.")).parse(input);

    let variations = alt((
        separated_pair(prosperous(parsing_context.home_emoji_team), tag(" "), prosperous(parsing_context.away_emoji_team)).map(|(home, away)| (Some(home), Some(away))),
        separated_pair(prosperous(parsing_context.away_emoji_team), tag(" "), prosperous(parsing_context.home_emoji_team)).map(|(away, home)| (Some(home), Some(away))),
        prosperous(parsing_context.away_emoji_team).map(|away| (None, Some(away))),
        prosperous(parsing_context.home_emoji_team).map(|home| (Some(home), None)),
    )).map(|(home_income, away_income)| {
        ParsedEventMessage::WeatherProsperity { home_income: home_income.unwrap_or_default(), away_income: away_income.unwrap_or_default() }
    });

    context("Weather Prosperity", all_consuming(
        alt((
            variations,
            value(ParsedEventMessage::KnownBug { bug: KnownBug::NoOneProspers }, verify(rest, |s: &str| s.is_empty()))
        ))
    ))
}

fn falling_star<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Falling Star", all_consuming(
        preceded(tag("<strong>🌠 "), parse_terminated(" is hit by a Falling Star!</strong>"))
            .map(|player_name| ParsedEventMessage::FallingStar { player_name }),
    ))
}

fn weather<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    let retirement = (
        preceded(tag("😇 "), parse_terminated(" retired from MMOLB!")),
        opt(preceded(
            tag(" "),
            parse_terminated(" was called up to take their place."),
        )),
    );

    let outcomes = delimited(
        tag("<strong>"),
        alt((
            parse_terminated(" was injured by the extreme force of the impact!")
                .map(|name| (name, FallingStarOutcome::Injury)),
            retirement.map(|(retired_player_name, replacement_player_name)| (retired_player_name, FallingStarOutcome::Retired(replacement_player_name))),
            parse_terminated(" was infused with a glimmer of celestial energy!")
                .map(|name| (name, FallingStarOutcome::InfusionI)),
            parse_terminated(" began to glow brightly with celestial energy!")
                .map(|name| (name, FallingStarOutcome::InfusionII)),
            parse_terminated(" was fully charged with an abundance of celestial energy!")
                .map(|name| (name, FallingStarOutcome::InfusionIII)),
            preceded(tag("It deflected off "), parse_terminated(" harmlessly.")).and_then(name_eof)
                .map(|name| (name, FallingStarOutcome::DeflectedHarmlessly)),
        )),
        tag("</strong>")
    );

    let deflection = delimited(
        tag("<strong>"),
        preceded(tag("It deflected off "), (parse_terminated(" and struck "), take_until("!</strong> <strong>"))),
        tag("!</strong> ")
    );

    context("Weather", all_consuming(
        verify(
            preceded(tag(" "), (opt(deflection), outcomes)),
            |(deflection, (player_name, _))| deflection.is_none_or(|(_, struck)| struck == *player_name) // Verify that if two names are present, they are consistent.
        ).map(|(deflection, (player_name, outcome))| ParsedEventMessage::FallingStarOutcome { deflection: deflection.map(|(deflected_off, _)| deflected_off), player_name, outcome })
    ))
}

fn game_over<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Game Over", all_consuming(
        rest.map_res(GameOverMessage::from_str)
    )).map(|message| ParsedEventMessage::GameOver { message })
}

fn record_keeping<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Record Keeping", all_consuming((
        all_consuming_sentence_and((
            parse_terminated(" defeated ").and_then(emoji_team_eof),
            emoji_team_eof
        ),
        preceded(tag(" Final score: "), separated_pair(u8, tag("-"), u8))
        )
    ).map(|((winning_team, losing_team), (winning_score, losing_score))|
        ParsedEventMessage::Recordkeeping { winning_team, losing_team, winning_score, losing_score }
    )))
}

fn inning_end<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Inning End", all_consuming(
        sentence((
            preceded(tag("End of the "), try_from_word),
            delimited(tag(" of the "), u8, ordinal_suffix)
        )).map(|(side, number)| ParsedEventMessage::InningEnd { number, side })
    ))
}

fn play_ball<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Play Ball", all_consuming(
        tag("\"PLAY BALL.\"")
    ).map(|_| ParsedEventMessage::PlayBall))
}

fn now_batting<'output>() -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Now Batting", all_consuming(alt((
        (
            preceded(tag("Now batting: "), parse_terminated(" (")),
            terminated(now_batting_stats, tag(")"))
        ).map(|(batter, stats)| ParsedEventMessage::NowBatting { batter, stats }),
        preceded(tag("Now batting: "), name_eof)
            .map(|batter| ParsedEventMessage::NowBatting { batter, stats:NowBattingStats::NoStats }),
    ))))
}

fn field<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let batter_to_base = all_consuming_sentence_and(
        (parse_and(try_from_word, " "), preceded(tag(" on a "), try_from_words_m_n(1,2)), preceded(tag(" to "), placed_player_eof)),
        (scores_and_advances, opt(ejection(parsing_context)))
    )
    .map(|(((batter, distance), fair_ball_type, fielder), ((scores, advances), ejection))| {
        ParsedEventMessage::BatterToBase { batter, distance, fair_ball_type, fielder, scores, advances, ejection }
    });

    let homers = bold(exclamation((parse_terminated(" homers on a "), try_from_words_m_n(1,2), preceded(tag(" to "), destination))))
    .and(many0(scores_sentence))
    .and(opt(ejection(parsing_context)))
    .map(|(((batter, fair_ball_type, destination), scores), ejection)| ParsedEventMessage::HomeRun { batter, fair_ball_type, destination, scores, grand_slam: false, ejection });

    let grand_slam = bold(exclamation((parse_terminated(" hits a grand slam on a "), try_from_words_m_n(1,2), preceded(tag(" to "), destination))))
    .and(many0(scores_sentence))
    .and(opt(ejection(parsing_context)))
    .map(|(((batter, fair_ball_type, destination), scores), ejection)| ParsedEventMessage::HomeRun { batter, fair_ball_type, destination, scores, grand_slam: true, ejection });

    let caught_out = all_consuming_sentence_and(
        (
            terminated(parse_and(fly_ball_type_verb_name, " "), tag(" out ")),
            opt(tag("on a sacrifice fly ")).map(|sacrifice| sacrifice.is_some()),
            preceded(tag("to "), placed_player_eof),
        ),
        (
            scores_and_advances,
            opt(bold(exclamation(tag("Perfect catch")))).map(|perfect| perfect.is_some()),
            opt(ejection(parsing_context)),
        )
    )
    .map(|(((batter, fair_ball_type), sacrifice, catcher), ((scores, advances), perfect, ejection))| ParsedEventMessage::CaughtOut { batter, fair_ball_type, caught_by: catcher, sacrifice, scores, advances, perfect, ejection });

    let grounded_out = all_consuming_sentence_and(
        (
            parse_terminated(" grounds out").and_then(name_eof),
            alt((
                preceded(tag(" to "), placed_player_eof).map(|fielder| vec![fielder]),
                preceded(tag(", "), fielders_eof)
            ))
        ),
        (
            scores_and_advances,
            opt(ejection(parsing_context)),
            opt(bold(exclamation(tag(if parsing_context.season < 5 { "Perfect catch" } else { "Amazing throw" })))).map(|perfect| perfect.is_some())
        )
    )
    .map(|((batter, fielders), ((scores, advances), ejection, amazing))| ParsedEventMessage::GroundedOut { batter, fielders, scores, advances, amazing, ejection });

    let forced_out = all_consuming_sentence_and(
        (
            parse_and(fair_ball_type_verb_name, " "),
            preceded(tag(" into a force out, "), fielders_eof)
        ),
        (sentence(out), scores_and_advances).and(opt(ejection(parsing_context)))
    )
    .map(|(((batter, fair_ball_type), fielders), ((out, (scores, advances)), ejection))|
        ParsedEventMessage::ForceOut { batter, fair_ball_type, fielders, out, scores, advances, ejection }
    );

    let reaches_on_fielders_choice_out = all_consuming_sentence_and(
        (parse_terminated(" reaches on a fielder's choice out, ").and_then(name_eof), fielders_eof),
        (
            sentence(out),
            scores_and_advances,
            opt(ejection(parsing_context))
        )
    )
    .map(|((batter, fielders), (out, (scores, advances), ejection))| {
        ParsedEventMessage::ReachOnFieldersChoice { batter, fielders, result: FieldingAttempt::Out { out }, scores, advances, ejection }
    });

    let reaches_on_fielders_choice_error = all_consuming_sentence_and(
        (parse_terminated(" reaches on a fielder's choice, fielded by ").and_then(name_eof), placed_player_eof),
        (
            scores_and_advances,
            sentence_eof(separated_pair(try_from_word, tag(" error by "), alt((
                pair(parse_terminated(". 🤖 ROBO-UMP ejected "), ejection_tail(parsing_context)).map(|(name, ejection)| (name, Some(ejection))),
                name_eof.map(|name| (name, None)),
            )))),
        )
    )
    .map(|((batter, fielder), ((scores, advances), (error, (error_fielder, ejection))))| {
        ParsedEventMessage::ReachOnFieldersChoice {batter,  fielders: vec![fielder], result: FieldingAttempt::Error { fielder: error_fielder, error }, scores, advances, ejection }
    });

    let reaches_on_error = all_consuming_sentence_and(
        (parse_terminated(" reaches on a ").and_then(name_eof), terminated(try_from_word, tag(" error by ")), placed_player_eof),
        scores_and_advances.and(opt(ejection(parsing_context)))
    )
    .map(|((batter, error, fielder), ((scores, advances), ejection))| {
        ParsedEventMessage::ReachOnFieldingError {batter, fielder, error, scores, advances, ejection }
    });

    let double_play_grounded = all_consuming_sentence_and(
        (
            // This will eat all instances of double_play_caught on a ground ball. Which is currently fine, because there has never been one. Ground ball double plays are always fielded.
            alt((parse_terminated(" grounds into a "), parse_terminated(" grounded into a "))).and_then(name_eof),
            terminated(opt(tag("sacrifice ")).map(|s| s.is_some()), tag("double play, ")),
            fielders_eof
        ),
        (sentence(out), sentence(out), scores_and_advances, opt(ejection(parsing_context)))
    )
    .map(|((batter, sacrifice, fielders), (out_one, out_two, (scores, advances), ejection))|
        ParsedEventMessage::DoublePlayGrounded { batter, fielders, out_one, out_two, scores, advances, sacrifice, ejection }
    );

    let double_play_caught = all_consuming_sentence_and(
        (terminated(parse_and( fair_ball_type_verb_name, " "), tag(" into a double play, ")),fielders_eof),
        (sentence(out), scores_and_advances, opt(ejection(parsing_context)))
    )
    .map(|(((batter, fair_ball_type), fielders), (out_two, (scores, advances), ejection))|
        ParsedEventMessage::DoublePlayCaught { batter, fair_ball_type, fielders, out_two, scores, advances, ejection }
    );

    let first_baseman_chooses_a_ghost = parse_terminated(" reaches on a fielder's choice out, 1B ").and_then(name_eof)
    .and(name_eof)
    .map(|(batter, first_baseman)| ParsedEventMessage::KnownBug { bug: KnownBug::FirstBasemanChoosesAGhost { batter, first_baseman } });

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
        first_baseman_chooses_a_ghost,
        fail()
    ));

    context("Field event", all_consuming(fielding_outcomes))
}

fn pitch<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let fair_ball = (sentence((
            parse_terminated(" hits a "),
            try_from_words_m_n(1,2),
            preceded(tag(" to "), destination)
        )),
        opt(preceded(tag(" "), aurora(parsing_context))),
        opt(preceded(tag(" "), cheer(parsing_context))),
        door_prizes
    )
    .map(|((batter, fair_ball_type, destination), aurora_photos, cheer, door_prizes)| ParsedEventMessage::FairBall { batter, fair_ball_type, destination, cheer, aurora_photos, door_prizes });

    let struck_out = (
        opt(sentence(preceded(tag("Foul "), try_from_word))),
        sentence((
            parse_terminated(strike_out_text(parsing_context.season, parsing_context.day, parsing_context.event_index)),
            try_from_word)
    ))
    .and(many0(base_steal_sentence))
    .and(opt(preceded(tag(" "), aurora(parsing_context))))
    .and(opt(preceded(tag(" "), cheer(parsing_context))))
    .and(opt(ejection(parsing_context)))
    .map(|(((((foul, (batter, strike)), steals), aurora_photos), cheer), ejection)|
        ParsedEventMessage::StrikeOut { foul, batter, strike, steals, cheer, aurora_photos, ejection }
    );

    let hit_by_pitch = sentence(parse_terminated(hit_by_pitch_text(parsing_context.season, parsing_context.day, parsing_context.event_index)))
    .and(scores_and_advances)
    .and(opt(preceded(tag(" "), aurora(parsing_context))))
    .and(opt(preceded(tag(" "), cheer(parsing_context))))
    .and(opt(ejection(parsing_context)))
    .and(door_prizes)
    .map(|(((((batter, (scores, advances)), aurora_photos), cheer), ejection), door_prizes)| ParsedEventMessage::HitByPitch { batter, scores, advances, cheer, aurora_photos, ejection, door_prizes });

    let walks = preceded(
        sentence(tag("Ball 4")),
        sentence(parse_terminated(" walks"))
    ).and(scores_and_advances)
    .and(opt(preceded(tag(" "), aurora(parsing_context))))
    .and(opt(preceded(tag(" "), cheer(parsing_context))))
    .and(opt(ejection(parsing_context)))
    .map(|((((batter, (scores, advances)), aurora_photos), cheer), ejection)| ParsedEventMessage::Walk { batter, scores, advances, cheer, aurora_photos, ejection });

    let ball = (preceded(sentence(tag("Ball")), sentence(score_update)))
    .and(many0(base_steal_sentence))
    .and(opt(preceded(tag(" "), aurora(parsing_context))))
    .and(opt(preceded(tag(" "), cheer(parsing_context))))
    .and(opt(ejection(parsing_context)))
    .and(door_prizes)
    .map(|(((((count, steals), aurora_photos), cheer), ejection), door_prizes)| ParsedEventMessage::Ball { steals, count, cheer, aurora_photos, ejection, door_prizes });

    let strike = sentence(preceded(tag("Strike, "), try_from_word))
    .and(cut((sentence(score_update), many0(base_steal_sentence))))
    .and(opt(preceded(tag(" "), aurora(parsing_context))))
    .and(opt(preceded(tag(" "), cheer(parsing_context))))
    .and(opt(ejection(parsing_context)))
    .and(door_prizes)
    .map(|(((((strike, (count, steals)), aurora_photos), cheer), ejection), door_prizes)| ParsedEventMessage::Strike { strike, steals, count, cheer, aurora_photos, ejection, door_prizes});

    let foul = sentence(preceded(tag("Foul "), try_from_word))
    .and(sentence(score_update))
    .and(many0(base_steal_sentence))
    .and(opt(preceded(tag(" "), aurora(parsing_context))))
    .and(opt(preceded(tag(" "), cheer(parsing_context))))
    .and(door_prizes)
    .map(|(((((foul, count), steals), aurora_photos), cheer), door_prizes)| ParsedEventMessage::Foul { foul, steals, count, cheer, aurora_photos, door_prizes });

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
fn pitching_matchup<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    context("Pitching matchup", all_consuming(
        (
            separated_pair(parsing_context.away_emoji_team.parser(), tag(" "), parse_terminated(" vs. ")),
            separated_pair(parsing_context.home_emoji_team.parser(), tag(" "), name_eof)
        ).map(|((away_team, away_pitcher), (home_team , home_pitcher))| ParsedEventMessage::PitchingMatchup { home_team, home_pitcher, away_team, away_pitcher })
    ))
}

fn lineup<'output>(side: HomeAway) -> impl MyParser<'output, ParsedEventMessage<&'output str>> {
    context("Lineup", all_consuming(
        many1(delimited(
            (digit1, tag(". ")),
            take_until("<br>").and_then(placed_player_eof),
             tag("<br>")
            ))
    ).map(move |players| ParsedEventMessage::Lineup {side, players }))
}

fn inning_start<'parse, 'output: 'parse>(event: &'output Event, parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {

    let parser = |input: &'output str| {
        let pitching_team_emoji = |input| match event.inning.pitching_team() {
            Some(side) => team_emoji(side, parsing_context).parse(input),
            None => fail().parse(input)
        };

        let keep_pitcher = sentence(separated_pair(pitching_team_emoji, tag(" "), parse_terminated(" pitching")))
        .map(|(emoji, name)| StartOfInningPitcher::Same { emoji, name });

        let swap_pitcher = (
            sentence((opt(terminated(pitching_team_emoji, tag(" "))), parse_terminated(" is leaving the game").and_then(placed_player_eof))),
            sentence((opt(terminated(pitching_team_emoji, tag(" "))), parse_terminated(" takes the mound").and_then(placed_player_eof)))
        ).map(|((leaving_emoji, leaving_pitcher), (arriving_emoji, arriving_pitcher))| StartOfInningPitcher::Different { leaving_emoji, leaving_pitcher, arriving_emoji, arriving_pitcher });

        let mut start_inning = (
            sentence((
                preceded(tag("Start of the "), try_from_word),
                delimited(tag(" of the "), u8, ordinal_suffix))),
            sentence(parse_terminated(" batting").and_then(emoji_team_eof))
        );

        let automatic_runner = sentence(
            parse_terminated(" starts the inning on second base")
        );

        let mut pitcher_status = alt((
            keep_pitcher,
            swap_pitcher
        ));

        if is_superstar_game(parsing_context.day) {
            let (input, ((side, number), batting_team)) = start_inning.parse(input)?;
            let (input, automatic_runner) = opt(automatic_runner).parse(input)?;
            let (input, pitcher_status) = opt(pitcher_status).parse(input)?;

            Ok((input, ParsedEventMessage::InningStart { number, side, batting_team, automatic_runner, pitcher_status }))
        } else {
            let (input, ((side, number), batting_team)) = start_inning.parse(input)?;
            let (input, automatic_runner) = opt(automatic_runner).parse(input)?;
            let (input, pitcher_status) = pitcher_status.parse(input)?;

            Ok((input, ParsedEventMessage::InningStart { number, side, batting_team, automatic_runner, pitcher_status: Some(pitcher_status) }))
        }
    };

    context("Inning Start",
        all_consuming(parser)
    )
}

fn mound_visit<'parse, 'output: 'parse>(event: &'output Event, parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let leaves_player = |i| {
        if parsing_context.before(Breakpoints::S2D152) {
            (terminated(try_from_word, tag(" ")), name_eof).map(|(place, name)| (Some(place), name)).parse(i)
        } else {
            (opt(terminated(try_from_word, tag(" "))), name_eof).map(|(place, name)| (place, name)).parse(i)
        }
    };

    let team_emoji = |input| match event.inning.pitching_team() {
        Some(side) => team_emoji(side, parsing_context).parse(input),
        None => fail().parse(input)
    };

    let mound_visit_options = alt((
        preceded(tag("The "), parse_terminated(" manager is making a mound visit.").and_then(emoji_team_eof))
        .map(|team| ParsedEventMessage::MoundVisit { team, mound_visit_type: MoundVisitType::MoundVisit }),
        preceded(tag("The "), parse_terminated(" manager is making a pitching change.").and_then(emoji_team_eof))
        .map(|team| ParsedEventMessage::MoundVisit { team, mound_visit_type: MoundVisitType::PitchingChange }),

        (
            sentence((opt(terminated(team_emoji, tag(" "))), parse_terminated(" is leaving the game").and_then(placed_player_eof))),
            sentence((opt(terminated(team_emoji, tag(" "))), parse_terminated(" takes the mound").and_then(leaves_player)))
        ).map(|((leaving_pitcher_emoji, leaving_pitcher), (arriving_pitcher_emoji, (arriving_pitcher_place, arriving_pitcher_name)))| ParsedEventMessage::PitcherSwap { leaving_pitcher_emoji, leaving_pitcher, arriving_pitcher_emoji, arriving_pitcher_place, arriving_pitcher_name }),

        sentence(parse_terminated(" remains in the game").and_then(placed_player_eof))
        .map(|remaining_pitcher| ParsedEventMessage::PitcherRemains { remaining_pitcher }),
    ));

    context("Mound visit", all_consuming(mound_visit_options))
}

fn live_now<'parse, 'output: 'parse>(parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let time_options = |input: &'output str| {
        if parsing_context.after(Breakpoints::Season3) {
            (
                parse_terminated(" vs ").and_then(emoji_team_eof),
                parse_terminated(" @ ").and_then(emoji_team_eof),
                rest
            )
            .map(|(away_team, home_team, stadium)| ParsedEventMessage::LiveNow { away_team, home_team, stadium: Some(stadium) } )
            .parse(input)
        } else {
            (parse_terminated(" @ ").and_then(emoji_team_eof), emoji_team_eof)
            .map(|(away_team, home_team)| ParsedEventMessage::LiveNow { away_team, home_team, stadium: None } )
            .parse(input)
        }
    };

    context("Live now", all_consuming(
        time_options
    ))
}

fn weather_reflection<'parse, 'output: 'parse>(_parsing_context: &'parse ParsingContext<'parse>) -> impl MyParser<'output, ParsedEventMessage<&'output str>> + 'parse {
    let weather_reflection = |input| {
        let (input, _) = tag("🪞 The reflection shatters. ").parse(input)?;
        let (input, team) = parse_terminated(" received a Fragment of Reflection.").parse(input)?;
        let (_, team) = emoji_team_eof(team)?;
        Ok((input, ParsedEventMessage::WeatherReflection {
            team
        }))
    };
    context("Weather Reflection", all_consuming(
        weather_reflection,
    ))
}

#[cfg(test)]
mod test {
    use nom::Parser;

    use crate::{enums::{Base, BaseNameVariant, Distance, FairBallType, Place}, nom_parsing::{shared::name_eof, ParsingContext}, parsed_event::{EmojiTeam, PlacedPlayer, RunnerAdvance, RunnerOut}, ParsedEventMessage};

    #[test]
    fn jr_test() {
        let text = "Traci Rivers grounded into a double play, SS Ellen Updog to 2B Chalia Jr. to 1B Elena Karapetyan. Lance Green out at second base. Traci Rivers out at first base.";
        let parsing_context = ParsingContext {
            game_id: "68aab8a3318f19d301830b7c",
            event_log: &[],
            event_index: None,
            home_emoji_team: EmojiTeam { emoji: "", name: "" },
            away_emoji_team: EmojiTeam { emoji: "", name: "" },
            season: 5,
            day: None
        };

        assert_eq!(
            super::field(&parsing_context).parse(text),
            Ok(("", ParsedEventMessage::DoublePlayGrounded { batter: "Traci Rivers", fielders: vec![PlacedPlayer { place: Place::ShortStop, name: "Ellen Updog"}, PlacedPlayer { place: Place::SecondBaseman, name: "Chalia Jr."}, PlacedPlayer {place: Place::FirstBaseman, name: "Elena Karapetyan"}], out_one: RunnerOut { runner: "Lance Green", base: BaseNameVariant::SecondBase }, out_two: RunnerOut { runner: "Traci Rivers", base: BaseNameVariant::FirstBase }, scores: Vec::new(), advances: Vec::new(), sacrifice: false, ejection: None }))
        );
    }

    #[test]
    fn bob_e_quiros() {
        let text = "Victor Rodriguez singles on a line drive to RF Bob E. Quiros. Myra Roussel to third base.";
        let parsing_context = ParsingContext {
            game_id: "68d5f3a3a4ec9a9adffeea83",
            event_log: &[],
            event_index: None,
            home_emoji_team: EmojiTeam { emoji: "", name: "" },
            away_emoji_team: EmojiTeam { emoji: "", name: "" },
            season: 5,
            day: None
        };

        assert_eq!(
            super::field(&parsing_context).parse(text),
            Ok(("", ParsedEventMessage::BatterToBase { batter: "Victor Rodriguez", distance: Distance::Single, fair_ball_type: FairBallType::LineDrive, fielder: PlacedPlayer { name: "Bob E. Quiros", place: Place::RightField }, scores: vec![], advances: vec![RunnerAdvance { runner: "Myra Roussel", base: Base::Third}], ejection: None }))
        );
    }

    #[test]
    fn a_big_pile_of_dirt() {
        let text = "🔨 Springfield Just Just Justice vs 🪱 Cabo Verde Caecilians @ A Big Pile of Dirt";
        let parsing_context = ParsingContext {
            game_id: "68d62be1a4ec9a9adffeebaf",
            event_log: &[],
            event_index: None,
            home_emoji_team: EmojiTeam { emoji: "🪱", name: "Cabo Verde Caecilians" },
            away_emoji_team: EmojiTeam { emoji: "🔨", name: "Springfield Just Just Justice" },
            season: 5,
            day: None
        };

        assert_eq!(
            super::live_now(&parsing_context).parse(text),
            Ok(("", ParsedEventMessage::LiveNow { away_team: parsing_context.away_emoji_team, home_team: parsing_context.home_emoji_team, stadium: Some("A Big Pile of Dirt") }))
        );
    }

    #[test]
    fn stanley_demir_i() {
        assert_eq!(
            name_eof("Stanley Demir I"),
            Ok(("", "Stanley Demir I"))
        )
    }

    #[test]
    fn dr_connor() {
        assert!(name_eof("Dr").is_err());
        assert!(name_eof("Dr. Connor").is_ok());
    }
}