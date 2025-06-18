
use std::{fs::File, io::Write, path::PathBuf};

use clap::Parser;
use futures::StreamExt;
use mmolb_parsing::{process_event, raw_game::RawGame, Game};
use serde::{Deserialize, Serialize};

use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use tracing::{error, info, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;


pub fn get_caching_http_client(cache: Option<PathBuf>) -> ClientWithMiddleware {
    ClientBuilder::new(Client::new())
        .with(Cache(HttpCache {
            mode: CacheMode::ForceCache,
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

    let client = get_caching_http_client(args.http_cache.map(Into::into));
    
    info!("Fetching games list");

    let mut games = Vec::new();
    let mut url = format!("https://freecashe.ws/api/games?season={}", args.season);
    loop {
        let response = client.get(&url).with_extension(CacheMode::Default).send().await.unwrap().json::<FreeCashewResponse>().await.unwrap();
        if response.next_page.is_none() {
            break;
        }
        games.extend(response.items.into_iter().filter(|game_info| game_info.state == "Complete" && args.from_day.is_none_or(|day| game_info.day >= day)));        
        url = format!("https://freecashe.ws/api/games?season={}&page={}", args.season, response.next_page.unwrap());
    }

    if let Some(output_file) = output_file {
        info!("Parsing {} games into {output_file}", games.len());
    } else {
        info!("Parsing {} games",  games.len())
    }

    let mut stream = futures::stream::iter(games).map(|game_info| ingest_game(&client, game_info, output_file)).buffered(30);

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
            if event.message != parsed_event_message.clone().unparse() {
                error!("{} s{}d{}: event round trip failure '{}'", game_info.game_id, game.season, game.day, event.message);
            }
        }

        if let Some(file) = file.as_mut() {
            writeln!(file, "{}", ron::to_string(&parsed_event_message).unwrap()).unwrap();
        }
    }

    info!("{} s{}d{} parsed", game_info.game_id, game_info.season, game_info.day);
}
