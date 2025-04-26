
use std::{env::args, fs::File, io::{self, Read, Write}};

use mmolb_parsing::{process_events, Game, ParsedEvent};
use serde::{Deserialize, Serialize};

pub fn downloaded(json_cache: &str) -> impl Iterator<Item = (String, Game)> {
    std::fs::read_dir(json_cache).unwrap()
        .map(|entry|  {
                let entry = entry.unwrap();

                let mut result = String::new();
                let game_id = entry.file_name().to_str().unwrap().strip_suffix(".json").unwrap().to_string();
                File::open(entry.path()).unwrap().read_to_string(&mut result).unwrap();
                (game_id, serde_json::from_str(&result).unwrap())
            }
        )
}

pub fn save_parsed_events<'de, S: Serialize + Deserialize<'de>>(ron_cache: &str, game_id: &str, events: Vec<ParsedEvent<S>>) {
    let ron_path = format!(r"{ron_cache}/{game_id}.ron");
    let mut file = File::create(ron_path).unwrap();
    for event in &events {
        writeln!(file, "{}", ron::to_string(&event).unwrap()).unwrap()
    }
}

fn main() {
    let mut args = args().skip(1);

    let mut json_cache = String::new();
    if let Some(cache) = args.next() {
        println!("Load json games from: {cache}");
        json_cache = cache;
    } else {
        println!("Load json games from:");
        io::stdin().read_line(&mut json_cache).unwrap();
        json_cache = json_cache.split_whitespace().next().unwrap().to_string();
    }

    let mut ron_cache = String::new();
    if let Some(cache) = args.next() {
        println!("Save ron events into: {cache}");
        ron_cache = cache;
    } else {
        println!("Save ron events into:");
        io::stdin().read_line(&mut ron_cache).unwrap();
        ron_cache = ron_cache.split_whitespace().next().unwrap().to_string();
    }



    let count = std::fs::read_dir(&json_cache).unwrap().count();
    println!("About to parse {count} games from {json_cache} into {ron_cache}");
    io::stdin().read_line(&mut String::new()).unwrap();

    let mut i = 0;
    for (game_id, game) in downloaded(&json_cache) {
        i += 1;
        if i % 100 == 0 {
            println!("{i}")
        }

        let events = process_events(&game);
        save_parsed_events(&ron_cache, &game_id, events);
    }
}
