use std::error::Error;

use clap::Parser;
use mmolb_parsing::{enums::{Attribute, EquipmentEffectType, ItemName, Position, Slot}, player::{Player, PlayerEquipment}, team::Team, NotRecognized};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FreeCashewResponse<T> {
    pub items: Vec<T>,
    pub next_page: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct EntityResponse<T> {
    pub kind: String,
    pub entity_id: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub data: T
}

#[derive(Parser, Debug)]
struct Args {
    team_id: String,
}

trait Fetchable {
    const URL: &str;
}

impl Fetchable for Player {
    const URL: &str = "https://mmolb.com/api/player";
}

impl Fetchable for Team {
    const URL: &str = "https://mmolb.com/api/team";
}

fn mmolb_fetch<'a, T: Fetchable + DeserializeOwned>(client: &'a Client, id: &str) -> Result<T, Box<dyn Error>> {
    let url = format!("{}/{id}", T::URL);

    Ok(client.get(url).send()?
            .json::<T>()?)
}


struct UnderstoodItem {
    effects: Vec<(Attribute, f64)>,
    item: ItemName
}

impl TryFrom<PlayerEquipment> for UnderstoodItem {
    type Error = &'static str;

    fn try_from(value: PlayerEquipment) -> Result<Self, Self::Error> {
        if let Ok(item) = value.name {
            let effects = value.effects.unwrap_or_default()
                .into_iter()
                .flat_map(|e| e)
                .filter(|e| matches!(e.effect_type, Ok(EquipmentEffectType::FlatBonus)))
                .flat_map(|e| Ok::<(_, _), NotRecognized>((e.attribute?, e.value)))
                .collect::<Vec<_>>();

            return Ok( UnderstoodItem { effects, item })
        }

        Err("Didn't recognize name")
    }
} 

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldPlace {
    Pitcher,
    Catcher,
    Infield,
    Outfield,
    DH
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let client = Client::default();

    let team = mmolb_fetch::<Team>(&client, &args.team_id)?;

    let _inventory = team.inventory?.into_iter().flat_map(|a| UnderstoodItem::try_from(a)).collect::<Vec<_>>();

    let mut badness = 0;
    let mut goodness = 0;

    for player in team.players {
        let slot = player.slot?;

        let player = mmolb_fetch::<Player>(&client, &player.player_id)?;

        let fielder_type = match slot {
            Slot::DesignatedHitter => FieldPlace::DH,
            Slot::ReliefPitcher(_) | Slot::StartingPitcher(_) | Slot::Closer => FieldPlace::Pitcher,
            Slot::Catcher => FieldPlace::Catcher,
            Slot::CenterField | Slot::RightField | Slot::LeftField => FieldPlace::Outfield,
            _ => FieldPlace::Infield
        };

        let items: Vec<PlayerEquipment> = player.equipment?.into();
        let items = items.into_iter().flat_map(UnderstoodItem::try_from).collect::<Vec<_>>();

        println!("{} {} {}/{}", player.first_name, player.last_name, player.position.as_ref().map(Position::to_string).unwrap_or("???".to_string()), slot);

        for item in items {
            let mut criticisms = Vec::new();

            let mut batter = 0;
            let mut pitcher = 0;

            // Acrobatics - line drives.  
            // Agility - fly balls
            // Patience - popups
            // Reaction - ground balls

            for (attribute, value) in item.effects {
                let value = (value * 100.0).round() as u16;

                let is_fielder = matches!(attribute, Attribute::Acrobatics | Attribute::Arm | Attribute::Awareness | Attribute::Composure | Attribute::Dexterity | Attribute::Patience | Attribute::Reaction);
                let is_batter = matches!(attribute, Attribute::Aiming | Attribute::Contact | Attribute::Cunning | Attribute::Determination | Attribute::Discipline | Attribute::Insight | Attribute::Intimidation | Attribute::Lift | Attribute::Muscle | Attribute::Selflessness | Attribute::Vision | Attribute::Wisdom);
                let is_baserunning = matches!(attribute, Attribute::Greed | Attribute::Performance | Attribute::Speed | Attribute::Stealth);
                let is_pitching = matches!(attribute, Attribute::Accuracy | Attribute::Control | Attribute::Defiance | Attribute::Guts | Attribute::Persuasion | Attribute::Presence | Attribute::Rotation | Attribute::Stamina | Attribute::Stuff | Attribute::Velocity);

                // Fielding positions
                match (fielder_type, attribute) {
                    (FieldPlace::Catcher, Attribute::Agility | Attribute::Acrobatics) => {
                        badness += value;
                        criticisms.push(format!("Catcher cannot use +{value} {attribute}"))
                    },
                    (FieldPlace::Catcher, Attribute::Reaction | Attribute::Patience) => {
                        badness += value;
                        criticisms.push(format!("Catcher makes poor use of +{value} {attribute}"))
                    },
                    (FieldPlace::Pitcher, Attribute::Agility) => {
                        badness += value;
                        criticisms.push(format!("Pitcher cannot use +{value} {attribute}"))
                    },
                    (FieldPlace::Pitcher, Attribute::Reaction | Attribute::Acrobatics | Attribute::Patience) => {
                        badness += value;
                        criticisms.push(format!("Pitcher makes poor use of +{value} {attribute}"))
                    },
                    (FieldPlace::Infield, Attribute::Agility | Attribute::Acrobatics) => {
                        badness += value;
                        criticisms.push(format!("Infielder makes poor use of +{value} {attribute}"))
                    },
                    (FieldPlace::Outfield, Attribute::Patience) => {
                        badness += value;
                        criticisms.push(format!("Outfielder cannot use +{value} {attribute}"))
                    },
                    (FieldPlace::Outfield, Attribute::Reaction) => {
                        badness += value;
                        criticisms.push(format!("Outfielder makes poor use of +{value} {attribute}"))
                    },
                    _ => if is_fielder {
                        goodness += value;
                    }
                };
                
                if is_fielder && fielder_type == FieldPlace::DH {
                    badness += value;
                    criticisms.push(format!("Designated hitter cannot make use of +{value} {attribute}"))
                }
                if is_baserunning || is_batter {
                    batter += value;

                    if fielder_type != FieldPlace::Pitcher {
                        goodness += value;
                    } else {
                        badness += value
                    }
                }
                if is_pitching {
                    pitcher += value;

                    if fielder_type == FieldPlace::Pitcher {
                        goodness += value;
                    } else {
                        badness += value
                    }
                }
                
            }

            if fielder_type == FieldPlace::Pitcher && batter > pitcher {
                let diff = batter - pitcher;
                criticisms.push(format!("Pitcher holding item with net +{diff} batting attributes"));
            } else if fielder_type != FieldPlace::Pitcher && batter < pitcher {
                let diff = pitcher - batter;
                criticisms.push(format!("Non pitcher holding item with net +{diff} pitcher attributes"));
            }

            if criticisms.len() > 0 {
                println!("- {}", item.item);
                for criticism in criticisms {
                    println!("- - {criticism}");
                }
            }
        }


    }

    println!("Badness: {badness}");
    println!("Goodness: {goodness}");

    Ok(())
}