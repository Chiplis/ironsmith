#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Supertype {
    Basic,
    Legendary,
    Snow,
    World,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subtype {
    // Basic land types
    Plains,
    Island,
    Swamp,
    Mountain,
    Forest,

    // Non-basic land types
    Urzas,
    Gate,

    // Creature types (alphabetical, common ones)
    Advisor,
    Ally,
    Angel,
    Ape,
    Archer,
    Artificer,
    Assassin,
    Astartes,
    Avatar,
    Barbarian,
    Bard,
    Bear,
    Beast,
    Berserker,
    Bird,
    Boar,
    Cat,
    Centaur,
    Changeling,
    Cleric,
    Construct,
    Crab,
    Crocodile,
    Dauthi,
    Demon,
    Dinosaur,
    Djinn,
    Efreet,
    Dog,
    Dragon,
    Drake,
    Druid,
    Dwarf,
    Eldrazi,
    Spawn,
    Scion,
    Elemental,
    Elephant,
    Elf,
    Faerie,
    Fish,
    Fox,
    Frog,
    Fungus,
    Gargoyle,
    Giant,
    Gnome,
    Goat,
    Goblin,
    God,
    Golem,
    Gorgon,
    Griffin,
    Hag,
    Halfling,
    Harpy,
    Hippo,
    Horror,
    Horse,
    Hound,
    Human,
    Hydra,
    Illusion,
    Imp,
    Insect,
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
    Minotaur,
    Mole,
    Monk,
    Moonfolk,
    Mouse,
    Mutant,
    Myr,
    Naga,
    Nightmare,
    Ninja,
    Noble,
    Octopus,
    Ogre,
    Ooze,
    Orc,
    Otter,
    Ox,
    Oyster,
    Peasant,
    Pegasus,
    Phyrexian,
    Phoenix,
    Pilot,
    Pirate,
    Plant,
    Praetor,
    Raccoon,
    Rabbit,
    Rat,
    Rebel,
    Rhino,
    Rogue,
    Robot,
    Salamander,
    Samurai,
    Satyr,
    Scarecrow,
    Scout,
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
    Spirit,
    Sponge,
    Squid,
    Squirrel,
    Starfish,
    Surrakar,
    Thopter,
    Thrull,
    Tiefling,
    Toy,
    Treefolk,
    Trilobite,
    Troll,
    Turtle,
    Unicorn,
    Vampire,
    Vedalken,
    Viashino,
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
    Ugin,
    Vraska,
}

impl Subtype {
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
                | Subtype::Urzas
        )
    }

    /// Returns true if this is a creature type.
    pub fn is_creature_type(&self) -> bool {
        matches!(
            self,
            Subtype::Advisor
                | Subtype::Ally
                | Subtype::Angel
                | Subtype::Ape
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
                | Subtype::Changeling
                | Subtype::Cleric
                | Subtype::Construct
                | Subtype::Crab
                | Subtype::Crocodile
                | Subtype::Demon
                | Subtype::Dinosaur
                | Subtype::Djinn
                | Subtype::Efreet
                | Subtype::Dog
                | Subtype::Dragon
                | Subtype::Drake
                | Subtype::Druid
                | Subtype::Dwarf
                | Subtype::Eldrazi
                | Subtype::Spawn
                | Subtype::Scion
                | Subtype::Elemental
                | Subtype::Elephant
                | Subtype::Elf
                | Subtype::Faerie
                | Subtype::Fish
                | Subtype::Fox
                | Subtype::Frog
                | Subtype::Fungus
                | Subtype::Gargoyle
                | Subtype::Giant
                | Subtype::Gnome
                | Subtype::Goat
                | Subtype::Goblin
                | Subtype::God
                | Subtype::Golem
                | Subtype::Gorgon
                | Subtype::Griffin
                | Subtype::Hag
                | Subtype::Halfling
                | Subtype::Harpy
                | Subtype::Hippo
                | Subtype::Horror
                | Subtype::Horse
                | Subtype::Hound
                | Subtype::Human
                | Subtype::Hydra
                | Subtype::Illusion
                | Subtype::Imp
                | Subtype::Insect
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
                | Subtype::Moonfolk
                | Subtype::Mouse
                | Subtype::Mutant
                | Subtype::Myr
                | Subtype::Naga
                | Subtype::Nightmare
                | Subtype::Ninja
                | Subtype::Noble
                | Subtype::Octopus
                | Subtype::Ogre
                | Subtype::Ooze
                | Subtype::Orc
                | Subtype::Otter
                | Subtype::Ox
                | Subtype::Oyster
                | Subtype::Peasant
                | Subtype::Pegasus
                | Subtype::Phyrexian
                | Subtype::Phoenix
                | Subtype::Pilot
                | Subtype::Pirate
                | Subtype::Plant
                | Subtype::Praetor
                | Subtype::Raccoon
                | Subtype::Rabbit
                | Subtype::Rat
                | Subtype::Rebel
                | Subtype::Rhino
                | Subtype::Rogue
                | Subtype::Robot
                | Subtype::Salamander
                | Subtype::Samurai
                | Subtype::Satyr
                | Subtype::Scarecrow
                | Subtype::Scout
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
                | Subtype::Spirit
                | Subtype::Sponge
                | Subtype::Squid
                | Subtype::Squirrel
                | Subtype::Starfish
                | Subtype::Surrakar
                | Subtype::Thopter
                | Subtype::Thrull
                | Subtype::Tiefling
                | Subtype::Toy
                | Subtype::Treefolk
                | Subtype::Trilobite
                | Subtype::Troll
                | Subtype::Turtle
                | Subtype::Unicorn
                | Subtype::Vampire
                | Subtype::Vedalken
                | Subtype::Viashino
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
}
