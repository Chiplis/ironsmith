use super::super::activation_and_restrictions::{contains_word_sequence, parse_named_number};
use super::super::grammar::primitives as grammar;
use super::super::grammar::values as shared_values;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::token_primitives::{
    contains_window, find_index, find_window_index, rfind_index, slice_contains, slice_ends_with,
    slice_starts_with, slice_strip_prefix, str_strip_prefix, str_strip_suffix,
};
use super::super::util::{
    is_article, is_permanent_type, is_source_reference_words, parse_card_type,
    parse_counter_type_word, parse_mana_symbol_word_flexible, parse_number, parse_target_phrase,
    parse_zone_word, span_from_tokens, token_index_for_word_index, trim_commas, words,
};
use super::super::value_helpers::parse_filter_comparison_tokens;
use super::{parse_effect_chain, parse_effect_chain_inner, parse_effect_chain_lexed};
use crate::card::{PowerToughness, PtValue};
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
        _ => None,
    }
}

pub(crate) fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    let trimmed = raw.trim();
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let power = parse_pt_value(parts[0].trim())?;
    let toughness = parse_pt_value(parts[1].trim())?;
    Some(PowerToughness::new(power, toughness))
}

pub(crate) fn parse_pt_value(raw: &str) -> Option<PtValue> {
    if raw == ".5" || raw == "0.5" {
        return Some(PtValue::Fixed(0));
    }
    if raw == "*" {
        return Some(PtValue::Star);
    }
    if let Some(stripped) = str_strip_prefix(raw, "*+") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Some(stripped) = str_strip_suffix(raw, "+*") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Ok(value) = raw.parse::<i32>() {
        return Some(PtValue::Fixed(value));
    }
    None
}

pub(crate) fn parse_for_each_opponent_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = crate::cards::builders::parser::token_word_refs(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = crate::cards::builders::parser::token_word_refs(clause_tokens);
    }
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let start = if grammar::words_match_prefix(clause_tokens, &["for", "each", "opponent"])
        .is_some()
        || grammar::words_match_prefix(clause_tokens, &["for", "each", "opponents"]).is_some()
    {
        3
    } else if grammar::words_match_prefix(clause_tokens, &["each", "opponent"]).is_some()
        || grammar::words_match_prefix(clause_tokens, &["each", "opponents"]).is_some()
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    let inner_words = crate::cards::builders::parser::token_word_refs(&inner_tokens);
    let starts_with_who = inner_words.first().copied() == Some("who");
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    if !starts_with_who {
        return Ok(None);
    }

    let effect_token_start = if let Some(comma_idx) =
        find_index(&inner_tokens, |token| token.is_comma())
    {
        comma_idx + 1
    } else if let Some(this_way_idx) = find_window_index(&inner_words, &["this", "way"]) {
        token_index_for_word_index(&inner_tokens, this_way_idx + 2).unwrap_or(inner_tokens.len())
    } else {
        token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
            .unwrap_or(inner_tokens.len())
    };
    let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each opponent who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(&effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot {
        effects,
        predicate,
    }))
}

pub(crate) fn parse_for_each_player_doesnt(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = crate::cards::builders::parser::token_word_refs(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = crate::cards::builders::parser::token_word_refs(clause_tokens);
    }
    if clause_words.len() < 5 {
        return Ok(None);
    }

    let start = if grammar::words_match_prefix(clause_tokens, &["for", "each", "player"]).is_some()
        || grammar::words_match_prefix(clause_tokens, &["for", "each", "players"]).is_some()
    {
        3
    } else if grammar::words_match_prefix(clause_tokens, &["each", "player"]).is_some()
        || grammar::words_match_prefix(clause_tokens, &["each", "players"]).is_some()
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    let inner_words = crate::cards::builders::parser::token_word_refs(&inner_tokens);
    let starts_with_who = inner_words.first().copied() == Some("who");
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    if !starts_with_who {
        return Ok(None);
    }

    let effect_token_start = if let Some(comma_idx) =
        find_index(&inner_tokens, |token| token.is_comma())
    {
        comma_idx + 1
    } else if let Some(this_way_idx) = find_window_index(&inner_words, &["this", "way"]) {
        token_index_for_word_index(&inner_tokens, this_way_idx + 2).unwrap_or(inner_tokens.len())
    } else {
        token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
            .unwrap_or(inner_tokens.len())
    };

    let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each player who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain_inner(&effect_tokens)?;
    let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
    Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }))
}

pub(crate) fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    if let Some(idx) = find_index(words, |word| {
        matches!(*word, "doesnt" | "didnt" | "doesn't" | "didn't")
    }) {
        return Some((idx, 1));
    }
    if let Some(idx) = find_window_index(words, &["do", "not"]) {
        return Some((idx, 2));
    }
    if let Some(idx) = find_window_index(words, &["did", "not"]) {
        return Some((idx, 2));
    }
    None
}

fn parse_negated_who_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let inner_words = crate::cards::builders::parser::token_word_refs(inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }
    let Some(this_way_idx) = find_window_index(&inner_words, &["this", "way"]) else {
        return Ok(None);
    };
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !matches!(verb, "discard" | "discarded") || this_way_idx <= verb_idx + 1 {
        return Ok(None);
    }

    let filter_start =
        token_index_for_word_index(inner_tokens, verb_idx + 1).unwrap_or(inner_tokens.len());
    let filter_end =
        token_index_for_word_index(inner_tokens, this_way_idx).unwrap_or(inner_tokens.len());
    if filter_start >= filter_end {
        return Ok(None);
    }

    let filter_tokens = trim_commas(&inner_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

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

pub(crate) fn parse_vote_start_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let vote_idx = find_index(&clause_words, |word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };

    let has_each = slice_contains(&clause_words[..vote_idx], &"each");
    let has_player = clause_words[..vote_idx]
        .iter()
        .any(|word| *word == "player" || *word == "players");
    if !has_each || !has_player {
        return Ok(None);
    }

    let for_idx = find_index(&clause_words, |word| *word == "for")
        .ok_or_else(|| CardTextError::ParseError("missing 'for' in vote clause".to_string()))?;
    if for_idx < vote_idx {
        return Ok(None);
    }

    let mut option_words = clause_words[for_idx + 1..].to_vec();
    if let Some(reveal_idx) = find_window_index(&option_words, &["then", "those", "votes", "are"]) {
        option_words.truncate(reveal_idx);
    }
    let option_tokens = option_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if let Ok(target) = parse_target_phrase(&option_tokens) {
        match target {
            TargetAst::Object(filter, _, _) => {
                return Ok(Some(EffectAst::VoteStartObjects {
                    filter,
                    count: ChoiceCount::exactly(1),
                }));
            }
            TargetAst::WithCount(inner, count) => {
                if let TargetAst::Object(filter, _, _) = *inner {
                    return Ok(Some(EffectAst::VoteStartObjects { filter, count }));
                }
            }
            _ => {}
        }
    }
    if let Ok(filter) = parse_object_filter_lexed(&option_tokens, false)
        && filter != ObjectFilter::default()
    {
        return Ok(Some(EffectAst::VoteStartObjects {
            filter,
            count: ChoiceCount::exactly(1),
        }));
    }

    let option_words = option_words;
    let mut options = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in &option_words {
        if *word == "or" {
            if !current.is_empty() {
                options.push(current.join(" "));
                current.clear();
            }
            continue;
        }
        if is_article(word) {
            continue;
        }
        current.push(word);
    }
    if !current.is_empty() {
        options.push(current.join(" "));
    }

    if options.len() < 2 {
        return Err(CardTextError::ParseError(
            "vote clause requires at least two options".to_string(),
        ));
    }

    Ok(Some(EffectAst::VoteStart { options }))
}

pub(crate) fn parse_for_each_vote_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    if grammar::words_match_prefix(tokens, &["for", "each"]).is_none() {
        return Ok(None);
    }

    let vote_idx = find_index(&words, |word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };
    if vote_idx <= 2 {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }

    let option_words: Vec<&str> = words[2..vote_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if option_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }
    let option = option_words.join(" ");

    let (_before, effect_tokens) =
        super::super::grammar::primitives::split_lexed_once_on_delimiter(
            tokens,
            super::super::lexer::TokenKind::Comma,
        )
        .ok_or_else(|| {
            CardTextError::ParseError("missing comma in for each vote clause".to_string())
        })?;

    let effects = parse_effect_chain(effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}

pub(crate) fn parse_vote_extra_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.len() < 3 || words.first().copied() != Some("you") {
        return None;
    }

    let has_vote = words.iter().any(|word| *word == "vote" || *word == "votes");
    let has_additional = grammar::contains_word(tokens, "additional");
    let has_time = words.iter().any(|word| *word == "time" || *word == "times");
    if !has_vote || !has_additional || !has_time {
        return None;
    }

    let optional = grammar::contains_word(tokens, "may");
    Some(EffectAst::VoteExtra { count: 1, optional })
}

pub(crate) fn parse_after_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = crate::cards::builders::parser::token_word_refs(tokens);
    if line_words.len() < 3
        || line_words[0] != "after"
        || line_words[1] != "that"
        || line_words[2] != "turn"
    {
        return Ok(None);
    }

    let remainder = if let Some((_before, after)) =
        super::super::grammar::primitives::split_lexed_once_on_delimiter(
            tokens,
            super::super::lexer::TokenKind::Comma,
        ) {
        after
    } else {
        &tokens[3..]
    };

    let remaining_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if remaining_words.len() < 4 {
        return Err(CardTextError::ParseError(
            "unsupported after turn clause".to_string(),
        ));
    }

    let player = if slice_starts_with(&remaining_words, &["that", "player"]) {
        PlayerAst::That
    } else if slice_starts_with(&remaining_words, &["target", "player"]) {
        PlayerAst::Target
    } else if slice_starts_with(&remaining_words, &["you"]) {
        PlayerAst::You
    } else {
        return Err(CardTextError::ParseError(
            "unsupported after turn player".to_string(),
        ));
    };

    if slice_contains(&remaining_words, &"extra") && slice_contains(&remaining_words, &"turn") {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player,
            anchor: ExtraTurnAnchorAst::ReferencedTurn,
        }));
    }

    Err(CardTextError::ParseError(
        "unsupported after turn clause".to_string(),
    ))
}

pub(crate) fn parse_sentence_counter_target_spell_if_it_was_kicked(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.as_slice() != ["counter", "target", "spell", "if", "it", "was", "kicked"] {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(&tokens[1..3]));
    let counter = EffectAst::Counter { target };
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
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
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
    let counter = EffectAst::Counter { target };
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
    let is_shape = grammar::words_match_prefix(tokens, &["exile", "target", "creature"]).is_some()
        && grammar::words_find_phrase(tokens, &["greatest", "power", "among", "creatures"])
            .is_some()
        && (grammar::words_find_phrase(tokens, &["on", "battlefield"]).is_some()
            || grammar::words_find_phrase(tokens, &["on", "the", "battlefield"]).is_some());
    if !is_shape {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..3]);
    let target = parse_target_phrase(&target_tokens)?;
    let exile = EffectAst::Exile {
        target: target.clone(),
        face_down: false,
    };
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetHasGreatestPowerAmongCreatures,
        if_true: vec![exile],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}
