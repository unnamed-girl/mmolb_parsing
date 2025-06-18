use std::path::PathBuf;

use clap::Parser;
use futures::StreamExt;
use mmolb_parsing::team::Team;
use serde::{Deserialize, Serialize};
use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use tracing::{info, error, Level};
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[derive(Serialize, Deserialize)]
struct Response {
    items: Vec<Response2>
}


#[derive(Serialize, Deserialize)]
struct Response2 {
    team_id: String
}


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

#[derive(Parser, Debug)]
struct Args {
    /// Parent folder which the cache folder will be created in/loaded from
    #[arg(long)]
    http_cache: Option<String>,
}

#[tokio::main]
async fn main() {
    let writer = std::io::stderr.with_max_level(Level::WARN).and(std::io::stdout);

    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    let args = Args::parse();

    info!("Fetching teams list");
    let client = get_caching_http_client(args.http_cache.map(Into::into), CacheMode::Default);
    let teams = client.get("https://freecashe.ws/api/teams").send()
        .await.unwrap().json::<Response>().await.unwrap();

    let mut stream = futures::stream::iter(teams.items).map(|team_info| parse_team(&client, team_info)).buffered(30);
    while let Some(()) = stream.next().await {}

    drop(guard);
}

async fn parse_team(client: &ClientWithMiddleware, team: Response2) {
    let team = client.get(format!("https://mmolb.com/api/team/{}", team.team_id)).send()
        .await.unwrap().json::<Team>().await.unwrap();


    for event in &team.feed {
        let parsed = event.text.parse(*event.event_type.inner().unwrap());

        if parsed.is_error() {
            error!("Parse error: {parsed:?}")
        }
        let unparsed = parsed.unparse();
        if event.text.as_str() != unparsed {
            error!("Round trip doesn't match: {} != {}", event.text.as_str(), unparsed)
        }
    }

    info!("Parsed the {}", team.name);

    if team.extra_fields.len() > 0 {
        error!("Extra fields on team: {:?}", team.extra_fields);
    }
    for player in team.players {
        if player.extra_fields.len() > 0 {
            error!("Extra fields on player: {:?}", player.extra_fields);
            break;
        }
    }    
}