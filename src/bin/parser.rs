use std::{collections::HashSet, fs::File, io::{Read, Write}, path::Path, pin::pin};
use clap::{Parser, ValueEnum};
use futures::{Stream, StreamExt};
use mmolb_parsing::{enums::{FeedEventSource, FoulType}, feed_event::parse_feed_event, player::Player, player_feed::{parse_player_feed_event, PlayerFeed}, process_event, team::Team, Game, ParsedEventMessage};
use serde::{Deserialize, Serialize, de::IntoDeserializer};

use reqwest::Client;
use tracing::{error, info, span::EnteredSpan, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;
use strum::{IntoDiscriminant};

#[derive(Serialize, Deserialize)]
pub struct FreeCashewResponse<T> {
    pub items: Vec<T>,
    pub next_page: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct EntityResponse<T> {
    pub kind: String,
    pub entity_id: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub data: T
}

#[derive(Serialize, Deserialize)]
pub struct CasheGameInfo {
    game_id: String,
    state: String,
    season: u8,
    day: u16,
    home_team_id: String,
    away_team_id: String
}

#[derive(Parser, Debug)]
struct Args {
    /// One or more ids (comma separated)
    #[arg(long)]
    id: Option<String>,

    /// The page to start at.
    #[arg(long)]
    start_page: Option<String>,

    #[clap(long, action)]
    no_round_trip: bool,

    #[clap(short, long, action)]
    refetch: bool,

    #[clap(short, long, action)]
    verbose: bool,

    #[clap(long, action)]
    desc: bool,
    #[clap(long)]
    after: Option<String>,
    #[clap(long)]
    before: Option<String>,

    #[clap(long, default_value = "game")]
    kind: Kind,

    /// Gather distinct versions of each parsed event, and save the list to the given file. Also loads
    /// the current list from that file. 
    ///
    /// Exclusive to games right now.
    #[clap(long)]
    export_event_variants: Option<String>,

    #[clap(long)]
    output_folder: Option<String>,
}

#[derive(ValueEnum, Clone, Default, Debug, Copy)]
enum Kind {
    #[default]
    Game,
    Team,
    Player,
    PlayerFeed,
    GameFeed
}

fn cashews_fetch_json<'a>(client: &'a Client, kind: Kind, extra: String, start_page: Option<String>) -> impl Stream<Item = Vec<EntityResponse<Box<serde_json::value::RawValue>>>> + 'a {
    let kind = match kind {
        Kind::Game => "game",
        Kind::Team => "team",
        Kind::Player => "player",
        Kind::PlayerFeed => "player_feed",
        Kind::GameFeed => "game_feed",
    };
    async_stream::stream! {
        let (mut url, mut page) = match start_page {
            Some(page) => (format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&count=1000{extra}&page={page}"), Some(page)),
            None => (format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&count=1000{extra}"), None)
        };
        loop {
            info!("Fetching {kind}s from cashews page {page:?}");
            let response = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<Box<serde_json::value::RawValue>>>>().await.unwrap();
            info!("{} {kind}s fetched from cashews page {page:?}", response.items.len());
            page = response.next_page;
            yield response.items;

            if let Some(page) = &page {
                url = format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&count=1000&page={page}{extra}");
            } else {
                break
            }
        }
    }
}


static mut EVENT_VARIANTS: Option<HashSet<String>> = None;

#[tokio::main]
async fn main() {
    let writer = std::io::stderr.with_max_level(Level::WARN).and(std::io::stdout);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let args = Args::parse();

    if let Some(f) = &args.export_event_variants {
        if Path::new(f).exists() {
            let mut text = String::new();
            let mut file = File::open(f).unwrap();
            file.read_to_string(&mut text).unwrap();
            let variants = text.lines().map(|s| s.split("###").next().unwrap()).map(str::to_string).collect();
            unsafe {
                EVENT_VARIANTS = Some(variants)
            }
        }
    } 
    
    let func = |response, progress_report| match args.kind {
        Kind::Game=>ingest(response, &args,progress_report, game_inner),
        Kind::Team=>ingest(response, &args,progress_report, team_inner),
        Kind::Player=>ingest(response, &args,progress_report, player_inner),
        Kind::PlayerFeed => ingest(response, &args, progress_report, player_feed_inner),
        Kind::GameFeed => todo!(),
    };

    if let Some(id) = &args.id {
        let kind = match args.kind {
            Kind::Game=>"game",
            Kind::Team=>"team",
            Kind::Player=>"player",
            Kind::PlayerFeed => "player_feed",
            Kind::GameFeed => "team_feed",
        };

        let client = Client::new();
        let url = format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&id={id}");
        let entities = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<Box<serde_json::value::RawValue>>>>().await.unwrap().items;
        for game in entities.into_iter() {
            func(game, true);
        }
        return;
    }

    let after = args.after.as_ref().map(|after| format!("&after={after}")).unwrap_or_default();
    let before = args.before.as_ref().map(|before| format!("&before={before}")).unwrap_or_default();
    let desc = args.desc.then_some("&order=desc").unwrap_or_default();
    let extra = format!("{after}{before}{desc}");

    let client = Client::new();


    let mut fetch = pin!(cashews_fetch_json(&client, args.kind, extra, args.start_page.clone()));

    while let Some(games) = fetch.next().await {
        let last = games.len().max(1) - 1;
        for (i, game) in games.into_iter().enumerate() {
            func(game, i == last)
        }
    }
    drop(guard);
}

fn ingest<'de, T: for<'a> Deserialize<'a> + Serialize>(response: EntityResponse<Box<serde_json::value::RawValue>>, args: &Args, progress_report: bool, inner_checks: impl Fn(T, EntityResponse<Box<serde_json::value::RawValue>>, &Args) -> EnteredSpan) {
    let _ingest_guard = tracing::span!(Level::INFO, "Entity Ingest", entity_id = response.entity_id).entered();

    let entity = T::deserialize(response.data.as_ref().into_deserializer()).map_err(|e| format!("Failed to deserialize {}, {e:?}", response.entity_id)).expect(&response.entity_id);

    let valid_from = response.valid_from.clone();
    
    if !args.no_round_trip {
        let data = serde_json::Value::deserialize(response.data.into_deserializer()).unwrap();
        let round_tripped = serde_json::to_value(&entity).unwrap();

        let diff = serde_json_diff::values(data, round_tripped);
        if let Some(diff) = diff {
            error!("round trip failed. Diff: {}", serde_json::to_string(&diff).unwrap());
        }
    }

    let span = inner_checks(entity, response, args);

    if progress_report {
        tracing::info!("Reached {}", valid_from);
    }

    drop(span);
}

fn player_inner(player: Player,response: EntityResponse<Box<serde_json::value::RawValue>>,  args: &Args) -> EnteredSpan {
    let _player_span_guard = tracing::span!(Level::INFO, "Player", name = format!("{} {}", player.first_name, player.last_name)).entered();
    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());
    
    for event in player.feed.unwrap_or_default() {
        let _event_span_guard = tracing::span!(Level::INFO, "Feed Event", season = event.season, day = format!("{:?}", event.day), timestamp = event.timestamp.to_string(), r#type = format!("{:?}", event.event_type), message = event.text).entered();

        let parsed_text = parse_player_feed_event(&event);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_text.unparse(&event);
            if event.text != unparsed {
                error!("Feed event round trip failure expected:\n'{}'\nGot:\n'{}'", event.text, unparsed);
            }
        }

        if args.verbose {
            info!("{:?} ({})", parsed_text, event.text);
        }

        if let Some(f) = &mut output {
            writeln!(f, "{}", ron::to_string(&parsed_text).unwrap()).unwrap();
        }

        drop(_event_span_guard);
    }
    _player_span_guard
}

fn team_inner(team: Team, response: EntityResponse<Box<serde_json::value::RawValue>>,  args: &Args) -> EnteredSpan {
    let _team_span_guard = tracing::span!(Level::INFO, "Team", name = team.name).entered();
    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());

    for event in team.feed.unwrap_or_default() {
        let _event_span_guard = tracing::span!(Level::INFO, "Feed Event", season = event.season, day = format!("{:?}", event.day), timestamp = event.timestamp.to_string(), r#type = format!("{:?}", event.event_type), message = format!("{:?}", event.text)).entered();

        let parsed_text = parse_feed_event(&event);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_text.unparse(&event, FeedEventSource::Team);
            if event.text != unparsed {
                error!("Feed event round trip failure expected:\n'{}'\nGot:\n'{}'", event.text, unparsed);
            }
        }

        if args.verbose {
            info!("{:?} ({})", parsed_text, event.text);
        }

        if let Some(f) = &mut output {
            writeln!(f, "{}", ron::to_string(&parsed_text).unwrap()).unwrap();
        }

        drop(_event_span_guard);
    }

    _team_span_guard
}

fn game_inner(game: Game, response: EntityResponse<Box<serde_json::value::RawValue>>,  args: &Args) -> EnteredSpan {
    let _game_guard = tracing::span!(Level::INFO, "Game", season = game.season, day = format!("{:?}", game.day), scale = format!("{:?}", game.league_scale)).entered();

    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());
    let mut event_variants_file = args.export_event_variants.as_ref().map(|f| {
            File::options().append(true).open(f).unwrap()
    });

    for event in &game.event_log {
        let _event_span_guard = tracing::span!(Level::INFO, "Event", index = event.index, r#type = format!("{:?}", event.event), message = event.message).entered();

        let parsed_event_message = process_event(event, &game, &response.entity_id);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_event_message.unparse(&game, event.index);
            if event.message != unparsed {
                error!("Event round trip failure expected:\n'{}'\nGot:\n'{}'", event.message, unparsed);
            }
        }

        if args.verbose {
            info!("{:?} ({})", parsed_event_message, event.message);
        }

        if let Some(ref mut f) = event_variants_file {
            let checked = check(&parsed_event_message);
            // SAFETY: we're single threaded and we later use `let _ = event_variants_ref` to stop holding onto the reference.
            // todo: do this literally any other way
            let event_variants_ref = unsafe {
                 EVENT_VARIANTS.as_mut().unwrap()
            };
            if !event_variants_ref.contains(&checked) {
                writeln!(f, "{checked}###{}?event={}", response.entity_id, event.index.map(|n| n as i32).unwrap_or(-1)).unwrap();
                
                event_variants_ref.insert(checked);
            }

            let _ = event_variants_ref; // stops us from accidentally holding onto a copy of a mutable reference
        }

        if let Some(f) = &mut output {
            writeln!(f, "{}", ron::to_string(&parsed_event_message).unwrap()).unwrap();
        }
        
        drop(_event_span_guard);
    }
    _game_guard
}

fn player_feed_inner(feed: PlayerFeed, response: EntityResponse<Box<serde_json::value::RawValue>>,  args: &Args) -> EnteredSpan {
    let _player_feed_span_guard = tracing::span!(Level::INFO, "Player Feed").entered();
    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());
    
    for event in feed.feed {
        let _event_span_guard = tracing::span!(Level::INFO, "Feed Event", season = event.season, day = format!("{:?}", event.day), timestamp = event.timestamp.to_string(), r#type = format!("{:?}", event.event_type), message = event.text).entered();

        let parsed_text = parse_player_feed_event(&event);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_text.unparse(&event);
            if event.text != unparsed {
                error!("Feed event round trip failure expected:\n'{}'\nGot:\n'{}'", event.text, unparsed);
            }
        }

        if args.verbose {
            info!("{:?} ({})", parsed_text, event.text);
        }

        if let Some(f) = &mut output {
            writeln!(f, "{}", ron::to_string(&parsed_text).unwrap()).unwrap();
        }

        drop(_event_span_guard);
    }
    _player_feed_span_guard
}

fn check<S>(event: &ParsedEventMessage<S>) -> String {
    let discriminant_name = event.discriminant().to_string();
    let unique = match event {
        ParsedEventMessage::ParseError { error: _, message: _ } => "".to_string(),
        ParsedEventMessage::KnownBug { bug } => {
            format!("Bug: {}", bug.discriminant())
        },
        ParsedEventMessage::LiveNow { away_team: _, home_team: _, stadium } => {
            format!("Stadium: {}", stadium.is_some())
        },
        ParsedEventMessage::PitchingMatchup { away_team: _, home_team: _, home_pitcher: _, away_pitcher: _ } => "".to_string(),
        ParsedEventMessage::Lineup { side, players } => {
            format!("Side: {side}, player_count: {}", players.len())
        },
        ParsedEventMessage::PlayBall => "".to_string(),
        ParsedEventMessage::GameOver { message } => format!("Message: {message}"),
        ParsedEventMessage::Recordkeeping { winning_team: _, losing_team: _, winning_score: _, losing_score: _ } => "".to_string(),
        ParsedEventMessage::InningStart { number, side, batting_team: _, automatic_runner, pitcher_status } => {
            format!("number: {number}, side: {side}, automatic_runner: {}, pitcher_status: {}", automatic_runner.is_some(), pitcher_status.discriminant())
        },
        ParsedEventMessage::NowBatting { batter: _, stats } => {
            format!("stats: {}", stats.discriminant())
        },
        ParsedEventMessage::InningEnd { number, side } => {
            format!("number: {number}, side: {side}")
        },
        ParsedEventMessage::MoundVisit { team: _, mound_visit_type } => {
            format!("type: {}", mound_visit_type)
        },
        ParsedEventMessage::PitcherRemains { remaining_pitcher: _ } => "".to_string(),
        ParsedEventMessage::PitcherSwap { leaving_pitcher_emoji, leaving_pitcher: _, arriving_pitcher_emoji, arriving_pitcher_place, arriving_pitcher_name: _ } => {
            format!("leaving_pitcher_emoji: {}, arriving_pitcher_emoji: {}, arriving_pitcher_place: {}", leaving_pitcher_emoji.is_some(), arriving_pitcher_emoji.is_some(), arriving_pitcher_place.is_some())  
        },
        ParsedEventMessage::Ball { steals, count: _, cheer, aurora_photos, ejection, door_prizes } => {
            format!("steals: {}, cheer: {}, aurora_photos: {}, ejection: {}, door_prizes: {}", steals.len(), cheer.is_some(), aurora_photos.is_some(), ejection.is_some(), door_prizes.len())
        },
        ParsedEventMessage::Strike { strike, steals, count: _, cheer, aurora_photos, ejection, door_prizes } => {
            format!("strike: {strike}, steals: {}, cheer: {}, aurora_photos: {}, ejection: {}, door_prizes: {}", steals.len(), cheer.is_some(), aurora_photos.is_some(), ejection.is_some(), door_prizes.len())
        },
        ParsedEventMessage::Foul { foul, steals, count: _, cheer, aurora_photos, door_prizes } => {
            format!("foul: {foul}, steals: {}, cheer: {}, aurora_photos: {}, door_prizes: {}", steals.len(), cheer.is_some(), aurora_photos.is_some(), door_prizes.len())
        },
        ParsedEventMessage::Walk { batter: _, scores, advances: _, cheer, aurora_photos, ejection } => {
            format!("scores: {}, cheer: {}, aurora_photos: {}, ejection: {}", scores.len(), cheer.is_some(), aurora_photos.is_some(), ejection.is_some())
        },
        ParsedEventMessage::HitByPitch { batter: _, scores, advances: _, cheer, aurora_photos, ejection, door_prizes } => {
            format!("scores: {}, cheer: {}, aurora_photos: {}, ejection: {}, door_prizes: {}", scores.len(), cheer.is_some(), aurora_photos.is_some(), ejection.is_some(), door_prizes.len())
        },
        ParsedEventMessage::FairBall { batter: _, fair_ball_type, destination, cheer, aurora_photos, door_prizes } => {
            format!("fair_ball_type: {fair_ball_type}, destination: {destination}, cheer: {}, aurora_photos: {}, door_prizes: {}", cheer.is_some(), aurora_photos.is_some(), door_prizes.len())
        },
        ParsedEventMessage::StrikeOut { foul, batter: _, strike, steals, cheer, aurora_photos, ejection } => {
            format!("foul: {}, strike: {strike}, steals: {}, cheer: {}, aurora_photos: {}, ejection: {}", foul.as_ref().map(FoulType::to_string).unwrap_or_else(|| "False".to_string()), steals.len(), cheer.is_some(), aurora_photos.is_some(), ejection.is_some())
        },
        ParsedEventMessage::BatterToBase { batter: _, distance, fair_ball_type, fielder: _, scores, advances, ejection } => {
            format!("distance: {distance}, fair_ball_type: {fair_ball_type}, scores: {}, advances: {}, ejection: {}", scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::HomeRun { batter: _, fair_ball_type, destination, scores, grand_slam, ejection } => {
            format!("fair_ball_type: {fair_ball_type}, destination: {destination}, grand_slam: {grand_slam}, scores: {}, ejection: {}", scores.len(), ejection.is_some())
        },
        ParsedEventMessage::CaughtOut { batter: _, fair_ball_type, caught_by: _, scores, advances, sacrifice, perfect, ejection } => {
            format!("fair_ball_type: {fair_ball_type}, sacrifice: {sacrifice}, perfect: {perfect}, scores: {}, advances: {}, ejection: {}", scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::GroundedOut { batter: _, fielders, scores, advances, perfect, ejection } => {
            format!("fielders: {}, perfect: {perfect}, scores: {}, advances: {}, ejection: {}", fielders.len(), scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::ForceOut { batter: _, fielders, fair_ball_type: _, out: _, scores, advances, ejection } => {
            format!("fielders: {}, scores: {}, advances: {}, ejection: {}", fielders.len(), scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::ReachOnFieldersChoice { batter: _, fielders, result, scores, advances, ejection } => {
            format!("fielders: {}, result: {}, scores: {}, advances: {}, ejection: {}", fielders.len(), result.discriminant(), scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::DoublePlayGrounded { batter: _, fielders, out_one: _, out_two: _, scores, advances, sacrifice, ejection } => {
            format!("fielders: {}, sacrifice: {sacrifice}, scores: {}, advances: {}, ejection: {}", fielders.len(), scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::DoublePlayCaught { batter: _, fair_ball_type, fielders, out_two: _, scores, advances, ejection } => {
            format!("fielders: {}, fair_ball_type: {fair_ball_type}, scores: {}, advances: {}, ejection: {}", fielders.len(), scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::ReachOnFieldingError { batter: _, fielder: _, error, scores, advances, ejection } => {
            format!("error: {error}, scores: {}, advances: {}, ejection: {}", scores.len(), advances.len(), ejection.is_some())
        },
        ParsedEventMessage::WeatherDelivery { delivery: _ } => "".to_string(),
        ParsedEventMessage::FallingStar { player_name: _ } => "".to_string(),
        ParsedEventMessage::FallingStarOutcome { deflection, player_name: _, outcome } => {
            format!("deflection: {}, outcome: {}", deflection.is_some(), outcome.discriminant())
        },
        ParsedEventMessage::WeatherShipment { deliveries } => format!("deliveries: {}", deliveries.len()),
        ParsedEventMessage::WeatherSpecialDelivery { delivery: _ } => "".to_string(),
        ParsedEventMessage::Balk { pitcher: _, scores, advances } => {
            format!("scores: {}, advances: {}", scores.len(), advances.len())
        },
        ParsedEventMessage::WeatherProsperity { home_income: _, away_income: _ } => "".to_string(),
        ParsedEventMessage::PhotoContest { winning_team: _, winning_tokens: _, winning_player: _, winning_score: _, losing_team: _, losing_tokens: _, losing_player: _, losing_score: _ } => "".to_string(),
        ParsedEventMessage::Party { pitcher_name: _, pitcher_amount_gained: _, pitcher_attribute: _, batter_name: _, batter_amount_gained: _, batter_attribute: _ } => "".to_string(),
    };

    format!("{discriminant_name} ({unique})")
}