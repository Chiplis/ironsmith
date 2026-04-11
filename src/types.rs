#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Supertype {
    Basic,
    Legendary,
    Snow,
    World,
}

impl Supertype {
    pub fn name(self) -> &'static str {
        match self {
            Supertype::Basic => "basic",
            Supertype::Legendary => "legendary",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtypeFamily {
    Land,
    Creature,
    Artifact,
    Enchantment,
    Spell,
    Planeswalker,
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
        }
    }
}

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

    // Creature types (alphabetical, common ones)
    Advisor,
    Ally,
    Alien,
    Angel,
    Ape,
    Army,
    Archer,
    Artificer,
    Assassin,
    Astartes,
    Avatar,
    Barbarian,
    Bard,
    Bat,
    Bear,
    Beast,
    Berserker,
    Bird,
    Boar,
    Cat,
    Centaur,
    Citizen,
    Coward,
    Changeling,
    Cleric,
    Construct,
    Crab,
    Crocodile,
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
    Eldrazi,
    Hamster,
    Spawn,
    Scion,
    Elemental,
    Elephant,
    Elk,
    Elf,
    Faerie,
    Fish,
    Fox,
    Frog,
    Fungus,
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
    Hag,
    Halfling,
    Harpy,
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
    Leviathan,
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
    Pest,
    Pegasus,
    Phyrexian,
    Phoenix,
    Pincher,
    Pilot,
    Pirate,
    Plant,
    Praetor,
    Raccoon,
    Rabbit,
    Rat,
    Reflection,
    Rebel,
    Rhino,
    Rogue,
    Robot,
    Salamander,
    Saproling,
    Samurai,
    Satyr,
    Scarecrow,
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
    Unicorn,
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
    Clue,
    Contraption,
    Equipment,
    Food,
    Fortification,
    Gold,
    Junk,
    Lander,
    Map,
    Treasure,
    Vehicle,

    // Enchantment subtypes
    Aura,
    Background,
    Cartouche,
    Class,
    Curse,
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

    // Planeswalker types
    Ajani,
    Ashiok,
    Chandra,
    Elspeth,
    Garruk,
    Gideon,
    Jace,
    Karn,
    Liliana,
    Nissa,
    Sorin,
    Teferi,
    Tyvar,
    Ugin,
    Vraska,
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
        ]
    }

    pub const fn all_creature_types() -> &'static [Subtype] {
        &[
            Subtype::Advisor,
            Subtype::Ally,
            Subtype::Alien,
            Subtype::Angel,
            Subtype::Ape,
            Subtype::Army,
            Subtype::Archer,
            Subtype::Artificer,
            Subtype::Assassin,
            Subtype::Astartes,
            Subtype::Avatar,
            Subtype::Barbarian,
            Subtype::Bard,
            Subtype::Bear,
            Subtype::Beast,
            Subtype::Berserker,
            Subtype::Bird,
            Subtype::Boar,
            Subtype::Cat,
            Subtype::Centaur,
            Subtype::Citizen,
            Subtype::Coward,
            Subtype::Changeling,
            Subtype::Cleric,
            Subtype::Construct,
            Subtype::Crab,
            Subtype::Crocodile,
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
            Subtype::Eldrazi,
            Subtype::Hamster,
            Subtype::Spawn,
            Subtype::Scion,
            Subtype::Elemental,
            Subtype::Elephant,
            Subtype::Elk,
            Subtype::Elf,
            Subtype::Faerie,
            Subtype::Fish,
            Subtype::Fox,
            Subtype::Frog,
            Subtype::Fungus,
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
            Subtype::Hag,
            Subtype::Halfling,
            Subtype::Harpy,
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
            Subtype::Leviathan,
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
            Subtype::Pegasus,
            Subtype::Phyrexian,
            Subtype::Phoenix,
            Subtype::Pincher,
            Subtype::Pilot,
            Subtype::Pirate,
            Subtype::Plant,
            Subtype::Praetor,
            Subtype::Raccoon,
            Subtype::Rabbit,
            Subtype::Rat,
            Subtype::Reflection,
            Subtype::Rebel,
            Subtype::Rhino,
            Subtype::Rogue,
            Subtype::Robot,
            Subtype::Salamander,
            Subtype::Saproling,
            Subtype::Samurai,
            Subtype::Satyr,
            Subtype::Scarecrow,
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
            Subtype::Unicorn,
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
        ]
    }

    pub const fn all_artifact_types() -> &'static [Subtype] {
        &[
            Subtype::Clue,
            Subtype::Contraption,
            Subtype::Equipment,
            Subtype::Food,
            Subtype::Fortification,
            Subtype::Gold,
            Subtype::Junk,
            Subtype::Lander,
            Subtype::Map,
            Subtype::Treasure,
            Subtype::Vehicle,
        ]
    }

    pub const fn all_enchantment_types() -> &'static [Subtype] {
        &[
            Subtype::Aura,
            Subtype::Background,
            Subtype::Cartouche,
            Subtype::Class,
            Subtype::Curse,
            Subtype::Role,
            Subtype::Rune,
            Subtype::Saga,
            Subtype::Shard,
            Subtype::Shrine,
        ]
    }

    pub const fn all_spell_types() -> &'static [Subtype] {
        &[
            Subtype::Adventure,
            Subtype::Arcane,
            Subtype::Lesson,
            Subtype::Trap,
        ]
    }

    pub const fn all_planeswalker_types() -> &'static [Subtype] {
        &[
            Subtype::Ajani,
            Subtype::Ashiok,
            Subtype::Chandra,
            Subtype::Elspeth,
            Subtype::Garruk,
            Subtype::Gideon,
            Subtype::Jace,
            Subtype::Karn,
            Subtype::Liliana,
            Subtype::Nissa,
            Subtype::Sorin,
            Subtype::Teferi,
            Subtype::Tyvar,
            Subtype::Ugin,
            Subtype::Vraska,
        ]
    }

    pub fn display_name(self) -> String {
        match self {
            Subtype::Urzas => "Urza's".to_string(),
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
        )
    }

    /// Returns true if this is a creature type.
    pub fn is_creature_type(&self) -> bool {
        matches!(
            self,
            Subtype::Advisor
                | Subtype::Ally
                | Subtype::Alien
                | Subtype::Angel
                | Subtype::Ape
                | Subtype::Army
                | Subtype::Archer
                | Subtype::Artificer
                | Subtype::Assassin
                | Subtype::Astartes
                | Subtype::Avatar
                | Subtype::Barbarian
                | Subtype::Bard
                | Subtype::Bear
                | Subtype::Beast
                | Subtype::Berserker
                | Subtype::Bird
                | Subtype::Boar
                | Subtype::Cat
                | Subtype::Centaur
                | Subtype::Citizen
                | Subtype::Coward
                | Subtype::Changeling
                | Subtype::Cleric
                | Subtype::Construct
                | Subtype::Crab
                | Subtype::Crocodile
                | Subtype::Detective
                | Subtype::Doctor
                | Subtype::Demon
                | Subtype::Devil
                | Subtype::Dinosaur
                | Subtype::Djinn
                | Subtype::Efreet
                | Subtype::Dog
                | Subtype::Drone
                | Subtype::Dragon
                | Subtype::Drake
                | Subtype::Druid
                | Subtype::Dwarf
                | Subtype::Elder
                | Subtype::Eldrazi
                | Subtype::Hamster
                | Subtype::Spawn
                | Subtype::Scion
                | Subtype::Elemental
                | Subtype::Elephant
                | Subtype::Elk
                | Subtype::Elf
                | Subtype::Faerie
                | Subtype::Fish
                | Subtype::Fox
                | Subtype::Frog
                | Subtype::Fungus
                | Subtype::Gargoyle
                | Subtype::Giant
                | Subtype::Gnome
                | Subtype::Glimmer
                | Subtype::Goat
                | Subtype::Goblin
                | Subtype::God
                | Subtype::Golem
                | Subtype::Gorgon
                | Subtype::Gremlin
                | Subtype::Germ
                | Subtype::Griffin
                | Subtype::Hag
                | Subtype::Halfling
                | Subtype::Harpy
                | Subtype::Hippo
                | Subtype::Horror
                | Subtype::Homunculus
                | Subtype::Horse
                | Subtype::Hound
                | Subtype::Human
                | Subtype::Hydra
                | Subtype::Illusion
                | Subtype::Imp
                | Subtype::Insect
                | Subtype::Inkling
                | Subtype::Jackal
                | Subtype::Jellyfish
                | Subtype::Kavu
                | Subtype::Kirin
                | Subtype::Kithkin
                | Subtype::Knight
                | Subtype::Kobold
                | Subtype::Kor
                | Subtype::Kraken
                | Subtype::Leviathan
                | Subtype::Lizard
                | Subtype::Manticore
                | Subtype::Mercenary
                | Subtype::Merfolk
                | Subtype::Minion
                | Subtype::Minotaur
                | Subtype::Mole
                | Subtype::Monk
                | Subtype::Monkey
                | Subtype::Moonfolk
                | Subtype::Mount
                | Subtype::Mouse
                | Subtype::Mutant
                | Subtype::Myr
                | Subtype::Naga
                | Subtype::Necron
                | Subtype::Nightmare
                | Subtype::Ninja
                | Subtype::Noble
                | Subtype::Octopus
                | Subtype::Ogre
                | Subtype::Ooze
                | Subtype::Orc
                | Subtype::Otter
                | Subtype::Ouphe
                | Subtype::Ox
                | Subtype::Oyster
                | Subtype::Peasant
                | Subtype::Pegasus
                | Subtype::Phyrexian
                | Subtype::Phoenix
                | Subtype::Pincher
                | Subtype::Pilot
                | Subtype::Pirate
                | Subtype::Plant
                | Subtype::Praetor
                | Subtype::Raccoon
                | Subtype::Rabbit
                | Subtype::Rat
                | Subtype::Reflection
                | Subtype::Rebel
                | Subtype::Rhino
                | Subtype::Rogue
                | Subtype::Robot
                | Subtype::Salamander
                | Subtype::Saproling
                | Subtype::Samurai
                | Subtype::Satyr
                | Subtype::Scarecrow
                | Subtype::Scout
                | Subtype::Servo
                | Subtype::Serpent
                | Subtype::Shade
                | Subtype::Shaman
                | Subtype::Shapeshifter
                | Subtype::Shark
                | Subtype::Sheep
                | Subtype::Skeleton
                | Subtype::Slith
                | Subtype::Sliver
                | Subtype::Slug
                | Subtype::Snake
                | Subtype::Soldier
                | Subtype::Sorcerer
                | Subtype::Sphinx
                | Subtype::Specter
                | Subtype::Spider
                | Subtype::Spike
                | Subtype::Splinter
                | Subtype::Spirit
                | Subtype::Sponge
                | Subtype::Squid
                | Subtype::Squirrel
                | Subtype::Starfish
                | Subtype::Surrakar
                | Subtype::Survivor
                | Subtype::Thopter
                | Subtype::Thrull
                | Subtype::Tiefling
                | Subtype::Tentacle
                | Subtype::Toy
                | Subtype::Treefolk
                | Subtype::Triskelavite
                | Subtype::Trilobite
                | Subtype::Troll
                | Subtype::Turtle
                | Subtype::Unicorn
                | Subtype::Vampire
                | Subtype::Vedalken
                | Subtype::Viashino
                | Subtype::Villain
                | Subtype::Wall
                | Subtype::Warlock
                | Subtype::Warrior
                | Subtype::Weird
                | Subtype::Werewolf
                | Subtype::Whale
                | Subtype::Wizard
                | Subtype::Wolf
                | Subtype::Wolverine
                | Subtype::Wombat
                | Subtype::Worm
                | Subtype::Wraith
                | Subtype::Wurm
                | Subtype::Yeti
                | Subtype::Zombie
                | Subtype::Zubera
        )
    }

    pub fn is_artifact_subtype(&self) -> bool {
        matches!(
            self,
            Subtype::Clue
                | Subtype::Contraption
                | Subtype::Equipment
                | Subtype::Food
                | Subtype::Fortification
                | Subtype::Gold
                | Subtype::Junk
                | Subtype::Lander
                | Subtype::Map
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
                | Subtype::Class
                | Subtype::Curse
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
                | Subtype::Chandra
                | Subtype::Elspeth
                | Subtype::Garruk
                | Subtype::Gideon
                | Subtype::Jace
                | Subtype::Karn
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
        }
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
