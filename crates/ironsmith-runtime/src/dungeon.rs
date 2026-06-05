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
