
use std::{path::PathBuf, pin::pin};

use clap::{Parser, ValueEnum};
use futures::{Stream, StreamExt};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use mmolb_parsing::{enums::MaybeRecognized, process_event, team::Team, Game};
use serde::{Deserialize, Serialize};

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

    #[clap(long, action)]
    round_trip: bool,

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
    kind: Kind
}

#[derive(ValueEnum, Clone, Default, Debug, Copy)]
enum Kind {
    #[default]
    Game,
    Team
}

fn cashews_fetch_json<'a>(client: &'a ClientWithMiddleware, kind: Kind, extra: String) -> impl Stream<Item = Vec<EntityResponse<serde_json::Value>>> + 'a {
    let kind = match kind {
        Kind::Game => "game",
        Kind::Team => "team"
    };
    async_stream::stream! {
        let mut url = format!("https://freecashe.ws/api/chron/v0/entities?kind={kind}&count=1000{extra}");
        loop {
            let response = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<serde_json::Value>>>().await.unwrap();
            info!("{} {kind}s fetched from cashews", response.items.len());
            yield response.items;

            if let Some(page) = response.next_page {
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

    let func = async |response, verbose| match args.kind {
        Kind::Game => ingest_game(response, verbose, args.round_trip).await,
        Kind::Team => ingest_team(response, args.round_trip).await
    };

    if let Some(id) = args.id {
        info!("Given a specific entities: skipping cashews arguments and not caching");
        let client = get_caching_http_client(args.http_cache.map(Into::into), CacheMode::NoCache);
        let url = format!("https://freecashe.ws/api/chron/v0/entities?kind=game&id={id}");
        let entities = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<serde_json::Value>>>().await.unwrap().items;
        for game in entities.into_iter() {
            func(game, true).await;
        }
        return;
    }

    let after = args.after.map(|after| format!("&after={after}")).unwrap_or_default();
    let before = args.before.map(|before| format!("&before={before}")).unwrap_or_default();
    let desc = args.desc.then_some("&order=desc").unwrap_or_default();
    let extra = format!("{after}{before}{desc}");

    let mode = if extra.is_empty() {
        info!("Requests being saved to cache");
        CacheMode::ForceCache
    } else {
        info!("Nonstandard chron arguments: no caching");
        CacheMode::NoCache
    };

    let client = get_caching_http_client(args.http_cache.map(Into::into), mode);


    let fetch = pin!(cashews_fetch_json(&client, args.kind, extra));
    fetch.flat_map(|games| {
        let last = games.len() - 1;
        futures::stream::iter(games.into_iter().enumerate().map(move |(i, o)| (i == last, o)))
    })
    .then(|(verbose, game_json)| func(game_json, verbose))
    .collect::<Vec<_>>()
    .await;
    drop(guard);
}

async fn ingest_game(response: EntityResponse<serde_json::Value>, verbose: bool, round_trip: bool) {
    let (game, round_tripped) = if round_trip {
        let game: Game = serde_json::from_value(response.data.clone()).unwrap();
        let round_tripped = serde_json::to_value(&game).unwrap();
        if response.data != round_tripped {
            error!("{} s{}d{}: round trip failed.", response.entity_id, game.season, game.day);
        }
        (game, true)
    } else {
        (serde_json::from_value(response.data).unwrap(), false)
    };


    for event in &game.event_log {
        let parsed_event_message = process_event(event, &game);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_event_message.clone().unparse();
            if event.message != unparsed {
                error!("{} s{}d{}: event round trip failure expected:\n'{}'\nGot:\n'{}'", response.entity_id, game.season, game.day, event.message, unparsed);
            }
        }
    }
    if verbose {
        let round_tripped = round_tripped.then_some(" with round trip").unwrap_or_default();
        info!("Parse{round_tripped} reached s{}d{}", game.season, game.day);
    }
}

async fn ingest_team(response: EntityResponse<serde_json::Value>, round_trip: bool) {
    let team = if round_trip {
        let team: Team = serde_json::from_value(response.data.clone()).unwrap();
        let round_tripped = serde_json::to_value(&team).unwrap();
        if response.data != round_tripped {
            error!("{} round trip failed.", response.entity_id);
        }
        team
    } else {
        serde_json::from_value(response.data).unwrap()
    };

    for event in team.feed {
        match event.event_type {
            MaybeRecognized::NotRecognized(event_type) => error!("{event_type} is not a recognized event type"),
            MaybeRecognized::Recognized(event_type) => {
                let parsed_text = event.text.parse(event_type);
                if tracing::enabled!(Level::ERROR) {
                    let unparsed = parsed_text.unparse();
                    if event.text.0 != unparsed {
                        error!("{}: feed event round trip failure expected:\n'{}'\nGot:\n'{}'", response.entity_id, event.text, unparsed);
                    }
                }
            }
        }
    }
}
