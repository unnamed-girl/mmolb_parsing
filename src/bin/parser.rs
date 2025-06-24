
use std::{fs::File, io::Write, path::PathBuf, pin::pin};

use clap::Parser;
use futures::{Stream, StreamExt};
use mmolb_parsing::{process_event, raw_game::RawGame, Game};
use serde::{Deserialize, Serialize};

use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
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
pub struct FreeCashewResponse {
    pub items: Vec<CasheGame>,
    pub next_page: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct CasheGame {
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

    /// Season
    #[arg(short = 's', long, default_value_t = 1)]
    season: u8,
    /// Earliest day
    #[arg(short = 'd', long)]
    from_day: Option<u16>,

    #[clap(long, short, action)]
    refetch_games: bool
}

fn cashews_fetch<'a>(client: &'a ClientWithMiddleware, season: u8) -> impl Stream<Item = Vec<CasheGame>> + 'a {
    async_stream::stream! {
        let mut url = format!("https://freecashe.ws/api/games?season={season}");
        loop {
            let response = client.get(&url).with_extension(CacheMode::ForceCache).send().await.unwrap().json::<FreeCashewResponse>().await.unwrap();
            yield response.items;

            if let Some(page) = response.next_page {
                url = format!("https://freecashe.ws/api/games?season={season}&page={page}");
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

    let mode = if args.refetch_games {
        CacheMode::Reload
    } else {
        CacheMode::ForceCache
    };

    let client = get_caching_http_client(args.http_cache.map(Into::into), mode);
    
    let fetch = pin!(cashews_fetch(&client, args.season));
    let mut stream = fetch.flat_map(|games| {
            info!("{} game infos fetched from cashews", games.len());
            futures::stream::iter(games)
        })
        .map(|game_info| ingest_game(&client, game_info, output_file))
        .buffered(10);

    while let Some(()) = stream.next().await {}

    drop(guard);
}

async fn ingest_game(client: &ClientWithMiddleware, game_info: CasheGame, output_file: Option<&str>) {
    let raw_game = client.get(format!("https://mmolb.com/api/game/{}", game_info.game_id)).send().await.unwrap().json::<RawGame>().await.unwrap();

    let game: Game = raw_game.clone().into();

    let mut file = output_file.map(|cache| {
        let ron_path = format!(r"{cache}/{}.ron", game_info.game_id);
        File::create(ron_path).unwrap()
    });

    for event in &game.event_log {
        let parsed_event_message = process_event(event, &game);
        if tracing::enabled!(Level::ERROR) {
            let unparsed = parsed_event_message.clone().unparse();
            if event.message != unparsed {
                error!("{} s{}d{}: event round trip failure expected:\n'{}'\nGot:\n'{}'", game_info.game_id, game.season, game.day, event.message, unparsed);
            }
        }

        if let Some(file) = file.as_mut() {
            writeln!(file, "{}", ron::to_string(&parsed_event_message).unwrap()).unwrap();
        }
    }

    info!("{} s{}d{} parsed", game_info.game_id, game_info.season, game_info.day);
}
