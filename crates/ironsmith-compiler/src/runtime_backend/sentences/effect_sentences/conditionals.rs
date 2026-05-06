use super::super::activation_and_restrictions::activated_line_core::{
    contains_word_sequence, find_word_sequence_start, parse_named_number,
};
use super::super::grammar::effects as effect_grammar;
use super::super::grammar::effects::{
    split_for_each_opponent_doesnt_clause_lexed, split_for_each_player_doesnt_clause_lexed,
    split_negated_who_this_way_filter_tokens_lexed,
};
use super::super::grammar::primitives as grammar;
use super::super::grammar::values as shared_values;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::token_primitives::{
    find_index, rfind_index, slice_contains, slice_ends_with, slice_starts_with,
    slice_strip_prefix, str_strip_prefix, str_strip_suffix,
};
use super::super::util::{
    is_article, is_permanent_type, is_source_reference_words, parse_card_type,
    parse_counter_type_word, parse_mana_symbol_word_flexible, parse_number, parse_target_phrase,
    parse_zone_word, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::{parse_effect_chain, parse_effect_chain_inner, parse_effect_chain_lexed};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, IT_TAG, IfResultPredicate, PlayerAst,
    PredicateAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Value};
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

#[cfg(test)]
pub(crate) fn parse_conditional_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    super::super::grammar::effects::parse_conditional_sentence_with_grammar_entrypoint_lexed(
        tokens,
        parse_effect_chain_lexed,
    )
}

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    shared_values::parse_scryfall_mana_cost(raw)
}

pub(crate) fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    shared_values::parse_mana_symbol_group(raw)
}

pub(crate) fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    shared_values::parse_mana_symbol(part)
}

pub(crate) fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    shared_values::parse_type_line_with(
        raw,
        parse_supertype_word,
        |word| parse_card_type(&word.to_ascii_lowercase()),
        parse_subtype_word,
    )
}

pub(crate) fn parse_supertype_word(word: &str) -> Option<Supertype> {
    match word.to_ascii_lowercase().as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

pub(crate) fn parse_subtype_word(word: &str) -> Option<Subtype> {
    match word.to_ascii_lowercase().as_str() {
        "plains" => Some(Subtype::Plains),
        "island" => Some(Subtype::Island),
        "swamp" => Some(Subtype::Swamp),
        "mountain" => Some(Subtype::Mountain),
        "forest" => Some(Subtype::Forest),
        "desert" | "deserts" => Some(Subtype::Desert),
        "urzas" => Some(Subtype::Urzas),
        "cave" | "caves" => Some(Subtype::Cave),
        "gate" | "gates" => Some(Subtype::Gate),
        "locus" | "loci" => Some(Subtype::Locus),
        "advisor" => Some(Subtype::Advisor),
        "ally" | "allies" => Some(Subtype::Ally),
        "alien" | "aliens" => Some(Subtype::Alien),
        "angel" => Some(Subtype::Angel),
        "ape" => Some(Subtype::Ape),
        "army" | "armies" => Some(Subtype::Army),
        "archer" => Some(Subtype::Archer),
        "artificer" => Some(Subtype::Artificer),
        "assassin" => Some(Subtype::Assassin),
        "astartes" => Some(Subtype::Astartes),
        "avatar" => Some(Subtype::Avatar),
        "barbarian" => Some(Subtype::Barbarian),
        "bard" => Some(Subtype::Bard),
        "bat" | "bats" => Some(Subtype::Bat),
        "bear" => Some(Subtype::Bear),
        "beast" => Some(Subtype::Beast),
        "berserker" => Some(Subtype::Berserker),
        "bird" => Some(Subtype::Bird),
        "boar" => Some(Subtype::Boar),
        "cat" => Some(Subtype::Cat),
        "centaur" => Some(Subtype::Centaur),
        "citizen" | "citizens" => Some(Subtype::Citizen),
        "coward" | "cowards" => Some(Subtype::Coward),
        "changeling" => Some(Subtype::Changeling),
        "cleric" => Some(Subtype::Cleric),
        "construct" => Some(Subtype::Construct),
        "crab" => Some(Subtype::Crab),
        "crocodile" => Some(Subtype::Crocodile),
        "dalek" => Some(Subtype::Dalek),
        "dauthi" => Some(Subtype::Dauthi),
        "detective" => Some(Subtype::Detective),
        "doctor" | "doctors" => Some(Subtype::Doctor),
        "demon" => Some(Subtype::Demon),
        "devil" => Some(Subtype::Devil),
        "dinosaur" => Some(Subtype::Dinosaur),
        "djinn" => Some(Subtype::Djinn),
        "efreet" | "efreets" => Some(Subtype::Efreet),
        "dog" => Some(Subtype::Dog),
        "drone" | "drones" => Some(Subtype::Drone),
        "dragon" => Some(Subtype::Dragon),
        "drake" => Some(Subtype::Drake),
        "druid" => Some(Subtype::Druid),
        "dwarf" => Some(Subtype::Dwarf),
        "elder" => Some(Subtype::Elder),
        "eldrazi" => Some(Subtype::Eldrazi),
        "hamster" | "hamsters" => Some(Subtype::Hamster),
        "spawn" | "spawns" => Some(Subtype::Spawn),
        "scion" | "scions" => Some(Subtype::Scion),
        "elemental" => Some(Subtype::Elemental),
        "elephant" => Some(Subtype::Elephant),
        "elk" => Some(Subtype::Elk),
        "elf" | "elves" => Some(Subtype::Elf),
        "faerie" => Some(Subtype::Faerie),
        "fish" => Some(Subtype::Fish),
        "fox" => Some(Subtype::Fox),
        "frog" => Some(Subtype::Frog),
        "fungus" => Some(Subtype::Fungus),
        "gargoyle" => Some(Subtype::Gargoyle),
        "giant" => Some(Subtype::Giant),
        "gnome" => Some(Subtype::Gnome),
        "glimmer" | "glimmers" => Some(Subtype::Glimmer),
        "goat" => Some(Subtype::Goat),
        "goblin" => Some(Subtype::Goblin),
        "god" => Some(Subtype::God),
        "golem" => Some(Subtype::Golem),
        "gorgon" => Some(Subtype::Gorgon),
        "germ" | "germs" => Some(Subtype::Germ),
        "gremlin" | "gremlins" => Some(Subtype::Gremlin),
        "griffin" => Some(Subtype::Griffin),
        "hag" => Some(Subtype::Hag),
        "halfling" => Some(Subtype::Halfling),
        "harpy" => Some(Subtype::Harpy),
        "hippo" => Some(Subtype::Hippo),
        "horror" => Some(Subtype::Horror),
        "homunculus" | "homunculi" => Some(Subtype::Homunculus),
        "horse" => Some(Subtype::Horse),
        "hound" => Some(Subtype::Hound),
        "human" => Some(Subtype::Human),
        "hydra" => Some(Subtype::Hydra),
        "illusion" => Some(Subtype::Illusion),
        "imp" => Some(Subtype::Imp),
        "insect" => Some(Subtype::Insect),
        "inkling" | "inklings" => Some(Subtype::Inkling),
        "jackal" | "jackals" => Some(Subtype::Jackal),
        "jellyfish" => Some(Subtype::Jellyfish),
        "kavu" => Some(Subtype::Kavu),
        "kirin" => Some(Subtype::Kirin),
        "kithkin" => Some(Subtype::Kithkin),
        "knight" => Some(Subtype::Knight),
        "kobold" => Some(Subtype::Kobold),
        "kor" => Some(Subtype::Kor),
        "kraken" => Some(Subtype::Kraken),
        "leviathan" => Some(Subtype::Leviathan),
        "lizard" => Some(Subtype::Lizard),
        "manticore" => Some(Subtype::Manticore),
        "mercenary" => Some(Subtype::Mercenary),
        "merfolk" => Some(Subtype::Merfolk),
        "minion" => Some(Subtype::Minion),
        "mite" | "mites" => Some(Subtype::Mite),
        "minotaur" => Some(Subtype::Minotaur),
        "mole" => Some(Subtype::Mole),
        "monk" => Some(Subtype::Monk),
        "monkey" | "monkeys" => Some(Subtype::Monkey),
        "moonfolk" => Some(Subtype::Moonfolk),
        "mount" | "mounts" => Some(Subtype::Mount),
        "mouse" | "mice" => Some(Subtype::Mouse),
        "mutant" => Some(Subtype::Mutant),
        "myr" => Some(Subtype::Myr),
        "naga" => Some(Subtype::Naga),
        "necron" | "necrons" => Some(Subtype::Necron),
        "nightmare" => Some(Subtype::Nightmare),
        "ninja" => Some(Subtype::Ninja),
        "noble" => Some(Subtype::Noble),
        "octopus" | "octopuses" => Some(Subtype::Octopus),
        "ogre" => Some(Subtype::Ogre),
        "ooze" => Some(Subtype::Ooze),
        "orc" => Some(Subtype::Orc),
        "otter" => Some(Subtype::Otter),
        "ouphe" | "ouphes" => Some(Subtype::Ouphe),
        "ox" => Some(Subtype::Ox),
        "oyster" => Some(Subtype::Oyster),
        "peasant" => Some(Subtype::Peasant),
        "pest" => Some(Subtype::Pest),
        "pegasus" => Some(Subtype::Pegasus),
        "phyrexian" => Some(Subtype::Phyrexian),
        "phoenix" => Some(Subtype::Phoenix),
        "pincher" | "pinchers" => Some(Subtype::Pincher),
        "pilot" => Some(Subtype::Pilot),
        "pirate" => Some(Subtype::Pirate),
        "plant" => Some(Subtype::Plant),
        "praetor" => Some(Subtype::Praetor),
        "raccoon" => Some(Subtype::Raccoon),
        "rabbit" => Some(Subtype::Rabbit),
        "rat" => Some(Subtype::Rat),
        "reflection" => Some(Subtype::Reflection),
        "rebel" => Some(Subtype::Rebel),
        "rhino" => Some(Subtype::Rhino),
        "rogue" => Some(Subtype::Rogue),
        "robot" => Some(Subtype::Robot),
        "salamander" => Some(Subtype::Salamander),
        "saproling" | "saprolings" => Some(Subtype::Saproling),
        "samurai" => Some(Subtype::Samurai),
        "satyr" => Some(Subtype::Satyr),
        "scarecrow" => Some(Subtype::Scarecrow),
        "scout" => Some(Subtype::Scout),
        "servo" | "servos" => Some(Subtype::Servo),
        "serpent" => Some(Subtype::Serpent),
        "shade" => Some(Subtype::Shade),
        "shaman" => Some(Subtype::Shaman),
        "shapeshifter" => Some(Subtype::Shapeshifter),
        "shark" => Some(Subtype::Shark),
        "sheep" => Some(Subtype::Sheep),
        "skeleton" => Some(Subtype::Skeleton),
        "slith" => Some(Subtype::Slith),
        "sliver" => Some(Subtype::Sliver),
        "slug" => Some(Subtype::Slug),
        "snake" => Some(Subtype::Snake),
        "soldier" => Some(Subtype::Soldier),
        "sorcerer" => Some(Subtype::Sorcerer),
        "spacecraft" => Some(Subtype::Spacecraft),
        "sphinx" => Some(Subtype::Sphinx),
        "specter" => Some(Subtype::Specter),
        "spider" => Some(Subtype::Spider),
        "spike" => Some(Subtype::Spike),
        "splinter" | "splinters" => Some(Subtype::Splinter),
        "spirit" => Some(Subtype::Spirit),
        "sponge" => Some(Subtype::Sponge),
        "squid" => Some(Subtype::Squid),
        "squirrel" => Some(Subtype::Squirrel),
        "starfish" => Some(Subtype::Starfish),
        "surrakar" => Some(Subtype::Surrakar),
        "survivor" | "survivors" => Some(Subtype::Survivor),
        "thopter" => Some(Subtype::Thopter),
        "thrull" => Some(Subtype::Thrull),
        "tiefling" => Some(Subtype::Tiefling),
        "tentacle" | "tentacles" => Some(Subtype::Tentacle),
        "toy" => Some(Subtype::Toy),
        "treefolk" => Some(Subtype::Treefolk),
        "triskelavite" | "triskelavites" => Some(Subtype::Triskelavite),
        "trilobite" => Some(Subtype::Trilobite),
        "troll" => Some(Subtype::Troll),
        "turtle" => Some(Subtype::Turtle),
        "unicorn" => Some(Subtype::Unicorn),
        "vampire" => Some(Subtype::Vampire),
        "vedalken" => Some(Subtype::Vedalken),
        "viashino" => Some(Subtype::Viashino),
        "villain" | "villains" => Some(Subtype::Villain),
        "wall" => Some(Subtype::Wall),
        "warlock" => Some(Subtype::Warlock),
        "warrior" => Some(Subtype::Warrior),
        "weird" => Some(Subtype::Weird),
        "werewolf" | "werewolves" => Some(Subtype::Werewolf),
        "whale" => Some(Subtype::Whale),
        "wizard" => Some(Subtype::Wizard),
        "wolf" | "wolves" => Some(Subtype::Wolf),
        "wolverine" => Some(Subtype::Wolverine),
        "wombat" => Some(Subtype::Wombat),
        "worm" => Some(Subtype::Worm),
        "wraith" => Some(Subtype::Wraith),
        "wurm" => Some(Subtype::Wurm),
        "yeti" => Some(Subtype::Yeti),
        "zombie" => Some(Subtype::Zombie),
        "zubera" => Some(Subtype::Zubera),
        "clue" => Some(Subtype::Clue),
        "contraption" => Some(Subtype::Contraption),
        "equipment" => Some(Subtype::Equipment),
        "food" => Some(Subtype::Food),
        "fortification" => Some(Subtype::Fortification),
        "gold" => Some(Subtype::Gold),
        "incubator" | "incubators" => Some(Subtype::Incubator),
        "junk" | "junks" => Some(Subtype::Junk),
        "lander" | "landers" => Some(Subtype::Lander),
        "map" | "maps" => Some(Subtype::Map),
        "treasure" => Some(Subtype::Treasure),
        "vehicle" => Some(Subtype::Vehicle),
        "aura" => Some(Subtype::Aura),
        "background" => Some(Subtype::Background),
        "cartouche" => Some(Subtype::Cartouche),
        "class" => Some(Subtype::Class),
        "curse" => Some(Subtype::Curse),
        "role" => Some(Subtype::Role),
        "rune" => Some(Subtype::Rune),
        "saga" => Some(Subtype::Saga),
        "shard" => Some(Subtype::Shard),
        "shrine" => Some(Subtype::Shrine),
        "adventure" => Some(Subtype::Adventure),
        "arcane" => Some(Subtype::Arcane),
        "lesson" => Some(Subtype::Lesson),
        "trap" => Some(Subtype::Trap),
        "ajani" => Some(Subtype::Ajani),
        "ashiok" => Some(Subtype::Ashiok),
        "chandra" => Some(Subtype::Chandra),
        "elspeth" => Some(Subtype::Elspeth),
        "garruk" => Some(Subtype::Garruk),
        "gideon" => Some(Subtype::Gideon),
        "jace" => Some(Subtype::Jace),
        "karn" => Some(Subtype::Karn),
        "liliana" => Some(Subtype::Liliana),
        "nissa" => Some(Subtype::Nissa),
        "sorin" => Some(Subtype::Sorin),
        "teferi" => Some(Subtype::Teferi),
        "tyvar" => Some(Subtype::Tyvar),
        "ugin" => Some(Subtype::Ugin),
        "vraska" => Some(Subtype::Vraska),
        "siege" | "sieges" => Some(Subtype::Siege),
        _ => None,
    }
}

pub(crate) fn parse_for_each_opponent_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(split) = split_for_each_opponent_doesnt_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each opponent who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(split.effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(split.inner_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot {
        effects,
        predicate,
    }))
}

pub(crate) fn parse_for_each_player_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(split) = split_for_each_player_doesnt_clause_lexed(tokens) else {
        return Ok(None);
    };
    if split.effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each player who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(split.effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(split.inner_tokens)?;
    Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }))
}

pub(crate) fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    effect_grammar::negated_action_word_index(words)
}

fn parse_negated_who_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(filter_tokens) = split_negated_who_this_way_filter_tokens_lexed(inner_tokens) else {
        return Ok(None);
    };

    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

pub(crate) fn parse_sentence_counter_target_spell_if_it_was_kicked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.as_slice() != ["counter", "target", "spell", "if", "it", "was", "kicked"] {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(&tokens[1..3]));
    let counter = EffectAst::subject_verb_counter(target);
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetWasKicked,
        if_true: vec![counter],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_sentence_counter_target_spell_thats_second_cast_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let matches = clause_words.as_slice()
        == [
            "counter", "target", "spell", "thats", "second", "spell", "cast", "this", "turn",
        ]
        || clause_words.as_slice()
            == [
                "counter", "target", "spell", "thats", "the", "second", "spell", "cast", "this",
                "turn",
            ];
    if !matches {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(&tokens[1..3]));
    let counter = EffectAst::subject_verb_counter(target);
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetSpellCastOrderThisTurn(2),
        if_true: vec![counter],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_sentence_exile_target_creature_with_greatest_power(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let is_shape = grammar::words_match_any_prefix(tokens, &[&["exile", "target", "creature"]])
        .is_some()
        && grammar::words_find_phrase(tokens, &["greatest", "power", "among", "creatures"])
            .is_some()
        && (grammar::words_find_phrase(tokens, &["on", "battlefield"]).is_some()
            || grammar::words_find_phrase(tokens, &["on", "the", "battlefield"]).is_some());
    if !is_shape {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..3]);
    let target = parse_target_phrase(&target_tokens)?;
    let exile = EffectAst::subject_verb_exile(target.clone(), false);
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetHasGreatestPowerAmongCreatures,
        if_true: vec![exile],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}
