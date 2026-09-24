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

// ============================================================================
// Room abilities (CR 309.4c)
// ============================================================================

/// A room ability: "When you move your venture marker into this room,
/// [effect]." The trigger condition is implicit; the venture action queues
/// the ability directly.
pub(crate) struct RoomAbility {
    pub effects: Vec<crate::effect::Effect>,
    pub choices: Vec<crate::target::ChooseSpec>,
}

fn room_token(
    name: &str,
    subtypes: Vec<crate::types::Subtype>,
    colors: crate::color::ColorSet,
    power: i32,
    toughness: i32,
) -> crate::cards::CardDefinitionBuilder {
    crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .token()
        .card_types(vec![crate::types::CardType::Creature])
        .subtypes(subtypes)
        .color_indicator(colors)
        .power_toughness(crate::card::PowerToughness::fixed(power, toughness))
}

fn target_creature() -> crate::target::ChooseSpec {
    crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::creature(),
    ))
}

fn untargeted(effects: Vec<crate::effect::Effect>) -> RoomAbility {
    RoomAbility {
        effects,
        choices: Vec::new(),
    }
}

fn targeted(
    target: crate::target::ChooseSpec,
    effects: Vec<crate::effect::Effect>,
) -> RoomAbility {
    RoomAbility {
        effects,
        choices: vec![target],
    }
}

/// "Each player loses N life unless they [alternative]."
fn each_player_loses_unless(
    amount: i32,
    alternative: crate::effect::Effect,
) -> crate::effect::Effect {
    use crate::effect::Effect;
    use crate::target::PlayerFilter;
    Effect::for_players(
        PlayerFilter::Any,
        vec![Effect::unless_action(
            vec![Effect::lose_life_player(amount, PlayerFilter::IteratedPlayer)],
            vec![alternative],
            PlayerFilter::IteratedPlayer,
        )],
    )
}

/// The room ability printed in `room_name` of `dungeon_name`.
pub(crate) fn room_ability(dungeon_name: &str, room_name: &str) -> Option<RoomAbility> {
    use crate::color::ColorSet;
    use crate::effect::{Effect, Restriction, Until};
    use crate::object::CounterType;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::types::{CardType, Subtype, Supertype};

    let dungeon = lookup_dungeon(dungeon_name)?;
    let room = dungeon
        .rooms
        .iter()
        .find(|room| room.name.eq_ignore_ascii_case(room_name))?
        .name;
    let treasure = || Effect::create_tokens(crate::cards::tokens::treasure_token_definition(), 1);
    let ability = match (dungeon.name, room) {
        // Lost Mine of Phandelver
        (_, "Cave Entrance") => untargeted(vec![Effect::scry(1)]),
        (_, "Goblin Lair") => untargeted(vec![Effect::create_tokens(
            room_token("Goblin", vec![Subtype::Goblin], ColorSet::RED, 1, 1).build(),
            1,
        )]),
        (_, "Mine Tunnels") => untargeted(vec![treasure()]),
        (_, "Storeroom") => {
            let target = target_creature();
            targeted(
                target.clone(),
                vec![Effect::put_counters(CounterType::PlusOnePlusOne, 1, target)],
            )
        }
        (_, "Dark Pool") => untargeted(vec![
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![Effect::lose_life_player(1, PlayerFilter::IteratedPlayer)],
            ),
            Effect::gain_life(1),
        ]),
        (_, "Fungi Cavern") => {
            let target = target_creature();
            targeted(
                target.clone(),
                vec![Effect::pump(-4, 0, target, Until::YourNextTurn)],
            )
        }
        (_, "Temple of Dumathoin") => untargeted(vec![Effect::draw(1)]),

        // Dungeon of the Mad Mage
        (_, "Yawning Portal") => untargeted(vec![Effect::gain_life(1)]),
        (_, "Dungeon Level") => untargeted(vec![Effect::scry(1)]),
        (_, "Goblin Bazaar") => untargeted(vec![treasure()]),
        (_, "Twisted Caverns") => {
            let target = target_creature();
            let tag = "__room_target";
            targeted(
                target.clone(),
                vec![
                    Effect::new(crate::effects::TargetOnlyEffect::new(target)).tag(tag),
                    Effect::cant_until(
                        Restriction::attack(ObjectFilter::exact_tagged(tag)),
                        Until::YourNextTurn,
                    ),
                ],
            )
        }
        (_, "Lost Level") => untargeted(vec![Effect::scry(2)]),
        (_, "Runestone Caverns") => {
            let tag = crate::tag::TagKey::from("__runestone_caverns_exiled");
            let mut exile = crate::effects::ExileTopOfLibraryEffect::new(
                crate::effect::Value::Fixed(2),
                PlayerFilter::You,
            );
            exile.moved_tags = vec![tag.clone()];
            untargeted(vec![
                Effect::new(exile),
                Effect::new(
                    crate::effects::GrantPlayTaggedEffect::new(
                        tag,
                        PlayerFilter::You,
                        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                        true,
                        false,
                    )
                    .cast_pool_is_plural(true),
                ),
            ])
        }
        (_, "Muiral's Graveyard") => untargeted(vec![Effect::create_tokens(
            room_token("Skeleton", vec![Subtype::Skeleton], ColorSet::BLACK, 1, 1).build(),
            2,
        )]),
        (_, "Deep Mines") => untargeted(vec![Effect::scry(3)]),
        (_, "Mad Wizard's Lair") => untargeted(vec![Effect::new(
            crate::effects::MadWizardsLairEffect,
        )]),

        // Tomb of Annihilation
        (_, "Trapped Entry") => untargeted(vec![Effect::for_players(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(1, PlayerFilter::IteratedPlayer)],
        )]),
        (_, "Veils of Fear") => untargeted(vec![each_player_loses_unless(
            2,
            Effect::discard_player(1, PlayerFilter::IteratedPlayer, false),
        )]),
        (_, "Sandfall Cell") => untargeted(vec![each_player_loses_unless(
            2,
            Effect::sacrifice_player(
                ObjectFilter {
                    zone: Some(crate::zone::Zone::Battlefield),
                    card_types: vec![CardType::Artifact, CardType::Creature, CardType::Land],
                    ..Default::default()
                },
                1,
                PlayerFilter::IteratedPlayer,
            ),
        )]),
        (_, "Oubliette") => {
            // "Discard a card and sacrifice an artifact, a creature, and a
            // land." Choose the three permanents first, then sacrifice them
            // in one simultaneous event (CR 701.21a).
            let chosen = "__oubliette_sacrificed";
            let choose = |filter: ObjectFilter| {
                Effect::choose_objects(
                    filter.you_control().not_tagged(chosen),
                    1,
                    PlayerFilter::You,
                    chosen,
                )
            };
            untargeted(vec![
                Effect::discard(1),
                choose(ObjectFilter::artifact()),
                choose(ObjectFilter::creature()),
                choose(ObjectFilter::land()),
                Effect::sacrifice_player(ObjectFilter::tagged(chosen), 3, PlayerFilter::You),
            ])
        }
        (_, "Cradle of the Death God") => untargeted(vec![Effect::create_tokens(
            room_token(
                "The Atropal",
                vec![Subtype::God, Subtype::Horror],
                ColorSet::BLACK,
                4,
                4,
            )
            .supertypes(vec![Supertype::Legendary])
            .deathtouch()
            .build(),
            1,
        )]),

        // Undercity
        (_, "Secret Entrance") => untargeted(vec![Effect::search_library_to_hand(
            ObjectFilter::default()
                .in_zone(crate::zone::Zone::Library)
                .owned_by(PlayerFilter::You)
                .with_all_type(CardType::Land)
                .with_supertype(Supertype::Basic),
            true,
        )]),
        (_, "Forge") => {
            let target = target_creature();
            targeted(
                target.clone(),
                vec![Effect::put_counters(CounterType::PlusOnePlusOne, 2, target)],
            )
        }
        (_, "Lost Well") => untargeted(vec![Effect::scry(2)]),
        (_, "Trap!") => {
            let target = ChooseSpec::target_player();
            targeted(target, vec![Effect::lose_life_target(5)])
        }
        (_, "Arena") => {
            let target = target_creature();
            targeted(target.clone(), vec![Effect::goad(target)])
        }
        (_, "Archives") => untargeted(vec![Effect::draw(1)]),
        (_, "Throne of the Dead Three") => untargeted(vec![Effect::new(
            crate::effects::ThroneOfTheDeadThreeEffect,
        )]),
        _ => return None,
    };
    Some(ability)
}
