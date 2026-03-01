use super::*;

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return Ok(ManaCost::new());
    }

    let mut pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut current = String::new();
    let mut in_brace = false;

    for ch in trimmed.chars() {
        if ch == '{' {
            in_brace = true;
            current.clear();
            continue;
        }
        if ch == '}' {
            if !in_brace {
                continue;
            }
            in_brace = false;
            if current.is_empty() {
                continue;
            }
            let alternatives = parse_mana_symbol_group(&current)?;
            if !alternatives.is_empty() {
                pips.push(alternatives);
            }
            continue;
        }
        if in_brace {
            current.push(ch);
        }
    }

    Ok(ManaCost::from_pips(pips))
}

pub(crate) fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let mut alternatives = Vec::new();
    for part in raw.split('/') {
        let symbol = parse_mana_symbol(part)?;
        alternatives.push(symbol);
    }
    Ok(alternatives)
}

pub(crate) fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    let upper = part.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return Err(CardTextError::ParseError("empty mana symbol".to_string()));
    }

    if upper.chars().all(|c| c.is_ascii_digit()) {
        let value = upper.parse::<u8>().map_err(|_| {
            CardTextError::ParseError(format!("invalid generic mana symbol '{part}'"))
        })?;
        return Ok(ManaSymbol::Generic(value));
    }

    match upper.as_str() {
        "W" => Ok(ManaSymbol::White),
        "U" => Ok(ManaSymbol::Blue),
        "B" => Ok(ManaSymbol::Black),
        "R" => Ok(ManaSymbol::Red),
        "G" => Ok(ManaSymbol::Green),
        "C" => Ok(ManaSymbol::Colorless),
        "S" => Ok(ManaSymbol::Snow),
        "X" => Ok(ManaSymbol::X),
        "P" => Ok(ManaSymbol::Life(2)),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported mana symbol '{part}'"
        ))),
    }
}

pub(crate) fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();

    let parts: Vec<&str> = raw.split('—').collect();
    let left = parts[0].trim();
    let right = parts.get(1).map(|s| s.trim());

    for word in left.split_whitespace() {
        if let Some(supertype) = parse_supertype_word(word) {
            supertypes.push(supertype);
            continue;
        }
        if let Some(card_type) = parse_card_type(&word.to_ascii_lowercase()) {
            card_types.push(card_type);
        }
    }

    if let Some(right) = right {
        for word in right.split_whitespace() {
            if let Some(subtype) = parse_subtype_word(word) {
                subtypes.push(subtype);
            }
        }
    }

    Ok((supertypes, card_types, subtypes))
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
        "wolf" => Some(Subtype::Wolf),
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
    if let Some(stripped) = raw.strip_prefix("*+") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Some(stripped) = raw.strip_suffix("+*") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Ok(value) = raw.parse::<i32>() {
        return Some(PtValue::Fixed(value));
    }
    None
}

pub(crate) fn parse_for_each_opponent_doesnt(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "opponent"])
        || clause_words.starts_with(&["for", "each", "opponents"])
    {
        3
    } else if clause_words.starts_with(&["each", "opponent"])
        || clause_words.starts_with(&["each", "opponents"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    let inner_words = words(&inner_tokens);
    let starts_with_who = inner_words.first().copied() == Some("who");
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    if !starts_with_who {
        return Ok(None);
    }

    let effect_token_start = if let Some(comma_idx) = inner_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    {
        comma_idx + 1
    } else if let Some(this_way_idx) = inner_words
        .windows(2)
        .position(|pair| pair == ["this", "way"])
    {
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

    let effects = parse_effect_chain(&effect_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot { effects }))
}

pub(crate) fn parse_for_each_player_doesnt(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 5 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "player"])
        || clause_words.starts_with(&["for", "each", "players"])
    {
        3
    } else if clause_words.starts_with(&["each", "player"])
        || clause_words.starts_with(&["each", "players"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    let inner_words = words(&inner_tokens);
    let starts_with_who = inner_words.first().copied() == Some("who");
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    if !starts_with_who {
        return Ok(None);
    }

    let effect_token_start = if let Some(comma_idx) = inner_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    {
        comma_idx + 1
    } else if let Some(this_way_idx) = inner_words
        .windows(2)
        .position(|pair| pair == ["this", "way"])
    {
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

    let effects = parse_effect_chain(&effect_tokens)?;
    Ok(Some(EffectAst::ForEachPlayerDoesNot { effects }))
}

pub(crate) fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    if let Some(idx) = words
        .iter()
        .position(|word| *word == "doesnt" || *word == "didnt")
    {
        return Some((idx, 1));
    }
    for (idx, pair) in words.windows(2).enumerate() {
        if pair == ["do", "not"] || pair == ["did", "not"] {
            return Some((idx, 2));
        }
    }
    None
}

pub(crate) fn parse_vote_start_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    let vote_idx = words
        .iter()
        .position(|word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };

    let has_each = words[..vote_idx].contains(&"each");
    let has_player = words[..vote_idx]
        .iter()
        .any(|word| *word == "player" || *word == "players");
    if !has_each || !has_player {
        return Ok(None);
    }

    let for_idx = words
        .iter()
        .position(|word| *word == "for")
        .ok_or_else(|| CardTextError::ParseError("missing 'for' in vote clause".to_string()))?;
    if for_idx < vote_idx {
        return Ok(None);
    }

    let option_words = &words[for_idx + 1..];
    let mut options = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in option_words {
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
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    if !words.starts_with(&["for", "each"]) {
        return Ok(None);
    }

    let vote_idx = words
        .iter()
        .position(|word| *word == "vote" || *word == "votes");
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

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError("missing comma in for each vote clause".to_string())
        })?;

    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}

pub(crate) fn parse_vote_extra_sentence(tokens: &[Token]) -> Option<EffectAst> {
    let words = words(tokens);
    if words.len() < 3 || words.first().copied() != Some("you") {
        return None;
    }

    let has_vote = words.iter().any(|word| *word == "vote" || *word == "votes");
    let has_additional = words.contains(&"additional");
    let has_time = words.iter().any(|word| *word == "time" || *word == "times");
    if !has_vote || !has_additional || !has_time {
        return None;
    }

    let optional = words.contains(&"may");
    Some(EffectAst::VoteExtra { count: 1, optional })
}

pub(crate) fn parse_after_turn_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 3
        || line_words[0] != "after"
        || line_words[1] != "that"
        || line_words[2] != "turn"
    {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[3..]
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if remaining_words.len() < 4 {
        return Err(CardTextError::ParseError(
            "unsupported after turn clause".to_string(),
        ));
    }

    let player = if remaining_words.starts_with(&["that", "player"]) {
        PlayerAst::That
    } else if remaining_words.starts_with(&["target", "player"]) {
        PlayerAst::Target
    } else if remaining_words.starts_with(&["you"]) {
        PlayerAst::You
    } else {
        return Err(CardTextError::ParseError(
            "unsupported after turn player".to_string(),
        ));
    };

    if remaining_words.contains(&"extra") && remaining_words.contains(&"turn") {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn { player }));
    }

    Err(CardTextError::ParseError(
        "unsupported after turn clause".to_string(),
    ))
}

pub(crate) fn parse_conditional_sentence(
    tokens: &[Token],
) -> Result<Vec<EffectAst>, CardTextError> {
    let comma_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| matches!(token, Token::Comma(_)).then_some(idx))
        .collect::<Vec<_>>();
    if comma_indices.is_empty() {
        return Err(CardTextError::ParseError(
            "missing comma in if clause".to_string(),
        ));
    }

    // For result predicates ("if you do, ..."), always split at the first comma.
    // The effect tail frequently contains additional commas (search/reveal/put, etc.)
    // that should stay in the true branch.
    let first_comma_idx = comma_indices[0];
    if first_comma_idx > 1 {
        let predicate_tokens = &tokens[1..first_comma_idx];
        if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let effects = parse_effect_chain(effect_tokens)?;
            return Ok(vec![EffectAst::IfResult { predicate, effects }]);
        }
        if let Ok(predicate) = parse_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let comma_fragment_looks_like_effect = if comma_indices.len() > 1 {
                let fragment_tokens = &tokens[first_comma_idx + 1..comma_indices[1]];
                parse_effect_chain(fragment_tokens)
                    .map(|effects| !effects.is_empty())
                    .unwrap_or(false)
            } else {
                true
            };
            if comma_fragment_looks_like_effect
                && let Ok(effects) = parse_effect_chain(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(vec![EffectAst::Conditional {
                    predicate,
                    if_true: effects,
                    if_false: Vec::new(),
                }]);
            }
        }
    }

    // Prefer the rightmost comma that yields a parseable effect clause so
    // predicates like "if it's an artifact, creature, enchantment, or land card,"
    // keep their internal comma-separated type list intact.
    let mut split: Option<(usize, Vec<EffectAst>)> = None;
    for idx in comma_indices.iter().rev().copied() {
        let effect_tokens = &tokens[idx + 1..];
        if effect_tokens.is_empty() {
            continue;
        }
        if let Ok(effects) = parse_effect_chain(effect_tokens)
            && !effects.is_empty()
        {
            split = Some((idx, effects));
            break;
        }
    }

    let (comma_idx, effects) = if let Some(split) = split {
        split
    } else {
        let first_idx = comma_indices[0];
        let effect_tokens = &tokens[first_idx + 1..];
        (first_idx, parse_effect_chain(effect_tokens)?)
    };
    let predicate_tokens = &tokens[1..comma_idx];

    if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
        return Ok(vec![EffectAst::IfResult { predicate, effects }]);
    }

    let predicate = parse_predicate(predicate_tokens)?;
    Ok(vec![EffectAst::Conditional {
        predicate,
        if_true: effects,
        if_false: Vec::new(),
    }])
}

pub(crate) fn parse_if_result_predicate(tokens: &[Token]) -> Option<IfResultPredicate> {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if words.len() >= 2 && words[0] == "you" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 2
        && words[0] == "you"
        && (words[1] == "win" || words[1] == "won")
        && (words.len() == 2 || words.iter().any(|word| *word == "clash"))
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 2 && words[0] == "they" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 2
        && (words[0] == "player" || words[0] == "players")
        && (words[1] == "do" || words[1] == "does")
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 6
        && words[0] == "you"
        && words[1] == "searched"
        && words[words.len() - 2] == "this"
        && words[words.len() - 1] == "way"
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 4
        && words[0] == "you"
        && matches!(
            words[1],
            "remove"
                | "removed"
                | "sacrifice"
                | "sacrificed"
                | "discard"
                | "discarded"
                | "exile"
                | "exiled"
        )
        && words[words.len() - 2] == "this"
        && words[words.len() - 1] == "way"
    {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 4
        && words[0] == "they"
        && matches!(
            words[1],
            "remove"
                | "removed"
                | "sacrifice"
                | "sacrificed"
                | "discard"
                | "discarded"
                | "exile"
                | "exiled"
        )
        && words[words.len() - 2] == "this"
        && words[words.len() - 1] == "way"
    {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && (words[0] == "that" || words[0] == "it")
        && words[1] == "spell"
        && words.iter().any(|word| *word == "countered")
        && words[words.len() - 2] == "this"
        && words[words.len() - 1] == "way"
    {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && (words[0] == "that" || words[0] == "it")
        && (words[1] == "creature" || words[1] == "permanent" || words[1] == "card")
        && words[2] == "dies"
        && words[3] == "this"
        && words[4] == "way"
    {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if words.len() >= 2 && words[0] == "you" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
    }
    if words.len() >= 2 && words[0] == "you" && words[1] == "cant" {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 3 && words[0] == "you" && words[1] == "can" && words[2] == "not" {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 2 && words[0] == "they" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
    }
    if words.len() >= 2 && words[0] == "they" && words[1] == "cant" {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 3 && words[0] == "they" && words[1] == "can" && words[2] == "not" {
        return Some(IfResultPredicate::DidNot);
    }

    None
}

pub(crate) fn parse_predicate(tokens: &[Token]) -> Result<PredicateAst, CardTextError> {
    let mut filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "is")
        .collect();

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    // Handle simple conjunction predicates like "... and have no cards in hand".
    if let Some(and_idx) = filtered.iter().position(|word| *word == "and")
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if matches!(right_first, Some("have") | Some("you")) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            // Inherit the subject when omitted ("... and have ...").
            if right_words.first().copied() == Some("have") {
                right_words.insert(0, "you");
            }
            let left_tokens = left_words
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let right_tokens = right_words
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }

    if filtered.as_slice() == ["this", "tapped"]
        || filtered.as_slice() == ["thiss", "tapped"]
        || ((filtered.first().copied() == Some("this")
            || filtered.first().copied() == Some("thiss"))
            && filtered.last().copied() == Some("tapped"))
    {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if filtered.starts_with(&["there", "are", "no"])
        && filtered.contains(&"counters")
        && filtered.windows(2).any(|window| window == ["on", "this"])
        && let Some(counters_idx) = filtered.iter().position(|word| *word == "counters")
        && counters_idx >= 4
        && let Some(counter_type) = parse_counter_type_word(filtered[counters_idx - 1])
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let raw_words = words(tokens);
    let triggering_object_had_no_counter_prefix_len = if raw_words.starts_with(&["it", "had", "no"])
    {
        Some(3)
    } else if raw_words.starts_with(&["this", "creature", "had", "no"])
        || raw_words.starts_with(&["that", "creature", "had", "no"])
        || raw_words.starts_with(&["this", "permanent", "had", "no"])
        || raw_words.starts_with(&["that", "permanent", "had", "no"])
    {
        Some(4)
    } else {
        None
    };
    if let Some(prefix_len) = triggering_object_had_no_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len])
        && matches!(raw_words[prefix_len + 1], "counter" | "counters")
        && raw_words[prefix_len + 2] == "on"
        && matches!(
            raw_words[prefix_len + 3],
            "it" | "them" | "this" | "that" | "itself"
        )
    {
        return Ok(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }

    if raw_words.starts_with(&["there", "are"])
        && raw_words.get(3).copied() == Some("or")
        && raw_words.get(4).copied() == Some("more")
        && raw_words
            .iter()
            .any(|w| *w == "counter" || *w == "counters")
    {
        if let Some((count, used)) = parse_number(&tokens[2..]) {
            let rest = &tokens[2 + used..];
            let rest_words = words(rest);
            // Pattern: "there are <N> or more <counter> counters on this <permanent>"
            if rest_words.len() >= 4
                && rest_words[0] == "or"
                && rest_words[1] == "more"
                && (rest_words[3] == "counter" || rest_words[3] == "counters")
                && let Some(counter_type) = parse_counter_type_word(rest_words[2])
            {
                return Ok(PredicateAst::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                });
            }
        }
    }

    // "there are N or more basic land types among lands that player controls"
    if filtered.len() >= 13
        && filtered[0] == "there"
        && filtered[1] == "are"
        && filtered.get(3).copied() == Some("or")
        && filtered.get(4).copied() == Some("more")
        && filtered.get(5).copied() == Some("basic")
        && filtered.get(6).copied() == Some("land")
        && matches!(filtered.get(7).copied(), Some("type" | "types"))
        && filtered.get(8).copied() == Some("among")
        && matches!(filtered.get(9).copied(), Some("land" | "lands"))
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported basic-land-types predicate count (predicate: '{}')",
                filtered.join(" ")
            )));
        };

        let tail = &filtered[10..];
        let player = if tail == ["that", "player", "controls"]
            || tail == ["that", "player", "control"]
            || tail == ["that", "players", "controls"]
        {
            PlayerAst::That
        } else if tail == ["you", "control"] || tail == ["you", "controls"] {
            PlayerAst::You
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported basic-land-types predicate tail (predicate: '{}')",
                filtered.join(" ")
            )));
        };

        return Ok(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore { player, count });
    }

    let parse_graveyard_card_types_subject = |words: &[&str]| -> Option<PlayerAst> {
        match words {
            [first, second] if *first == "your" && *second == "graveyard" => Some(PlayerAst::You),
            [first, second, third]
                if *first == "that"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::That)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "player" || *second == "players")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::Target)
            }
            [first, second, third]
                if *first == "target"
                    && (*second == "opponent" || *second == "opponents")
                    && *third == "graveyard" =>
            {
                Some(PlayerAst::TargetOpponent)
            }
            [first, second]
                if (*first == "opponent" || *first == "opponents") && *second == "graveyard" =>
            {
                Some(PlayerAst::Opponent)
            }
            _ => None,
        }
    };
    if filtered.len() >= 11 {
        let (count_idx, subject_start, constrained_player) =
            if filtered[0] == "there" && filtered[1] == "are" {
                (2usize, 10usize, None)
            } else if filtered[0] == "you" && filtered[1] == "have" {
                (2usize, 10usize, Some(PlayerAst::You))
            } else {
                (usize::MAX, usize::MAX, None)
            };
        if count_idx != usize::MAX
            && filtered.get(count_idx + 1).copied() == Some("or")
            && filtered.get(count_idx + 2).copied() == Some("more")
            && filtered.get(count_idx + 3).copied() == Some("card")
            && matches!(filtered.get(count_idx + 4).copied(), Some("type" | "types"))
            && filtered.get(count_idx + 5).copied() == Some("among")
            && matches!(filtered.get(count_idx + 6).copied(), Some("card" | "cards"))
            && filtered.get(count_idx + 7).copied() == Some("in")
            && subject_start <= filtered.len()
            && let Some(count) = parse_named_number(filtered[count_idx])
            && let Some(player) = parse_graveyard_card_types_subject(&filtered[subject_start..])
            && constrained_player.map_or(true, |expected| expected == player)
        {
            return Ok(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count });
        }
    }

    let parse_cards_in_hand_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        match words {
            [first, second, ..] if *first == "that" && *second == "player" => {
                Some((PlayerAst::That, 2))
            }
            [first, second, ..] if *first == "target" && *second == "player" => {
                Some((PlayerAst::Target, 2))
            }
            [first, second, ..] if *first == "target" && *second == "opponent" => {
                Some((PlayerAst::TargetOpponent, 2))
            }
            [first, second, ..] if *first == "each" && *second == "opponent" => {
                Some((PlayerAst::Opponent, 2))
            }
            [first, ..] if *first == "you" => Some((PlayerAst::You, 1)),
            [first, ..] if *first == "opponent" || *first == "opponents" => {
                Some((PlayerAst::Opponent, 1))
            }
            [first, second, ..] if *first == "player" && *second == "who" => {
                Some((PlayerAst::That, 1))
            }
            _ => None,
        }
    };
    if let Some((player, subject_len)) = parse_cards_in_hand_subject(&filtered)
        && filtered.get(subject_len).copied() == Some("has")
        && let Some(count_word) = filtered.get(subject_len + 1).copied()
        && let Some(count) = parse_named_number(count_word)
        && filtered.get(subject_len + 2).copied() == Some("or")
        && let Some(comp_word) = filtered.get(subject_len + 3).copied()
        && matches!(comp_word, "more" | "fewer" | "less")
        && matches!(
            filtered.get(subject_len + 4).copied(),
            Some("card" | "cards")
        )
        && filtered.get(subject_len + 5).copied() == Some("in")
        && filtered.get(subject_len + 6).copied() == Some("hand")
        && filtered.len() == subject_len + 7
    {
        return Ok(if comp_word == "more" {
            PredicateAst::PlayerCardsInHandOrMore { player, count }
        } else {
            PredicateAst::PlayerCardsInHandOrFewer { player, count }
        });
    }

    if filtered.as_slice() == ["you", "have", "no", "cards", "in", "hand"] {
        return Ok(PredicateAst::YouHaveNoCardsInHand);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["creature", "died", "this", "turn"] | ["creatures", "died", "this", "turn"]
    ) {
        return Ok(PredicateAst::CreatureDiedThisTurn);
    }

    if matches!(
        filtered.as_slice(),
        [
            "you",
            "had",
            "land",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "land",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "enter",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ] | [
            "you",
            "had",
            "lands",
            "entered",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn"
        ]
    ) {
        return Ok(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
            player: PlayerAst::You,
        });
    }

    if filtered.as_slice() == ["you", "attacked", "this", "turn"] {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if filtered.as_slice() == ["you", "cast", "it"]
        || filtered.as_slice() == ["you", "cast", "this", "spell"]
    {
        return Ok(PredicateAst::SourceWasCast);
    }

    if filtered.as_slice() == ["no", "spells", "were", "cast", "last", "turn"]
        || filtered.as_slice() == ["no", "spell", "was", "cast", "last", "turn"]
    {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if filtered.as_slice() == ["this", "spell", "was", "kicked"] {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if filtered.as_slice() == ["it", "was", "kicked"]
        || filtered.as_slice() == ["that", "was", "kicked"]
    {
        return Ok(PredicateAst::TargetWasKicked);
    }
    if filtered.as_slice() == ["its", "controller", "poisoned"]
        || filtered.as_slice() == ["that", "spells", "controller", "poisoned"]
    {
        return Ok(PredicateAst::TargetSpellControllerIsPoisoned);
    }
    if filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "that", "spell"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "that", "spell"]
    {
        return Ok(PredicateAst::TargetSpellNoManaSpentToCast);
    }
    if filtered.as_slice()
        == [
            "you",
            "control",
            "more",
            "creatures",
            "than",
            "that",
            "spells",
            "controller",
        ]
        || filtered.as_slice()
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "its",
                "controller",
            ]
    {
        return Ok(PredicateAst::YouControlMoreCreaturesThanTargetSpellController);
    }
    if filtered.len() == 7
        && matches!(filtered[0], "w" | "u" | "b" | "r" | "g" | "c")
        && filtered[1] == "was"
        && filtered[2] == "spent"
        && filtered[3] == "to"
        && filtered[4] == "cast"
        && filtered[5] == "this"
        && filtered[6] == "spell"
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered[0] == "its" {
        filtered[0] = "it";
    }

    if filtered.len() >= 2 {
        let tag = if filtered.starts_with(&["equipped", "creature"]) {
            Some("equipped")
        } else if filtered.starts_with(&["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let is_it = filtered.first().is_some_and(|word| *word == "it");
    let has_card = filtered.contains(&"card");

    if is_it {
        if filtered.len() >= 3 && filtered[1] == "mana" && filtered[2] == "value" {
            let mana_value_tail = &filtered[3..];
            let compares_to_colors_spent = mana_value_tail
                == [
                    "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana",
                    "spent", "to", "cast", "this", "spell",
                ]
                || mana_value_tail
                    == [
                        "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                        "spent", "to", "cast", "this", "spell",
                    ];
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 3 && (filtered[1] == "power" || filtered[1] == "toughness") {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if axis == "power" {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }

        let mut card_types = Vec::new();
        for word in &filtered {
            if let Some(card_type) = parse_card_type(word)
                && !card_types.contains(&card_type)
            {
                card_types.push(card_type);
            }
        }
        let mut subtypes = Vec::new();
        for word in &filtered {
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }
        if !card_types.is_empty() || !subtypes.is_empty() {
            if has_card && card_types.len() == 1 && card_types[0] == CardType::Land {
                return Ok(PredicateAst::ItIsLandCard);
            }
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                card_types,
                subtypes,
                ..Default::default()
            }));
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && (filtered[2] == "no" || filtered[2] == "neither")
    {
        let control_tokens = filtered[3..]
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::You);
            if filtered[2] == "neither" {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 7
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
        && let Some(or_idx) = filtered.iter().position(|word| *word == "or")
        && or_idx > 2
    {
        let left_tokens = filtered[2..or_idx]
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if right_words.first().copied() == Some("there") {
            right_words = right_words[1..].to_vec();
        }
        if right_words.contains(&"graveyard") && right_words.contains(&"your") {
            let right_tokens = right_words
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3
        && filtered[0] == "you"
        && (filtered[1] == "control" || filtered[1] == "controls")
    {
        let mut filter_start = 2usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(2)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(3).copied() == Some("or")
            && filtered.get(4).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(2).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 4;
        } else if filtered.get(2).copied() == Some("at")
            && filtered.get(3).copied() == Some("least")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 5;
        }

        let mut control_words = filtered[filter_start..].to_vec();
        let mut requires_different_powers = false;
        if control_words.ends_with(&["with", "different", "powers"])
            || control_words.ends_with(&["with", "different", "power"])
        {
            requires_different_powers = true;
            control_words.truncate(control_words.len().saturating_sub(3));
        }
        let control_tokens = control_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                if requires_different_powers {
                    return Ok(PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: PlayerAst::You,
                        filter,
                        count,
                    });
                }
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::You,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4
        && filtered[0] == "that"
        && (filtered[1] == "player" || filtered[1] == "players")
        && (filtered[2] == "control" || filtered[2] == "controls")
    {
        let mut filter_start = 3usize;
        let mut min_count: Option<u32> = None;
        let mut exact_count: Option<u32> = None;
        if let Some(raw_count) = filtered.get(3)
            && let Some(parsed_count) = parse_named_number(raw_count)
            && filtered.get(4).copied() == Some("or")
            && filtered.get(5).copied() == Some("more")
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        } else if filtered.get(3).copied() == Some("exactly")
            && let Some(raw_count) = filtered.get(4)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            exact_count = Some(parsed_count);
            filter_start = 5;
        } else if filtered.get(3).copied() == Some("at")
            && filtered.get(4).copied() == Some("least")
            && let Some(raw_count) = filtered.get(5)
            && let Some(parsed_count) = parse_named_number(raw_count)
        {
            min_count = Some(parsed_count);
            filter_start = 6;
        }

        let control_tokens = filtered[filter_start..]
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let other = control_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        if let Ok(filter) = parse_object_filter(&control_tokens, other) {
            if let Some(count) = exact_count {
                return Ok(PredicateAst::PlayerControlsExactly {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            if let Some(count) = min_count
                && count > 1
            {
                return Ok(PredicateAst::PlayerControlsAtLeast {
                    player: PlayerAst::That,
                    filter,
                    count,
                });
            }
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            });
        }
    }

    if filtered.as_slice() == ["you", "controlled", "that", "permanent"]
        || filtered.as_slice() == ["you", "control", "that", "permanent"]
    {
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default(),
        });
    }

    if filtered.as_slice() == ["it", "wasnt", "blocking"]
        || filtered.as_slice() == ["it", "was", "not", "blocking"]
        || filtered.as_slice() == ["that", "creature", "wasnt", "blocking"]
    {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }

    if filtered.as_slice() == ["no", "creatures", "are", "on", "battlefield"] {
        return Ok(PredicateAst::PlayerControlsNo {
            player: PlayerAst::Any,
            filter: ObjectFilter::creature(),
        });
    }

    if filtered.as_slice() == ["you", "have", "citys", "blessing"]
        || filtered.as_slice() == ["you", "have", "city", "blessing"]
        || filtered.starts_with(&["you", "have", "citys", "blessing", "for", "each"])
        || filtered.starts_with(&["you", "have", "city", "blessing", "for", "each"])
    {
        return Ok(PredicateAst::PlayerControlsAtLeast {
            player: PlayerAst::You,
            filter: ObjectFilter::permanent().you_control(),
            count: 10,
        });
    }

    let unsupported_unmodeled = filtered.as_slice() == ["you", "gained", "life", "this", "turn"]
        || filtered.as_slice() == ["opponent", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["opponents", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["an", "opponent", "lost", "life", "this", "turn"]
        || filtered.as_slice() == ["that", "creature", "would", "die", "this", "turn"]
        || filtered.as_slice()
            == [
                "this", "second", "time", "this", "ability", "has", "resolved", "this", "turn",
            ]
        || filtered.as_slice()
            == [
                "this",
                "ability",
                "has",
                "been",
                "activated",
                "four",
                "or",
                "more",
                "times",
                "this",
                "turn",
            ]
        || filtered.as_slice() == ["it", "first", "combat", "phase", "of", "turn"]
        || filtered.as_slice()
            == [
                "two",
                "or",
                "more",
                "creatures",
                "are",
                "tied",
                "for",
                "least",
                "power",
            ];
    if unsupported_unmodeled {
        return Ok(PredicateAst::Unmodeled(filtered.join(" ")));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

pub(crate) fn parse_graveyard_threshold_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let (count, tail_start, constrained_player) = if filtered.len() >= 5
        && filtered[0] == "there"
        && filtered[1] == "are"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, None)
    } else if filtered.len() >= 5
        && filtered[0] == "you"
        && filtered[1] == "have"
        && filtered[3] == "or"
        && filtered[4] == "more"
    {
        let Some(count) = parse_named_number(filtered[2]) else {
            return Ok(None);
        };
        (count, 5usize, Some(PlayerAst::You))
    } else {
        return Ok(None);
    };

    let tail = &filtered[tail_start..];
    let Some(in_idx) = tail.iter().rposition(|word| *word == "in") else {
        return Ok(None);
    };
    if in_idx == 0 || in_idx + 1 >= tail.len() {
        return Ok(None);
    }

    let graveyard_owner_words = &tail[in_idx + 1..];
    let player = match graveyard_owner_words {
        ["your", "graveyard"] => PlayerAst::You,
        ["that", "player", "graveyard"] | ["that", "players", "graveyard"] => PlayerAst::That,
        ["target", "player", "graveyard"] | ["target", "players", "graveyard"] => PlayerAst::Target,
        ["target", "opponent", "graveyard"] | ["target", "opponents", "graveyard"] => {
            PlayerAst::TargetOpponent
        }
        ["opponent", "graveyard"] | ["opponents", "graveyard"] => PlayerAst::Opponent,
        _ => return Ok(None),
    };
    if constrained_player.is_some_and(|expected| expected != player) {
        return Ok(None);
    }

    let raw_filter_words = &tail[..in_idx];
    if raw_filter_words.is_empty()
        || raw_filter_words.contains(&"type")
        || raw_filter_words.contains(&"types")
    {
        return Ok(None);
    }

    let mut normalized_filter_words = Vec::with_capacity(raw_filter_words.len());
    for (idx, word) in raw_filter_words.iter().enumerate() {
        // Normalize "instant and/or sorcery" -> "instant or sorcery".
        if *word == "and"
            && raw_filter_words
                .get(idx + 1)
                .is_some_and(|next| *next == "or")
        {
            continue;
        }
        normalized_filter_words.push(*word);
    }
    if normalized_filter_words.is_empty() {
        return Ok(None);
    }

    let mut filter = if matches!(normalized_filter_words.as_slice(), ["card"] | ["cards"]) {
        ObjectFilter::default()
    } else {
        let filter_tokens = normalized_filter_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Ok(filter) = parse_object_filter(&filter_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    filter.zone = Some(Zone::Graveyard);

    Ok(Some(PredicateAst::PlayerControlsAtLeast {
        player,
        filter,
        count,
    }))
}

pub(crate) fn parse_sentence_counter_target_spell_if_it_was_kicked(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
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
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
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
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let is_shape = clause_words.starts_with(&["exile", "target", "creature"])
        && contains_word_sequence(&clause_words, &["greatest", "power", "among", "creatures"])
        && (clause_words
            .windows(2)
            .any(|pair| pair == ["on", "battlefield"])
            || clause_words
                .windows(3)
                .any(|triplet| triplet == ["on", "the", "battlefield"]));
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

pub(crate) fn parse_mana_spent_to_cast_predicate(
    words: &[&str],
) -> Option<(u32, Option<ManaSymbol>)> {
    if words.len() < 10 || words[0] != "at" || words[1] != "least" {
        return None;
    }

    let amount_tokens = vec![Token::Word(words[2].to_string(), TextSpan::synthetic())];
    let (amount, _) = parse_number(&amount_tokens)?;

    let mut idx = 3;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    let symbol = if let Some(word) = words.get(idx).copied() {
        if let Some(parsed) = parse_mana_symbol_word(word) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    let tail = &words[idx..];
    let canonical_tail = ["mana", "was", "spent", "to", "cast", "this", "spell"];
    let plural_tail = ["mana", "were", "spent", "to", "cast", "this", "spell"];
    if tail == canonical_tail || tail == plural_tail {
        return Some((amount, symbol));
    }

    None
}

pub(crate) fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    match word {
        "white" => Some(ManaSymbol::White),
        "blue" => Some(ManaSymbol::Blue),
        "black" => Some(ManaSymbol::Black),
        "red" => Some(ManaSymbol::Red),
        "green" => Some(ManaSymbol::Green),
        "colorless" => Some(ManaSymbol::Colorless),
        _ => None,
    }
}
