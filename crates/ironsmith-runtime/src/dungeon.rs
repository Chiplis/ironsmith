use crate::card::PowerToughness;
use crate::cards::{CardDefinition, CardDefinitionBuilder};
use crate::color::ColorSet;
use crate::effect::{Effect, Until};
use crate::effects::DiscardEffect;
use crate::filter::ObjectFilter;
use crate::ids::CardId;
use crate::object::CounterType;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveDungeonProgress {
    pub dungeon_name: String,
    pub room_name: String,
}

impl ActiveDungeonProgress {
    pub fn new(dungeon_name: impl Into<String>, room_name: impl Into<String>) -> Self {
        Self {
            dungeon_name: dungeon_name.into(),
            room_name: room_name.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonDefinition {
    pub name: &'static str,
    pub first_room: &'static str,
    pub rooms: &'static [DungeonRoomDefinition],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DungeonRoomDefinition {
    pub name: &'static str,
    pub next_rooms: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq)]
pub struct DungeonRoomAbilityDefinition {
    pub effects: Vec<Effect>,
    pub choices: Vec<ChooseSpec>,
}

const LOST_MINE_ROOMS: &[DungeonRoomDefinition] = &[
    DungeonRoomDefinition {
        name: "Cave Entrance",
        next_rooms: &["Goblin Lair", "Mine Tunnels"],
    },
    DungeonRoomDefinition {
        name: "Goblin Lair",
        next_rooms: &["Storeroom", "Dark Pool"],
    },
    DungeonRoomDefinition {
        name: "Mine Tunnels",
        next_rooms: &["Dark Pool", "Fungi Cavern"],
    },
    DungeonRoomDefinition {
        name: "Storeroom",
        next_rooms: &["Temple of Dumathoin"],
    },
    DungeonRoomDefinition {
        name: "Dark Pool",
        next_rooms: &["Temple of Dumathoin"],
    },
    DungeonRoomDefinition {
        name: "Fungi Cavern",
        next_rooms: &["Temple of Dumathoin"],
    },
    DungeonRoomDefinition {
        name: "Temple of Dumathoin",
        next_rooms: &[],
    },
];

const DUNGEON_OF_THE_MAD_MAGE_ROOMS: &[DungeonRoomDefinition] = &[
    DungeonRoomDefinition {
        name: "Yawning Portal",
        next_rooms: &["Dungeon Level"],
    },
    DungeonRoomDefinition {
        name: "Dungeon Level",
        next_rooms: &["Goblin Bazaar", "Twisted Caverns"],
    },
    DungeonRoomDefinition {
        name: "Goblin Bazaar",
        next_rooms: &["Lost Level"],
    },
    DungeonRoomDefinition {
        name: "Twisted Caverns",
        next_rooms: &["Lost Level"],
    },
    DungeonRoomDefinition {
        name: "Lost Level",
        next_rooms: &["Runestone Caverns", "Muiral's Graveyard"],
    },
    DungeonRoomDefinition {
        name: "Runestone Caverns",
        next_rooms: &["Deep Mines"],
    },
    DungeonRoomDefinition {
        name: "Muiral's Graveyard",
        next_rooms: &["Deep Mines"],
    },
    DungeonRoomDefinition {
        name: "Deep Mines",
        next_rooms: &["Mad Wizard's Lair"],
    },
    DungeonRoomDefinition {
        name: "Mad Wizard's Lair",
        next_rooms: &[],
    },
];

const TOMB_OF_ANNIHILATION_ROOMS: &[DungeonRoomDefinition] = &[
    DungeonRoomDefinition {
        name: "Trapped Entry",
        next_rooms: &["Veils of Fear", "Oubliette"],
    },
    DungeonRoomDefinition {
        name: "Veils of Fear",
        next_rooms: &["Sandfall Cell"],
    },
    DungeonRoomDefinition {
        name: "Oubliette",
        next_rooms: &["Cradle of the Death God"],
    },
    DungeonRoomDefinition {
        name: "Sandfall Cell",
        next_rooms: &["Cradle of the Death God"],
    },
    DungeonRoomDefinition {
        name: "Cradle of the Death God",
        next_rooms: &[],
    },
];

const UNDERCITY_ROOMS: &[DungeonRoomDefinition] = &[
    DungeonRoomDefinition {
        name: "Secret Entrance",
        next_rooms: &["Forge", "Lost Well"],
    },
    DungeonRoomDefinition {
        name: "Forge",
        next_rooms: &["Trap!"],
    },
    DungeonRoomDefinition {
        name: "Lost Well",
        next_rooms: &["Arena"],
    },
    DungeonRoomDefinition {
        name: "Trap!",
        next_rooms: &["Archives"],
    },
    DungeonRoomDefinition {
        name: "Arena",
        next_rooms: &["Archives"],
    },
    DungeonRoomDefinition {
        name: "Archives",
        next_rooms: &["Catacombs"],
    },
    DungeonRoomDefinition {
        name: "Catacombs",
        next_rooms: &["Throne of the Dead Three"],
    },
    DungeonRoomDefinition {
        name: "Throne of the Dead Three",
        next_rooms: &[],
    },
];

pub const LOST_MINE_OF_PHANDELVER: DungeonDefinition = DungeonDefinition {
    name: "Lost Mine of Phandelver",
    first_room: "Cave Entrance",
    rooms: LOST_MINE_ROOMS,
};

pub const DUNGEON_OF_THE_MAD_MAGE: DungeonDefinition = DungeonDefinition {
    name: "Dungeon of the Mad Mage",
    first_room: "Yawning Portal",
    rooms: DUNGEON_OF_THE_MAD_MAGE_ROOMS,
};

pub const TOMB_OF_ANNIHILATION: DungeonDefinition = DungeonDefinition {
    name: "Tomb of Annihilation",
    first_room: "Trapped Entry",
    rooms: TOMB_OF_ANNIHILATION_ROOMS,
};

pub const UNDERCITY: DungeonDefinition = DungeonDefinition {
    name: "Undercity",
    first_room: "Secret Entrance",
    rooms: UNDERCITY_ROOMS,
};

const NORMAL_VENTURE_DUNGEONS: &[&str] = &[
    LOST_MINE_OF_PHANDELVER.name,
    DUNGEON_OF_THE_MAD_MAGE.name,
    TOMB_OF_ANNIHILATION.name,
];

pub fn lookup_dungeon(name: &str) -> Option<&'static DungeonDefinition> {
    [
        &LOST_MINE_OF_PHANDELVER,
        &DUNGEON_OF_THE_MAD_MAGE,
        &TOMB_OF_ANNIHILATION,
        &UNDERCITY,
    ]
    .into_iter()
    .find(|definition| definition.name.eq_ignore_ascii_case(name))
}

pub fn normal_venture_dungeon_names() -> Vec<String> {
    NORMAL_VENTURE_DUNGEONS
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

pub fn undercity_name() -> &'static str {
    UNDERCITY.name
}

pub fn first_room_name(dungeon_name: &str) -> Option<&'static str> {
    lookup_dungeon(dungeon_name).map(|definition| definition.first_room)
}

pub fn next_room_names(dungeon_name: &str, room_name: &str) -> Option<Vec<String>> {
    let dungeon = lookup_dungeon(dungeon_name)?;
    let room = dungeon
        .rooms
        .iter()
        .find(|room| room.name.eq_ignore_ascii_case(room_name))?;
    Some(
        room.next_rooms
            .iter()
            .map(|room_name| (*room_name).to_string())
            .collect(),
    )
}

fn creature_token(
    name: &str,
    colors: ColorSet,
    subtypes: Vec<Subtype>,
    power: i32,
    toughness: i32,
) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes)
        .color_indicator(colors)
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn atropal_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Atropal")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::God, Subtype::Horror])
        .color_indicator(ColorSet::BLACK)
        .power_toughness(PowerToughness::fixed(4, 4))
        .with_ability(crate::ability::deathtouch())
        .build()
}

pub fn room_ability_definition(
    dungeon_name: &str,
    room_name: &str,
) -> Option<DungeonRoomAbilityDefinition> {
    let dungeon = lookup_dungeon(dungeon_name)?;
    let room = dungeon
        .rooms
        .iter()
        .find(|room| room.name.eq_ignore_ascii_case(room_name))?;
    let target_creature = || ChooseSpec::target_creature();
    let target_player = || ChooseSpec::target_player();

    let (effects, choices) = match (dungeon.name, room.name) {
        ("Lost Mine of Phandelver", "Cave Entrance") => (vec![Effect::scry(1)], vec![]),
        ("Lost Mine of Phandelver", "Goblin Lair") => (
            vec![Effect::create_tokens(
                creature_token("Goblin", ColorSet::RED, vec![Subtype::Goblin], 1, 1),
                1,
            )],
            vec![],
        ),
        ("Lost Mine of Phandelver", "Mine Tunnels") => (
            vec![Effect::create_tokens(
                crate::cards::tokens::treasure_token_definition(),
                1,
            )],
            vec![],
        ),
        ("Lost Mine of Phandelver", "Storeroom") => {
            let target = target_creature();
            (
                vec![Effect::plus_one_counters(1, target.clone())],
                vec![target],
            )
        }
        ("Lost Mine of Phandelver", "Dark Pool") => (
            vec![
                Effect::for_each_opponent(vec![Effect::lose_life_player(
                    1,
                    PlayerFilter::IteratedPlayer,
                )]),
                Effect::gain_life(1),
            ],
            vec![],
        ),
        ("Lost Mine of Phandelver", "Fungi Cavern") => {
            let target = target_creature();
            (
                vec![Effect::pump(-4, 0, target.clone(), Until::YourNextTurn)],
                vec![target],
            )
        }
        ("Lost Mine of Phandelver", "Temple of Dumathoin") => (vec![Effect::draw(1)], vec![]),
        ("Dungeon of the Mad Mage", "Yawning Portal") => (vec![Effect::gain_life(1)], vec![]),
        ("Dungeon of the Mad Mage", "Dungeon Level") => (vec![Effect::scry(1)], vec![]),
        ("Dungeon of the Mad Mage", "Goblin Bazaar") => (
            vec![Effect::create_tokens(
                crate::cards::tokens::treasure_token_definition(),
                1,
            )],
            vec![],
        ),
        ("Dungeon of the Mad Mage", "Twisted Caverns") => (vec![], vec![target_creature()]),
        ("Dungeon of the Mad Mage", "Lost Level") => (vec![Effect::scry(2)], vec![]),
        ("Dungeon of the Mad Mage", "Runestone Caverns") => (
            vec![Effect::exile_top_of_library_player(2, PlayerFilter::You)],
            vec![],
        ),
        ("Dungeon of the Mad Mage", "Muiral's Graveyard") => (
            vec![Effect::create_tokens(
                creature_token("Skeleton", ColorSet::BLACK, vec![Subtype::Skeleton], 1, 1),
                2,
            )],
            vec![],
        ),
        ("Dungeon of the Mad Mage", "Deep Mines") => (vec![Effect::scry(3)], vec![]),
        ("Dungeon of the Mad Mage", "Mad Wizard's Lair") => (vec![Effect::draw(3)], vec![]),
        ("Tomb of Annihilation", "Trapped Entry") => (
            vec![Effect::for_players(
                PlayerFilter::Any,
                vec![Effect::lose_life_player(1, PlayerFilter::IteratedPlayer)],
            )],
            vec![],
        ),
        ("Tomb of Annihilation", "Veils of Fear") => (vec![], vec![]),
        ("Tomb of Annihilation", "Oubliette") => (
            vec![
                Effect::new(DiscardEffect::new(1, PlayerFilter::You, false)),
                Effect::sacrifice(ObjectFilter::artifact(), 1),
                Effect::sacrifice(ObjectFilter::creature(), 1),
                Effect::sacrifice(ObjectFilter::land(), 1),
            ],
            vec![],
        ),
        ("Tomb of Annihilation", "Sandfall Cell") => (vec![], vec![]),
        ("Tomb of Annihilation", "Cradle of the Death God") => {
            (vec![Effect::create_tokens(atropal_token_definition(), 1)], vec![])
        }
        ("Undercity", "Secret Entrance") => (
            vec![Effect::search_library(
                ObjectFilter::default()
                    .with_type(CardType::Land)
                    .with_supertype(Supertype::Basic),
                Zone::Hand,
                PlayerFilter::You,
                true,
            )],
            vec![],
        ),
        ("Undercity", "Forge") => {
            let target = target_creature();
            (
                vec![Effect::put_counters(
                    CounterType::PlusOnePlusOne,
                    2,
                    target.clone(),
                )],
                vec![target],
            )
        }
        ("Undercity", "Lost Well") => (vec![Effect::scry(2)], vec![]),
        ("Undercity", "Trap!") => {
            let target = target_player();
            (
                vec![Effect::lose_life_player(
                    5,
                    PlayerFilter::Target(Box::new(PlayerFilter::Any)),
                )],
                vec![target],
            )
        }
        ("Undercity", "Arena") => {
            let target = target_creature();
            (vec![Effect::goad(target.clone())], vec![target])
        }
        ("Undercity", "Archives") => (vec![Effect::draw(1)], vec![]),
        ("Undercity", "Catacombs") => (
            vec![Effect::create_tokens(
                creature_token("Skeleton", ColorSet::BLACK, vec![Subtype::Skeleton], 4, 1),
                1,
            )],
            vec![],
        ),
        ("Undercity", "Throne of the Dead Three") => (vec![], vec![]),
        _ => return None,
    };

    Some(DungeonRoomAbilityDefinition { effects, choices })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_defined_dungeon_room_has_room_ability_data() {
        for dungeon in [
            &LOST_MINE_OF_PHANDELVER,
            &DUNGEON_OF_THE_MAD_MAGE,
            &TOMB_OF_ANNIHILATION,
            &UNDERCITY,
        ] {
            for room in dungeon.rooms {
                assert!(
                    room_ability_definition(dungeon.name, room.name).is_some(),
                    "missing room ability data for {} of {}",
                    room.name,
                    dungeon.name
                );
            }
        }
    }
}
