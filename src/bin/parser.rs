
use std::{env::args, fs::File, io::{self, Read, Write}};

use mmolb_parsing::{process_game, raw_game::RawGame, Game, ParsedEventMessage};
use serde::{Deserialize, Serialize};

#[cfg(feature = "rand")]
use rand::seq::SliceRandom;

pub fn downloaded(json_cache: &str) -> impl Iterator<Item = (String, RawGame)> {
    let mut entries = std::fs::read_dir(json_cache).unwrap().collect::<Vec<_>>();

    #[cfg(feature = "rand")] {
        let mut rng = rand::rng();
        entries.shuffle(&mut rng);
    }

    entries.into_iter().map(|entry|  {
                let entry = entry.unwrap();

                let mut result = String::new();
                let game_id = entry.file_name().to_str().unwrap().strip_suffix(".json").unwrap().to_string();
                File::open(entry.path()).unwrap().read_to_string(&mut result).unwrap();
                (game_id, serde_json::from_str(&result).unwrap())
            }
        )
}

pub fn save_parsed_messages<'de, S: Serialize + Deserialize<'de>>(ron_cache: &str, game_id: &str, events: Vec<ParsedEventMessage<S>>) {
    let ron_path = format!(r"{ron_cache}/{game_id}.ron");
    let mut file = File::create(ron_path).unwrap();
    for event in &events {
        writeln!(file, "{}", ron::to_string(&event).unwrap()).unwrap()
    }
}

fn main() {
    let mut args = args().skip(1);

    let json_cache = args.next().expect("first argument \"json_cache\" should be present");
    let ron_cache = args.next().expect("second argument \"ron_cache\" should be present");

    let count = std::fs::read_dir(&json_cache).unwrap().count();
    println!("About to parse {count} games from {json_cache} into {ron_cache}. Press enter to continue");
    io::stdin().read_line(&mut String::new()).unwrap();

    let mut i = 0;
    for (game_id, raw_game) in downloaded(&json_cache) {
        i += 1;
        if i % 100 == 0 {
            println!("{i} {game_id}")
        }


        let game = {
            #[cfg(feature = "panic_on_parse_error")]
            {
                let game: Game = raw_game.clone().into();
                let unparsed_game = RawGame::from(game.clone());
                assert_eq!(unparsed_game.away_sp, raw_game.away_sp, "{game_id}");
                assert_eq!(unparsed_game.away_team_abbreviation, raw_game.away_team_abbreviation, "{game_id}");
                assert_eq!(unparsed_game.away_team_color, raw_game.away_team_color, "{game_id}");
                assert_eq!(unparsed_game.away_team_id, raw_game.away_team_id, "{game_id}");
                assert_eq!(unparsed_game.away_team_emoji, raw_game.away_team_emoji, "{game_id}");
                assert_eq!(unparsed_game.away_team_name, raw_game.away_team_name, "{game_id}");
                assert_eq!(unparsed_game.day, raw_game.day, "{game_id}");
                assert_eq!(unparsed_game.home_sp, raw_game.home_sp, "{game_id}");
                assert_eq!(unparsed_game.home_team_abbreviation, raw_game.home_team_abbreviation, "{game_id}");
                assert_eq!(unparsed_game.home_team_color, raw_game.home_team_color, "{game_id}");
                assert_eq!(unparsed_game.home_team_emoji, raw_game.home_team_emoji, "{game_id}");
                assert_eq!(unparsed_game.home_team_id, raw_game.home_team_id, "{game_id}");
                assert_eq!(unparsed_game.home_team_name, raw_game.home_team_name, "{game_id}");
                assert_eq!(unparsed_game.season, raw_game.season, "{game_id}");
                assert_eq!(unparsed_game.state, raw_game.state, "{game_id}");
                assert_eq!(unparsed_game.stats, raw_game.stats, "{game_id}");
                assert_eq!(unparsed_game.realm, raw_game.realm, "{game_id}");

                for i in 0..unparsed_game.event_log.len() {
                    assert_eq!(unparsed_game.event_log[i], raw_game.event_log[i], "{game_id}");
                }
                game
            }
            #[cfg(not(feature = "panic_on_parse_error"))]
            {
                raw_game.into()
            }
        };

        let messages = process_game(&game);
        save_parsed_messages(&ron_cache, &game_id, messages);
    }
}
