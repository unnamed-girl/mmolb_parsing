
use std::{fs::File, io::Write, path::PathBuf, pin::pin};

use clap::{Parser, ValueEnum};
use futures::{Stream, StreamExt};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use mmolb_parsing::{enums::MaybeRecognized, process_event, team::Team, Game};
use serde::{Serialize, Deserialize};

use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use tracing::{error, info, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;


pub fn get_caching_http_client(cache: Option<PathBuf>, mode: CacheMode) -> ClientWithMiddleware {
    ClientBuilder::new(Client::new())
        .with(Cache(HttpCache {
            mode,
            manager: cache.map(|cache| CACacheManager {
                path: cache.join("http-cacache"),
            }).unwrap_or_default(),
            options: HttpCacheOptions::default(),
        }))
        .build()
}

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
    /// Parent folder which the cache folder will be created in/loaded from
    #[arg(long)]
    http_cache: Option<String>,

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
    #[arg(long)]
    start_page: Option<String>,

    #[clap(long, action)]
    round_trip: bool,

    #[clap(short, long, action)]
    refetch: bool,

    #[clap(short, long, action)]
    verbose: bool,

    /// Nonstandard chron requests aren't cached
    #[clap(long, action)]
    desc: bool,
    /// Nonstandard chron requests aren't cached
    #[clap(long)]
    after: Option<String>,
    /// Nonstandard chron requests aren't cached
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
    Team
}

fn cashews_fetch_json<'a>(client: &'a ClientWithMiddleware, kind: Kind, extra: String, start_page: Option<String>) -> impl Stream<Item = Vec<EntityResponse<serde_json::Value>>> + 'a {
    let kind = match kind {
        Kind::Game => "game",
        Kind::Team => "team"
    };
    async_stream::stream! {
        let (mut url, mut page) = match start_page {
            Some(page) => (format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&count=1000{extra}&page={page}"), Some(page)),
            None => (format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&count=1000{extra}"), None)
        };
        loop {
            let response = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<serde_json::Value>>>().await.unwrap();
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
        Kind::Team => ingest_team(response, &args).await
    };

    if let Some(id) = &args.id {
        info!("Given a specific entities: skipping cashews arguments and not caching");
        let client = get_caching_http_client(args.http_cache.as_ref().map(Into::into), CacheMode::NoCache);
        let url = format!("https://freecashe.ws/api/chron/v0/entities?kind=game&id={id}");
        let entities = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<serde_json::Value>>>().await.unwrap().items;
        for game in entities.into_iter() {
            func(game, true).await;
        }
        return;
    }

    let after = args.after.as_ref().map(|after| format!("&after={after}")).unwrap_or_default();
    let before = args.before.as_ref().map(|before| format!("&before={before}")).unwrap_or_default();
    let desc = args.desc.then_some("&order=desc").unwrap_or_default();
    let extra = format!("{after}{before}{desc}");

    let mode = if extra.is_empty() {
        info!("Requests being saved to cache");
        if args.refetch {
            CacheMode::Reload
        } else {
            CacheMode::ForceCache
        }
    } else {
        info!("Nonstandard chron arguments: no caching");
        CacheMode::NoCache
    };

    let client = get_caching_http_client(args.http_cache.as_ref().map(Into::into), mode);


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

async fn ingest_game(response: EntityResponse<serde_json::Value>, progress_report: bool, args: &Args) {
    let round_trip_data = args.round_trip.then(|| response.data.clone());
    let game: Game = serde_json::from_value(response.data).map_err(|e| format!("Failed to deserialize {}, {e:?}", response.entity_id)).unwrap();

    let _span_guard = tracing::span!(Level::INFO, "Game", game_id = response.entity_id, season = game.season, day = game.day.to_string(), scale = game.league_scale.to_string()).entered();

    if let Some(data) = round_trip_data {
        let round_tripped = serde_json::to_value(&game).unwrap();

        let diff = serde_json_diff::values(data, round_tripped);
        if let Some(diff) = diff {
            error!("round trip failed. Diff: {}", serde_json::to_string(&diff).unwrap());
        }
    }

    let mut output = args.output_folder.as_ref().map(|folder| File::create(format!("{folder}/{}.ron", response.entity_id)).unwrap());

    for event in &game.event_log {
        let _event_span_guard = tracing::span!(Level::INFO, "Event", index = event.index, r#type = event.event.to_string(), message = event.message).entered();

        let parsed_event_message = process_event(event, &game, &response.entity_id);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_event_message.clone().unparse(&game, event.index);
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

async fn ingest_team(response: EntityResponse<serde_json::Value>, args: &Args) {
    let round_trip_data = args.round_trip.then(|| response.data.clone());
    let team: Team = serde_json::from_value(response.data).unwrap();

    let _team_span_guard = tracing::span!(Level::INFO, "Team", team_id = response.entity_id, name = team.name).entered();
    
    if let Some(data) = round_trip_data {
        let round_tripped = serde_json::to_value(&team).unwrap();
        let diff = serde_json_diff::values(data, round_tripped);
        if let Some(diff) = diff {
            error!("{} round trip failed. Diff: {}", response.entity_id, serde_json::to_string(&diff).unwrap());
        }
    }

    for event in team.feed {
        let _event_span_guard = tracing::span!(Level::INFO, "Feed Event", season = event.season, day = event.season.to_string(), r#type = event.event_type.to_string(), message = event.text.to_string()).entered();
        match event.event_type {
            MaybeRecognized::NotRecognized(event_type) => error!("{event_type} is not a recognized event type"),
            MaybeRecognized::Recognized(event_type) => {
                let parsed_text = event.text.parse(event_type);
                if tracing::enabled!(Level::ERROR) {
                    let unparsed = parsed_text.unparse(&event);
                    if event.text.0 != unparsed {
                        error!("{} s{}d{}: feed event round trip failure expected:\n'{}'\nGot:\n'{}'", response.entity_id, event.season, event.day, event.text, unparsed);
                    }
                }
            }
        }
        drop(_event_span_guard);
    }
    drop(_team_span_guard);
}
