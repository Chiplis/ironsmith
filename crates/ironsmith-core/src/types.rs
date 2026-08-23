#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Supertype {
    Basic,
    Legendary,
    Ongoing,
    Snow,
    World,
}

impl Supertype {
    pub fn name(self) -> &'static str {
        match self {
            Supertype::Basic => "basic",
            Supertype::Legendary => "legendary",
            Supertype::Ongoing => "ongoing",
            Supertype::Snow => "snow",
            Supertype::World => "world",
        }
    }
}

impl std::fmt::Display for Supertype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardType {
    Land,
    Creature,
    Artifact,
    Enchantment,
    Planeswalker,
    Instant,
    Sorcery,
    Battle,
    Plane,
    Phenomenon,
    Vanguard,
    Scheme,
    Conspiracy,
    Kindred, // Formerly Tribal
}

impl CardType {
    pub fn name(self) -> &'static str {
        match self {
            CardType::Land => "land",
            CardType::Creature => "creature",
            CardType::Artifact => "artifact",
            CardType::Enchantment => "enchantment",
            CardType::Planeswalker => "planeswalker",
            CardType::Instant => "instant",
            CardType::Sorcery => "sorcery",
            CardType::Battle => "battle",
            CardType::Plane => "plane",
            CardType::Phenomenon => "phenomenon",
            CardType::Vanguard => "vanguard",
            CardType::Scheme => "scheme",
            CardType::Conspiracy => "conspiracy",
            CardType::Kindred => "kindred",
        }
    }

    pub fn card_phrase(self) -> &'static str {
        match self {
            CardType::Land => "land card",
            CardType::Creature => "creature card",
            CardType::Artifact => "artifact card",
            CardType::Enchantment => "enchantment card",
            CardType::Planeswalker => "planeswalker card",
            CardType::Instant => "instant card",
            CardType::Sorcery => "sorcery card",
            CardType::Battle => "battle card",
            CardType::Plane => "plane card",
            CardType::Phenomenon => "phenomenon card",
            CardType::Vanguard => "vanguard card",
            CardType::Scheme => "scheme card",
            CardType::Conspiracy => "conspiracy card",
            CardType::Kindred => "kindred card",
        }
    }

    pub fn plural_name(self) -> &'static str {
        match self {
            CardType::Land => "lands",
            CardType::Creature => "creatures",
            CardType::Artifact => "artifacts",
            CardType::Enchantment => "enchantments",
            CardType::Planeswalker => "planeswalkers",
            CardType::Instant => "instants",
            CardType::Sorcery => "sorceries",
            CardType::Battle => "battles",
            CardType::Plane => "planes",
            CardType::Phenomenon => "phenomena",
            CardType::Vanguard => "vanguards",
            CardType::Scheme => "schemes",
            CardType::Conspiracy => "conspiracies",
            CardType::Kindred => "kindred cards",
        }
    }

    pub fn selection_name(self) -> &'static str {
        match self {
            CardType::Battle | CardType::Kindred => "permanent",
            _ => self.name(),
        }
    }

    pub fn self_subject(self, fallback: &'static str) -> &'static str {
        match self {
            CardType::Instant | CardType::Sorcery => fallback,
            CardType::Creature => "creature",
            _ => self.name(),
        }
    }
}

impl std::fmt::Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtypeFamily {
    Land,
    Creature,
    Artifact,
    Enchantment,
    Spell,
    Planeswalker,
    Battle,
}

impl SubtypeFamily {
    pub const fn type_phrase(self) -> &'static str {
        match self {
            SubtypeFamily::Land => "land type",
            SubtypeFamily::Creature => "creature type",
            SubtypeFamily::Artifact => "artifact type",
            SubtypeFamily::Enchantment => "enchantment type",
            SubtypeFamily::Spell => "spell type",
            SubtypeFamily::Planeswalker => "planeswalker type",
            SubtypeFamily::Battle => "battle type",
        }
    }

    pub const fn all_subtypes(self) -> &'static [Subtype] {
        match self {
            SubtypeFamily::Land => Subtype::all_land_types(),
            SubtypeFamily::Creature => Subtype::all_creature_types(),
            SubtypeFamily::Artifact => Subtype::all_artifact_types(),
            SubtypeFamily::Enchantment => Subtype::all_enchantment_types(),
            SubtypeFamily::Spell => Subtype::all_spell_types(),
            SubtypeFamily::Planeswalker => Subtype::all_planeswalker_types(),
            SubtypeFamily::Battle => Subtype::all_battle_types(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subtype {
    // Basic land types
    Plains,
    Island,
    Swamp,
    Mountain,
    Forest,

    // Non-basic land types
    Desert,
    Urzas,
    Cave,
    Gate,
    Locus,
    Town,
    Lair,
    Mine,
    Planet,
    PowerPlant,
    Sphere,
    Tower,

    // Artifact/enchantment/spell types (generated from card pool sweep)
    Blood,
    Infinity,
    Powerstone,
    Stone,
    Vibranium,
    Plan,
    Quest,
    Chorus,
    Omen,
    // Creature types (alphabetical, common ones)
    Aetherborn,
    Advisor,
    Ally,
    Alien,
    Angel,
    Antelope,
    Ape,
    Aurochs,
    Army,
    Archer,
    Archon,
    Artificer,
    Assassin,
    Astartes,
    Atog,
    Avatar,
    Barbarian,
    Bard,
    Bat,
    Bear,
    Beast,
    Berserker,
    Bird,
    Blinkmoth,
    Boar,
    Cat,
    Centaur,
    Camarid,
    Citizen,
    Clown,
    Coward,
    Changeling,
    Cleric,
    Construct,
    Crab,
    Crocodile,
    Cyclops,
    Cyberman,
    Dalek,
    Dauthi,
    Detective,
    Doctor,
    Demon,
    Devil,
    Dinosaur,
    Djinn,
    Efreet,
    Dog,
    Drone,
    Dragon,
    Drake,
    Druid,
    Dwarf,
    Elder,
    Egg,
    Eldrazi,
    Hamster,
    Spawn,
    Scion,
    Elemental,
    Elephant,
    Elk,
    Elf,
    Employee,
    Eye,
    Faerie,
    Fish,
    Fox,
    Fractal,
    Frog,
    Fungus,
    Gamer,
    Gargoyle,
    Giant,
    Gnome,
    Glimmer,
    Goat,
    Goblin,
    God,
    Golem,
    Gorgon,
    Gremlin,
    Germ,
    Griffin,
    Guest,
    Hag,
    Halfling,
    Harpy,
    Hellion,
    Hero,
    Hippo,
    Horror,
    Homunculus,
    Horse,
    Hound,
    Human,
    Hydra,
    Illusion,
    Imp,
    Insect,
    Inkling,
    Jackal,
    Jellyfish,
    Kavu,
    Kirin,
    Kithkin,
    Knight,
    Kobold,
    Kor,
    Kraken,
    Leech,
    Leviathan,
    Lhurgoyf,
    Lizard,
    Manticore,
    Mercenary,
    Merfolk,
    Minion,
    Mite,
    Minotaur,
    Mole,
    Monk,
    Monkey,
    Moonfolk,
    Mount,
    Mouse,
    Mutant,
    Myr,
    Naga,
    Necron,
    Nightmare,
    Ninja,
    Noble,
    Octopus,
    Ogre,
    Ooze,
    Orc,
    Otter,
    Ouphe,
    Ox,
    Oyster,
    Peasant,
    Performer,
    Pest,
    Pegasus,
    Phyrexian,
    Phoenix,
    Pincher,
    Pilot,
    Pirate,
    Plant,
    Praetor,
    Prism,
    Raccoon,
    Rabbit,
    Rat,
    Ranger,
    Reflection,
    Rebel,
    Rhino,
    Rigger,
    Rogue,
    Robot,
    Salamander,
    Saproling,
    Samurai,
    Satyr,
    Scarecrow,
    Scientist,
    Scout,
    Servo,
    Serpent,
    Shade,
    Shaman,
    Shapeshifter,
    Shark,
    Sheep,
    Skeleton,
    Slith,
    Sliver,
    Slug,
    Snake,
    Soldier,
    Sorcerer,
    Spacecraft,
    Spellshaper,
    Sphinx,
    Specter,
    Spider,
    Spike,
    Splinter,
    Spirit,
    Sponge,
    Squid,
    Squirrel,
    Starfish,
    Surrakar,
    Survivor,
    Thopter,
    Thrull,
    Tiefling,
    Tentacle,
    Toy,
    Treefolk,
    Triskelavite,
    Trilobite,
    Troll,
    Turtle,
    Tyranid,
    Unicorn,
    Utrom,
    Vampire,
    Vedalken,
    Viashino,
    Villain,
    Wall,
    Warlock,
    Warrior,
    Weird,
    Werewolf,
    Whale,
    Wizard,
    Wolf,
    Wolverine,
    Wombat,
    Worm,
    Wraith,
    Wurm,
    Yeti,
    Zombie,
    Zubera,

    // Artifact subtypes
    Attraction,
    Bobblehead,
    Book,
    Clue,
    Contraption,
    Equipment,
    Food,
    Fortification,
    Gold,
    Incubator,
    Junk,
    Lander,
    Map,
    Mutagen,
    Treasure,
    Vehicle,

    // Enchantment subtypes
    Aura,
    Background,
    Cartouche,
    Case,
    Class,
    Curse,
    Room,
    Role,
    Rune,
    Saga,
    Shard,
    Shrine,

    // Spell subtypes
    Adventure,
    Arcane,
    Lesson,
    Trap,

    // Creature types (generated from card pool sweep)
    Armadillo,
    AssemblyWorker,
    Azra,
    Badger,
    Balloon,
    Basilisk,
    Beaver,
    Beeble,
    Beholder,
    Bison,
    Bringer,
    Brushwagg,
    Ctan,
    Camel,
    Caribou,
    Capybara,
    Carrier,
    Child,
    Chimera,
    Cockatrice,
    Coyote,
    Custodes,
    Demigod,
    Dreadnought,
    Drix,
    Dryad,
    Echidna,
    Eternal,
    Ferret,
    Flagbearer,
    Gamma,
    Giraffe,
    Gith,
    Gnoll,
    Graveborn,
    Hedgehog,
    Hippogriff,
    Homarid,
    Hyena,
    Incarnation,
    Inhuman,
    Inquisitor,
    Juggernaut,
    Kangaroo,
    Kree,
    Lamia,
    Lammasu,
    Lemur,
    Licid,
    Lobster,
    Lord,
    Masticore,
    Metathran,
    Monger,
    Mongoose,
    Moogle,
    Mystic,
    Nautilus,
    Nephilim,
    Nightstalker,
    Noggle,
    Nomad,
    Nymph,
    Orgg,
    Pangolin,
    Pentavite,
    Phelddagrif,
    Platypus,
    Porcupine,
    Possum,
    Primarch,
    Processor,
    Qu,
    Rukh,
    Sable,
    Sand,
    Scorpion,
    Sculpture,
    Seal,
    Serf,
    Shiar,
    Siren,
    Skrull,
    Skunk,
    Sloth,
    Snail,
    Soltari,
    Spy,
    Symbiote,
    Synth,
    Thalakos,
    Time,
    TimeLord,
    Varmint,
    Volver,
    Walrus,
    Weasel,
    // Planeswalker types
    Ajani,
    Ashiok,
    Bolas,
    Chandra,
    Elspeth,
    Garruk,
    Gideon,
    Jace,
    Karn,
    Kaya,
    Liliana,
    Nissa,
    Sorin,
    Teferi,
    Tyvar,
    Ugin,
    Vraska,

    // Planeswalker types (generated from card pool sweep)
    Aminatou,
    Angrath,
    Arlinn,
    Bahamut,
    Basri,
    Calix,
    Comet,
    Dack,
    Dakkon,
    Daretti,
    Davriel,
    Dellian,
    Dihada,
    Domri,
    Dovin,
    Ellywick,
    Elminster,
    Estrid,
    Freyalise,
    Grist,
    Guff,
    Huatli,
    Jared,
    Jaya,
    Jeska,
    Kaito,
    Kasmina,
    Kiora,
    Koth,
    Lolth,
    Lukka,
    Minsc,
    Mordenkainen,
    Nahiri,
    Narset,
    Niko,
    Nixilis,
    Oko,
    Quintorius,
    Ral,
    Rowan,
    Saheeli,
    Samut,
    Sarkhan,
    Serra,
    Sivitri,
    Szat,
    Tamiyo,
    Tasha,
    Teyo,
    Tezzeret,
    Tibalt,
    Urza,
    Venser,
    Vivien,
    Vronos,
    Will,
    Windgrace,
    Wrenn,
    Xenagos,
    Yanggu,
    Yanling,
    Zariel,
    // Battle subtypes
    Siege,
}

impl Subtype {
    pub const fn all_land_types() -> &'static [Subtype] {
        &[
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
            Subtype::Desert,
            Subtype::Urzas,
            Subtype::Cave,
            Subtype::Gate,
            Subtype::Locus,
            Subtype::Town,
            Subtype::Lair,
            Subtype::Mine,
            Subtype::Planet,
            Subtype::PowerPlant,
            Subtype::Sphere,
            Subtype::Tower,
        ]
    }

    pub const fn all_creature_types() -> &'static [Subtype] {
        &[
            Subtype::Aetherborn,
            Subtype::Advisor,
            Subtype::Ally,
            Subtype::Alien,
            Subtype::Angel,
            Subtype::Antelope,
            Subtype::Ape,
            Subtype::Aurochs,
            Subtype::Army,
            Subtype::Archer,
            Subtype::Archon,
            Subtype::Artificer,
            Subtype::Assassin,
            Subtype::Astartes,
            Subtype::Atog,
            Subtype::Avatar,
            Subtype::Barbarian,
            Subtype::Bard,
            Subtype::Bat,
            Subtype::Bear,
            Subtype::Beast,
            Subtype::Berserker,
            Subtype::Bird,
            Subtype::Blinkmoth,
            Subtype::Boar,
            Subtype::Cat,
            Subtype::Centaur,
            Subtype::Camarid,
            Subtype::Citizen,
            Subtype::Clown,
            Subtype::Coward,
            Subtype::Changeling,
            Subtype::Cleric,
            Subtype::Construct,
            Subtype::Crab,
            Subtype::Crocodile,
            Subtype::Cyclops,
            Subtype::Cyberman,
            Subtype::Detective,
            Subtype::Doctor,
            Subtype::Demon,
            Subtype::Devil,
            Subtype::Dinosaur,
            Subtype::Djinn,
            Subtype::Efreet,
            Subtype::Dog,
            Subtype::Drone,
            Subtype::Dragon,
            Subtype::Drake,
            Subtype::Druid,
            Subtype::Dwarf,
            Subtype::Elder,
            Subtype::Egg,
            Subtype::Eldrazi,
            Subtype::Hamster,
            Subtype::Spawn,
            Subtype::Scion,
            Subtype::Elemental,
            Subtype::Elephant,
            Subtype::Elk,
            Subtype::Elf,
            Subtype::Employee,
            Subtype::Eye,
            Subtype::Faerie,
            Subtype::Fish,
            Subtype::Fox,
            Subtype::Fractal,
            Subtype::Frog,
            Subtype::Fungus,
            Subtype::Gamer,
            Subtype::Gargoyle,
            Subtype::Giant,
            Subtype::Gnome,
            Subtype::Glimmer,
            Subtype::Goat,
            Subtype::Goblin,
            Subtype::God,
            Subtype::Golem,
            Subtype::Gorgon,
            Subtype::Gremlin,
            Subtype::Germ,
            Subtype::Griffin,
            Subtype::Guest,
            Subtype::Hag,
            Subtype::Halfling,
            Subtype::Harpy,
            Subtype::Hellion,
            Subtype::Hero,
            Subtype::Hippo,
            Subtype::Horror,
            Subtype::Homunculus,
            Subtype::Horse,
            Subtype::Hound,
            Subtype::Human,
            Subtype::Hydra,
            Subtype::Illusion,
            Subtype::Imp,
            Subtype::Insect,
            Subtype::Inkling,
            Subtype::Jackal,
            Subtype::Jellyfish,
            Subtype::Kavu,
            Subtype::Kirin,
            Subtype::Kithkin,
            Subtype::Knight,
            Subtype::Kobold,
            Subtype::Kor,
            Subtype::Kraken,
            Subtype::Leech,
            Subtype::Leviathan,
            Subtype::Lhurgoyf,
            Subtype::Lizard,
            Subtype::Manticore,
            Subtype::Mercenary,
            Subtype::Merfolk,
            Subtype::Minion,
            Subtype::Minotaur,
            Subtype::Mole,
            Subtype::Monk,
            Subtype::Monkey,
            Subtype::Moonfolk,
            Subtype::Mount,
            Subtype::Mouse,
            Subtype::Mutant,
            Subtype::Myr,
            Subtype::Naga,
            Subtype::Necron,
            Subtype::Nightmare,
            Subtype::Ninja,
            Subtype::Noble,
            Subtype::Octopus,
            Subtype::Ogre,
            Subtype::Ooze,
            Subtype::Orc,
            Subtype::Otter,
            Subtype::Ouphe,
            Subtype::Ox,
            Subtype::Oyster,
            Subtype::Peasant,
            Subtype::Performer,
            Subtype::Pest,
            Subtype::Pegasus,
            Subtype::Phyrexian,
            Subtype::Phoenix,
            Subtype::Pincher,
            Subtype::Pilot,
            Subtype::Pirate,
            Subtype::Plant,
            Subtype::Praetor,
            Subtype::Prism,
            Subtype::Raccoon,
            Subtype::Rabbit,
            Subtype::Rat,
            Subtype::Ranger,
            Subtype::Reflection,
            Subtype::Rebel,
            Subtype::Rhino,
            Subtype::Rigger,
            Subtype::Rogue,
            Subtype::Robot,
            Subtype::Salamander,
            Subtype::Saproling,
            Subtype::Samurai,
            Subtype::Satyr,
            Subtype::Scarecrow,
            Subtype::Scientist,
            Subtype::Scout,
            Subtype::Servo,
            Subtype::Serpent,
            Subtype::Shade,
            Subtype::Shaman,
            Subtype::Shapeshifter,
            Subtype::Shark,
            Subtype::Sheep,
            Subtype::Skeleton,
            Subtype::Slith,
            Subtype::Sliver,
            Subtype::Slug,
            Subtype::Snake,
            Subtype::Soldier,
            Subtype::Sorcerer,
            Subtype::Spellshaper,
            Subtype::Sphinx,
            Subtype::Specter,
            Subtype::Spider,
            Subtype::Spike,
            Subtype::Splinter,
            Subtype::Spirit,
            Subtype::Sponge,
            Subtype::Squid,
            Subtype::Squirrel,
            Subtype::Starfish,
            Subtype::Surrakar,
            Subtype::Survivor,
            Subtype::Thopter,
            Subtype::Thrull,
            Subtype::Tiefling,
            Subtype::Tentacle,
            Subtype::Toy,
            Subtype::Treefolk,
            Subtype::Triskelavite,
            Subtype::Trilobite,
            Subtype::Troll,
            Subtype::Turtle,
            Subtype::Tyranid,
            Subtype::Unicorn,
            Subtype::Utrom,
            Subtype::Vampire,
            Subtype::Vedalken,
            Subtype::Viashino,
            Subtype::Villain,
            Subtype::Wall,
            Subtype::Warlock,
            Subtype::Warrior,
            Subtype::Weird,
            Subtype::Werewolf,
            Subtype::Whale,
            Subtype::Wizard,
            Subtype::Wolf,
            Subtype::Wolverine,
            Subtype::Wombat,
            Subtype::Worm,
            Subtype::Wraith,
            Subtype::Wurm,
            Subtype::Yeti,
            Subtype::Zombie,
            Subtype::Zubera,
            Subtype::Armadillo,
            Subtype::AssemblyWorker,
            Subtype::Azra,
            Subtype::Badger,
            Subtype::Balloon,
            Subtype::Basilisk,
            Subtype::Beaver,
            Subtype::Beeble,
            Subtype::Beholder,
            Subtype::Bison,
            Subtype::Bringer,
            Subtype::Brushwagg,
            Subtype::Ctan,
            Subtype::Camel,
            Subtype::Caribou,
            Subtype::Capybara,
            Subtype::Carrier,
            Subtype::Child,
            Subtype::Chimera,
            Subtype::Cockatrice,
            Subtype::Coyote,
            Subtype::Custodes,
            Subtype::Demigod,
            Subtype::Dreadnought,
            Subtype::Drix,
            Subtype::Dryad,
            Subtype::Echidna,
            Subtype::Eternal,
            Subtype::Ferret,
            Subtype::Flagbearer,
            Subtype::Gamma,
            Subtype::Giraffe,
            Subtype::Gith,
            Subtype::Gnoll,
            Subtype::Graveborn,
            Subtype::Hedgehog,
            Subtype::Hippogriff,
            Subtype::Homarid,
            Subtype::Hyena,
            Subtype::Incarnation,
            Subtype::Inhuman,
            Subtype::Inquisitor,
            Subtype::Juggernaut,
            Subtype::Kangaroo,
            Subtype::Kree,
            Subtype::Lamia,
            Subtype::Lammasu,
            Subtype::Lemur,
            Subtype::Licid,
            Subtype::Lobster,
            Subtype::Lord,
            Subtype::Masticore,
            Subtype::Metathran,
            Subtype::Monger,
            Subtype::Mongoose,
            Subtype::Moogle,
            Subtype::Mystic,
            Subtype::Nautilus,
            Subtype::Nephilim,
            Subtype::Nightstalker,
            Subtype::Noggle,
            Subtype::Nomad,
            Subtype::Nymph,
            Subtype::Orgg,
            Subtype::Pangolin,
            Subtype::Pentavite,
            Subtype::Phelddagrif,
            Subtype::Platypus,
            Subtype::Porcupine,
            Subtype::Possum,
            Subtype::Primarch,
            Subtype::Processor,
            Subtype::Qu,
            Subtype::Rukh,
            Subtype::Sable,
            Subtype::Sand,
            Subtype::Scorpion,
            Subtype::Sculpture,
            Subtype::Seal,
            Subtype::Serf,
            Subtype::Shiar,
            Subtype::Siren,
            Subtype::Skrull,
            Subtype::Skunk,
            Subtype::Sloth,
            Subtype::Snail,
            Subtype::Soltari,
            Subtype::Spy,
            Subtype::Symbiote,
            Subtype::Synth,
            Subtype::Thalakos,
            Subtype::Time,
            Subtype::TimeLord,
            Subtype::Varmint,
            Subtype::Volver,
            Subtype::Walrus,
            Subtype::Weasel,
        ]
    }

    pub const fn all_artifact_types() -> &'static [Subtype] {
        &[
            Subtype::Attraction,
            Subtype::Bobblehead,
            Subtype::Book,
            Subtype::Clue,
            Subtype::Contraption,
            Subtype::Equipment,
            Subtype::Food,
            Subtype::Fortification,
            Subtype::Gold,
            Subtype::Incubator,
            Subtype::Junk,
            Subtype::Lander,
            Subtype::Map,
            Subtype::Mutagen,
            Subtype::Treasure,
            Subtype::Vehicle,
            Subtype::Blood,
            Subtype::Infinity,
            Subtype::Powerstone,
            Subtype::Spacecraft,
            Subtype::Stone,
            Subtype::Vibranium,
        ]
    }

    pub const fn all_enchantment_types() -> &'static [Subtype] {
        &[
            Subtype::Aura,
            Subtype::Background,
            Subtype::Cartouche,
            Subtype::Case,
            Subtype::Class,
            Subtype::Curse,
            Subtype::Room,
            Subtype::Role,
            Subtype::Rune,
            Subtype::Saga,
            Subtype::Shard,
            Subtype::Shrine,
            Subtype::Plan,
            Subtype::Quest,
        ]
    }

    pub const fn all_spell_types() -> &'static [Subtype] {
        &[
            Subtype::Adventure,
            Subtype::Arcane,
            Subtype::Lesson,
            Subtype::Trap,
            Subtype::Chorus,
            Subtype::Omen,
        ]
    }

    pub const fn all_planeswalker_types() -> &'static [Subtype] {
        &[
            Subtype::Ajani,
            Subtype::Ashiok,
            Subtype::Bolas,
            Subtype::Chandra,
            Subtype::Elspeth,
            Subtype::Garruk,
            Subtype::Gideon,
            Subtype::Jace,
            Subtype::Karn,
            Subtype::Kaya,
            Subtype::Liliana,
            Subtype::Nissa,
            Subtype::Sorin,
            Subtype::Teferi,
            Subtype::Tyvar,
            Subtype::Ugin,
            Subtype::Vraska,
            Subtype::Aminatou,
            Subtype::Angrath,
            Subtype::Arlinn,
            Subtype::Bahamut,
            Subtype::Basri,
            Subtype::Calix,
            Subtype::Comet,
            Subtype::Dack,
            Subtype::Dakkon,
            Subtype::Daretti,
            Subtype::Davriel,
            Subtype::Dellian,
            Subtype::Dihada,
            Subtype::Domri,
            Subtype::Dovin,
            Subtype::Ellywick,
            Subtype::Elminster,
            Subtype::Estrid,
            Subtype::Freyalise,
            Subtype::Grist,
            Subtype::Guff,
            Subtype::Huatli,
            Subtype::Jared,
            Subtype::Jaya,
            Subtype::Jeska,
            Subtype::Kaito,
            Subtype::Kasmina,
            Subtype::Kiora,
            Subtype::Koth,
            Subtype::Lolth,
            Subtype::Lukka,
            Subtype::Minsc,
            Subtype::Mordenkainen,
            Subtype::Nahiri,
            Subtype::Narset,
            Subtype::Niko,
            Subtype::Nixilis,
            Subtype::Oko,
            Subtype::Quintorius,
            Subtype::Ral,
            Subtype::Rowan,
            Subtype::Saheeli,
            Subtype::Samut,
            Subtype::Sarkhan,
            Subtype::Serra,
            Subtype::Sivitri,
            Subtype::Szat,
            Subtype::Tamiyo,
            Subtype::Tasha,
            Subtype::Teyo,
            Subtype::Tezzeret,
            Subtype::Tibalt,
            Subtype::Urza,
            Subtype::Venser,
            Subtype::Vivien,
            Subtype::Vronos,
            Subtype::Will,
            Subtype::Windgrace,
            Subtype::Wrenn,
            Subtype::Xenagos,
            Subtype::Yanggu,
            Subtype::Yanling,
            Subtype::Zariel,
        ]
    }

    pub const fn all_battle_types() -> &'static [Subtype] {
        &[Subtype::Siege]
    }

    pub fn display_name(self) -> String {
        match self {
            Subtype::Urzas => "Urza's".to_string(),
            Subtype::AssemblyWorker => "Assembly-Worker".to_string(),
            Subtype::Ctan => "C'tan".to_string(),
            Subtype::PowerPlant => "Power-Plant".to_string(),
            Subtype::Shiar => "Shi'ar".to_string(),
            _ => split_pascal_case_identifier(&format!("{self:?}")),
        }
    }

    /// Returns true if this is a basic land type.
    pub fn is_basic_land_type(&self) -> bool {
        matches!(
            self,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    }

    /// Returns true if this is a land subtype (basic or non-basic).
    ///
    /// Used by Blood Moon and similar effects to determine which subtypes
    /// to replace. Non-land subtypes (Saga, Aura, creature types, etc.)
    /// are preserved.
    pub fn is_land_subtype(&self) -> bool {
        matches!(
            self,
            // Basic land types
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                // Non-basic land types
                | Subtype::Desert
                | Subtype::Urzas
                | Subtype::Cave
                | Subtype::Gate
                | Subtype::Locus
                | Subtype::Town
        )
    }

    /// Returns true if this is a creature type.
    pub fn is_creature_type(&self) -> bool {
        Self::all_creature_types().contains(self)
    }

    pub fn is_artifact_subtype(&self) -> bool {
        matches!(
            self,
            Subtype::Attraction
                | Subtype::Clue
                | Subtype::Bobblehead
                | Subtype::Book
                | Subtype::Contraption
                | Subtype::Equipment
                | Subtype::Food
                | Subtype::Fortification
                | Subtype::Gold
                | Subtype::Incubator
                | Subtype::Junk
                | Subtype::Lander
                | Subtype::Map
                | Subtype::Mutagen
                | Subtype::Spacecraft
                | Subtype::Treasure
                | Subtype::Vehicle
        )
    }

    pub fn is_enchantment_subtype(&self) -> bool {
        matches!(
            self,
            Subtype::Aura
                | Subtype::Background
                | Subtype::Cartouche
                | Subtype::Case
                | Subtype::Class
                | Subtype::Curse
                | Subtype::Room
                | Subtype::Role
                | Subtype::Rune
                | Subtype::Saga
                | Subtype::Shard
                | Subtype::Shrine
        )
    }

    pub fn is_spell_subtype(&self) -> bool {
        matches!(
            self,
            Subtype::Adventure | Subtype::Arcane | Subtype::Lesson | Subtype::Trap
        )
    }

    pub fn is_planeswalker_subtype(&self) -> bool {
        matches!(
            self,
            Subtype::Ajani
                | Subtype::Ashiok
                | Subtype::Bolas
                | Subtype::Chandra
                | Subtype::Elspeth
                | Subtype::Garruk
                | Subtype::Gideon
                | Subtype::Jace
                | Subtype::Karn
                | Subtype::Kaya
                | Subtype::Liliana
                | Subtype::Nissa
                | Subtype::Sorin
                | Subtype::Teferi
                | Subtype::Tyvar
                | Subtype::Ugin
                | Subtype::Vraska
        )
    }

    pub fn belongs_to_family(&self, family: SubtypeFamily) -> bool {
        match family {
            SubtypeFamily::Land => self.is_land_subtype(),
            SubtypeFamily::Creature => self.is_creature_type(),
            SubtypeFamily::Artifact => self.is_artifact_subtype(),
            SubtypeFamily::Enchantment => self.is_enchantment_subtype(),
            SubtypeFamily::Spell => self.is_spell_subtype(),
            SubtypeFamily::Planeswalker => self.is_planeswalker_subtype(),
            SubtypeFamily::Battle => self.is_battle_subtype(),
        }
    }

    pub fn is_battle_subtype(&self) -> bool {
        matches!(self, Subtype::Siege)
    }
}

impl std::fmt::Display for Subtype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display_name())
    }
}

fn split_pascal_case_identifier(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_land_types() {
        assert!(Subtype::Plains.is_basic_land_type());
        assert!(Subtype::Island.is_basic_land_type());
        assert!(Subtype::Swamp.is_basic_land_type());
        assert!(Subtype::Mountain.is_basic_land_type());
        assert!(Subtype::Forest.is_basic_land_type());
        assert!(!Subtype::Human.is_basic_land_type());
    }

    #[test]
    fn test_creature_types() {
        assert!(Subtype::Human.is_creature_type());
        assert!(Subtype::Elf.is_creature_type());
        assert!(Subtype::Goblin.is_creature_type());
        assert!(Subtype::Bat.is_creature_type());
        assert!(Subtype::Pest.is_creature_type());
        assert!(Subtype::Fractal.is_creature_type());
        assert!(Subtype::TimeLord.is_creature_type());
        assert_eq!(Subtype::TimeLord.to_string(), "Time Lord");
        assert!(!Subtype::Plains.is_creature_type());
        assert!(!Subtype::Equipment.is_creature_type());
    }

    #[test]
    fn test_subtype_family_membership() {
        assert!(Subtype::Equipment.belongs_to_family(SubtypeFamily::Artifact));
        assert!(Subtype::Aura.belongs_to_family(SubtypeFamily::Enchantment));
        assert!(Subtype::Arcane.belongs_to_family(SubtypeFamily::Spell));
        assert!(Subtype::Jace.belongs_to_family(SubtypeFamily::Planeswalker));
        assert!(!Subtype::Elf.belongs_to_family(SubtypeFamily::Artifact));
    }
}
