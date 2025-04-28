use std::{env::args, fs::File, io::{self, Read}};

use mmolb_parsing::{enums::{Inning, Side}, ParsedEvent};

pub struct RonFile(String);
impl RonFile {
    pub fn new(contents: String) -> Self {
        Self(contents)
    }
    pub fn events<'a>(&'a self) -> impl Iterator<Item = ParsedEvent<String>> + use<'a>{
        self.0.lines().map(|line| ron::from_str(line).unwrap())
    }
}

pub fn parsed(ron_cache: &str) -> impl Iterator<Item = (String, Vec<ParsedEvent<String>>)> {
    std::fs::read_dir(ron_cache).unwrap()
        .map(|entry|  {
                let entry = entry.unwrap();

                let mut result = String::new();
                let game_id = entry.file_name().to_str().unwrap().strip_suffix(".ron").unwrap().to_string();
                File::open(entry.path()).unwrap().read_to_string(&mut result).unwrap();

                let result = result.lines().map(|line| ron::from_str(line).unwrap()).collect();
                (game_id, result)
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

    for (i, (game_id, events)) in parsed(&ron_cache).enumerate() {
        let mut inning = Inning::BeforeGame;
        let mut away_perfect = true;
        let mut home_perfect = true;
        for event in events {
            match event {
                ParsedEvent::InningStart { number, side, .. } => {
                    inning = Inning::DuringGame { number, batting_side: side }
                },
                ParsedEvent::Advance { .. } | ParsedEvent::BatterToBase { .. } | ParsedEvent::Steal {..} => {
                    match inning.batting_side().unwrap() {
                        Side::Away => away_perfect = false,
                        Side::Home => home_perfect = false,
                    }
                }
                _ => ()
            }
            if !away_perfect && !home_perfect {
                break
            }
        }
        if away_perfect || home_perfect {
            println!("{game_id} {away_perfect} {home_perfect}")
        }
        if i % 100 == 0 {
            println!("{i}")
        }
    }
}