const ATTACH_TAGGED_OBJECT_WORDS: &[&[&str]] = &[&["it"], &["them"]];
const ATTACH_TAGGED_EQUIPMENT_WORDS: &[&[&str]] =
    &[&["that", "equipment"], &["those", "equipment"]];
const ATTACH_TAGGED_AURA_WORDS: &[&[&str]] = &[&["that", "aura"], &["those", "auras"]];
const ATTACH_TAGGED_ARTIFACT_WORDS: &[&[&str]] = &[&["that", "artifact"], &["those", "artifacts"]];
const ATTACH_TAGGED_ENCHANTMENT_WORDS: &[&[&str]] = &[&["that", "enchantment"]];
const ATTACH_IT_WORDS: &[&str] = &["it"];
const ATTACH_TOKEN_TARGET_WORDS: &[&str] = &["the", "token"];
const DAMAGE_EACH_OPPONENT_HAND_SIZE_PREFIX: &[&str] =
    &["damage", "to", "each", "opponent", "equal", "to"];
const DAMAGE_HAND_SIZE_WORDS: &[&str] = &["number", "cards", "hand"];
const COMBAT_EQUAL_TO_WORDS: &[&str] = &["equal", "to"];
const COMBAT_IT_OR_THEM_WORDS: &[&str] = &["it", "them"];
const COMBAT_TO_WORD: &str = "to";
const COMBAT_THE_RESULT_WORDS: &[&str] = &["the", "result"];
const COMBAT_EACH_PLAYER_TARGET_WORDS: &[&[&str]] = &[&["each", "player"], &["each", "players"]];
const COMBAT_EACH_OPPONENT_TARGET_WORDS: &[&[&str]] = &[
    &["each", "opponent"],
    &["each", "opponents"],
    &["each", "other", "player"],
    &["each", "other", "players"],
];
const COMBAT_EACH_OTHER_OPPONENT_TARGET_WORDS: &[&[&str]] = &[
    &["each", "other", "opponent"],
    &["each", "other", "opponents"],
    &["all", "other", "opponents"],
];
const COMBAT_EACH_OR_ALL_HEAD_PREFIXES: &[&[&str]] = &[&["each"], &["all"]];
const COMBAT_THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const COMBAT_ITERATED_PLAYER_CONTROL_PHRASES: &[&[&str]] =
    &[&["they", "control"], &["that", "player", "controls"]];
const COMBAT_AND_EACH_OR_ALL_PHRASES: &[&[&str]] = &[&["and", "each"], &["and", "all"]];
const COMBAT_AND_EACH_PLAYER_PHRASES: &[&[&str]] =
    &[&["and", "each", "player"], &["and", "each", "players"]];
const COMBAT_DAMAGE_WORD: &str = "damage";
const COMBAT_AMONG_WORD: &str = "among";
const COMBAT_TARGET_OR_TARGETS_WORDS: &[&str] = &["target", "targets"];
const COMBAT_PLAYER_OR_PLAYERS_WORDS: &[&str] = &["player", "players"];
const COMBAT_NEGATION_WORDS: &[&str] = &["does", "doesnt", "doesn", "dont", "not"];
const COMBAT_INSTEAD_WORD: &str = "instead";
const COMBAT_IF_WORD: &str = "if";
const COMBAT_EVENLY_WORD: &str = "evenly";
const COMBAT_INSTEAD_TARGET_PHRASES: &[&[&str]] = &[&["instead"]];
const COMBAT_CREATURE_CONTROLLER_TARGET_WORDS: &[&[&str]] = &[
    &["the", "creatures", "controller"],
    &["that", "creatures", "controller"],
    &["the", "creature's", "controller"],
    &["that", "creature's", "controller"],
];
const COMBAT_THE_PLAYER_TARGET_PHRASES: &[&[&str]] = &[&["the", "player"]];
const COMBAT_MAX_SPEED_PHRASE: &[&str] = &["max", "speed"];
const COMBAT_DOES_NOT_PHRASE: &[&str] = &["does", "not"];
const COMBAT_END_OF_COMBAT_TIMINGS: &[&[&str]] = &[
    &["at", "end", "of", "combat"],
    &["at", "the", "end", "of", "combat"],
];
const COMBAT_AS_YOU_CAST_THIS_SPELL_MARKER_WORDS: &[&str] = &["as", "you", "cast", "this", "spell"];
const COMBAT_THIS_TURN_MARKER_WORDS: &[&str] = &["this", "turn"];
const COMBAT_CAST_OR_TURN_MARKER_PHRASES: &[&[&str]] = &[
    COMBAT_AS_YOU_CAST_THIS_SPELL_MARKER_WORDS,
    COMBAT_THIS_TURN_MARKER_WORDS,
];
const COMBAT_WITH_DIFFERENT_POWER_SUFFIXES: &[&[&str]] = &[
    &["with", "different", "powers"],
    &["with", "different", "power"],
];
const COMBAT_AT_WORD: &str = "at";
const COMBAT_OTHER_WORDS: &[&str] = &["another", "other"];

#[derive(Clone, Copy)]
enum AttachTaggedObjectShape {
    Plain,
    Equipment,
    Aura,
    Artifact,
    Enchantment,
}

#[derive(Clone, Copy)]
enum CombatPlayerDamageTarget {
    EachPlayer,
    EachOpponent,
    EachOtherOpponent,
}

#[derive(Clone, Copy)]
enum CombatSimpleDamageTarget {
    DefaultAny,
    CreatureController,
    IteratedPlayer,
}

struct CombatSimpleDamageTargetEntry {
    phrases: &'static [&'static [&'static str]],
    target: CombatSimpleDamageTarget,
}

const COMBAT_SIMPLE_DAMAGE_TARGETS: &[CombatSimpleDamageTargetEntry] = &[
    CombatSimpleDamageTargetEntry {
        phrases: COMBAT_INSTEAD_TARGET_PHRASES,
        target: CombatSimpleDamageTarget::DefaultAny,
    },
    CombatSimpleDamageTargetEntry {
        phrases: COMBAT_CREATURE_CONTROLLER_TARGET_WORDS,
        target: CombatSimpleDamageTarget::CreatureController,
    },
    CombatSimpleDamageTargetEntry {
        phrases: COMBAT_THE_PLAYER_TARGET_PHRASES,
        target: CombatSimpleDamageTarget::IteratedPlayer,
    },
];

fn parse_attach_tagged_object_shape(tokens: &[OwnedLexToken]) -> Option<AttachTaggedObjectShape> {
    let object_words = TokenWordView::new(tokens).word_refs();
    if word_slice_eq_any(&object_words, ATTACH_TAGGED_OBJECT_WORDS) {
        return Some(AttachTaggedObjectShape::Plain);
    }
    if word_slice_eq_any(&object_words, ATTACH_TAGGED_EQUIPMENT_WORDS) {
        return Some(AttachTaggedObjectShape::Equipment);
    }
    if word_slice_eq_any(&object_words, ATTACH_TAGGED_AURA_WORDS) {
        return Some(AttachTaggedObjectShape::Aura);
    }
    if word_slice_eq_any(&object_words, ATTACH_TAGGED_ARTIFACT_WORDS) {
        return Some(AttachTaggedObjectShape::Artifact);
    }
    if word_slice_eq_any(&object_words, ATTACH_TAGGED_ENCHANTMENT_WORDS) {
        return Some(AttachTaggedObjectShape::Enchantment);
    }
    None
}

fn combat_words_contain_word(words: &[&str], expected: &str) -> bool {
    let expected_words = [expected];
    let atoms = [LexPattern::object(
        "word",
        LexCaptureKind::OneOf(&expected_words),
    )];
    LexPattern::new(&atoms)
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("word"))
        .is_some()
}

fn combat_words_contain_all(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|word| combat_words_contain_word(words, word))
}

fn combat_hand_size_damage_tail_contains(tail_words: &[&str]) -> bool {
    combat_words_contain_all(tail_words, DAMAGE_HAND_SIZE_WORDS)
}

fn attach_tagged_filter(shape: AttachTaggedObjectShape) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    match shape {
        AttachTaggedObjectShape::Plain => return None,
        AttachTaggedObjectShape::Equipment => {
            filter.card_types.push(CardType::Artifact);
            filter.subtypes.push(Subtype::Equipment);
        }
        AttachTaggedObjectShape::Aura => {
            filter.card_types.push(CardType::Enchantment);
            filter.subtypes.push(Subtype::Aura);
        }
        AttachTaggedObjectShape::Artifact => {
            filter.card_types.push(CardType::Artifact);
        }
        AttachTaggedObjectShape::Enchantment => {
            filter.card_types.push(CardType::Enchantment);
        }
    }
    filter.zone = Some(Zone::Battlefield);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    Some(filter)
}

fn parse_combat_player_damage_target(
    tokens: &[OwnedLexToken],
    allow_prefix: bool,
) -> Option<CombatPlayerDamageTarget> {
    let target_words = TokenWordView::new(tokens).word_refs();
    let each_player = if allow_prefix {
        word_slice_starts_with_any(&target_words, COMBAT_EACH_PLAYER_TARGET_WORDS)
    } else {
        word_slice_eq_any(&target_words, COMBAT_EACH_PLAYER_TARGET_WORDS)
    };
    if each_player {
        return Some(CombatPlayerDamageTarget::EachPlayer);
    }
    let each_other_opponent = if allow_prefix {
        word_slice_starts_with_any(&target_words, COMBAT_EACH_OTHER_OPPONENT_TARGET_WORDS)
    } else {
        word_slice_eq_any(&target_words, COMBAT_EACH_OTHER_OPPONENT_TARGET_WORDS)
    };
    if each_other_opponent {
        return Some(CombatPlayerDamageTarget::EachOtherOpponent);
    }
    let each_opponent = if allow_prefix {
        word_slice_starts_with_any(&target_words, COMBAT_EACH_OPPONENT_TARGET_WORDS)
    } else {
        word_slice_eq_any(&target_words, COMBAT_EACH_OPPONENT_TARGET_WORDS)
    };
    if each_opponent {
        return Some(CombatPlayerDamageTarget::EachOpponent);
    }
    None
}

fn combat_player_damage_target_effect(
    amount: Value,
    target: CombatPlayerDamageTarget,
) -> EffectAst {
    match target {
        CombatPlayerDamageTarget::EachPlayer => EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        },
        CombatPlayerDamageTarget::EachOpponent => EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        },
        CombatPlayerDamageTarget::EachOtherOpponent => damage_each_other_opponent(amount),
    }
}

fn parse_combat_simple_damage_target(tokens: &[OwnedLexToken]) -> Option<TargetAst> {
    let target_words = TokenWordView::new(tokens).word_refs();
    COMBAT_SIMPLE_DAMAGE_TARGETS.iter().find_map(|entry| {
        if !word_slice_eq_any(&target_words, entry.phrases) {
            return None;
        }
        Some(match entry.target {
            CombatSimpleDamageTarget::DefaultAny => {
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
            }
            CombatSimpleDamageTarget::CreatureController => TargetAst::Player(
                PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(IT_TAG)),
                span_from_tokens(tokens),
            ),
            CombatSimpleDamageTarget::IteratedPlayer => {
                TargetAst::Player(PlayerFilter::IteratedPlayer, span_from_tokens(tokens))
            }
        })
    })
}

fn is_divided_damage_clause(words: &[&str]) -> bool {
    let Some(divided_idx) = words.iter().position(|word| *word == "divided") else {
        return false;
    };
    words[divided_idx + 1..].iter().any(|word| *word == "among")
}

fn damage_each_other_opponent(amount: Value) -> EffectAst {
    EffectAst::ForEachPlayersFiltered {
        filter: PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::DamagedPlayer),
        effects: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    }
}

pub(crate) fn parse_attach_object_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let object_words = crate::runtime_backend::token_word_refs(tokens);
    let object_span = span_from_tokens(tokens);
    if object_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object to attach".to_string(),
        ));
    }

    let is_source_attachment = is_source_reference_words(&object_words)
        || grammar::words_match_any_prefix(tokens, SOURCE_ATTACHMENT_PREFIXES).is_some();
    if is_source_attachment {
        return Ok(TargetAst::Source(object_span));
    }

    if let Some(shape) = parse_attach_tagged_object_shape(tokens) {
        if let Some(tagged_filter) = attach_tagged_filter(shape) {
            return Ok(TargetAst::Object(tagged_filter, object_span, None));
        }
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span));
    }

    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "target")
        && let Some((head_slice, _after_attached_to)) =
            super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
                use winnow::Parser as _;
                super::super::grammar::primitives::phrase(&["attached", "to"]).void()
            })
    {
        let head_tokens = trim_commas(head_slice);
        if !head_tokens.is_empty() {
            return parse_target_phrase(&head_tokens);
        }
    }
    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "target") {
        return parse_target_phrase(tokens);
    }

    if object_words.len() >= 2
        && !word_slice_contains_any_word(&object_words, COMBAT_TARGET_OR_TARGETS_WORDS)
        && object_words
            .iter()
            .all(|word| word.chars().all(|ch| ch.is_ascii_alphanumeric()))
    {
        return Ok(TargetAst::Source(object_span));
    }

    parse_target_phrase(tokens)
}

pub(crate) fn parse_attach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "attach clause missing object and destination".to_string(),
        ));
    }

    if crate::runtime_backend::lexer::token_slice_first_is(tokens, "to") {
        let rest = trim_commas(&tokens[1..]);
        let Some(first) = rest.first() else {
            return Err(CardTextError::ParseError(format!(
                "attach clause missing object or destination (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if first
            .as_word()
            .is_some_and(|word| COMBAT_IT_OR_THEM_WORDS.contains(&word))
        {
            let target_tokens = vec![first.clone()];
            let object_tokens = trim_commas(&rest[1..]);
            if object_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "attach clause missing object or destination (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens));
            let object = parse_attach_object_phrase(&object_tokens)?;
            return Ok(EffectAst::subject_verb_attach(object, target));
        }
    }

    let Some(to_idx) = rfind_index(tokens, |token| token.is_word(COMBAT_TO_WORD)) else {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing destination (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if to_idx == 0 || to_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object_tokens = trim_commas(&tokens[..to_idx]);
    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
    if object_tokens.is_empty() || target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object = parse_attach_object_phrase(&object_tokens)?;
    let object_words = crate::runtime_backend::token_word_refs(&object_tokens);
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if word_slice_eq(&object_words, ATTACH_IT_WORDS)
        && word_slice_eq(&target_words, ATTACH_TOKEN_TARGET_WORDS)
    {
        return Ok(EffectAst::subject_verb_attach(
            TargetAst::Tagged(TagKey::from("triggering"), span_from_tokens(&object_tokens)),
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
        ));
    }
    let target = if matches!(
        parse_attach_tagged_object_shape(&target_tokens),
        Some(AttachTaggedObjectShape::Plain)
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
    } else {
        parse_target_phrase(&target_tokens)?
    };

    Ok(EffectAst::subject_verb_attach(object, target))
}

fn parse_attached_object_reference(tokens: &[OwnedLexToken]) -> Option<TargetAst> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let (tag, noun_words) = match words.as_slice() {
        ["enchanted", rest @ ..] => ("enchanted", rest),
        ["equipped", rest @ ..] => ("equipped", rest),
        _ => return None,
    };
    let noun_words = strip_leading_article_word_refs(noun_words);
    let mut filter = match noun_words {
        ["equipment"] | ["equipments"] => {
            let mut filter = ObjectFilter::permanent();
            filter.card_types.push(CardType::Artifact);
            filter.subtypes.push(Subtype::Equipment);
            filter
        }
        ["artifact"] | ["artifacts"] => {
            let mut filter = ObjectFilter::permanent();
            filter.card_types.push(CardType::Artifact);
            filter
        }
        ["creature"] | ["creatures"] => ObjectFilter::creature(),
        ["enchantment"] | ["enchantments"] => {
            let mut filter = ObjectFilter::permanent();
            filter.card_types.push(CardType::Enchantment);
            filter
        }
        ["land"] | ["lands"] => ObjectFilter::land(),
        ["permanent"] | ["permanents"] => ObjectFilter::permanent(),
        _ => return None,
    }
    .match_tagged(TagKey::from(tag), TaggedOpbjectRelation::IsTaggedObject);
    filter.zone = Some(Zone::Battlefield);
    Some(TargetAst::Object(filter, span_from_tokens(tokens), None))
}

pub(crate) fn parse_unattach(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "unattach clause missing object".to_string(),
        ));
    }

    let object_tokens = trim_commas(tokens);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "unattach clause missing object".to_string(),
        ));
    }

    let object = parse_attached_object_reference(&object_tokens)
        .map(Ok)
        .unwrap_or_else(|| parse_target_phrase(&object_tokens))?;
    Ok(EffectAst::subject_verb_unattach(object))
}

pub(crate) fn parse_deal_damage(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let tokens = if token_slice_first_is_any(tokens, &["deal", "deals"]) {
        &tokens[1..]
    } else {
        tokens
    };
    let tokens =
        if let Some((_, rest)) = grammar::words_match_any_prefix(tokens, ADDITIONAL_PREFIXES) {
            rest
        } else {
            tokens
        };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if word_slice_starts_with(&clause_words, DAMAGE_EACH_OPPONENT_HAND_SIZE_PREFIX)
        && combat_hand_size_damage_tail_contains(
            &clause_words[DAMAGE_EACH_OPPONENT_HAND_SIZE_PREFIX.len()..],
        )
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                Value::CardsInHand(PlayerFilter::IteratedPlayer),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    if is_divided_damage_clause(&clause_words) {
        if let Some((value, used)) = parse_value(tokens) {
            return parse_divided_damage_with_amount(tokens, value, used);
        }
        if let Some(effect) = parse_divided_damage_equal_to_amount(tokens)? {
            return Ok(effect);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage distribution clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_deal_damage_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_deal_damage_to_target_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if let Some((prefix, _)) = grammar::words_match_any_prefix(tokens, EVENT_AMOUNT_PREFIXES) {
        return parse_deal_damage_with_amount(
            tokens,
            Value::EventValue(EventValueSpec::Amount),
            prefix.len(),
        );
    }

    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    if let Some(prefix) = DAMAGE_TO_EACH_OPPONENT_PREFIXES
        .iter()
        .find(|prefix| word_slice_starts_with(&clause_words, prefix))
        && combat_hand_size_damage_tail_contains(&clause_words[prefix.len()..])
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb_damage(
                value,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_divided_damage_equal_to_amount(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !matches!(words.as_slice(), ["damage", "equal", "to", ..]) {
        return Ok(None);
    }
    let Some(divided_word_idx) = words.iter().position(|word| *word == "divided") else {
        return Ok(None);
    };
    let Some(divided_token_idx) = token_index_for_word_index(tokens, divided_word_idx) else {
        return Ok(None);
    };
    let amount_tokens = trim_commas(&tokens[3..divided_token_idx]);
    let Some((amount, used)) = parse_value(&amount_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage amount (clause: '{}')",
            words.join(" ")
        )));
    };
    if used != amount_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage amount (clause: '{}')",
            words.join(" ")
        )));
    }
    let target = parse_divided_damage_target(&tokens[divided_token_idx..])?;
    Ok(Some(EffectAst::subject_verb_distributed_damage(
        amount, target,
    )))
}

pub(crate) fn parse_deal_damage_to_target_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "to"]).is_none() {
        return Ok(None);
    }

    let Some(equal_word_idx) = word_slice_find_phrase_start(&clause_words, COMBAT_EQUAL_TO_WORDS)
    else {
        return Ok(None);
    };
    let Some(equal_token_idx) = token_index_for_word_index(tokens, equal_word_idx) else {
        return Ok(None);
    };

    let mut target_tokens = trim_commas(&tokens[1..equal_token_idx]);
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word(COMBAT_TO_WORD))
    {
        target_tokens.remove(0);
    }
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let amount = parse_add_mana_equal_amount_value(tokens)
        .or(parse_equal_to_aggregate_filter_value(tokens))
        .or(parse_devotion_value_from_add_clause(tokens)?)
        .or(parse_equal_to_number_of_filter_value(tokens))
        .or_else(|| {
            word_slice_eq(
                &crate::runtime_backend::token_word_refs(&tokens[equal_token_idx + 2..]),
                COMBAT_THE_RESULT_WORDS,
            )
            .then_some(Value::EventValue(EventValueSpec::Amount))
        })
        .or(parse_dynamic_cost_modifier_value(tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if let Some(target) = parse_combat_player_damage_target(&target_tokens, false) {
        return Ok(Some(combat_player_damage_target_effect(
            amount.clone(),
            target,
        )));
    }
    if word_slice_starts_with_any(&target_words, COMBAT_EACH_OR_ALL_HEAD_PREFIXES) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}

pub(crate) fn parse_deal_damage_equal_to_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["damage", "equal", "to"]).is_none() {
        return Ok(None);
    }

    let mut target_to_idx = None;
    for idx in 3..tokens.len() {
        if !tokens[idx].is_word(COMBAT_TO_WORD) {
            continue;
        }
        let tail_words = crate::runtime_backend::token_word_refs(&tokens[idx + 1..]);
        if tail_words.is_empty() {
            continue;
        }
        let looks_like_target =
            word_slice_contains_any_word(&tail_words, COMBAT_TARGET_OR_TARGETS_WORDS)
                || matches!(
                    tail_words.first().copied(),
                    Some(
                        "any"
                            | "each"
                            | "all"
                            | "it"
                            | "itself"
                            | "them"
                            | "him"
                            | "her"
                            | "that"
                            | "this"
                            | "you"
                            | "player"
                            | "opponent"
                            | "creature"
                            | "planeswalker"
                    )
                )
                || parse_target_phrase(&tokens[idx + 1..]).is_ok();
        if looks_like_target {
            target_to_idx = Some(idx);
        }
    }

    let Some(target_to_idx) = target_to_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let amount_tokens = if tokens
        .first()
        .is_some_and(|token| token.is_word(COMBAT_DAMAGE_WORD))
    {
        &tokens[1..target_to_idx]
    } else {
        &tokens[..target_to_idx]
    };
    let amount = parse_add_mana_equal_amount_value(amount_tokens)
        .or(parse_equal_to_aggregate_filter_value(amount_tokens))
        .or(parse_devotion_value_from_add_clause(amount_tokens)?)
        .or(parse_equal_to_number_of_filter_plus_or_minus_fixed_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_filter_value(amount_tokens))
        .or(parse_equal_to_number_of_opponents_you_have_value(
            amount_tokens,
        ))
        .or(parse_equal_to_number_of_counters_on_reference_value(
            amount_tokens,
        ))
        .or(parse_dynamic_cost_modifier_value(amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let target_tokens = &tokens[target_to_idx + 1..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut normalized_target_tokens = target_tokens;
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        if word_slice_contains_any_word(
            &crate::runtime_backend::token_word_refs(each_of_tokens),
            COMBAT_TARGET_OR_TARGETS_WORDS,
        ) {
            normalized_target_tokens = each_of_tokens;
        }
    }
    if let Some(target) = parse_combat_player_damage_target(normalized_target_tokens, true) {
        return Ok(Some(combat_player_damage_target_effect(
            amount.clone(),
            target,
        )));
    }
    if matches!(
        crate::runtime_backend::token_word_refs(normalized_target_tokens).first(),
        Some(&"each") | Some(&"all")
    ) {
        if normalized_target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter = parse_object_filter(&normalized_target_tokens[1..], false)?;
        return Ok(Some(EffectAst::subject_verb_damage_each(amount, filter)));
    }
    let target = parse_target_phrase(normalized_target_tokens)?;
    Ok(Some(EffectAst::subject_verb_damage(amount, target)))
}

fn parse_divided_damage_target(
    target_tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word(COMBAT_AMONG_WORD)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage targets after 'among' (clause: '{}')",
            crate::runtime_backend::token_word_refs(target_tokens).join(" ")
        )));
    };
    let among_tail = trim_commas(&target_tokens[among_idx + 1..]);
    let among_words = crate::runtime_backend::token_word_refs(&among_tail);
    let Some(target_idx) = find_index(&among_words, |word| {
        COMBAT_TARGET_OR_TARGETS_WORDS.contains(word)
    }) else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target phrase (clause: '{}')",
            crate::runtime_backend::token_word_refs(target_tokens).join(" ")
        )));
    };

    let count = if let Some((count, used)) = parse_choice_count_before_target_prefix(&among_tail) {
        if used != target_idx {
            return Err(CardTextError::ParseError(format!(
                "unsupported divided-damage target count (clause: '{}')",
                crate::runtime_backend::token_word_refs(target_tokens).join(" ")
            )));
        }
        count
    } else if let Some(max_targets) = among_words[..target_idx]
        .iter()
        .filter_map(|word| parse_number_word_u32(word))
        .max()
    {
        ChoiceCount {
            min: 1,
            max: Some(max_targets as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        }
    } else {
        return Err(CardTextError::ParseError(format!(
            "missing divided-damage target count (clause: '{}')",
            crate::runtime_backend::token_word_refs(target_tokens).join(" ")
        )));
    };

    let target_token_idx =
        token_index_for_word_index(&among_tail, target_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing divided-damage target phrase (clause: '{}')",
                crate::runtime_backend::token_word_refs(target_tokens).join(" ")
            ))
        })?;
    let target_phrase_tokens = &among_tail[target_token_idx..];
    let target_phrase_words = crate::runtime_backend::token_word_refs(target_phrase_tokens);
    let base_target = if target_phrase_words.len() == 1
        && COMBAT_TARGET_OR_TARGETS_WORDS.contains(&target_phrase_words[0])
    {
        TargetAst::AnyTarget(span_from_tokens(target_phrase_tokens))
    } else {
        parse_target_phrase(target_phrase_tokens)?
    };
    Ok(TargetAst::WithCount(Box::new(base_target), count))
}

fn parse_divided_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    if !crate::runtime_backend::lexer::token_slice_first_is(rest, "damage") {
        return Err(CardTextError::ParseError(format!(
            "missing damage keyword in divided-damage clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let mut target_tokens = &rest[1..];
    if crate::runtime_backend::lexer::token_slice_first_is(target_tokens, "to") {
        target_tokens = &target_tokens[1..];
    }
    if word_slice_contains_word(
        &crate::runtime_backend::token_word_refs(target_tokens),
        COMBAT_EVENLY_WORD,
    ) && let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word(COMBAT_AMONG_WORD)
    }) {
        let among_tail = trim_commas(&target_tokens[among_idx + 1..]);
        if matches!(
            among_tail.first().and_then(OwnedLexToken::as_word),
            Some("all" | "each" | "every")
        ) && among_tail.len() > 1
        {
            let filter = parse_object_filter(&among_tail[1..], false)?;
            return Ok(EffectAst::subject_verb_damage_each(amount, filter));
        }
    }
    let target = parse_divided_damage_target(target_tokens)?;
    Ok(EffectAst::subject_verb_distributed_damage(amount, target))
}

pub(crate) fn parse_deal_damage_with_amount(
    tokens: &[OwnedLexToken],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    let Some(word) = rest.first().and_then(OwnedLexToken::as_word) else {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    };
    if word != COMBAT_DAMAGE_WORD {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word(COMBAT_TO_WORD))
    {
        target_tokens = &target_tokens[1..];
    }
    if let Some(among_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word(COMBAT_AMONG_WORD)
    }) {
        let among_tail = &target_tokens[among_idx + 1..];
        if crate::runtime_backend::lexer::contains_token_word(among_tail, "target")
            && crate::runtime_backend::lexer::contains_token_any_word(
                among_tail,
                &["player", "players", "creature", "creatures"],
            )
        {
            target_tokens = among_tail;
        }
    }

    if crate::runtime_backend::lexer::contains_token_word(target_tokens, "where") {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing where damage clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    if let Some(instead_idx) = find_index(target_tokens, |token: &OwnedLexToken| {
        token.is_word(COMBAT_INSTEAD_WORD)
    }) && target_tokens
        .get(instead_idx + 1)
        .is_some_and(|token| token.is_word(COMBAT_IF_WORD))
    {
        let pre_target_tokens = trim_commas(&target_tokens[..instead_idx]);
        let predicate = if let Some(predicate) =
            parse_instead_if_control_predicate(&trim_commas(&target_tokens[instead_idx + 2..]))?
        {
            predicate
        } else {
            parse_trailing_instead_if_predicate_lexed(&target_tokens[instead_idx..]).ok_or_else(
                || {
                    CardTextError::ParseError(format!(
                        "unsupported trailing instead-if clause in damage effect (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    ))
                },
            )?
        };
        let target = if pre_target_tokens.is_empty() {
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
        } else {
            parse_target_phrase(&pre_target_tokens)?
        };
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::subject_verb_damage(amount.clone(), target)],
            if_false: Vec::new(),
        });
    }

    if let Some(spec) = split_trailing_if_clause_lexed(target_tokens) {
        let target = parse_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: spec.predicate,
            if_true: vec![EffectAst::subject_verb_damage(amount, target)],
            if_false: Vec::new(),
        });
    }

    if target_tokens
        .first()
        .is_some_and(|token| token.is_word(COMBAT_IF_WORD))
    {
        let predicate = parse_trailing_if_predicate_lexed(target_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported trailing if clause in damage effect (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::subject_verb_damage(
                amount,
                // Follow-up "deals N damage if ..." clauses can omit the target and rely
                // on parser-level merge with a prior damage sentence.
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None),
            )],
            if_false: Vec::new(),
        });
    }

    if find_index(&target_tokens, |token| token.is_word(COMBAT_IF_WORD)).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing if clause in damage effect (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let target_words = crate::runtime_backend::token_word_refs(target_tokens);
    if let Some(target) = parse_combat_simple_damage_target(target_tokens) {
        return Ok(EffectAst::subject_verb_damage(amount, target));
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OF_PREFIXES).is_some() {
        let each_of_tokens = &target_tokens[2..];
        if let Some((count, used)) = parse_choice_count_before_target_prefix(each_of_tokens)
            && each_of_tokens.len() == used + 1
        {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(each_of_tokens))),
                count,
            );
            return Ok(EffectAst::subject_verb_damage(amount, target));
        }
        if word_slice_contains_any_word(
            &crate::runtime_backend::token_word_refs(each_of_tokens),
            COMBAT_TARGET_OR_TARGETS_WORDS,
        ) {
            let target = parse_target_phrase(each_of_tokens)?;
            return Ok(EffectAst::subject_verb_damage(amount, target));
        }
    }
    if let Some(target) = parse_combat_player_damage_target(target_tokens, false) {
        return Ok(combat_player_damage_target_effect(amount.clone(), target));
    }
    let normalized_target_words =
        crate::runtime_backend::lexer::parser_token_word_refs(target_tokens);
    let each_player_max_speed_filter =
        word_slice_starts_with_any(&normalized_target_words, COMBAT_EACH_OR_ALL_HEAD_PREFIXES)
            && normalized_target_words
                .iter()
                .any(|word| COMBAT_PLAYER_OR_PLAYERS_WORDS.contains(word))
            && word_slice_contains_phrase(&normalized_target_words, COMBAT_MAX_SPEED_PHRASE);
    if each_player_max_speed_filter {
        let has_max_speed = !(normalized_target_words
            .iter()
            .any(|word| COMBAT_NEGATION_WORDS.contains(word))
            || word_slice_contains_phrase(&normalized_target_words, COMBAT_DOES_NOT_PHRASE));
        let filter = if has_max_speed {
            PlayerFilter::with_max_speed(PlayerFilter::Any)
        } else {
            PlayerFilter::without_max_speed(PlayerFilter::Any)
        };
        return Ok(EffectAst::ForEachPlayersFiltered {
            filter,
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_WHO_PREFIXES).is_some()
        && word_slice_contains_phrase(&target_words, COMBAT_THIS_WAY_PHRASE)
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachOpponentDid {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
            predicate,
        });
    }
    if grammar::words_match_any_prefix(target_tokens, EACH_PLAYER_WHO_PREFIXES).is_some()
        && word_slice_contains_phrase(&target_words, COMBAT_THIS_WAY_PHRASE)
    {
        let predicate = parse_who_did_this_way_predicate(&target_tokens[2..])?;
        return Ok(EffectAst::ForEachPlayerDid {
            effects: vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
            predicate,
        });
    }

    if let Some((_, and_each_idx)) =
        word_slice_find_any_phrase_start(&target_words, COMBAT_AND_EACH_OR_ALL_PHRASES)
        && and_each_idx > 0
    {
        let player_target_tokens = trim_commas(&target_tokens[..and_each_idx]);
        let object_filter_tokens = trim_commas(&target_tokens[and_each_idx + 1..]);
        if !player_target_tokens.is_empty()
            && !object_filter_tokens.is_empty()
            && let Ok(TargetAst::Player(player_filter, span)) =
                parse_target_phrase(&player_target_tokens)
            && crate::runtime_backend::lexer::contains_token_any_word(
                &object_filter_tokens,
                &["creature", "creatures"],
            )
        {
            let mut filter = parse_object_filter(&object_filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(player_filter.clone());
            }
            return Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_damage(
                        amount.clone(),
                        TargetAst::Player(player_filter, span),
                    ),
                    EffectAst::subject_verb_damage_each(amount.clone(), filter),
                ],
            });
        }
    }

    if word_slice_starts_with_any(&target_words, COMBAT_EACH_OR_ALL_HEAD_PREFIXES)
        && let Some((_, and_each_idx)) =
            word_slice_find_any_phrase_start(&target_words, COMBAT_AND_EACH_PLAYER_PHRASES)
        && and_each_idx >= 1
        && and_each_idx + 3 == target_words.len()
    {
        let filter_tokens = &target_tokens[1..and_each_idx];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        if filter.controller.is_none() {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::subject_verb_damage(
                    amount.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                ),
                EffectAst::subject_verb_damage_each(amount.clone(), filter),
            ],
        });
    }

    if grammar::words_match_any_prefix(target_tokens, EACH_OPPONENT_AND_EACH_PREFIXES).is_some()
        && combat_words_contain_word(&target_words, "creature")
        && combat_words_contain_word(&target_words, "planeswalker")
        && word_slice_contains_any_phrase(&target_words, COMBAT_ITERATED_PLAYER_CONTROL_PHRASES)
    {
        let mut filter = ObjectFilter::default();
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_damage(
                    amount.clone(),
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                ),
                EffectAst::subject_verb_damage_each(amount.clone(), filter),
            ],
        });
    }

    if word_slice_starts_with_any(&target_words, COMBAT_EACH_OR_ALL_HEAD_PREFIXES) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::subject_verb_damage_each(amount.clone(), filter));
    }

    if let Some(at_idx) = find_index(&target_tokens, |token| token.is_word(COMBAT_AT_WORD))
        && at_idx >= 1
    {
        let pre_target_tokens = trim_commas(&target_tokens[..at_idx]);
        if !pre_target_tokens.is_empty()
            && word_slice_eq_any(
                &crate::runtime_backend::token_word_refs(&target_tokens[at_idx..]),
                COMBAT_END_OF_COMBAT_TIMINGS,
            )
        {
            let target = parse_target_phrase(&pre_target_tokens)?;
            return Ok(EffectAst::DelayedUntilEndOfCombat {
                effects: vec![EffectAst::subject_verb_damage(amount, target)],
            });
        }
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(EffectAst::subject_verb_damage(amount, target))
}

pub(crate) fn parse_instead_if_control_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let starts_with_you_control =
        grammar::words_match_any_prefix(tokens, YOU_CONTROL_PREFIXES).is_some();
    if !starts_with_you_control {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[2..];
    let mut min_count: Option<u32> = None;
    if let Ok((comparison, used)) =
        parse_quantity_comparison_prefix(filter_tokens, false, false, "control predicate")
    {
        if let Some(count) = comparison_to_strict_at_least_threshold(&comparison) {
            min_count = Some(count);
            filter_tokens = &filter_tokens[used..];
        } else if matches!(
            comparison,
            crate::effect::Comparison::LessThan(_) | crate::effect::Comparison::LessThanOrEqual(_)
        ) {
            // Keep unsupported upper-bound variants as plain control checks for now.
            filter_tokens = &filter_tokens[used..];
        }
    }
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if let Some((_, idx)) =
        word_slice_find_any_phrase_start(&filter_words, COMBAT_CAST_OR_TURN_MARKER_PHRASES)
    {
        let cut_idx = token_index_for_word_index(filter_tokens, idx).unwrap_or(filter_tokens.len());
        filter_tokens = &filter_tokens[..cut_idx];
    }
    let mut filter_tokens = trim_commas(filter_tokens);
    let mut requires_different_powers = false;
    if word_slice_ends_with_any(
        &crate::runtime_backend::token_word_refs(&filter_tokens),
        COMBAT_WITH_DIFFERENT_POWER_SUFFIXES,
    ) {
        requires_different_powers = true;
        let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
        let cut_word_idx = filter_words.len().saturating_sub(3);
        let cut_token_idx =
            token_index_for_word_index(&filter_tokens, cut_word_idx).unwrap_or(filter_tokens.len());
        filter_tokens = trim_commas(&filter_tokens[..cut_token_idx]);
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let other = filter_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| COMBAT_OTHER_WORDS.contains(&word));
    let filter = parse_object_filter(&filter_tokens, other)?;
    if let Some(count) = min_count {
        if requires_different_powers {
            return Ok(Some(PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                player: PlayerAst::You,
                filter,
                count,
            }));
        }
        Ok(Some(PredicateAst::PlayerHasAtLeast {
            player: PlayerAst::You,
            filter,
            count,
        }))
    } else {
        Ok(Some(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        }))
    }
}
