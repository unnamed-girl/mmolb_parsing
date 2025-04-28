
use std::{env::args, fs::File, io::{self, Write}, path::Path};

use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CasheGame {
    game_id: String,
    state: String,
}

pub async fn async_game_list() -> impl Iterator<Item =  String> {
    reqwest::get("https://freecashe.ws/api/games").await.unwrap().json::<Vec<CasheGame>>().await.unwrap().into_iter().filter(|game| game.state == "Complete").map(|game| game.game_id)
}

pub async fn ensure_in_cache(json_cache:&str, game_id: String) {
    let path = format!(r"{json_cache}/{game_id}.json");
    if !Path::exists(path.as_ref()) {
        let response = reqwest::get(format!("https://mmolb.com/api/game/{game_id}"))
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .text()
            .await
            .unwrap();
        let mut file = File::create(&path).unwrap();
        write!(file, "{response}").unwrap();
    };
}

#[tokio::main]
async fn main() {
    let mut args = args().skip(1);

    let mut json_cache = String::new();
    if let Some(cache) = args.next() {
        println!("Save json games into: {cache}");
        json_cache = cache;
    } 

    json_cache = json_cache.split_whitespace().next().unwrap().to_string();
    println!("About to download games into {json_cache}");
    io::stdin().read_line(&mut String::new()).unwrap();

    let games = async_game_list().await;
    let mut stream = futures::stream::iter(games).map(|game| ensure_in_cache(&json_cache, game)).buffered(30);
    let mut i = 0;
    while let Some(()) = stream.next().await {
        i += 1;
        if i % 100 == 0 {
            println!("{i}");
        }
    }
}