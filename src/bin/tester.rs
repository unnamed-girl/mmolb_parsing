use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use mmolb_parsing::{
    player::Player,
    player_feed::{parse_player_feed_event, PlayerFeed},
    process_event,
    team::Team,
    team_feed::{parse_team_feed_event, TeamFeed},
    Game,
};
use nom::{
    bytes::complete::{tag, take_until},
    combinator::rest,
    multi::many0,
    sequence::terminated,
    Parser as NomParser,
};
use nom_language::error::VerboseError;
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tracing::{info_span, Level};
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};

type IResult<'a, O> = nom::IResult<&'a str, O, VerboseError<&'a str>>;

#[derive(Parser, Debug)]
struct App {
    #[clap(flatten)]
    global_opts: GlobalOpts,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// txt file layed out like test-cases.txt
    #[arg(default_value = "test-cases.txt")]
    test_cases: PathBuf,
    /// a folder to store test data
    #[arg(default_value = "test_data")]
    test_data_folder: PathBuf,

    /// Max logging level
    #[clap(long)]
    with_max_log_level: Option<Level>,
}

impl GlobalOpts {
    pub fn read_test_cases(&self) -> TestCases {
        let mut f = File::open(&self.test_cases).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        let (_, test_cases) = parse_test_cases(&buf).unwrap();
        return test_cases;
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Sync answers with freecashe.ws
    Sync(SyncArgs),
    RoundTrip(RoundTripArgs),
}

#[derive(Debug, Args)]
struct SyncArgs {
    /// Fetch from cheapcashe.ws instead
    #[clap(long, action)]
    beiju: bool,

    /// Only sync certain kinds
    #[clap(long)]
    only_sync_kind: Vec<Kind>,
}

impl SyncArgs {
    fn sync_kind(&self, kind: Kind) -> bool {
        self.only_sync_kind.is_empty() || self.only_sync_kind.contains(&kind)
    }
}

#[derive(Debug, Args)]
struct RoundTripArgs {
    /// Only sync certain kinds
    #[clap(long)]
    only_sync_kind: Vec<Kind>,
}

impl RoundTripArgs {
    fn sync_kind(&self, kind: Kind) -> bool {
        self.only_sync_kind.is_empty() || self.only_sync_kind.contains(&kind)
    }
}

#[derive(Debug)]
struct TestCases {
    teams: Vec<TestCase>,
    games: Vec<TestCase>,
    players: Vec<TestCase>,
    team_feeds: Vec<TestCase>,
    player_feeds: Vec<TestCase>,
}

#[derive(Debug)]
struct TestCase {
    id: String,
}

#[derive(ValueEnum, Clone, Debug, Copy, PartialEq, Eq)]
enum Kind {
    Game,
    Team,
    Player,
    PlayerFeed,
    TeamFeed,
}

impl Kind {
    fn as_chron_kind(&self) -> &'static str {
        match self {
            Kind::Game => "game",
            Kind::Team => "team",
            Kind::Player => "player",
            Kind::PlayerFeed => "player_feed",
            Kind::TeamFeed => "team_feed",
        }
    }
}

fn main() {
    let args = App::parse();

    let err_layer = tracing_subscriber::fmt::Layer::new()
        .with_ansi(false)
        .with_writer(std::io::stderr.with_max_level(Level::ERROR));

    let stdout_layer = tracing_subscriber::fmt::Layer::new()
        .with_writer(std::io::stdout.with_max_level(args.global_opts.with_max_log_level.unwrap_or(Level::INFO)));

    let collector = tracing_subscriber::registry()
        .with(err_layer)
        .with(stdout_layer);

    let guard = tracing::subscriber::set_global_default(collector).unwrap();

    match args.command {
        Command::Sync(sync_args) => sync(args.global_opts, sync_args),
        Command::RoundTrip(round_trip_args) => round_trip(args.global_opts, round_trip_args),
    }
}

fn parse_test_cases(input: &str) -> IResult<'_, TestCases> {
    let (input, _) = many0(parse_comment).parse(input)?;
    let (input, _) = tag("Teams:\n").parse(input)?;
    let (input, teams) = many0(parse_test_case).parse(input)?;

    let (input, _) = many0(parse_comment).parse(input)?;
    let (input, _) = tag("Games:\n").parse(input)?;
    let (input, games) = many0(parse_test_case).parse(input)?;

    let (input, _) = many0(parse_comment).parse(input)?;
    let (input, _) = tag("Team Feeds:\n").parse(input)?;
    let (input, team_feeds) = many0(parse_test_case).parse(input)?;

    let (input, _) = many0(parse_comment).parse(input)?;
    let (input, _) = tag("Players:\n").parse(input)?;
    let (input, players) = many0(parse_test_case).parse(input)?;

    let (input, _) = many0(parse_comment).parse(input)?;
    let (input, _) = tag("Player Feeds:\n").parse(input)?;
    let (input, player_feeds) = many0(parse_test_case).parse(input)?;

    Ok((
        input,
        TestCases {
            teams,
            games,
            players,
            team_feeds,
            player_feeds,
        },
    ))
}

fn parse_comment(input: &'_ str) -> IResult<'_, ()> {
    let (input, _) = tag("//").parse(input)?;
    let (input, _) = terminated(take_until("\n"), tag("\n"))
        .or(rest)
        .parse(input)?;
    Ok((input, ()))
}

fn parse_test_case(input: &'_ str) -> IResult<'_, TestCase> {
    let (input, _) = many0(parse_comment).parse(input)?;
    let (input, _) = tag("- ").parse(input)?;
    let (input, id) = terminated(take_until("\n"), tag("\n"))
        .or(rest)
        .parse(input)?;

    assert!(!id.contains("//"), "No comments on id lines: {input}");

    Ok((input, TestCase { id: id.to_string() }))
}

#[derive(Serialize, Deserialize)]
pub struct FreeCashewResponse<T> {
    pub items: Vec<T>,
    pub next_page: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EntityResponse<T> {
    pub kind: String,
    pub entity_id: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub data: T,
}

fn sync(global: GlobalOpts, sync_args: SyncArgs) {
    let test_cases = global.read_test_cases();
    let client = Client::builder().timeout(None).build().unwrap();
    let endpoint = if sync_args.beiju {
        "https://cheapcashews.beiju.me/chron/v0/entities"
    } else {
        "https://freecashe.ws/api/chron/v0/entities"
    };
    let guard = info_span!("Fetching").entered();
    if sync_args.sync_kind(Kind::Game) {
        tracing::info!("Fetching games");
        for game in test_cases.games {
            let _guard = info_span!("Game", id = game.id).entered();
            fetch_save(
                &global.test_data_folder,
                &client,
                endpoint,
                "game",
                &game.id,
            );
        }
    }

    if sync_args.sync_kind(Kind::Team) {
        tracing::info!("Fetching teams");
        for team in test_cases.teams {
            let _guard = info_span!("Team", id = team.id).entered();
            fetch_save(
                &global.test_data_folder,
                &client,
                endpoint,
                "team",
                &team.id,
            );
        }
    }

    if sync_args.sync_kind(Kind::Game) {
        tracing::info!("Fetching players");
        for player in test_cases.players {
            let _guard = info_span!("Player", id = player.id).entered();
            fetch_save(
                &global.test_data_folder,
                &client,
                endpoint,
                "player",
                &player.id,
            );
        }
    }
    if sync_args.sync_kind(Kind::PlayerFeed) {
        tracing::info!("Fetching player feeds");
        for player_feed in test_cases.player_feeds {
            let _guard = info_span!("Player Feed", id = player_feed.id).entered();
            fetch_save(
                &global.test_data_folder,
                &client,
                endpoint,
                "player_feed",
                &player_feed.id,
            );
        }
    }

    if sync_args.sync_kind(Kind::TeamFeed) {
        tracing::info!("Fetching team feeds");
        for team_feed in test_cases.team_feeds {
            let _guard = info_span!("Team feed", id = team_feed.id).entered();
            fetch_save(
                &global.test_data_folder,
                &client,
                endpoint,
                "team_feed",
                &team_feed.id,
            );
        }
    }
    drop(guard);
}

fn fetch_save(test_data_folder: &Path, client: &Client, endpoint: &str, kind: &str, id: &str) {
    let url = format!("{endpoint}?kind={kind}&id={id}");
    let entities = client
        .get(&url)
        .send()
        .unwrap()
        .json::<FreeCashewResponse<EntityResponse<Box<serde_json::value::RawValue>>>>()
        .unwrap()
        .items;

    if entities.len() != 1 {
        tracing::error!(
            "Error while fetching {id}: expected 1, found {}",
            entities.len()
        );
        return;
    }

    let path = test_data_folder
        .join("raw")
        .join(kind)
        .join(id)
        .with_extension("json");
    let mut f = File::create(path).unwrap();
    write!(f, "{}", entities[0].data).unwrap();
}

fn round_trip(global_opts: GlobalOpts, round_trip_args: RoundTripArgs) {
    let test_cases = global_opts.read_test_cases();
    if round_trip_args.sync_kind(Kind::Game) {
        tracing::info!("Testing games");
        for game in test_cases.games {
            let _guard = info_span!("Game", id = game.id).entered();
            _round_trip::<Game>(
                &global_opts.test_data_folder,
                Kind::Game,
                &game.id,
                game_inner,
            );
        }
    }

    if round_trip_args.sync_kind(Kind::Team) {
        tracing::info!("Testing teams");
        for team in test_cases.teams {
            let _guard = info_span!("Team", id = team.id).entered();
            _round_trip::<Team>(
                &global_opts.test_data_folder,
                Kind::Team,
                &team.id,
                team_inner,
            );
        }
    }

    if round_trip_args.sync_kind(Kind::Game) {
        tracing::info!("Fetching players");
        for player in test_cases.players {
            let _guard = info_span!("Player", id = player.id).entered();
            _round_trip::<Player>(
                &global_opts.test_data_folder,
                Kind::Player,
                &player.id,
                player_inner,
            );
        }
    }
    if round_trip_args.sync_kind(Kind::PlayerFeed) {
        tracing::info!("Fetching player feeds");
        for player_feed in test_cases.player_feeds {
            let _guard = info_span!("Player Feed", id = player_feed.id).entered();
            _round_trip::<PlayerFeed>(
                &global_opts.test_data_folder,
                Kind::PlayerFeed,
                &player_feed.id,
                player_feed_inner,
            );
        }
    }

    if round_trip_args.sync_kind(Kind::TeamFeed) {
        tracing::info!("Fetching team feeds");
        for team_feed in test_cases.team_feeds {
            let _guard = info_span!("Team feed", id = team_feed.id).entered();
            _round_trip::<TeamFeed>(
                &global_opts.test_data_folder,
                Kind::TeamFeed,
                &team_feed.id,
                team_feed_inner,
            );
        }
    }
}

fn _round_trip<T: DeserializeOwned + Serialize>(
    test_data_folder: &Path,
    kind: Kind,
    id: &str,
    inner: impl Fn(T, &str),
) {
    let path = test_data_folder
        .join("raw")
        .join(kind.as_chron_kind())
        .join(id)
        .with_extension("json");
    let f = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("{e}");
            return;  
        },
    };
    let value =
        serde_json::Value::deserialize(&mut serde_json::Deserializer::from_reader(f)).unwrap();
    let t: T = match serde_json::from_value(value.clone()) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("{e}");
            return;
        },
    };
    let round_tripped = serde_json::to_value(&t).unwrap();
    let diff = serde_json_diff::values(value, round_tripped);
    if let Some(diff) = diff {
        tracing::warn!(
            "round trip failed. Diff: {}",
            serde_json::to_string(&diff).unwrap()
        );
    }

    inner(t, id)
}

fn game_inner(game: Game, id: &str) {
    let _game_guard = tracing::span!(
        Level::INFO,
        "Game",
        season = game.season,
        day = format!("{:?}", game.day),
        scale = format!("{:?}", game.league_scale)
    )
    .entered();

    for event in &game.event_log {
        let _event_span_guard = tracing::span!(
            Level::INFO,
            "Event",
            index = event.index,
            r#type = format!("{:?}", event.event),
            message = event.message
        )
        .entered();

        let parsed_event_message = process_event(event, &game, id);
        if tracing::enabled!(Level::WARN) {
            let unparsed = parsed_event_message.unparse(&game, event.index);
            if event.message != unparsed {
                tracing::warn!(
                    "Event round trip failure expected:\n'{}'\nGot:\n'{}'",
                    event.message,
                    unparsed
                );
            }
        }

        drop(_event_span_guard);
    }
}

fn player_feed_inner(feed: PlayerFeed, _id: &str) {
    let _player_feed_span_guard = tracing::span!(Level::INFO, "Player Feed").entered();

    for event in feed.feed {
        let _event_span_guard = tracing::span!(
            Level::INFO,
            "Feed Event",
            season = event.season,
            day = format!("{:?}", event.day),
            timestamp = event.timestamp.to_string(),
            r#type = format!("{:?}", event.event_type),
            message = event.text
        )
        .entered();

        let parsed_text = parse_player_feed_event(&event);
        if tracing::enabled!(Level::WARN) {
            let unparsed = parsed_text.unparse(&event);
            if event.text != unparsed {
                tracing::warn!(
                    "Feed event round trip failure expected:\n'{}'\nGot:\n'{}'",
                    event.text,
                    unparsed
                );
            }
        }
    }
}

fn team_feed_inner(feed: TeamFeed, _id: &str) {
    let _team_feed_span_guard = tracing::span!(Level::INFO, "Team Feed").entered();

    for event in feed.feed {
        let _event_span_guard = tracing::span!(
            Level::INFO,
            "Feed Event",
            season = event.season,
            day = format!("{:?}", event.day),
            timestamp = event.timestamp.to_string(),
            r#type = format!("{:?}", event.event_type),
            message = event.text
        )
        .entered();

        let parsed_text = parse_team_feed_event(&event);
        if tracing::enabled!(Level::WARN) {
            let unparsed = parsed_text.unparse(&event);
            if event.text != unparsed {
                tracing::warn!(
                    "Feed event round trip failure expected:\n'{}'\nGot:\n'{}'",
                    event.text,
                    unparsed
                );
            }
        }
    }
}

fn player_inner(player: Player, _id: &str) {
    let _player_span_guard = tracing::span!(
        Level::INFO,
        "Player",
        name = format!("{} {}", player.first_name, player.last_name)
    )
    .entered();

    for event in player.feed.unwrap_or_default() {
        let _event_span_guard = tracing::span!(
            Level::INFO,
            "Feed Event",
            season = event.season,
            day = format!("{:?}", event.day),
            timestamp = event.timestamp.to_string(),
            r#type = format!("{:?}", event.event_type),
            message = event.text
        )
        .entered();

        let parsed_text = parse_player_feed_event(&event);
        if tracing::enabled!(Level::WARN) {
            let unparsed = parsed_text.unparse(&event);
            if event.text != unparsed {
                tracing::warn!(
                    "Feed event round trip failure expected:\n'{}'\nGot:\n'{}'",
                    event.text,
                    unparsed
                );
            }
        }
    }
}

fn team_inner(team: Team, _id: &str) {
    let _team_span_guard = tracing::span!(Level::INFO, "Team", name = team.name).entered();

    for event in team.feed.unwrap_or_default() {
        let _event_span_guard = tracing::span!(
            Level::INFO,
            "Feed Event",
            season = event.season,
            day = format!("{:?}", event.day),
            timestamp = event.timestamp.to_string(),
            r#type = format!("{:?}", event.event_type),
            message = format!("{:?}", event.text)
        )
        .entered();

        let parsed_text = parse_team_feed_event(&event);
        if tracing::enabled!(Level::WARN) {
            let unparsed = parsed_text.unparse(&event);
            if event.text != unparsed {
                tracing::warn!(
                    "Feed event round trip failure expected:\n'{}'\nGot:\n'{}'",
                    event.text,
                    unparsed
                );
            }
        }
    }
}
