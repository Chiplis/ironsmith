use super::*;

pub(crate) fn parse_card_type(word: &str) -> Option<CardType> {
    match word {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

pub(crate) fn parse_non_type(word: &str) -> Option<CardType> {
    let rest = word.strip_prefix("non")?;
    parse_card_type(rest)
}

pub(crate) fn parse_non_supertype(word: &str) -> Option<Supertype> {
    let rest = word.strip_prefix("non")?;
    parse_supertype_word(rest)
}

pub(crate) fn parse_non_color(word: &str) -> Option<ColorSet> {
    let rest = word.strip_prefix("non")?;
    match rest {
        "white" => Some(ColorSet::WHITE),
        "blue" => Some(ColorSet::BLUE),
        "black" => Some(ColorSet::BLACK),
        "red" => Some(ColorSet::RED),
        "green" => Some(ColorSet::GREEN),
        _ => None,
    }
}

pub(crate) fn parse_non_subtype(word: &str) -> Option<Subtype> {
    let rest = word.strip_prefix("non")?;
    parse_subtype_word(rest).or_else(|| rest.strip_suffix('s').and_then(parse_subtype_word))
}

pub(crate) fn is_outlaw_word(word: &str) -> bool {
    matches!(word, "outlaw" | "outlaws")
}

pub(crate) fn is_non_outlaw_word(word: &str) -> bool {
    matches!(
        word,
        "nonoutlaw" | "nonoutlaws" | "non-outlaw" | "non-outlaws"
    )
}

pub(crate) fn push_outlaw_subtypes(out: &mut Vec<Subtype>) {
    for subtype in [
        Subtype::Assassin,
        Subtype::Mercenary,
        Subtype::Pirate,
        Subtype::Rogue,
        Subtype::Warlock,
    ] {
        if !out.contains(&subtype) {
            out.push(subtype);
        }
    }
}

pub(crate) fn parse_color(word: &str) -> Option<ColorSet> {
    match word {
        "white" => Some(ColorSet::WHITE),
        "blue" => Some(ColorSet::BLUE),
        "black" => Some(ColorSet::BLACK),
        "red" => Some(ColorSet::RED),
        "green" => Some(ColorSet::GREEN),
        _ => None,
    }
}

pub(crate) fn parse_zone_word(word: &str) -> Option<Zone> {
    match word {
        "battlefield" => Some(Zone::Battlefield),
        "graveyard" | "graveyards" => Some(Zone::Graveyard),
        "hand" | "hands" => Some(Zone::Hand),
        "library" | "libraries" => Some(Zone::Library),
        "exile" | "exiled" => Some(Zone::Exile),
        "stack" => Some(Zone::Stack),
        _ => None,
    }
}

pub(crate) fn parse_alternative_cast_words(words: &[&str]) -> Option<(AlternativeCastKind, usize)> {
    match words {
        ["flashback", ..] => Some((AlternativeCastKind::Flashback, 1)),
        ["jump", "start", ..] => Some((AlternativeCastKind::JumpStart, 2)),
        ["jumpstart", ..] => Some((AlternativeCastKind::JumpStart, 1)),
        ["escape", ..] => Some((AlternativeCastKind::Escape, 1)),
        ["madness", ..] => Some((AlternativeCastKind::Madness, 1)),
        ["miracle", ..] => Some((AlternativeCastKind::Miracle, 1)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterKeywordConstraint {
    Static(StaticAbilityId),
    Marker(&'static str),
}

pub(crate) fn keyword_action_to_filter_constraint(
    action: KeywordAction,
) -> Option<FilterKeywordConstraint> {
    use FilterKeywordConstraint::{Marker, Static};
    let ability = match action {
        KeywordAction::Flying => Static(StaticAbilityId::Flying),
        KeywordAction::Menace => Static(StaticAbilityId::Menace),
        KeywordAction::Hexproof => Static(StaticAbilityId::Hexproof),
        KeywordAction::Haste => Static(StaticAbilityId::Haste),
        KeywordAction::FirstStrike => Static(StaticAbilityId::FirstStrike),
        KeywordAction::DoubleStrike => Static(StaticAbilityId::DoubleStrike),
        KeywordAction::Deathtouch => Static(StaticAbilityId::Deathtouch),
        KeywordAction::Lifelink => Static(StaticAbilityId::Lifelink),
        KeywordAction::Vigilance => Static(StaticAbilityId::Vigilance),
        KeywordAction::Trample => Static(StaticAbilityId::Trample),
        KeywordAction::Reach => Static(StaticAbilityId::Reach),
        KeywordAction::Defender => Static(StaticAbilityId::Defender),
        KeywordAction::Flash => Static(StaticAbilityId::Flash),
        KeywordAction::Indestructible => Static(StaticAbilityId::Indestructible),
        KeywordAction::Shroud => Static(StaticAbilityId::Shroud),
        KeywordAction::Wither => Static(StaticAbilityId::Wither),
        KeywordAction::Infect => Static(StaticAbilityId::Infect),
        KeywordAction::Fear => Static(StaticAbilityId::Fear),
        KeywordAction::Intimidate => Static(StaticAbilityId::Intimidate),
        KeywordAction::Shadow => Static(StaticAbilityId::Shadow),
        KeywordAction::Horsemanship => Static(StaticAbilityId::Horsemanship),
        KeywordAction::Flanking => Static(StaticAbilityId::Flanking),
        KeywordAction::Landwalk(subtype) => {
            let marker = match subtype {
                Subtype::Island => "islandwalk",
                Subtype::Swamp => "swampwalk",
                Subtype::Mountain => "mountainwalk",
                Subtype::Forest => "forestwalk",
                Subtype::Plains => "plainswalk",
                _ => return Some(Static(StaticAbilityId::Landwalk)),
            };
            Marker(marker)
        }
        KeywordAction::Bloodthirst(_) => return None,
        KeywordAction::Rampage(_) => return None,
        KeywordAction::Changeling => Static(StaticAbilityId::Changeling),
        _ => return None,
    };
    Some(ability)
}

pub(crate) fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    if words.is_empty() {
        return None;
    }
    if words.len() >= 2 && words[0] == "mana" && matches!(words[1], "ability" | "abilities") {
        return Some((FilterKeywordConstraint::Marker("mana ability"), 2));
    }
    if words[0] == "cycling" || words[0].ends_with("cycling") {
        return Some((FilterKeywordConstraint::Marker("cycling"), 1));
    }
    if words.len() >= 2 && words[0] == "basic" && words[1] == "landcycling" {
        return Some((FilterKeywordConstraint::Marker("cycling"), 2));
    }

    let max_len = words.len().min(4);
    for len in (1..=max_len).rev() {
        let tokens = words[..len]
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Some(action) = parse_ability_phrase(&tokens) else {
            continue;
        };
        if let Some(constraint) = keyword_action_to_filter_constraint(action) {
            return Some((constraint, len));
        }
    }
    None
}

pub(crate) fn parse_filter_counter_constraint_words(
    words: &[&str],
) -> Option<(crate::filter::CounterConstraint, usize)> {
    if words.len() < 3 {
        return None;
    }
    let counter_idx = words
        .iter()
        .position(|word| *word == "counter" || *word == "counters")?;
    if words.get(counter_idx + 1) != Some(&"on") {
        return None;
    }
    if !words
        .get(counter_idx + 2)
        .is_some_and(|word| matches!(*word, "it" | "them"))
    {
        return None;
    }

    let descriptor_words = words[..counter_idx]
        .iter()
        .copied()
        .filter(|word| !matches!(*word, "a" | "an" | "one" | "or" | "more"))
        .collect::<Vec<_>>();
    let consumed = counter_idx + 3;
    if descriptor_words.is_empty() {
        return Some((crate::filter::CounterConstraint::Any, consumed));
    }
    let descriptor_tokens = descriptor_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let counter_type = parse_counter_type_from_tokens(&descriptor_tokens)?;
    Some((
        crate::filter::CounterConstraint::Typed(counter_type),
        consumed,
    ))
}

pub(crate) fn apply_filter_keyword_constraint(
    filter: &mut ObjectFilter,
    constraint: FilterKeywordConstraint,
    excluded: bool,
) {
    match constraint {
        FilterKeywordConstraint::Static(ability_id) => {
            if excluded {
                if !filter.excluded_static_abilities.contains(&ability_id) {
                    filter.excluded_static_abilities.push(ability_id);
                }
            } else if !filter.static_abilities.contains(&ability_id) {
                filter.static_abilities.push(ability_id);
            }
        }
        FilterKeywordConstraint::Marker(marker) => {
            if excluded {
                if !filter
                    .excluded_ability_markers
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(marker))
                {
                    filter.excluded_ability_markers.push(marker.to_string());
                }
            } else if !filter
                .ability_markers
                .iter()
                .any(|value| value.eq_ignore_ascii_case(marker))
            {
                filter.ability_markers.push(marker.to_string());
            }
        }
    }
}

pub(crate) fn is_permanent_type(card_type: CardType) -> bool {
    matches!(
        card_type,
        CardType::Artifact
            | CardType::Creature
            | CardType::Enchantment
            | CardType::Land
            | CardType::Planeswalker
            | CardType::Battle
    )
}

pub(crate) fn is_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

pub(crate) fn parse_number(tokens: &[Token]) -> Option<(u32, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?;

    if let Ok(value) = word.parse::<u32>() {
        return Some((value, 1));
    }

    let value = match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    };

    Some((value, 1))
}

pub(crate) fn parse_value(tokens: &[Token]) -> Option<(Value, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?;

    if word == "x" {
        return Some((Value::X, 1));
    }

    let (number, used) = parse_number(tokens)?;
    Some((Value::Fixed(number as i32), used))
}

#[derive(Debug, Clone)]
pub(crate) struct CompileContext {
    pub(crate) next_effect_id: u32,
    pub(crate) next_tag_id: u32,
    pub(crate) last_effect_id: Option<EffectId>,
    pub(crate) last_object_tag: Option<String>,
    pub(crate) last_player_filter: Option<PlayerFilter>,
    pub(crate) iterated_player: bool,
    pub(crate) auto_tag_object_targets: bool,
    pub(crate) force_auto_tag_object_targets: bool,
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
}

impl CompileContext {
    pub(crate) fn new() -> Self {
        Self {
            next_effect_id: 0,
            next_tag_id: 0,
            last_effect_id: None,
            last_object_tag: None,
            last_player_filter: None,
            iterated_player: false,
            auto_tag_object_targets: false,
            force_auto_tag_object_targets: false,
            allow_life_event_value: false,
            bind_unbound_x_to_last_effect: false,
        }
    }

    pub(crate) fn next_effect_id(&mut self) -> EffectId {
        let id = EffectId(self.next_effect_id);
        self.next_effect_id += 1;
        id
    }

    pub(crate) fn next_tag(&mut self, prefix: &str) -> String {
        let tag = format!("{prefix}_{}", self.next_tag_id);
        self.next_tag_id += 1;
        tag
    }
}
