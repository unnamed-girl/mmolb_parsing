use std::{env::args, fs::File, io::Read};

use mmolb_parsing::ParsedEventMessage;

pub struct LoadedEvents {
    pub game_id: String,
    pub events: Vec<ParsedEventMessage<String>>
}

pub fn parsed(ron_cache: &str) -> impl Iterator<Item = LoadedEvents> {
    std::fs::read_dir(ron_cache).unwrap()
        .map(|entry|  {
                let entry = entry.unwrap();

                let mut result = String::new();
                let game_id = entry.file_name().to_str().unwrap().strip_suffix(".ron").expect("Should be passed .ron files").to_string();
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
    
    let ron_cache = args.next().expect("single argument \"ron_cache\" should be present");

    let mut result = Vec::new();
    for (i, mut events) in parsed(&ron_cache).enumerate() {
        if events.events.len() > 99 {
            result.push(events.events.remove(99))
        }
        if i % 100 == 0 {
            println!("{i}")
        }
    }
    println!("{result:?}");
    println!("{}", result.len());
}