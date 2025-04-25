
use std::{fs::File, io::{self, Read, Write}};

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

pub async fn load_or_download(json_cache:&str, game_id: String) -> String {
    let path = format!(r"{json_cache}/{game_id}.json");
    let result = if let Ok(mut file) = File::open(&path) {
        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();
        buf
    } else {
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
        response
    };

    result
}

#[tokio::main]
async fn main() {
    let mut json_cache = String::new();
    println!("Save json games into:");
    io::stdin().read_line(&mut json_cache).unwrap();

    json_cache = json_cache.split_whitespace().next().unwrap().to_string();
    println!("About to download games into {json_cache}");
    io::stdin().read_line(&mut String::new()).unwrap();

    let games = async_game_list().await;
    let mut stream = futures::stream::iter(games).map(|game| load_or_download(&json_cache, game)).buffered(10);
    let mut i = 0;
    while let Some(_) = stream.next().await {
        i += 1;
        println!("{i}");
    }
}