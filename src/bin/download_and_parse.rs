
use std::{env::args, fs::File, io::{self, Write}, path::Path};

use futures::StreamExt;
use mmolb_parsing::{process_game, Game, ParsedEventMessage};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CasheGame {
    game_id: String,
    state: String,
}

pub fn save_parsed_messages<'de, S: Serialize + Deserialize<'de>>(ron_cache: &str, game_id: &str, events: Vec<ParsedEventMessage<S>>) {
    let ron_path = format!(r"{ron_cache}/{game_id}.ron");
    let mut file = File::create(ron_path).unwrap();
    for event in &events {
        writeln!(file, "{}", ron::to_string(&event).unwrap()).unwrap()
    }
}

pub async fn async_game_list() -> impl Iterator<Item =  String> {
    reqwest::get("https://freecashe.ws/api/games").await.unwrap().json::<Vec<CasheGame>>().await.unwrap().into_iter().filter(|game| game.state == "Complete").map(|game| game.game_id)
}

pub async fn ensure_in_cache_and_parse(json_cache:&str, ron_cache: &str, game_id: String) {
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

        let game: Game = serde_json::from_str(&response).unwrap();
        let events = process_game(&game);
        save_parsed_messages(ron_cache, &game_id, events);

        let mut file = File::create(&path).unwrap();
        write!(file, "{response}").unwrap();
    };
}

#[tokio::main]
async fn main() {
    let mut args = args().skip(1);

    let json_cache = args.next().expect("single argument \"json_cache\" should be present");
    let ron_cache = args.next().expect("second argument \"ron_cache\" should be present");
    println!("About to download games into {json_cache} and parse them into {ron_cache}. Press enter to continue");
    io::stdin().read_line(&mut String::new()).unwrap();

    let games = async_game_list().await;
    let mut stream = futures::stream::iter(games).map(|game| ensure_in_cache_and_parse(&json_cache, &ron_cache, game)).buffered(30);
    let mut i = 0;
    while let Some(()) = stream.next().await {
        i += 1;
        if i % 100 == 0 {
            println!("{i}");
        }
    }
}