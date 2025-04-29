use std::{env::args, fs::File, io::{self, Read}};

use mmolb_parsing::ParsedEvent;

pub struct LoadedEvents {
    pub game_id: String,
    pub events: Vec<ParsedEvent<String>>
}

pub fn parsed(ron_cache: &str) -> impl Iterator<Item = LoadedEvents> {
    std::fs::read_dir(ron_cache).unwrap()
        .map(|entry|  {
                let entry = entry.unwrap();

                let mut result = String::new();
                let game_id = entry.file_name().to_str().unwrap().strip_suffix(".ron").unwrap().to_string();
                File::open(entry.path()).unwrap().read_to_string(&mut result).unwrap();

                let events = result.lines().map(|line| ron::from_str(line).unwrap()).collect();
                LoadedEvents {
                    game_id,
                    events
                }
        }
    )
}
fn main() {
    let mut args = args().skip(1);

    let mut ron_cache = String::new();
    if let Some(cache) = args.next() {
        println!("Load ron events from: {cache}");
        ron_cache = cache;
    } else {
        println!("Load ron events from:");
        io::stdin().read_line(&mut ron_cache).unwrap();
        ron_cache = ron_cache.split_whitespace().next().unwrap().to_string();
    }

    let mut result = Vec::new();
    for (i, mut game) in parsed(&ron_cache).enumerate() {
        if game.events.len() > 99 {
            result.push(game.events.remove(99))
        }
        if i % 100 == 0 {
            println!("{i}")
        }
    }
    println!("{result:?}");
    println!("{}", result.len());
}