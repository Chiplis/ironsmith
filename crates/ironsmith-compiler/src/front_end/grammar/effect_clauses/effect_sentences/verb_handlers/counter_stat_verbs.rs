use crate::grammar::effects::counter_stat_shapes as counter_grammar;

const COUNTER_TARGET_WORDS: &[&str] = &["target", "targets"];
const COUNTER_FROM_WORD: &str = "from";
const COUNTER_AND_OR_WORDS: &[&str] = &["and", "or", "and/or"];
const COUNTER_YOU_CONTROL_PREFIXES: &[&[&str]] = &[&["you", "control"], &["you", "controls"]];
const COUNTER_YOU_DONT_CONTROL_PREFIXES: &[&[&str]] = &[
    &["you", "dont", "control"],
    &["you", "don't", "control"],
    &["you", "do", "not", "control"],
];
const COUNTER_OPPONENTS_CONTROL_PREFIXES: &[&[&str]] = &[
    &["your", "opponents", "control"],
    &["your", "opponents", "controls"],
    &["opponents", "control"],
    &["opponents", "controls"],
    &["an", "opponent", "controls"],
    &["opponent", "controls"],
];
const COUNTER_ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const COUNTER_ACTIVATED_OR_TRIGGERED_ABILITY_PREFIX: &[&str] =
    &["activated", "or", "triggered", "ability"];
const COUNTER_TRIGGERED_OR_ACTIVATED_ABILITY_PREFIX: &[&str] =
    &["triggered", "or", "activated", "ability"];
const COUNTER_ACTIVATED_ABILITY_PREFIX: &[&str] = &["activated", "ability"];
const COUNTER_TRIGGERED_ABILITY_PREFIX: &[&str] = &["triggered", "ability"];
const COUNTER_ABILITY_PREFIXES: &[&[&str]] = &[&["ability"], &["abilities"]];
const COUNTER_ABILITY_MARKER_WORDS: &[&str] = &["ability", "abilities"];
const COUNTER_ACTIVATED_OR_TRIGGERED_MARKER_WORDS: &[&str] = &["activated", "triggered"];
const COUNTER_SPELL_WORD: &str = "spell";
const COUNTER_INSTANT_SPELL_PREFIX: &[&str] = &["instant", "spell"];
const COUNTER_SORCERY_SPELL_PREFIX: &[&str] = &["sorcery", "spell"];
const COUNTER_LEGENDARY_SPELL_PREFIX: &[&str] = &["legendary", "spell"];
const COUNTER_NONCREATURE_SPELL_PREFIX: &[&str] = &["noncreature", "spell"];
const COUNTER_COLORLESS_SPELL_PREFIX: &[&str] = &["colorless", "spell"];
const COUNTER_ARTICLE_WORDS: &[&str] = &["a", "an", "the"];
const COUNTER_SOURCE_OR_SOURCES_WORDS: &[&str] = &["source", "sources"];
const PARTY_SIZE_EQUAL_TO_PREFIX: &[&str] = &[
    "equal",
    "to",
    "the",
    "number",
    "of",
    "creatures",
    "in",
    "your",
    "party",
];
const THAT_MANY_TOP_CARDS_PREFIXES: &[&[&str]] = &[
    &["that", "many", "cards", "from", "the", "top", "of"],
    &["that", "many", "cards", "from", "top", "of"],
];
const TOP_THE_TOP_PREFIXES: &[&[&str]] = &[&["the", "top"], &["top"]];
const TOP_LIBRARY_ZONE_WORDS: &[&str] = &["library", "libraries"];
const WHERE_X_IS_PREFIX: &[&str] = &["where", "x", "is"];
const REVEAL_CARD_WORDS: &[&str] = &["card", "cards"];
const REVEAL_HAND_WORD: &str = "hand";
const REVEAL_CARDS_WORD: &str = "cards";
const REVEAL_HAND_WORDS: &[&str] = &[REVEAL_HAND_WORD];
const REVEAL_CARDS_WORDS: &[&str] = &[REVEAL_CARDS_WORD];
const THAT_MUCH_LIFE_WORDS: &[&str] = &["that", "much", "life"];
const EQUAL_WORD: &str = "equal";
const EQUAL_TO_PREFIX: &[&str] = &["equal", "to"];
const FOR_EACH_PREFIX: &[&str] = &["for", "each"];
const LIFE_EQUAL_TO_PREFIX: &[&str] = &["life", "equal", "to"];
const THE_WORD: &str = "the";
const LIFE_WORD: &str = "life";

fn counter_prefix_at(words: &[&str], idx: usize, prefixes: &[&[&str]]) -> bool {
    counter_grammar::parse_prefix_at(words, idx, prefixes).is_some()
}

fn counter_token_prefix_at(tokens: &[OwnedLexToken], idx: usize, prefix: &[&str]) -> bool {
    counter_grammar::parse_token_prefix_at(tokens, idx, prefix).is_some()
}

fn counter_word_choice(word: &str, choices: &[&str]) -> bool {
    counter_grammar::parse_word(word, choices).is_some()
}

fn counter_has_any_choice(words: &[&str], choices: &[&str]) -> bool {
    counter_grammar::find_word(words, choices).is_some()
}

fn generic_mana_amount_from_group(group: &[ManaSymbol]) -> Option<i32> {
    let [ManaSymbol::Generic(amount)] = group else {
        return None;
    };
    Some(*amount as i32)
}

fn generic_mana_amount_from_symbol(symbol: ManaSymbol) -> Option<i32> {
    match symbol {
        ManaSymbol::Generic(amount) => Some(amount as i32),
        _ => None,
    }
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        [
            "all",
            "spells",
            "your",
            "opponents",
            "control",
            "and",
            "all",
            "abilities",
            "your",
            "opponents",
            "control"
        ]
    ) {
        let mut spells = ObjectFilter::spell();
        spells.controller = Some(PlayerFilter::Opponent);
        let mut abilities = ObjectFilter::ability();
        abilities.controller = Some(PlayerFilter::Opponent);
        let mut union = ObjectFilter::default();
        union.any_of = vec![spells, abilities];
        union.set_union_connective(crate::filter::ObjectFilterUnionConnective::Or);
        union.set_conjunctive_set_surface(true);
        return Ok(TargetAst::Object(union, None, None));
    }
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    let words = TokenWordView::new(tokens).word_refs();
    if counter_has_any_choice(&words, COUNTER_ABILITY_MARKER_WORDS)
        && counter_has_any_choice(&words, COUNTER_ACTIVATED_OR_TRIGGERED_MARKER_WORDS)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    let mut target = parse_target_phrase(tokens)?;
    preserve_spell_kind_on_counter_target(&mut target);
    Ok(target)
}

fn preserve_spell_kind_on_counter_target(target: &mut TargetAst) {
    match target {
        TargetAst::Object(filter, ..) => {
            // Once ability targets have been routed above, a typed object in a
            // counter instruction is a spell on the stack. Generic object
            // parsing preserves the Stack zone but otherwise loses this kind.
            // A non-Stack zone on an already typed spell is positive cast-
            // origin provenance ("spell cast from a graveyard"), not the
            // object's current zone; retain it for legality and rendering.
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Stack);
            }
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            preserve_spell_kind_on_counter_target(inner);
        }
        _ => {}
    }
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let mut clause_tokens = trim_commas(tokens);
    if clause_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some("counter"))
    {
        clause_tokens.drain(..1);
    }
    let clause_words = crate::lexer::token_word_refs(&clause_tokens);
    let is_controller_tail = |idx: usize| {
        counter_prefix_at(&clause_words, idx, COUNTER_YOU_CONTROL_PREFIXES)
            || counter_prefix_at(&clause_words, idx, COUNTER_YOU_DONT_CONTROL_PREFIXES)
            || counter_prefix_at(&clause_words, idx, COUNTER_OPPONENTS_CONTROL_PREFIXES)
    };
    if !counter_has_any_choice(&clause_words, COUNTER_ABILITY_MARKER_WORDS) {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut target_count: Option<ChoiceCount> = None;
    if let Some((count, used)) = parse_choice_count_before_target_prefix(&clause_tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    }

    let explicit_target = clause_tokens.get(idx).is_some_and(|token| {
        token
            .as_word()
            .is_some_and(|word| counter_word_choice(word, COUNTER_TARGET_WORDS))
    });
    if explicit_target
        || clause_tokens.get(idx).is_some_and(|token| {
            token
                .as_word()
                .is_some_and(|word| counter_word_choice(word, COUNTER_ALL_OR_EACH_WORDS))
        })
    {
        idx += 1;
    } else {
        return Ok(None);
    }

    #[derive(Clone, Copy)]
    enum CounterTargetTerm {
        Ability,
        Spell,
    }

    let mut term_filters: Vec<(ObjectFilter, CounterTargetTerm)> = Vec::new();
    let mut saw_and_or_connective = false;
    let mut list_end = clause_tokens.len();
    let mut scan = idx;
    while scan < clause_tokens.len() {
        if clause_tokens
            .get(scan)
            .is_some_and(|token| token.as_word() == Some(COUNTER_FROM_WORD))
        {
            list_end = scan;
            break;
        }
        if is_controller_tail(scan) {
            list_end = scan;
            break;
        }
        scan += 1;
    }

    while idx < list_end {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if counter_word_choice(word, COUNTER_AND_OR_WORDS) {
            saw_and_or_connective |= word == "and/or";
            idx += 1;
            continue;
        }

        if counter_token_prefix_at(
            &clause_tokens,
            idx,
            COUNTER_ACTIVATED_OR_TRIGGERED_ABILITY_PREFIX,
        ) {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 4;
            continue;
        }

        if counter_token_prefix_at(
            &clause_tokens,
            idx,
            COUNTER_TRIGGERED_OR_ACTIVATED_ABILITY_PREFIX,
        ) {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 4;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_ACTIVATED_ABILITY_PREFIX) {
            term_filters.push((
                ObjectFilter::activated_ability(),
                CounterTargetTerm::Ability,
            ));
            idx += 2;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_TRIGGERED_ABILITY_PREFIX) {
            let mut triggered = ObjectFilter::ability();
            triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
            term_filters.push((triggered, CounterTargetTerm::Ability));
            idx += 2;
            continue;
        }

        if counter_prefix_at(&clause_words, idx, COUNTER_ABILITY_PREFIXES) {
            term_filters.push((ObjectFilter::ability(), CounterTargetTerm::Ability));
            idx += 1;
            continue;
        }

        if word == COUNTER_SPELL_WORD || word == "spells" {
            term_filters.push((ObjectFilter::spell(), CounterTargetTerm::Spell));
            idx += 1;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_INSTANT_SPELL_PREFIX) {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Instant),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_SORCERY_SPELL_PREFIX) {
            term_filters.push((
                ObjectFilter::spell().with_type(CardType::Sorcery),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_LEGENDARY_SPELL_PREFIX) {
            term_filters.push((
                ObjectFilter::spell().with_supertype(Supertype::Legendary),
                CounterTargetTerm::Spell,
            ));
            idx += 2;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_NONCREATURE_SPELL_PREFIX) {
            let mut filter = ObjectFilter::noncreature_spell().in_zone(Zone::Stack);
            filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
            term_filters.push((filter, CounterTargetTerm::Spell));
            idx += 2;
            continue;
        }

        if counter_token_prefix_at(&clause_tokens, idx, COUNTER_COLORLESS_SPELL_PREFIX) {
            term_filters.push((ObjectFilter::spell().colorless(), CounterTargetTerm::Spell));
            idx += 2;
            continue;
        }

        // "that targets <object filter>" — the countered object's own target
        // scope, shared by every term ("spell or ability that targets a
        // permanent you control").
        if word == "that"
            && clause_tokens.get(idx + 1).and_then(OwnedLexToken::as_word) == Some("targets")
            && !term_filters.is_empty()
        {
            let targets_only =
                clause_tokens.get(idx + 2).and_then(OwnedLexToken::as_word) == Some("only");
            let filter_start = idx + if targets_only { 3 } else { 2 };
            let filter_tokens: Vec<OwnedLexToken> = clause_tokens[filter_start..].to_vec();
            if let Ok((target_player, target_object, targets_any_of)) =
                crate::keyword_static::parse_cost_modifier_target_spec(
                    &filter_tokens,
                )
            {
                for (filter, _) in &mut term_filters {
                    if targets_only {
                        filter.targets_only_player = target_player.clone();
                        filter.targets_only_object = target_object.clone();
                        filter.targets_only_any_of = targets_any_of;
                    } else {
                        filter.targets_player = target_player.clone();
                        filter.targets_object = target_object.clone();
                        filter.targets_any_of = targets_any_of;
                    }
                }
                idx = clause_tokens.len();
                break;
            }
            return Ok(None);
        }

        return Ok(None);
    }

    if term_filters.is_empty() {
        return Ok(None);
    }

    let mut source_types: Vec<CardType> = Vec::new();
    let mut controller_filter: Option<PlayerFilter> = None;
    while idx < clause_tokens.len() {
        let Some(word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word) else {
            idx += 1;
            continue;
        };
        if counter_word_choice(word, COUNTER_AND_OR_WORDS) {
            idx += 1;
            continue;
        }
        if counter_prefix_at(&clause_words, idx, COUNTER_YOU_CONTROL_PREFIXES) {
            controller_filter = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if counter_prefix_at(&clause_words, idx, COUNTER_YOU_DONT_CONTROL_PREFIXES) {
            controller_filter = Some(PlayerFilter::NotYou);
            idx += if clause_tokens
                .get(idx + 1)
                .is_some_and(|token| token.as_word() == Some("do"))
            {
                4
            } else {
                3
            };
            continue;
        }
        if counter_prefix_at(&clause_words, idx, COUNTER_OPPONENTS_CONTROL_PREFIXES) {
            controller_filter = Some(PlayerFilter::Opponent);
            idx += if clause_tokens.get(idx).is_some_and(|token| {
                matches!(token.as_word(), Some("your" | "an"))
            })
            {
                3
            } else {
                2
            };
            continue;
        }
        // A controller qualifier may precede the shared target relation:
        // "spell or ability an opponent controls that targets a land you
        // control". The term loop stops at the controller tail, so retain
        // the remaining relation here and apply it to every parsed Stack
        // branch rather than letting the generic spell fallback claim only
        // the first term.
        if word == "that"
            && clause_tokens.get(idx + 1).and_then(OwnedLexToken::as_word) == Some("targets")
        {
            let targets_only =
                clause_tokens.get(idx + 2).and_then(OwnedLexToken::as_word) == Some("only");
            let filter_start = idx + if targets_only { 3 } else { 2 };
            let filter_tokens = clause_tokens[filter_start..].to_vec();
            let Ok((target_player, target_object, targets_any_of)) =
                crate::keyword_static::parse_cost_modifier_target_spec(
                    &filter_tokens,
                )
            else {
                return Ok(None);
            };
            for (filter, _) in &mut term_filters {
                if targets_only {
                    filter.targets_only_player = target_player.clone();
                    filter.targets_only_object = target_object.clone();
                    filter.targets_only_any_of = targets_any_of;
                } else {
                    filter.targets_player = target_player.clone();
                    filter.targets_object = target_object.clone();
                    filter.targets_any_of = targets_any_of;
                }
            }
            idx = clause_tokens.len();
            continue;
        }
        if word == COUNTER_FROM_WORD {
            idx += 1;
            if clause_tokens.get(idx).is_some_and(|token| {
                token
                    .as_word()
                    .is_some_and(|word| counter_word_choice(word, COUNTER_ARTICLE_WORDS))
            }) {
                idx += 1;
            }

            let mut parsed_type = false;
            while idx < clause_tokens.len() {
                let Some(type_word) = clause_tokens.get(idx).and_then(OwnedLexToken::as_word)
                else {
                    idx += 1;
                    continue;
                };
                if counter_word_choice(type_word, COUNTER_SOURCE_OR_SOURCES_WORDS) {
                    idx += 1;
                    break;
                }
                if counter_word_choice(type_word, COUNTER_AND_OR_WORDS) {
                    idx += 1;
                    continue;
                }
                let Some(card_type) = counter_grammar::parse_source_card_type(type_word) else {
                    return Ok(None);
                };
                source_types.push(card_type);
                parsed_type = true;
                idx += 1;
            }
            if !parsed_type {
                return Ok(None);
            }
            continue;
        }

        return Ok(None);
    }

    for (filter, term) in &mut term_filters {
        if let Some(controller) = controller_filter.clone() {
            let mut updated = filter.clone();
            updated.controller = Some(controller);
            *filter = updated;
        }
        if !source_types.is_empty() && matches!(term, CounterTargetTerm::Ability) {
            for card_type in &source_types {
                *filter = filter.clone().with_type(*card_type);
            }
        }
    }

    let target_filter = if term_filters.len() == 1 {
        term_filters
            .pop()
            .map(|(filter, _)| filter)
            .expect("single term filter should be present")
    } else {
        let mut any = ObjectFilter::default();
        any.any_of = term_filters.into_iter().map(|(filter, _)| filter).collect();
        if saw_and_or_connective {
            any = any.with_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        }
        any
    };

    let target = wrap_target_count(
        TargetAst::Object(
            target_filter,
            explicit_target
                .then(|| span_from_tokens(&clause_tokens))
                .flatten(),
            None,
        ),
        target_count,
    );
    Ok(Some(target))
}

pub(crate) fn scale_value_multiplier(value: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return value;
    }
    match value {
        Value::Fixed(amount) => Value::Fixed(amount * multiplier),
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => Value::Scaled(Box::new(other), multiplier),
    }
}

pub(crate) fn parse_counter_unless_additional_generic_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let Some(shape) = counter_grammar::parse_additional_payment_head(tokens) else {
        return Ok(None);
    };

    let multiplier = {
        let token = shape.multiplier_token;
        if let Some(group) = mana_pips_from_token(token) {
            generic_mana_amount_from_group(&group).ok_or_else(|| {
                CardTextError::ParseError(
                    "unsupported nongeneric additional counter payment".to_string(),
                )
            })?
        } else {
            let symbol_word = token.as_word().ok_or_else(|| {
                CardTextError::ParseError("missing additional mana symbol".to_string())
            })?;
            let symbol = parse_mana_symbol(symbol_word).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported additional payment symbol '{}' in counter clause",
                    symbol_word
                ))
            })?;
            generic_mana_amount_from_symbol(symbol).ok_or_else(|| {
                CardTextError::ParseError(
                    "unsupported nongeneric additional counter payment".to_string(),
                )
            })?
        }
    };

    let filter_tokens = trim_commas(shape.filter_tokens);
    let filter_words = crate::lexer::token_word_refs(&filter_tokens);
    if counter_grammar::parse_prefix(&filter_words, &[FOR_EACH_PREFIX]).is_none() {
        return Err(CardTextError::ParseError(format!(
            "unsupported additional counter payment tail (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    let dynamic = parse_dynamic_cost_modifier_value(&filter_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported additional counter payment filter (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        ))
    })?;
    Ok(Some(scale_value_multiplier(dynamic, multiplier)))
}

pub(crate) fn parse_reveal(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let words = crate::lexer::token_word_refs(tokens);
    // Revealing every card in a library is a collection operation, not the
    // ordinary singular `RevealTop` fallback. Tag the exact zone contents so
    // later chooser and movement clauses can consume the same stable set.
    if matches!(
        words.as_slice(),
        ["cards", "in", "your", "library"]
            | ["the", "cards", "in", "your", "library"]
            | ["reveal", "the", "cards", "in", "your", "library"]
    )
    {
        let tag = TagKey::from("__revealed_library__");
        let filter = ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::You);
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_tag_matching_objects(
                    filter,
                    vec![Zone::Library],
                    tag.clone(),
                ),
                EffectAst::subject_verb_reveal_tagged(tag),
            ],
        });
    }
    // Many effects split "reveal it/that card/those cards" into a standalone clause.
    // The engine does not model hidden information, so this compiles to a semantic no-op
    // that still allows parsing and auditing to proceed.
    if counter_grammar::parse_reveal_reference(tokens).is_some() {
        return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
    }
    if counter_grammar::find_word(&words, REVEAL_HAND_WORDS).is_some() {
        let is_full_hand_reveal = counter_grammar::parse_reveal_full_hand(tokens).is_some();
        if !is_full_hand_reveal {
            if counter_grammar::find_from_preposition(tokens).is_some() {
                if let Some(equal_idx) = counter_grammar::find_word(&words, &[EQUAL_WORD]) {
                    let tail = &words[equal_idx..];
                    let equal_token_idx = counter_grammar::word_token_boundary(tokens, equal_idx)
                        .unwrap_or(equal_idx);
                    let parsed_expression = counter_grammar::parse_prefix(tail, &[EQUAL_TO_PREFIX])
                        .and_then(|prefix| {
                            counter_grammar::word_token_boundary(tokens, equal_idx + prefix.end)
                        })
                        .and_then(|value_token_idx| parse_value(&tokens[value_token_idx..]))
                        .map(|(value, _)| {
                            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
                        });
                    let count_value =
                        if counter_grammar::parse_prefix(tail, &[PARTY_SIZE_EQUAL_TO_PREFIX])
                            .is_some()
                        {
                            Some(Value::PartySize(PlayerFilter::You))
                        } else if let Some(value) =
                            parse_devotion_value_from_add_clause(&tokens[equal_token_idx..])?
                        {
                            Some(value)
                        } else if let Some(value) =
                            parse_equal_to_number_of_filter_value(&tokens[equal_token_idx..])
                        {
                            Some(value)
                        } else if parsed_expression.is_some() {
                            parsed_expression
                        } else {
                            parse_dynamic_cost_modifier_value(&tokens[equal_token_idx..])?
                        };
                    if let Some(count_value) = count_value
                        && counter_grammar::find_word(&words, REVEAL_CARD_WORDS).is_some()
                        && counter_grammar::find_reveal_hand_source(tokens).is_some()
                    {
                        return Ok(EffectAst::subject_verb_reveal_cards_from_hand(
                            player,
                            ChoiceCount::dynamic_x(),
                            Some(count_value),
                            TagKey::from(IT_TAG),
                        ));
                    }
                }
                if let Some((count_value, _used)) = parse_value(tokens)
                    && matches!(
                        count_value.unhinted(),
                        Value::EventValue(EventValueSpec::Amount)
                    )
                    && counter_grammar::find_word(&words, REVEAL_CARDS_WORDS).is_some()
                    && counter_grammar::find_reveal_hand_source(tokens).is_some()
                {
                    return Ok(EffectAst::subject_verb_reveal_cards_from_hand(
                        player,
                        ChoiceCount::dynamic_x(),
                        Some(count_value),
                        TagKey::from(IT_TAG),
                    ));
                }
                if matches!(parse_value(tokens), Some((Value::X, _)))
                    && counter_grammar::find_word(&words, REVEAL_CARDS_WORDS).is_some()
                    && counter_grammar::find_reveal_hand_source(tokens).is_some()
                {
                    let count_value = counter_grammar::find_phrase(&words, &[WHERE_X_IS_PREFIX])
                        .and_then(|shape| counter_grammar::word_token_boundary(tokens, shape.start))
                        .and_then(|where_token_idx| {
                            parse_value_binding_clause(&tokens[where_token_idx..])
                        })
                        .map(|value| {
                            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
                        });
                    return Ok(EffectAst::subject_verb_reveal_cards_from_hand(
                        player,
                        ChoiceCount::dynamic_x(),
                        count_value,
                        TagKey::from(IT_TAG),
                    ));
                }
                if let Some((count, _used)) = parse_number(tokens)
                    && counter_grammar::find_word(&words, REVEAL_CARDS_WORDS).is_some()
                    && counter_grammar::find_reveal_hand_source(tokens).is_some()
                {
                    return Ok(EffectAst::subject_verb_reveal_cards_from_hand(
                        player,
                        ChoiceCount::exactly(count as usize),
                        None,
                        TagKey::from(IT_TAG),
                    ));
                }
                return Ok(EffectAst::subject_verb_reveal_tagged(TagKey::from(IT_TAG)));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported reveal-hand clause (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(EffectAst::subject_verb_reveal_hand(player));
    }

    let has_card = counter_grammar::find_word(&words, REVEAL_CARD_WORDS).is_some();
    let has_library = counter_grammar::find_word(&words, TOP_LIBRARY_ZONE_WORDS).is_some();
    let top_library_source = counter_grammar::parse_top_library_source(tokens).is_some();
    let explicit_top_card = counter_grammar::parse_explicit_top_card(tokens).is_some()
        || (top_library_source
            || (counter_grammar::parse_prefix(&words, TOP_THE_TOP_PREFIXES).is_some()
                && has_card
                && has_library));
    let top_library_reveal = top_library_source
        || counter_grammar::parse_prefix(&words, TOP_THE_TOP_PREFIXES).is_some_and(|_| has_library);

    if (!has_card && !top_library_reveal)
        || (!has_library && !explicit_top_card && !top_library_reveal)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let player = match counter_grammar::parse_top_library_owner(tokens) {
        // "of their library" / "of that player's library" back-references the
        // clause's own explicit subject; keep it ("Target player reveals the
        // top four cards of their library" — Bamboozle).
        Some(PlayerAst::That) if !matches!(player, PlayerAst::Implicit) => player,
        Some(owner) => owner,
        None => player,
    };

    if counter_grammar::parse_prefix(&words, THAT_MANY_TOP_CARDS_PREFIXES).is_some() {
        return Ok(EffectAst::subject_verb_reveal_top_cards(
            player,
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::Count,
            },
            TagKey::from(IT_TAG),
        ));
    }

    let top_prefix = counter_grammar::parse_prefix(&words, TOP_THE_TOP_PREFIXES);
    if let Some(prefix) = top_prefix
        && let Some(count_token_idx) = counter_grammar::word_token_boundary(tokens, prefix.end)
        && let Some((mut count, used)) = parse_value(&tokens[count_token_idx..])
    {
        let after_count = &tokens[count_token_idx + used..];
        let top_library_tail = counter_grammar::parse_library_tail(after_count).is_some();
        if top_library_tail {
            if count == Value::X
                && let Some(where_word_idx) =
                    counter_grammar::find_phrase(&words, &[WHERE_X_IS_PREFIX])
                        .map(|shape| shape.start)
                && let Some(where_token_idx) =
                    counter_grammar::word_token_boundary(tokens, where_word_idx)
                && let Some(where_value) =
                    parse_prior_effect_count_binding_clause(&tokens[where_token_idx..])
                        .or_else(|| parse_value_binding_clause(&tokens[where_token_idx..]))
            {
                count = where_value;
            }
            if count != Value::Fixed(1) {
                return Ok(EffectAst::subject_verb_reveal_top_cards(
                    player,
                    count,
                    TagKey::from(IT_TAG),
                ));
            }
        }
    }

    Ok(EffectAst::subject_verb_reveal_top(player))
}

fn parse_prior_effect_count_binding_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let source = counter_grammar::parse_prior_effect_count_source(trim_commas(tokens).as_slice())?;
    Some(Value::PendingEffectMetric {
        source,
        metric: ironsmith_core::EffectMetric::Count,
    })
}

pub(crate) fn parse_life_amount(
    tokens: &[OwnedLexToken],
    amount_kind: &str,
) -> Result<(Value, usize), CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if counter_grammar::parse_exact(&clause_words, &[THAT_MUCH_LIFE_WORDS]).is_some() {
        // "that much life" binds to the triggering event amount.
        return Ok((Value::EventValue(EventValueSpec::Amount), 2));
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing {amount_kind} amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

pub(crate) fn parse_life_equal_to_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if counter_grammar::parse_prefix(&clause_words, &[LIFE_EQUAL_TO_PREFIX]).is_none() {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let amount_words = crate::lexer::token_word_refs(amount_tokens);

    if let Some(value) = parse_add_mana_equal_amount_value(amount_tokens) {
        return Ok(Some(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ));
    }
    if let Some(value) = parse_devotion_value_from_add_clause(amount_tokens)? {
        return Ok(Some(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ));
    }
    if let Some(value) = parse_equal_to_number_of_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_equal_to_aggregate_filter_value(amount_tokens) {
        return Ok(Some(value));
    }
    if let Some(surface) = counter_grammar::parse_life_equal_surface(&amount_words) {
        let value = match surface {
            counter_grammar::LifeEqualSurface::LifeLostThisWay => {
                Value::EventValue(EventValueSpec::LifeAmount)
            }
            counter_grammar::LifeEqualSurface::DamagePreventedThisWay => {
                Value::EventValue(EventValueSpec::Amount)
            }
            counter_grammar::LifeEqualSurface::AllPlayersLifeLostThisTurn => {
                Value::LifeLostThisTurn(PlayerFilter::Any)
            }
            counter_grammar::LifeEqualSurface::IteratedPlayerLifeLostThisTurn => {
                Value::LifeLostThisTurn(PlayerFilter::IteratedPlayer)
            }
            counter_grammar::LifeEqualSurface::TargetPlayerDamageThisTurn => {
                Value::DamageDealtToPlayersThisTurn(PlayerFilter::target_player())
            }
        };
        return Ok(Some(value));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(amount_tokens)? {
        return Ok(Some(value));
    }
    if counter_grammar::parse_prefix(&amount_words, &[EQUAL_TO_PREFIX]).is_some() {
        let value_tokens = &amount_tokens[2..];
        let mut value_words = crate::lexer::token_word_refs(value_tokens);

        let parse_stat_of_target =
            |stat_words: &[&str], constructor: fn(Box<ChooseSpec>) -> Value| {
                if counter_grammar::parse_prefix(&value_words, &[stat_words]).is_some() {
                    let target_tokens = &value_tokens[stat_words.len()..];
                    if let Ok(target) = parse_target_phrase(target_tokens) {
                        let spec = crate::reference_helpers::choose_spec_for_target(&target);
                        return Some(constructor(Box::new(spec)));
                    }
                }
                None
            };
        if let Some(value) = parse_stat_of_target(&["power", "of"], Value::PowerOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["the", "power", "of"], Value::PowerOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["toughness", "of"], Value::ToughnessOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["the", "toughness", "of"], Value::ToughnessOf) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_stat_of_target(&["mana", "value", "of"], Value::ManaValueOf) {
            return Ok(Some(value));
        }
        if let Some(value) =
            parse_stat_of_target(&["the", "mana", "value", "of"], Value::ManaValueOf)
        {
            return Ok(Some(value));
        }
        if let Some(value) = parse_possessive_target_stat_value(value_tokens) {
            return Ok(Some(value));
        }
        if let Some(value) = parse_life_total_as_turn_began_value(&value_words) {
            return Ok(Some(value));
        }

        if let Some((value, used)) = parse_value(value_tokens)
            && used == value_tokens.len()
        {
            return Ok(Some(value));
        }
        if value_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word == THE_WORD)
        {
            let stripped_tokens = &value_tokens[1..];
            if let Some((value, used)) = parse_value(stripped_tokens)
                && used == stripped_tokens.len()
            {
                return Ok(Some(value));
            }
            value_words = crate::lexer::token_word_refs(stripped_tokens);
        }
        for (prefix, stat_words) in [
            (&["power", "of"][..], &["power"][..]),
            (&["toughness", "of"][..], &["toughness"][..]),
            (&["mana", "value", "of"][..], &["mana", "value"][..]),
        ] {
            if counter_grammar::parse_prefix(&value_words, &[prefix]).is_some() {
                let mut reordered = value_words[prefix.len()..].to_vec();
                reordered.extend_from_slice(stat_words);
                if let Some((value, used)) =
                    crate::util::parse_value_expr_words(
                        &reordered,
                    )
                    && used == reordered.len()
                {
                    return Ok(Some(value));
                }
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "missing life amount in equal-to clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_possessive_target_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let shape = counter_grammar::parse_possessive_target_stat(tokens)?;
    let constructor: fn(Box<ChooseSpec>) -> Value = match shape.stat {
        counter_grammar::TargetStatKind::Power => Value::PowerOf,
        counter_grammar::TargetStatKind::Toughness => Value::ToughnessOf,
        counter_grammar::TargetStatKind::ManaValue => Value::ManaValueOf,
    };
    let target = parse_target_phrase(&shape.target_tokens).ok()?;
    let spec =
        crate::reference_helpers::choose_spec_for_target(&target);
    Some(constructor(Box::new(spec)))
}

fn parse_life_total_as_turn_began_value(words: &[&str]) -> Option<Value> {
    counter_grammar::parse_life_total_as_turn_began_words(words)
}

pub(crate) fn parse_life_amount_from_trailing(
    base_amount: &Value,
    trailing: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if trailing.is_empty() {
        return Ok(None);
    }

    // Preserve both terms in authored additive life amounts such as
    // `2 life plus 2 life for each Spirit sacrificed this way`. Looking for a
    // later `each` across the whole suffix would otherwise recover only the
    // scaled dynamic term and silently discard the leading base amount.
    let additive_tail = trim_commas(trailing);
    if additive_tail
        .first()
        .is_some_and(|token| token.is_word("plus"))
    {
        let mut addend_tokens = &additive_tail[1..];
        if addend_tokens
            .first()
            .is_some_and(|token| token.is_word("additional"))
        {
            addend_tokens = &addend_tokens[1..];
        } else if addend_tokens
            .first()
            .is_some_and(|token| token.is_word("an"))
            && addend_tokens
                .get(1)
                .is_some_and(|token| token.is_word("additional"))
        {
            addend_tokens = &addend_tokens[2..];
        }
        let (addend_base, used) = parse_life_amount(addend_tokens, "additive life amount")?;
        let Some(rest) = addend_tokens.get(used..) else {
            return Ok(None);
        };
        if !rest.first().is_some_and(|token| token.is_word(LIFE_WORD)) {
            return Ok(None);
        }
        let addend_trailing = trim_commas(&rest[1..]);
        let addend = if addend_trailing.is_empty() {
            addend_base
        } else {
            let Some(resolved) = parse_life_amount_from_trailing(&addend_base, &addend_trailing)?
            else {
                return Ok(None);
            };
            resolved
        };
        return Ok(Some(Value::Add(
            Box::new(base_amount.clone()),
            Box::new(addend),
        )));
    }

    // Prefer the generic tagged-count representation for prior-action
    // correlations. Unlike a bare pending effect count, this preserves noun
    // restrictions such as "land card discarded this way" and resolves the
    // IT_TAG placeholder to the prior action's affected-object snapshots.
    let trailing_words = crate::lexer::token_word_refs(trailing);
    if let Some((value, used)) =
        crate::grammar::shared_util::count_shapes::parse_for_each_count_value_words(
            &trailing_words,
        )
        && used == trailing_words.len()
        && let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        }
    {
        return Ok(Some(
            scale_value_multiplier(value, multiplier)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }

    if let Some(counter_value) = parse_for_each_counter_on_reference_value(trailing)
        && let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        }
    {
        return Ok(Some(
            scale_value_multiplier(counter_value, multiplier)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(trailing)?
        && let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        } {
            return Ok(Some(scale_value_multiplier(dynamic, multiplier)));
        }

    if let Some(where_value) = parse_value_binding_clause(trailing) {
        if value_contains_unbound_x(base_amount) {
            let clause = crate::lexer::token_word_refs(trailing).join(" ");
            return Ok(Some(replace_unbound_x_with_value(
                base_amount.clone(),
                &where_value,
                &clause,
            )?));
        }
        if matches!(base_amount, Value::Fixed(1)) {
            return Ok(Some(where_value));
        }
    }

    Ok(None)
}

fn parse_for_each_counter_on_reference_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    match counter_grammar::parse_counter_reference(tokens)? {
        counter_grammar::CounterReferenceShape::Source {
            counter_type_tokens,
        } => {
            let counter_type =
                crate::grammar::filters::parse_counter_type_from_tokens(counter_type_tokens);
            Some(match counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            })
        }
        counter_grammar::CounterReferenceShape::Tagged {
            counter_type_tokens,
        } => Some(Value::CountersOn(
            Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            crate::grammar::filters::parse_counter_type_from_tokens(counter_type_tokens),
        )),
    }
}

pub(crate) fn validate_life_keyword(rest: &[OwnedLexToken]) -> Result<(), CardTextError> {
    if rest
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word != LIFE_WORD)
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn remap_source_stat_value_to_it(value: Value) -> Value {
    match value {
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(remap_source_stat_value_to_it(*value)),
            hints,
        },
        Value::PowerOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ToughnessOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::ManaValueOf(spec) if matches!(spec.as_ref(), ChooseSpec::Source) => {
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
        }
        Value::Add(left, right) => Value::Add(
            Box::new(remap_source_stat_value_to_it(*left)),
            Box::new(remap_source_stat_value_to_it(*right)),
        ),
        other => other,
    }
}

fn player_filter_for_life_reference(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Active => Some(PlayerFilter::Active),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::PlayerToYourLeft => Some(PlayerFilter::PlayerToYourLeft),
        PlayerAst::PlayerToYourRight => Some(PlayerFilter::PlayerToYourRight),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::ThatPlayerOrTargetController => None,
        PlayerAst::TriggeringSourceController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering_source"),
        )),
        PlayerAst::ItsController | PlayerAst::ItsOwner | PlayerAst::Enchanted => None,
    }
}

fn parse_half_life_value(tokens: &[OwnedLexToken], player: PlayerAst) -> Option<Value> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let shape = counter_grammar::parse_half_life(&clause_words)?;
    let player_filter = player_filter_for_life_reference(player)?;
    if shape.rounded_down {
        Some(Value::HalfLifeTotalRoundedDown(player_filter))
    } else {
        Some(Value::HalfLifeTotalRoundedUp(player_filter))
    }
}

#[cfg(test)]
mod filtered_prior_action_life_tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn life_multiplier_preserves_discarded_land_filter() {
        let tokens = lex_line("for each land card discarded this way", 0).unwrap();
        let expected = Value::Scaled(
            Box::new(
                Value::PendingPriorEffectMetric(
                    ironsmith_core::PriorEffectMetricQuery::new(
                        ironsmith_core::EffectMetricSource::AffectedObjects,
                        ironsmith_core::EffectMetric::Count,
                    )
                    .with_filter({
                        let mut filter = ObjectFilter::land();
                        filter.set_explicit_card_noun(true);
                        filter
                    })
                    .with_action(ironsmith_core::PriorEffectAction::Discarded),
                )
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::CardsDiscardedThisWay),
            ),
            3,
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        assert_eq!(
            parse_life_amount_from_trailing(&Value::Fixed(3), &tokens).unwrap(),
            Some(expected)
        );
    }
}

#[cfg(test)]
mod counter_spell_target_kind_tests {
    use super::*;
    use crate::lexer::lex_line;

    fn assert_typed_spell_target(text: &str) {
        let target = parse_counter_target_phrase(&lex_line(text, 0).unwrap()).unwrap();
        let TargetAst::Object(filter, ..) = target else {
            panic!("expected typed counter target for {text}");
        };
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
    }

    #[test]
    fn qualified_counter_targets_remain_spells() {
        assert_typed_spell_target("target creature spell with mana value 4 or less");
        assert_typed_spell_target("target spell with mana value 3 or less");
    }

    #[test]
    fn counter_target_keeps_positive_cast_origin_as_spell_provenance() {
        let target = parse_counter_target_phrase(
            &lex_line("target spell cast from a graveyard", 0).unwrap(),
        )
        .unwrap();
        let TargetAst::Object(filter, ..) = target else {
            panic!("expected typed counter target");
        };

        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{filter:#?}"
        );
    }

    #[test]
    fn counter_spell_or_ability_uses_any_matching_target_relation() {
        let target = parse_counter_target_phrase(
            &lex_line(
                "target spell or ability that targets a creature you control",
                0,
            )
            .unwrap(),
        )
        .unwrap();
        let TargetAst::Object(filter, ..) = target else {
            panic!("expected typed counter target");
        };
        assert_eq!(filter.any_of.len(), 2);
        assert!(filter.any_of.iter().all(|branch| {
            branch.targets_object.is_some() && branch.targets_only_object.is_none()
        }));
    }

    #[test]
    fn counter_spell_or_ability_preserves_player_or_object_target_union() {
        let target = parse_counter_target_phrase(
            &lex_line(
                "target spell or ability that targets you or a creature you control",
                0,
            )
            .unwrap(),
        )
        .unwrap();
        let TargetAst::Object(filter, ..) = target else {
            panic!("expected typed counter target");
        };
        assert_eq!(filter.any_of.len(), 2);
        assert!(filter.any_of.iter().all(|branch| {
            branch.targets_player == Some(PlayerFilter::You)
                && branch.targets_object.is_some()
                && branch.targets_any_of
        }));
    }

    #[test]
    fn counter_spell_or_ability_keeps_controller_before_shared_land_target_relation() {
        let target = parse_counter_target_phrase(
            &lex_line(
                "target spell or ability an opponent controls that targets a land you control",
                0,
            )
            .unwrap(),
        )
        .unwrap();
        let TargetAst::Object(filter, ..) = target else {
            panic!("expected typed counter target");
        };
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().all(|branch| {
            branch.controller == Some(PlayerFilter::Opponent)
                && branch.targets_object.as_ref().is_some_and(|target| {
                    target.card_types == [CardType::Land]
                        && target.controller == Some(PlayerFilter::You)
                })
        }));
        assert!(
            filter
                .any_of
                .iter()
                .any(|branch| { branch.stack_kind == Some(crate::filter::StackObjectKind::Spell) })
        );
        assert!(
            filter.any_of.iter().any(|branch| {
                branch.stack_kind == Some(crate::filter::StackObjectKind::Ability)
            })
        );
    }
}

#[cfg(test)]
mod reveal_hand_count_tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn that_many_cards_from_hand_keeps_the_prior_effect_amount() {
        let tokens = lex_line("that many cards from their hand", 0)
            .expect("dependent hand-reveal count should lex");
        let parsed = parse_reveal(&tokens, Some(SubjectAst::Player(PlayerAst::TargetOpponent)))
            .expect("dependent hand-reveal count should parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::TargetOpponent,
                    ..
                },
            action:
                SubjectVerbActionAst::RevealCardsFromHand {
                    count,
                    count_value: Some(count_value),
                    ..
                },
        }) = parsed
        else {
            panic!("expected a typed dependent hand reveal, got {parsed:#?}");
        };

        assert!(count.dynamic_x);
        assert!(matches!(
            count_value.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
        ));
    }

    #[test]
    fn life_payment_reveal_choose_exile_pipeline_stays_typed() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Life-Payment Confessor",
        )
        .card_types(vec![crate::CardType::Creature])
        .parse_text(
            "When this creature enters, pay any amount of life. Target opponent reveals that many cards from their hand. You choose one of them and exile it.",
        )
        .expect("life-payment hand-selection pipeline should parse");
        let debug = format!("{:#?}", definition.abilities);

        assert!(debug.contains("PayAnyLifeEffect"), "{debug}");
        assert!(
            debug.contains("ChooseObjectsEffect")
                && debug.contains("reveal: true")
                && debug.contains("count_value: Some"),
            "{debug}"
        );
        assert!(
            debug.contains("MoveToZoneEffect") && debug.contains("zone: Exile"),
            "{debug}"
        );
        assert!(!debug.contains("RevealTaggedEffect"), "{debug}");
    }

    #[test]
    fn coordinated_all_spells_and_all_abilities_keeps_both_stack_domains() {
        let tokens = lex_line(
            "all spells your opponents control and all abilities your opponents control",
            0,
        )
        .expect("coordinated stack domain should lex");
        let parsed =
            parse_counter_target_phrase(&tokens).expect("coordinated stack domain should parse");
        let TargetAst::Object(filter, None, None) = parsed else {
            panic!("expected one object-filter union, got {parsed:#?}");
        };
        assert_eq!(filter.any_of.len(), 2);
        assert_eq!(
            filter.union_connective(),
            crate::filter::ObjectFilterUnionConnective::Or
        );
        assert!(filter.has_conjunctive_set_surface());
        assert!(filter.any_of.iter().any(|arm| {
            arm.stack_kind == Some(crate::filter::StackObjectKind::Spell)
                && arm.controller == Some(PlayerFilter::Opponent)
        }));
        assert!(filter.any_of.iter().any(|arm| {
            arm.stack_kind == Some(crate::filter::StackObjectKind::Ability)
                && arm.controller == Some(PlayerFilter::Opponent)
        }));
    }
}
