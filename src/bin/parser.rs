use std::{fs::File, io::Write, pin::pin};

use clap::{Parser, ValueEnum};
use futures::{Stream, StreamExt};
use mmolb_parsing::{enums::FeedEventSource, feed_event::parse_feed_event, player::Player, process_event, team::Team, Game};
use serde::{Deserialize, Serialize, de::IntoDeserializer};

use reqwest::Client;
use tracing::{error, info, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;


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
    /// AAY2hh1aLKs2ODEwNDBiYTU1NWZjODRhNjdiYTE5NjQ= - just before s1d1
    /// AAY3kbe9JHU2ODRlMGQwNDgzYzQzNTM1YzBjYTgzMWU= - just before s1d120
    /// AAY31bN8HNA2ODUyODAxNTJhOWQxOWZkMGFjZjI5OGY= - just before s1d200
    /// AAY4FghvuKk2ODU2YmFlNWQ2MjRiOTk4M2M2MjYxOTk= - just before s2d1
    /// AAY4idGlfZw2ODVlNGY4NmUyOTZlMTU0MjIwOTI2MTI= - just before s2d100
    /// AAY4r5t0A2k2ODYwYzg4OTFlNjVmNWZiNTJjYjVhODI= - just before s2d122 (after superstar day)
    /// AAY4xu6sKlQ2ODYyNTIxNzFlNjVmNWZiNTJjYjg0ZGE= - just before s2d150
    /// AAY41iifME82ODYzNGYzYTI0OGIxZjM1YmJlM2Q1YjQ= - just before s2d170
    /// 
    /// 
    /// AAY6COqRNIY2ODc3NmE3MjIwNmJjNGQyYTIwMDA5MTA= descending ~48
    /// AAY59Mru_z42ODc2MThmZjIwNmJjNGQyYTJmZmM4ZDg= descending ~24
    /// AAY56QeFxdc2ODc1NTQ0ZDYxNTQ5ODJjMzFmNWM1NmM= descending ~10
    #[arg(long)]
    start_page: Option<String>,

    #[clap(long, action)]
    round_trip: bool,

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

    #[clap(long)]
    output_folder: Option<String>,
}

#[derive(ValueEnum, Clone, Default, Debug, Copy)]
enum Kind {
    #[default]
    Game,
    Team,
    Player
}

fn cashews_fetch_json<'a>(client: &'a Client, kind: Kind, extra: String, start_page: Option<String>) -> impl Stream<Item = Vec<EntityResponse<Box<serde_json::value::RawValue>>>> + 'a {
    let kind = match kind {
        Kind::Game => "game",
        Kind::Team => "team",
        Kind::Player => "player"
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

#[tokio::main]
async fn main() {
    let writer = std::io::stderr.with_max_level(Level::WARN).and(std::io::stdout);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let args = Args::parse();

    let func = async |response, progress_report| match args.kind {
        Kind::Game => ingest_game(response, progress_report, &args).await,
        Kind::Team => ingest_team(response, &args).await,
        Kind::Player => ingest_player(response, &args).await
    };

    if let Some(id) = &args.id {
        let kind = match args.kind {
            Kind::Game => "game",
            Kind::Team => "team",
            Kind::Player => "player"
        };

        let client = Client::new();
        let url = format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&id={id}");
        let entities = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<Box<serde_json::value::RawValue>>>>().await.unwrap().items;
        for game in entities.into_iter() {
            func(game, true).await;
        }
        return;
    }

    let after = args.after.as_ref().map(|after| format!("&after={after}")).unwrap_or_default();
    let before = args.before.as_ref().map(|before| format!("&before={before}")).unwrap_or_default();
    let desc = args.desc.then_some("&order=desc").unwrap_or_default();
    let extra = format!("{after}{before}{desc}");

    let client = Client::new();


    let fetch = pin!(cashews_fetch_json(&client, args.kind, extra, args.start_page.clone()));
    fetch.flat_map(|games| {
        let last = games.len().max(1) - 1;
        futures::stream::iter(games.into_iter().enumerate().map(move |(i, o)| (i == last, o)))
    })
    .then(|(progress_report, game_json)| func(game_json, progress_report))
    .collect::<Vec<_>>()
    .await;
    drop(guard);
}

async fn ingest_game(response: EntityResponse<Box<serde_json::value::RawValue>>, progress_report: bool, args: &Args) {
    let _ingest_guard = tracing::span!(Level::INFO, "Entity Ingest", game_id = response.entity_id).entered();

    let game: Game = Game::deserialize(response.data.as_ref().into_deserializer()).map_err(|e| format!("Failed to deserialize {}, {e:?}", response.entity_id)).expect(&response.entity_id);

    let _span_guard = tracing::span!(Level::INFO, "Game", game_id = response.entity_id, season = game.season, day = format!("{:?}", game.day), scale = format!("{:?}", game.league_scale)).entered();

    if args.round_trip {
        let data = serde_json::Value::deserialize(response.data.into_deserializer()).unwrap();
        let round_tripped = serde_json::to_value(&game).unwrap();

        let diff = serde_json_diff::values(data, round_tripped);
        if let Some(diff) = diff {
            error!("round trip failed. Diff: {}", serde_json::to_string(&diff).unwrap());
        }
    }

    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());

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

        if let Some(f) = &mut output {
            writeln!(f, "{}", ron::to_string(&parsed_event_message).unwrap()).unwrap();
        }
        
        drop(_event_span_guard);
    }

    if progress_report {
        let round_tripped = args.round_trip.then_some(" with round trip").unwrap_or_default();
        info!("Parse{round_tripped} completed");
    }

    drop(_span_guard);
}

async fn ingest_team(response: EntityResponse<Box<serde_json::value::RawValue>>, args: &Args) {
    let team = Team::deserialize(response.data.as_ref().into_deserializer()).map_err(|e| format!("Failed to deserialize {}, {e:?}", response.entity_id)).expect(&response.entity_id);

    let _team_span_guard = tracing::span!(Level::INFO, "Team", team_id = response.entity_id, name = team.name).entered();

    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());
    
    if args.round_trip {
        let data = serde_json::Value::deserialize(response.data.into_deserializer()).unwrap();
        let round_tripped = serde_json::to_value(&team).unwrap();

        let diff = serde_json_diff::values(data, round_tripped);
        if let Some(diff) = diff {
            error!("round trip failed. Diff: {}", serde_json::to_string(&diff).unwrap());
        }
    }

    for event in team.feed.unwrap_or_default() {
        let _event_span_guard = tracing::span!(Level::INFO, "Feed Event", season = event.season, day = format!("{:?}", event.day), r#type = format!("{:?}", event.event_type), message = format!("{:?}", event.text)).entered();

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
    drop(_team_span_guard);
}

async fn ingest_player(response: EntityResponse<Box<serde_json::value::RawValue>>, args: &Args) {
    let _player_span_guard = tracing::span!(Level::INFO, "Deserializing Player", player_id = response.entity_id).entered();

    let player = Player::deserialize(response.data.as_ref().into_deserializer()).map_err(|e| format!("Failed to deserialize {}, {e:?}", response.entity_id)).expect(&response.entity_id);

    drop(_player_span_guard);

    let _player_span_guard = tracing::span!(Level::INFO, "Player", player_id = response.entity_id, name = format!("{} {}", player.first_name, player.last_name)).entered();
    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());
    
    if args.round_trip {
        let data = serde_json::Value::deserialize(response.data.into_deserializer()).unwrap();
        let round_tripped = serde_json::to_value(&player).unwrap();

        let diff = serde_json_diff::values(data, round_tripped);
        if let Some(diff) = diff {
            error!("round trip failed. Diff: {}", serde_json::to_string(&diff).unwrap());
        }
    }


    for event in player.feed.unwrap_or_default() {
        let _event_span_guard = tracing::span!(Level::INFO, "Feed Event", season = event.season, day = format!("{:?}", event.day), r#type = format!("{:?}", event.event_type), message = event.text).entered();

        let parsed_text = parse_feed_event(&event);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_text.unparse(&event, FeedEventSource::Player);
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
    drop(_player_span_guard);
}
