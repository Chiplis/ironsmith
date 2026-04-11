use super::*;

pub(super) const FOR_EACH_PLAYER_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["each", "player"],
    &["each", "players"],
];
pub(super) const EACH_OPPONENT_PREFIXES: &[&[&str]] =
    &[&["each", "opponent"], &["each", "opponents"]];
pub(super) const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
pub(super) const CHOOSE_ALL_OR_PUT_ALL_PREFIXES: &[&[&str]] =
    &[&["choose", "all"], &["put", "all"]];
pub(super) const UP_TO_PREFIXES: &[&[&str]] = &[&["up", "to"]];
pub(super) const ANY_NUMBER_OF_PREFIXES: &[&[&str]] = &[&["any", "number", "of"]];
pub(super) const CHOOSE_ALL_PREFIXES: &[&[&str]] = &[&["choose", "all"]];
pub(super) const THAT_PREFIXES: &[&[&str]] = &[&["that"]];
pub(super) const MECHANIC_MARKER_PREFIXES: &[&[&str]] = &[
    &["you", "choose", "one", "of", "them"],
    &[
        "you", "may", "put", "a", "land", "card", "from", "among", "them", "into", "your", "hand",
    ],
    &["stand", "and", "fight"],
    &["it", "doesnt", "untap", "during"],
];
pub(crate) type SentencePrimitiveParser =
    fn(&[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError>;

pub(super) type SentencePrimitiveNormalizedWords<'a> = TokenWordView<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SentencePrimitiveStage {
    PreDiagnostic,
    PostDiagnostic,
}

pub(crate) struct SentencePrimitive {
    pub(crate) id: &'static str,
    pub(crate) priority: u16,
    pub(crate) stage: SentencePrimitiveStage,
    pub(crate) head_hints: &'static [LexRuleHeadHint],
    pub(crate) shape_mask: u32,
    pub(crate) parser: SentencePrimitiveParser,
}

pub(super) fn find_token_word(tokens: &[OwnedLexToken], word: &str) -> Option<usize> {
    find_index(tokens, |token| token.is_word(word))
}

pub(super) fn rfind_token_word(tokens: &[OwnedLexToken], word: &str) -> Option<usize> {
    rfind_index(tokens, |token| token.is_word(word))
}

pub(super) fn find_comma_then_idx(tokens: &[OwnedLexToken]) -> Option<usize> {
    split_lexed_once_on_comma_then(tokens).map(|(head, _)| head.len())
}

pub(super) fn contains_word_window(words: &[&str], pattern: &[&str]) -> bool {
    contains_word_sequence(words, pattern)
}

pub(super) fn strip_quoted_possessive_suffix(word: &str) -> &str {
    str_strip_suffix(word, "'s")
        .or_else(|| str_strip_suffix(word, "’s"))
        .or_else(|| str_strip_suffix(word, "s'"))
        .or_else(|| str_strip_suffix(word, "s’"))
        .unwrap_or(word)
}

pub(super) fn parse_pluralized_subtype_word(word: &str) -> Option<Subtype> {
    parse_subtype_word(word).or_else(|| str_strip_suffix(word, "s").and_then(parse_subtype_word))
}

fn run_sentence_primitive(
    primitive: &SentencePrimitive,
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    match (primitive.parser)(tokens) {
        Ok(Some(effects)) => {
            let stage = format!("parse_effect_sentence:primitive-hit:{}", primitive.id);
            parser_trace(&stage, tokens);
            if effects.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "primitive '{}' produced empty effects (clause: '{}')",
                    primitive.id,
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                )));
            }
            Ok(Some(effects))
        }
        Ok(None) => Ok(None),
        Err(err) => {
            if parser_trace_enabled() {
                eprintln!(
                    "[parser-flow] stage=parse_effect_sentence:primitive-error primitive={} clause='{}' error={err:?}",
                    primitive.id,
                    crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
                );
            }
            Err(err)
        }
    }
}

fn normalize_parser_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    for token in &mut normalized {
        match token.kind {
            crate::cards::builders::compiler::lexer::TokenKind::Word
            | crate::cards::builders::compiler::lexer::TokenKind::Number
            | crate::cards::builders::compiler::lexer::TokenKind::Tilde => {
                let replacement = token.parser_text().to_string();
                let _ = token.replace_word(replacement);
            }
            _ => {}
        }
    }
    normalized
}

fn run_sentence_primitive_lexed(
    primitive: &SentencePrimitive,
    tokens: &[OwnedLexToken],
    lowered: &OnceCell<Vec<OwnedLexToken>>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let lowered_tokens = lowered.get_or_init(|| normalize_parser_tokens(tokens));
    run_sentence_primitive(primitive, lowered_tokens)
}

pub(crate) fn run_sentence_primitives_lexed(
    tokens: &[OwnedLexToken],
    primitives: &'static [SentencePrimitive],
    index: &LexRuleHintIndex,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head, second) = lexed_head_words(tokens).unwrap_or(("", None));
    let mut tried = vec![false; primitives.len()];
    let lowered = OnceCell::new();
    let mut candidate_indices = index.candidate_indices(head, second);
    candidate_indices.sort_by_key(|idx| (primitives[*idx].priority, primitives[*idx].shape_mask));
    for idx in candidate_indices {
        tried[idx] = true;
        if let Some(effects) = run_sentence_primitive_lexed(&primitives[idx], tokens, &lowered)? {
            return Ok(Some(effects));
        }
    }

    let mut fallback_indices = primitives
        .iter()
        .enumerate()
        .filter_map(|(idx, _)| (!tried[idx]).then_some(idx))
        .collect::<Vec<_>>();
    fallback_indices.sort_by_key(|idx| (primitives[*idx].priority, primitives[*idx].shape_mask));

    for idx in fallback_indices {
        let primitive = &primitives[idx];
        if let Some(effects) = run_sentence_primitive_lexed(primitive, tokens, &lowered)? {
            return Ok(Some(effects));
        }
    }

    Ok(None)
}

pub(super) fn parse_preconditional_sentence_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    debug_assert!(
        PRE_CONDITIONAL_SENTENCE_PRIMITIVES
            .iter()
            .all(|primitive| primitive.stage == SentencePrimitiveStage::PreDiagnostic)
    );
    run_sentence_primitives_lexed(
        view.tokens,
        PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
        &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
    )
}

pub(super) fn parse_postconditional_sentence_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    debug_assert!(
        POST_CONDITIONAL_SENTENCE_PRIMITIVES
            .iter()
            .all(|primitive| primitive.stage == SentencePrimitiveStage::PostDiagnostic)
    );
    run_sentence_primitives_lexed(
        view.tokens,
        POST_CONDITIONAL_SENTENCE_PRIMITIVES,
        &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
    )
}

pub(crate) const PRIMITIVE_PRE_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        id: "preconditional-primitives",
        priority: 135,
        heads: &[],
        shape_mask: 0,
        run: parse_preconditional_sentence_primitives_rule_lexed,
    }];

pub(crate) const PRIMITIVE_POST_DIAGNOSTIC_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 1] =
    [LexRuleDef {
        id: "postconditional-primitives",
        priority: 160,
        heads: &[],
        shape_mask: 0,
        run: parse_postconditional_sentence_primitives_rule_lexed,
    }];

pub(crate) const PRIMITIVE_PRE_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&PRIMITIVE_PRE_DIAGNOSTIC_RULES_LEXED);

pub(crate) const PRIMITIVE_POST_DIAGNOSTIC_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&PRIMITIVE_POST_DIAGNOSTIC_RULES_LEXED);

pub(crate) fn parse_sentence_return_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_return_with_counters_on_it(tokens)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_put_onto_battlefield_with_counters_on_it(tokens)
}

pub(crate) fn parse_sentence_exile_source_with_counters_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sentence_exile_source_with_counters(tokens)
}

pub(crate) fn parse_you_and_target_player_each_draw_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if grammar::words_match_prefix(tokens, &["you", "and", "target"]).is_none() {
        return Ok(None);
    }

    let target_player = match clause_words.get(3).copied() {
        Some("opponent" | "opponents") => PlayerAst::TargetOpponent,
        Some("player" | "players") => PlayerAst::Target,
        _ => return Ok(None),
    };

    let mut idx = 4usize;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let remainder_words = &clause_words[idx..];
    if remainder_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some((count, used_words)) = parse_half_rounded_down_draw_count_words(remainder_words) {
        let trailing_words = &remainder_words[used_words..];
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing shared draw clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(vec![
            EffectAst::Draw {
                count: count.clone(),
                player: PlayerAst::You,
            },
            EffectAst::Draw {
                count,
                player: target_player,
            },
        ]));
    }
    let synthetic_tokens = remainder_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_value(&synthetic_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if synthetic_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words =
        crate::cards::builders::compiler::token_word_refs(&synthetic_tokens[used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count,
            player: target_player,
        },
    ]))
}

pub(crate) fn parse_sentence_you_and_target_player_each_draw(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(tokens)
}

pub(crate) fn parse_sentence_choose_player_to_effect(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let mut stripped = trim_commas(tokens);
    while stripped
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        stripped.remove(0);
    }
    if stripped.is_empty() {
        return Ok(None);
    }

    let Some((choose_slice, tail_slice)) =
        grammar::split_lexed_once_on_separator(&stripped, || grammar::kw("to").void())
    else {
        return Ok(None);
    };

    let choose_tokens = trim_commas(choose_slice);
    let tail_tokens = trim_commas(tail_slice);
    if choose_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }
    let Some((chooser, filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(&choose_tokens)?
    else {
        return Ok(None);
    };

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    for effect in &mut tail_effects {
        bind_implicit_player_context(effect, PlayerAst::That);
    }

    let mut effects = vec![EffectAst::ChoosePlayer {
        chooser,
        filter,
        tag: TagKey::from(IT_TAG),
        random,
        exclude_previous_choices,
    }];
    effects.extend(tail_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut stripped = trim_commas(tokens);
    while stripped
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        stripped.remove(0);
    }
    if stripped.len() < 10
        || !stripped
            .first()
            .is_some_and(|token| token.is_word("return"))
    {
        return Ok(None);
    }

    let words = crate::cards::builders::compiler::token_word_refs(&stripped);
    let Some(the_idx) = words.iter().position(|word| *word == "the") else {
        return Ok(None);
    };
    let Some(they_idx) = words.iter().position(|word| *word == "they") else {
        return Ok(None);
    };
    let Some(control_idx) = words.iter().position(|word| *word == "control") else {
        return Ok(None);
    };
    let Some(to_idx) = words.iter().position(|word| *word == "to") else {
        return Ok(None);
    };
    let Some(owner_idx) = words
        .iter()
        .position(|word| matches!(*word, "owner's" | "owners'" | "owners" | "owner"))
    else {
        return Ok(None);
    };
    if the_idx + 1 >= they_idx
        || they_idx + 1 != control_idx
        || control_idx >= to_idx
        || to_idx >= owner_idx
        || !words
            .get(owner_idx + 1)
            .is_some_and(|word| matches!(*word, "hand" | "hands"))
        || !words.ends_with(&["rounded", "up"])
    {
        return Ok(None);
    }

    let filter_tokens = trim_commas(&stripped[the_idx + 1..they_idx]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(&filter_tokens, false)?;
    if filter.controller.is_none() {
        filter.controller = Some(PlayerFilter::IteratedPlayer);
    }
    let count_value = Value::HalfRoundedDown(Box::new(Value::Add(
        Box::new(Value::Count(filter.clone())),
        Box::new(Value::Fixed(1)),
    )));
    let chosen_tag = TagKey::from("chosen");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::dynamic_x(),
            count_value: Some(count_value),
            player: PlayerAst::That,
            tag: chosen_tag.clone(),
        },
        EffectAst::ReturnAllToHand {
            filter: ObjectFilter::tagged(chosen_tag),
        },
    ]))
}

pub(crate) fn parse_sentence_damage_to_that_player_half_damage_of_those_spells(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut stripped = trim_commas(tokens);
    while stripped
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        stripped.remove(0);
    }
    if stripped.is_empty() {
        return Ok(None);
    }

    use super::super::super::grammar::primitives as grammar;

    let deal_split =
        grammar::split_lexed_once_on_separator(&stripped, || grammar::kw("deal").void()).or_else(
            || grammar::split_lexed_once_on_separator(&stripped, || grammar::kw("deals").void()),
        );
    let Some((_before_deal, after_deal)) = deal_split else {
        return Ok(None);
    };
    let tail_words = crate::cards::builders::compiler::token_word_refs(after_deal);
    if tail_words.len() != 20 {
        return Ok(None);
    }
    if tail_words[..14]
        != [
            "damage", "to", "that", "player", "equal", "to", "half", "the", "damage", "dealt",
            "by", "one", "of", "those",
        ]
        || tail_words[15..] != ["spells", "this", "turn", "rounded", "down"]
    {
        return Ok(None);
    }

    let card_type = parse_card_type(tail_words[14]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported spell type in historical half-damage sentence (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        ))
    })?;
    Ok(Some(vec![
        EffectAst::ChooseSpellCastHistory {
            chooser: PlayerAst::You,
            cast_by: PlayerAst::That,
            filter: ObjectFilter::default().with_type(card_type),
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::DealDamage {
            amount: Value::HalfRoundedDown(Box::new(Value::DamageDealtThisTurnByTaggedSpellCast(
                TagKey::from(IT_TAG),
            ))),
            target: TargetAst::Player(PlayerFilter::target_player(), None),
        },
    ]))
}

pub(crate) fn parse_draw_for_each_card_exiled_from_hand_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }

    let clause_words = crate::cards::builders::compiler::token_word_refs(&clause_tokens);
    let (player, mut effects) = match clause_words.as_slice() {
        [
            "that",
            "player",
            "shuffles",
            "then",
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "their",
            "hand",
            "this",
            "way",
        ] => (
            PlayerAst::That,
            vec![EffectAst::ShuffleLibrary {
                player: PlayerAst::That,
            }],
        ),
        [
            "that",
            "player",
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "their",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::That, Vec::new()),
        [
            "you",
            "draw",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "your",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::You, Vec::new()),
        [
            "draw",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "your",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::Implicit, Vec::new()),
        [
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "their",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::That, Vec::new()),
        [
            "draws",
            "a",
            "card",
            "for",
            "each",
            "card",
            "exiled",
            "from",
            "your",
            "hand",
            "this",
            "way",
        ] => (PlayerAst::Implicit, Vec::new()),
        _ => return Ok(None),
    };

    effects.push(EffectAst::DrawForEachTaggedMatching {
        player,
        tag: TagKey::from(IT_TAG),
        filter: ObjectFilter::default().in_zone(Zone::Hand),
    });
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_draw_for_each_card_exiled_from_hand_this_way(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_for_each_card_exiled_from_hand_this_way_sentence(tokens)
}

pub(crate) fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.len() < 11 || grammar::words_match_prefix(tokens, &["you", "and"]).is_none() {
        return Ok(None);
    }

    let mut idx = 2usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"attacking") || clause_words.get(idx + 1) != Some(&"player") {
        return Ok(None);
    }
    idx += 2;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let draw_tokens = clause_words[idx..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let draw_words = crate::cards::builders::compiler::token_word_refs(&draw_tokens);
    let (draw_count, after_draw_words) = if let Some((draw_count, used_words)) =
        parse_half_rounded_down_draw_count_words(&draw_words)
    {
        (draw_count, draw_words[used_words..].to_vec())
    } else {
        let (draw_count, draw_used) = parse_value(&draw_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing shared draw count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if draw_tokens
            .get(draw_used)
            .and_then(OwnedLexToken::as_word)
            .is_none_or(|word| word != "card" && word != "cards")
        {
            return Err(CardTextError::ParseError(format!(
                "missing card keyword in shared draw/lose sentence (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        (
            draw_count,
            crate::cards::builders::compiler::token_word_refs(&draw_tokens[draw_used + 1..]),
        )
    };
    if after_draw_words.first() != Some(&"and")
        || !matches!(after_draw_words.get(1).copied(), Some("lose" | "loses"))
    {
        return Ok(None);
    }

    let lose_tokens = after_draw_words[2..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (lose_amount, lose_used) = parse_value(&lose_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if lose_tokens
        .get(lose_used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "life")
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words =
        crate::cards::builders::compiler::token_word_refs(&lose_tokens[lose_used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw/lose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: draw_count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count: draw_count,
            player: PlayerAst::Attacking,
        },
        EffectAst::LoseLife {
            amount: lose_amount.clone(),
            player: PlayerAst::You,
        },
        EffectAst::LoseLife {
            amount: lose_amount,
            player: PlayerAst::Attacking,
        },
    ]))
}

pub(crate) fn parse_sentence_sacrifice_it_next_end_step(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    // "sacrifice <object> at the beginning of [the] next end step"
    let Some(object_tokens) = grammar::strip_lexed_prefix_phrase(tokens, &["sacrifice"]) else {
        return Ok(None);
    };
    let Some((object_tokens, _timing)) =
        grammar::split_lexed_once_on_separator(object_tokens, || {
            winnow::combinator::alt((
                grammar::phrase(&["at", "the", "beginning", "of", "the", "next", "end", "step"]),
                grammar::phrase(&["at", "the", "beginning", "of", "next", "end", "step"]),
            ))
        })
    else {
        return Ok(None);
    };

    let object_tokens = trim_commas(object_tokens);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in delayed next-end-step clause (clause: '{}')",
            crate::cards::builders::compiler::token_word_refs(tokens).join(" ")
        )));
    }

    let object_words = crate::cards::builders::compiler::token_word_refs(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["the", "creature"]
            | ["that", "creature"]
            | ["the", "permanent"]
            | ["that", "permanent"]
            | ["the", "token"]
            | ["that", "token"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: PlayerFilter::Any,
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
            target: None,
        }],
    }]))
}

pub(crate) fn parse_sentence_if_tagged_cards_remain_exiled(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use super::super::super::grammar::primitives as grammar;

    let has_prefix = grammar::strip_lexed_prefix_phrase(
        tokens,
        &["if", "any", "of", "those", "cards", "remain", "exiled"],
    )
    .is_some()
        || grammar::strip_lexed_prefix_phrase(
            tokens,
            &["if", "those", "cards", "remain", "exiled"],
        )
        .is_some()
        || grammar::strip_lexed_prefix_phrase(tokens, &["if", "that", "card", "remains", "exiled"])
            .is_some()
        || grammar::strip_lexed_prefix_phrase(tokens, &["if", "it", "remains", "exiled"]).is_some();
    if !has_prefix {
        return Ok(None);
    }

    parse_conditional_sentence_with_grammar_entrypoint_lexed(tokens, parse_effect_chain_lexed)
        .map(Some)
}
