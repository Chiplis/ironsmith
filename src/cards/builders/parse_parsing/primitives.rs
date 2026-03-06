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
    parse_color(rest)
}

pub(crate) fn parse_non_subtype(word: &str) -> Option<Subtype> {
    let rest = word.strip_prefix("non")?;
    parse_subtype_flexible(rest)
}

pub(crate) fn parse_subtype_flexible(word: &str) -> Option<Subtype> {
    parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
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
    crate::color::Color::from_name(word).map(ColorSet::from_color)
}

pub(crate) fn parse_mana_symbol_color_word(word: &str) -> Option<ManaSymbol> {
    match parse_color(word)? {
        ColorSet::WHITE => Some(ManaSymbol::White),
        ColorSet::BLUE => Some(ManaSymbol::Blue),
        ColorSet::BLACK => Some(ManaSymbol::Black),
        ColorSet::RED => Some(ManaSymbol::Red),
        ColorSet::GREEN => Some(ManaSymbol::Green),
        _ => None,
    }
}

pub(crate) fn parse_mana_symbol_word_flexible(word: &str) -> Option<ManaSymbol> {
    if word == "colorless" {
        return Some(ManaSymbol::Colorless);
    }
    parse_mana_symbol_color_word(word)
}

pub(crate) fn parse_number_word_i32(word: &str) -> Option<i32> {
    if let Ok(value) = word.parse::<i32>() {
        return Some(value);
    }

    match word {
        "zero" => Some(0),
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        _ => None,
    }
}

pub(crate) fn parse_number_word_u32(word: &str) -> Option<u32> {
    parse_number_word_i32(word).and_then(|value| value.try_into().ok())
}

pub(crate) fn is_until_end_of_turn(words: &[&str]) -> bool {
    words == ["until", "end", "of", "turn"]
}

pub(crate) fn starts_with_until_end_of_turn(words: &[&str]) -> bool {
    words.starts_with(&["until", "end", "of", "turn"])
}

pub(crate) fn ends_with_until_end_of_turn(words: &[&str]) -> bool {
    words.ends_with(&["until", "end", "of", "turn"])
}

pub(crate) fn contains_until_end_of_turn(words: &[&str]) -> bool {
    words.windows(4).any(|window| is_until_end_of_turn(window))
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

    if let KeywordAction::Landwalk(subtype) = action {
        let constraint = match subtype {
            Subtype::Island => Marker("islandwalk"),
            Subtype::Swamp => Marker("swampwalk"),
            Subtype::Mountain => Marker("mountainwalk"),
            Subtype::Forest => Marker("forestwalk"),
            Subtype::Plains => Marker("plainswalk"),
            _ => Static(StaticAbilityId::Landwalk),
        };
        return Some(constraint);
    }

    let static_id = keyword_action_to_static_ability(action)?.id();
    match static_id {
        StaticAbilityId::Flying
        | StaticAbilityId::Menace
        | StaticAbilityId::Hexproof
        | StaticAbilityId::Haste
        | StaticAbilityId::FirstStrike
        | StaticAbilityId::DoubleStrike
        | StaticAbilityId::Deathtouch
        | StaticAbilityId::Lifelink
        | StaticAbilityId::Vigilance
        | StaticAbilityId::Trample
        | StaticAbilityId::Reach
        | StaticAbilityId::Defender
        | StaticAbilityId::Flash
        | StaticAbilityId::Indestructible
        | StaticAbilityId::Shroud
        | StaticAbilityId::Wither
        | StaticAbilityId::Infect
        | StaticAbilityId::Fear
        | StaticAbilityId::Intimidate
        | StaticAbilityId::Shadow
        | StaticAbilityId::Horsemanship
        | StaticAbilityId::Flanking
        | StaticAbilityId::Changeling => Some(Static(static_id)),
        _ => None,
    }
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

fn parse_value_expr_term_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.is_empty() {
        return None;
    }

    if words[0] == "x" {
        return Some((Value::X, 1));
    }

    if let Some(value) = parse_number_word_i32(words[0]) {
        return Some((Value::Fixed(value), 1));
    }

    let mut idx = 0usize;
    if words[idx] == "the" {
        idx += 1;
    }
    if words.get(idx).copied() != Some("number") || words.get(idx + 1).copied() != Some("of") {
        return None;
    }
    idx += 2;

    let mut counter_idx = idx;
    if words
        .get(counter_idx)
        .is_some_and(|word| is_article(word) || *word == "one")
    {
        counter_idx += 1;
    }

    let mut parsed_counter_type = None;
    if let Some(word) = words.get(counter_idx).copied()
        && let Some(counter_type) = parse_counter_type_word(word)
    {
        parsed_counter_type = Some(counter_type);
        counter_idx += 1;
    }

    if matches!(
        words.get(counter_idx).copied(),
        Some("counter" | "counters")
    ) && words.get(counter_idx + 1).copied() == Some("on")
    {
        let reference_start = counter_idx + 2;
        let mut reference_end = reference_start;
        while reference_end < words.len() && !matches!(words[reference_end], "plus" | "minus") {
            reference_end += 1;
        }
        let reference = &words[reference_start..reference_end];
        if matches!(
            reference,
            ["it"]
                | ["this"]
                | ["this", "card"]
                | ["this", "creature"]
                | ["this", "permanent"]
                | ["this", "source"]
                | ["this", "artifact"]
                | ["this", "land"]
                | ["this", "enchantment"]
        ) {
            let value = match parsed_counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            };
            return Some((value, reference_end));
        }
        if matches!(
            reference,
            ["that"]
                | ["that", "card"]
                | ["that", "creature"]
                | ["that", "permanent"]
                | ["that", "object"]
                | ["those"]
                | ["those", "cards"]
                | ["those", "creatures"]
                | ["those", "permanents"]
        ) {
            let value = Value::CountersOn(
                Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                parsed_counter_type,
            );
            return Some((value, reference_end));
        }
    }

    let filter_start = idx;
    let mut filter_end = filter_start;
    while filter_end < words.len() && !matches!(words[filter_end], "plus" | "minus") {
        filter_end += 1;
    }
    if filter_end <= filter_start {
        return None;
    }
    let filter_tokens = words[filter_start..filter_end]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some((Value::Count(filter), filter_end))
}

pub(crate) fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    let (mut value, mut used) = parse_value_expr_term_words(words)?;

    while used < words.len() {
        let operator = words[used];
        if !matches!(operator, "plus" | "minus") {
            break;
        }

        let (rhs, rhs_used) = parse_value_expr_term_words(&words[used + 1..])?;
        used += 1 + rhs_used;

        let rhs = if operator == "minus" {
            match rhs {
                Value::Fixed(fixed) => Value::Fixed(-fixed),
                _ => return None,
            }
        } else {
            rhs
        };

        value = Value::Add(Box::new(value), Box::new(rhs));
    }

    Some((value, used))
}

pub(crate) fn parse_value_expr(tokens: &[Token]) -> Option<(Value, usize)> {
    let words = tokens.iter().filter_map(Token::as_word).collect::<Vec<_>>();
    let (value, used_words) = parse_value_expr_words(&words)?;
    let used = token_index_for_word_index(tokens, used_words).unwrap_or(tokens.len());
    Some((value, used))
}

pub(crate) fn parse_value(tokens: &[Token]) -> Option<(Value, usize)> {
    parse_value_expr(tokens)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IdGenContext {
    pub(crate) next_effect_id: u32,
    pub(crate) next_tag_id: u32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LoweringReferenceFrame {
    pub(crate) last_effect_id: Option<EffectId>,
    pub(crate) last_object_tag: Option<String>,
    pub(crate) last_player_filter: Option<PlayerFilter>,
    pub(crate) iterated_player: bool,
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
}

pub(crate) trait ReferenceFrameSource {
    fn reference_frame(&self) -> LoweringReferenceFrame;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LoweringFrame {
    pub(crate) last_effect_id: Option<EffectId>,
    pub(crate) last_object_tag: Option<String>,
    pub(crate) last_player_filter: Option<PlayerFilter>,
    pub(crate) iterated_player: bool,
    pub(crate) auto_tag_object_targets: bool,
    pub(crate) force_auto_tag_object_targets: bool,
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
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
        Self::from_parts(IdGenContext::default(), LoweringFrame::default())
    }

    pub(crate) fn from_parts(id_gen: IdGenContext, frame: LoweringFrame) -> Self {
        Self {
            next_effect_id: id_gen.next_effect_id,
            next_tag_id: id_gen.next_tag_id,
            last_effect_id: frame.last_effect_id,
            last_object_tag: frame.last_object_tag,
            last_player_filter: frame.last_player_filter,
            iterated_player: frame.iterated_player,
            auto_tag_object_targets: frame.auto_tag_object_targets,
            force_auto_tag_object_targets: frame.force_auto_tag_object_targets,
            allow_life_event_value: frame.allow_life_event_value,
            bind_unbound_x_to_last_effect: frame.bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn id_gen_context(&self) -> IdGenContext {
        IdGenContext {
            next_effect_id: self.next_effect_id,
            next_tag_id: self.next_tag_id,
        }
    }

    pub(crate) fn apply_id_gen_context(&mut self, id_gen: IdGenContext) {
        self.next_effect_id = id_gen.next_effect_id;
        self.next_tag_id = id_gen.next_tag_id;
    }

    pub(crate) fn reference_frame(&self) -> LoweringReferenceFrame {
        LoweringReferenceFrame {
            last_effect_id: self.last_effect_id,
            last_object_tag: self.last_object_tag.clone(),
            last_player_filter: self.last_player_filter.clone(),
            iterated_player: self.iterated_player,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn lowering_frame(&self) -> LoweringFrame {
        LoweringFrame {
            last_effect_id: self.last_effect_id,
            last_object_tag: self.last_object_tag.clone(),
            last_player_filter: self.last_player_filter.clone(),
            iterated_player: self.iterated_player,
            auto_tag_object_targets: self.auto_tag_object_targets,
            force_auto_tag_object_targets: self.force_auto_tag_object_targets,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn apply_reference_frame(&mut self, frame: LoweringReferenceFrame) {
        self.last_effect_id = frame.last_effect_id;
        self.last_object_tag = frame.last_object_tag;
        self.last_player_filter = frame.last_player_filter;
        self.iterated_player = frame.iterated_player;
        self.allow_life_event_value = frame.allow_life_event_value;
        self.bind_unbound_x_to_last_effect = frame.bind_unbound_x_to_last_effect;
    }

    pub(crate) fn apply_lowering_frame(&mut self, frame: LoweringFrame) {
        self.last_effect_id = frame.last_effect_id;
        self.last_object_tag = frame.last_object_tag;
        self.last_player_filter = frame.last_player_filter;
        self.iterated_player = frame.iterated_player;
        self.auto_tag_object_targets = frame.auto_tag_object_targets;
        self.force_auto_tag_object_targets = frame.force_auto_tag_object_targets;
        self.allow_life_event_value = frame.allow_life_event_value;
        self.bind_unbound_x_to_last_effect = frame.bind_unbound_x_to_last_effect;
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

impl ReferenceFrameSource for CompileContext {
    fn reference_frame(&self) -> LoweringReferenceFrame {
        CompileContext::reference_frame(self)
    }
}

impl ReferenceFrameSource for LoweringReferenceFrame {
    fn reference_frame(&self) -> LoweringReferenceFrame {
        self.clone()
    }
}
