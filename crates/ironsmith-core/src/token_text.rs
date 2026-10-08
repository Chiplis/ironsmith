//! Authored word roles of a token-creating instruction (CR 111.4, 612.2a).
//! This metadata belongs to the instruction, not to an already-created token.
use crate::{Subtype, tag::TagKeyWalk};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum TokenNameTextRole {
    /// The instruction names the token explicitly. Its name is never a word
    /// occurrence, even when the spelling is also a color or creature type.
    Explicit,
    /// CR 111.4 derives the future token's name from its declared subtypes.
    SubtypeDerived,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum TokenWordRole {
    Authored,
    /// A predefined token or wordless keyword supplies this characteristic.
    RulesImplied,
    /// A source owner supplied the value but has not retained its word role.
    Unrecorded,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct TokenTextRoles {
    pub name: TokenNameTextRole,
    pub colors: TokenWordRole,
    pub subtypes: TokenWordRole,
    /// Exact definition occurrences, in their authored order. A missing or
    /// mismatched occurrence inventory is not evidence that every ability
    /// came from the creating instruction's explicit text.
    pub abilities: Vec<TokenWordRole>,
}

impl TokenTextRoles {
    pub fn authored(name: TokenNameTextRole, ability_count: usize) -> Self {
        Self { name, colors: TokenWordRole::Authored, subtypes: TokenWordRole::Authored,
            abilities: vec![TokenWordRole::Authored; ability_count] }
    }
    pub fn rules_implied(name: TokenNameTextRole, ability_count: usize) -> Self {
        Self { name, colors: TokenWordRole::RulesImplied, subtypes: TokenWordRole::RulesImplied,
            abilities: vec![TokenWordRole::RulesImplied; ability_count] }
    }
    pub fn has_complete_ability_inventory(&self, ability_count: usize) -> bool {
        self.abilities.len() == ability_count
    }
}

/// Canonical rules-word spellings used to construct a name under CR 111.4.
/// These literals are semantic vocabulary, independent of Debug, serde enum
/// names, UI rendering, and source-card or token-name comparisons.
pub fn token_subtype_rules_word(subtype: Subtype) -> Option<&'static str> {
    macro_rules! vocabulary {
        ($($kind:ident => $word:literal),* $(,)?) => {
            match subtype { $(Subtype::$kind => Some($word),)* _ => None }
        };
    }
    vocabulary! {
        Aetherborn=>"Aetherborn", Advisor=>"Advisor", Ally=>"Ally", Alien=>"Alien", Angel=>"Angel",
        Antelope=>"Antelope", Ape=>"Ape", Aurochs=>"Aurochs", Army=>"Army", Archer=>"Archer",
        Archon=>"Archon", Artificer=>"Artificer", Assassin=>"Assassin", Astartes=>"Astartes", Atog=>"Atog",
        Avatar=>"Avatar", Barbarian=>"Barbarian", Bard=>"Bard", Bat=>"Bat", Bear=>"Bear", Beast=>"Beast",
        Berserker=>"Berserker", Bird=>"Bird", Blinkmoth=>"Blinkmoth", Boar=>"Boar", Cat=>"Cat",
        Centaur=>"Centaur", Camarid=>"Camarid", Citizen=>"Citizen", Clown=>"Clown", Coward=>"Coward",
        Changeling=>"Changeling", Cleric=>"Cleric", Construct=>"Construct", Crab=>"Crab", Crocodile=>"Crocodile",
        Cyclops=>"Cyclops", Cyberman=>"Cyberman", Dalek=>"Dalek", Deserter=>"Deserter", Detective=>"Detective", Doctor=>"Doctor",
        Demon=>"Demon", Devil=>"Devil", Dinosaur=>"Dinosaur", Djinn=>"Djinn", Efreet=>"Efreet", Dog=>"Dog",
        Drone=>"Drone", Dragon=>"Dragon", Drake=>"Drake", Druid=>"Druid", Dwarf=>"Dwarf", Elder=>"Elder",
        Egg=>"Egg", Eldrazi=>"Eldrazi", Hamster=>"Hamster", Spawn=>"Spawn", Scion=>"Scion",
        Elemental=>"Elemental", Elephant=>"Elephant", Elk=>"Elk", Elf=>"Elf", Employee=>"Employee",
        Eye=>"Eye", Faerie=>"Faerie", Fish=>"Fish", Fox=>"Fox", Fractal=>"Fractal", Frog=>"Frog",
        Fungus=>"Fungus", Gamer=>"Gamer", Gargoyle=>"Gargoyle", Giant=>"Giant", Gnome=>"Gnome",
        Glimmer=>"Glimmer", Goat=>"Goat", Goblin=>"Goblin", God=>"God", Golem=>"Golem", Gorgon=>"Gorgon",
        Gremlin=>"Gremlin", Germ=>"Germ", Griffin=>"Griffin", Guest=>"Guest", Hag=>"Hag", Halfling=>"Halfling",
        Harpy=>"Harpy", Hellion=>"Hellion", Hero=>"Hero", Hippo=>"Hippo", Horror=>"Horror",
        Homunculus=>"Homunculus", Horse=>"Horse", Hound=>"Hound", Human=>"Human", Hydra=>"Hydra",
        Illusion=>"Illusion", Imp=>"Imp", Insect=>"Insect", Inkling=>"Inkling", Jackal=>"Jackal",
        Jellyfish=>"Jellyfish", Kavu=>"Kavu", Kirin=>"Kirin", Kithkin=>"Kithkin", Knight=>"Knight",
        Kobold=>"Kobold", Kor=>"Kor", Kraken=>"Kraken", Leech=>"Leech", Leviathan=>"Leviathan",
        Lhurgoyf=>"Lhurgoyf", Lizard=>"Lizard", Manticore=>"Manticore", Mercenary=>"Mercenary",
        Merfolk=>"Merfolk", Minion=>"Minion", Minotaur=>"Minotaur", Mole=>"Mole", Monk=>"Monk",
        Monkey=>"Monkey", Moonfolk=>"Moonfolk", Mount=>"Mount", Mouse=>"Mouse", Mutant=>"Mutant", Myr=>"Myr",
        Naga=>"Naga", Necron=>"Necron", Nightmare=>"Nightmare", Ninja=>"Ninja", Noble=>"Noble",
        Octopus=>"Octopus", Ogre=>"Ogre", Ooze=>"Ooze", Orc=>"Orc", Otter=>"Otter", Ouphe=>"Ouphe",
        Ox=>"Ox", Oyster=>"Oyster", Peasant=>"Peasant", Performer=>"Performer", Pest=>"Pest",
        Pegasus=>"Pegasus", Phyrexian=>"Phyrexian", Phoenix=>"Phoenix", Pincher=>"Pincher", Pilot=>"Pilot",
        Pirate=>"Pirate", Plant=>"Plant", Praetor=>"Praetor", Prism=>"Prism", Raccoon=>"Raccoon",
        Rabbit=>"Rabbit", Rat=>"Rat", Ranger=>"Ranger", Reflection=>"Reflection", Rebel=>"Rebel",
        Rhino=>"Rhino", Rigger=>"Rigger", Rogue=>"Rogue", Robot=>"Robot", Salamander=>"Salamander",
        Saproling=>"Saproling", Samurai=>"Samurai", Satyr=>"Satyr", Scarecrow=>"Scarecrow",
        Scientist=>"Scientist", Scout=>"Scout", Servo=>"Servo", Serpent=>"Serpent", Shade=>"Shade",
        Shaman=>"Shaman", Shapeshifter=>"Shapeshifter", Shark=>"Shark", Sheep=>"Sheep", Skeleton=>"Skeleton",
        Slith=>"Slith", Sliver=>"Sliver", Slug=>"Slug", Snake=>"Snake", Soldier=>"Soldier", Sorcerer=>"Sorcerer",
        Spellshaper=>"Spellshaper", Sphinx=>"Sphinx", Specter=>"Specter", Spider=>"Spider", Spike=>"Spike",
        Splinter=>"Splinter", Spirit=>"Spirit", Sponge=>"Sponge", Squid=>"Squid", Squirrel=>"Squirrel",
        Starfish=>"Starfish", Surrakar=>"Surrakar", Survivor=>"Survivor", Thopter=>"Thopter", Thrull=>"Thrull",
        Tiefling=>"Tiefling", Tentacle=>"Tentacle", Toy=>"Toy", Treefolk=>"Treefolk", Triskelavite=>"Triskelavite",
        Trilobite=>"Trilobite", Troll=>"Troll", Turtle=>"Turtle", Tyranid=>"Tyranid", Unicorn=>"Unicorn",
        Utrom=>"Utrom", Vampire=>"Vampire", Vedalken=>"Vedalken", Viashino=>"Viashino", Villain=>"Villain",
        Wall=>"Wall", Warlock=>"Warlock", Warrior=>"Warrior", Weird=>"Weird", Werewolf=>"Werewolf",
        Whale=>"Whale", Wizard=>"Wizard", Wolf=>"Wolf", Wolverine=>"Wolverine", Wombat=>"Wombat",
        Worm=>"Worm", Wraith=>"Wraith", Wurm=>"Wurm", Yeti=>"Yeti", Zombie=>"Zombie", Zubera=>"Zubera",
        Armadillo=>"Armadillo", AssemblyWorker=>"Assembly-Worker", Azra=>"Azra", Badger=>"Badger",
        Balloon=>"Balloon", Basilisk=>"Basilisk", Beaver=>"Beaver", Beeble=>"Beeble", Beholder=>"Beholder",
        Bison=>"Bison", Bringer=>"Bringer", Brushwagg=>"Brushwagg", Ctan=>"C'tan", Camel=>"Camel",
        Caribou=>"Caribou", Capybara=>"Capybara", Carrier=>"Carrier", Child=>"Child", Chimera=>"Chimera",
        Cockatrice=>"Cockatrice", Coyote=>"Coyote", Custodes=>"Custodes", Demigod=>"Demigod",
        Dreadnought=>"Dreadnought", Drix=>"Drix", Dryad=>"Dryad", Echidna=>"Echidna", Eternal=>"Eternal",
        Ferret=>"Ferret", Flagbearer=>"Flagbearer", Gamma=>"Gamma", Giraffe=>"Giraffe", Gith=>"Gith",
        Gnoll=>"Gnoll", Graveborn=>"Graveborn", Hedgehog=>"Hedgehog", Hippogriff=>"Hippogriff",
        Homarid=>"Homarid", Hyena=>"Hyena", Incarnation=>"Incarnation", Inhuman=>"Inhuman",
        Inquisitor=>"Inquisitor", Juggernaut=>"Juggernaut", Kangaroo=>"Kangaroo", Kree=>"Kree", Lamia=>"Lamia",
        Lammasu=>"Lammasu", Lemur=>"Lemur", Licid=>"Licid", Lobster=>"Lobster", Lord=>"Lord",
        Masticore=>"Masticore", Metathran=>"Metathran", Monger=>"Monger", Mongoose=>"Mongoose",
        Moogle=>"Moogle", Mystic=>"Mystic", Nautilus=>"Nautilus", Nephilim=>"Nephilim",
        Nightstalker=>"Nightstalker", Noggle=>"Noggle", Nomad=>"Nomad", Nymph=>"Nymph", Orgg=>"Orgg",
        Pangolin=>"Pangolin", Pentavite=>"Pentavite", Phelddagrif=>"Phelddagrif", Platypus=>"Platypus",
        Porcupine=>"Porcupine", Possum=>"Possum", Primarch=>"Primarch", Processor=>"Processor", Qu=>"Qu",
        Rukh=>"Rukh", Sable=>"Sable", Sand=>"Sand", Scorpion=>"Scorpion", Sculpture=>"Sculpture", Seal=>"Seal",
        Serf=>"Serf", Shiar=>"Shi'ar", Siren=>"Siren", Skrull=>"Skrull", Skunk=>"Skunk", Sloth=>"Sloth",
        Snail=>"Snail", Soltari=>"Soltari", Spy=>"Spy", Symbiote=>"Symbiote", Synth=>"Synth",
        Thalakos=>"Thalakos", Time=>"Time", TimeLord=>"Time Lord", Varmint=>"Varmint", Volver=>"Volver",
        Walrus=>"Walrus", Weasel=>"Weasel", Llama=>"Llama",
        Plains=>"Plains", Island=>"Island", Swamp=>"Swamp", Mountain=>"Mountain", Forest=>"Forest",
        Desert=>"Desert", Urzas=>"Urza's", Cave=>"Cave", Gate=>"Gate", Locus=>"Locus", Town=>"Town",
        Lair=>"Lair", Mine=>"Mine", Planet=>"Planet", PowerPlant=>"Power-Plant", Sphere=>"Sphere", Tower=>"Tower",
        Attraction=>"Attraction", Bobblehead=>"Bobblehead", Book=>"Book", Clue=>"Clue", Contraption=>"Contraption",
        Equipment=>"Equipment", Food=>"Food", Fortification=>"Fortification", Gold=>"Gold", Incubator=>"Incubator",
        Junk=>"Junk", Lander=>"Lander", Map=>"Map", Mutagen=>"Mutagen", Treasure=>"Treasure", Vehicle=>"Vehicle",
        Blood=>"Blood", Infinity=>"Infinity", Powerstone=>"Powerstone", Spacecraft=>"Spacecraft", Stone=>"Stone",
        Vibranium=>"Vibranium", Heartwood=>"Heartwood", Aura=>"Aura", Background=>"Background", Cartouche=>"Cartouche",
        Case=>"Case", Class=>"Class", Curse=>"Curse", Room=>"Room", Role=>"Role", Rune=>"Rune", Saga=>"Saga",
        Shard=>"Shard", Shrine=>"Shrine", Plan=>"Plan", Quest=>"Quest"
    }
}

pub fn subtype_derived_token_name(subtypes: &[Subtype]) -> Option<String> {
    let mut words = Vec::new();
    for subtype in subtypes { words.push(token_subtype_rules_word(*subtype)?); }
    words.push("Token");
    Some(words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_word_table_covers_the_existing_token_subtype_vocabularies() {
        for subtype in Subtype::all_creature_types().iter().chain(Subtype::all_land_types())
            .chain(Subtype::all_artifact_types()).chain(Subtype::all_enchantment_types())
        { assert!(token_subtype_rules_word(*subtype).is_some()); }
        assert_eq!(subtype_derived_token_name(&[Subtype::Dwarf, Subtype::Berserker]).as_deref(), Some("Dwarf Berserker Token"));
        assert_eq!(subtype_derived_token_name(&[Subtype::AssemblyWorker, Subtype::TimeLord, Subtype::Ctan, Subtype::Shiar]).as_deref(),
            Some("Assembly-Worker Time Lord C'tan Shi'ar Token"));
        assert_eq!(subtype_derived_token_name(&[]).as_deref(), Some("Token"));
        assert!(subtype_derived_token_name(&[Subtype::Ajani]).is_none(), "unmodeled token subtype vocabulary is explicit");
    }
    #[test]
    fn authored_and_implied_occurrences_are_distinct_and_inventory_must_be_complete() {
        let authored = TokenTextRoles::authored(TokenNameTextRole::SubtypeDerived, 2);
        let implied = TokenTextRoles::rules_implied(TokenNameTextRole::Explicit, 2);
        assert_ne!(authored, implied);
        assert!(authored.has_complete_ability_inventory(2));
        assert!(!authored.has_complete_ability_inventory(1));
        assert!(!authored.has_complete_ability_inventory(3));
    }
    #[test]
    fn omitted_roles_preserve_historical_native_debug_identity_input() {
        let legacy = crate::CreateTokenEffect::one(());
        let expected = "CreateTokenEffect { token: (), count: Fixed(1), controller: You, controller_target: None, use_source_chosen_color: false, use_source_chosen_creature_type: false, actor_surface_explicit: false, suppress_aura_attachment_choice: false, ability_presentation: None, enters_tapped: false, enters_attacking: false, attack_target_mode: None, enters_blocking: None, exile_at_end_of_combat: false, sacrifice_at_end_of_combat: false, sacrifice_at_next_end_step: false, exile_at_next_end_step: false, next_end_step_player: Any, link_source_exiled_this_resolution: false }";
        assert_eq!(format!("{legacy:?}"), expected);
        let current = legacy.with_text_roles(TokenTextRoles::authored(TokenNameTextRole::Explicit, 0));
        assert!(format!("{current:?}").contains("text_roles: Some("));
    }
    #[cfg(feature = "serde")]
    #[test]
    fn historical_token_payload_omission_remains_unknown_and_new_roles_round_trip() {
        let legacy = crate::CreateTokenEffect::one(());
        let wire = serde_json::to_value(&legacy).unwrap();
        assert!(wire.get("text_roles").is_none());
        let restored: crate::CreateTokenEffect<()> = serde_json::from_value(wire).unwrap();
        assert!(restored.text_roles.is_none());
        let current = legacy.with_text_roles(TokenTextRoles::authored(TokenNameTextRole::SubtypeDerived, 2));
        let restored: crate::CreateTokenEffect<()> = serde_json::from_value(serde_json::to_value(&current).unwrap()).unwrap();
        assert_eq!(restored.text_roles, current.text_roles);
    }
}
