
use std::{fs::File, io::Write, path::PathBuf, pin::pin};

use clap::Parser;
use futures::{Stream, StreamExt};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use mmolb_parsing::{process_event, Game};
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
    /// Where objects are saved to
    output_file: Option<String>,

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
}

fn cashews_fetch_games_json<'a>(client: &'a ClientWithMiddleware, extra: String) -> impl Stream<Item = Vec<EntityResponse<serde_json::Value>>> + 'a {
    async_stream::stream! {
        let mut url = format!("https://freecashe.ws/api/chron/v0/entities?kind=game&count=1000{extra}");
        loop {
            let response = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<serde_json::Value>>>().await.unwrap();
            info!("{} games fetched from cashews", response.items.len());
            yield response.items;

            if let Some(page) = response.next_page {
                url = format!("https://freecashe.ws/api/chron/v0/entities?kind=game&count=1000&page={page}{extra}");
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
    let output_file = args.output_file.as_ref().map(String::as_str);

    if let Some(id) = args.id {
        info!("Given a list of games: skipping cashews arguments and not caching");
        let client = get_caching_http_client(args.http_cache.map(Into::into), CacheMode::NoCache);
        let url = format!("https://freecashe.ws/api/chron/v0/entities?kind=game&id={id}");
        let games = client.get(&url).send().await.unwrap().json::<FreeCashewResponse<EntityResponse<serde_json::Value>>>().await.unwrap().items;
        for game in games.into_iter() {
            ingest_game(game, true, output_file, args.round_trip).await;
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


    let fetch = pin!(cashews_fetch_games_json(&client, extra));
    fetch.flat_map(|games| {
            let last = games.len() - 1;
            futures::stream::iter(games.into_iter().enumerate().map(move |(i, g)| (i == last, g)))
        })
        .then(|(verbose, game_json)| ingest_game(game_json, verbose, output_file, args.round_trip))
        .collect::<Vec<_>>()
        .await;
    drop(guard);
}

async fn ingest_game(response: EntityResponse<serde_json::Value>, verbose: bool, output_file: Option<&str>, round_trip: bool) {
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
    

    let mut file = output_file.map(|cache| {
        let ron_path = format!(r"{cache}/{}.ron", response.entity_id);
        File::create(ron_path).unwrap()
    });

    for event in &game.event_log {
        let parsed_event_message = process_event(event, &game);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_event_message.clone().unparse();
            if event.message != unparsed {
                error!("{} s{}d{}: event round trip failure expected:\n'{}'\nGot:\n'{}'", response.entity_id, game.season, game.day, event.message, unparsed);
            }
        }

        if let Some(file) = file.as_mut() {
            writeln!(file, "{}", ron::to_string(&parsed_event_message).unwrap()).unwrap();
        }
    }
    if verbose {
        let round_tripped = round_tripped.then_some(" with round trip").unwrap_or_default();
        info!("Parse{round_tripped} reached s{}d{}", game.season, game.day);
    }
}
