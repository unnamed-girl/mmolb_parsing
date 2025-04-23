use std::env;
use std::{fs::File, io::Write};

use parser::parsing::process_events;
use parser::game::Game;

// tested on:
// cargo run 68075a97d0ee3895dbc0dc97 680776b9d0ee3895dbc0de78 680792d7d0ee3895dbc0dff3 6807aef6128045e526322a90 6807cb18128045e526322d57 6807e739128045e5263230a7 6808035b11f35e62dba394e4 68081f7c11f35e62dba39863 68083b9a11f35e62dba39b6e

fn main() {
    for game in env::args().skip(1) {
        let resp = reqwest::blocking::get(format!("https://mmolb.com/api/game/{game}"))
        .unwrap()
        .json::<Game>()
        .unwrap();
        analyse_game(game, resp);
    }
}


fn analyse_game(id: String, game: Game) {
    let mut file = File::create(format!("{id}.ron")).unwrap();
    for event in process_events(&game.event_log) {
        writeln!(file, "{}", ron::to_string(&event).unwrap()).unwrap();
    }
    println!("Finished parsing {id}");
}