use super::super::zone_handlers::parse_return;
use super::*;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::grammar::effects::combat_damage_family_shapes as combat_shapes;
use crate::grammar::effects::delayed_step_shapes as delayed_shapes;
use crate::lexer::trim_lexed_commas;
const TRANSFORM_WORD: &str = "transform";
const CONVERT_WORD: &str = "convert";
const DISTRIBUTE_WORD: &str = "distribute";

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word() == Some(expected)
}

pub fn parse_sentence_destroy_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if combat_shapes::parse_destroy_creature_type_choice_shape(clause.tokens()).is_none() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_destroy_all(ObjectFilter::creature().of_chosen_creature_type()),
    ]))
}

pub fn parse_sentence_pump_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_pump_creature_type_choice_shape(clause.tokens()) else {
        return Ok(None);
    };
    if !shape.trailing_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            clause.text()
        )));
    }
    let trimmed_subject_clause =
        SubjectVerbPrimitiveClause::new(shape.base_subject_tokens).trimmed();
    if trimmed_subject_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }
    let get_tail_clause = SubjectVerbPrimitiveClause::new(shape.get_tail_tokens).trimmed();

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_clause =
        SubjectVerbPrimitiveOwnedClause::from_clause(trimmed_subject_clause);
    gain_candidate_clause.append_clause(get_tail_clause);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(gain_candidate_clause.tokens())? {
        if super::super::gain_ability::patch_creature_type_choice_effects(&mut gain_effects) {
            return Ok(Some(gain_effects));
        }
    }

    let filter_clause = SubjectVerbPrimitiveClause::new(shape.filter_subject_tokens).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }

    let modifier = get_tail_clause
        .token(1)
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                clause.text()
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            clause.text()
        ))
    })?;
    let (power, toughness, duration, condition) = parse_get_modifier_values_with_tail(
        get_tail_clause.from(1).tokens(),
        base_power,
        base_toughness,
    )?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            clause.text()
        )));
    }

    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_pump_all(filter, power, toughness, duration),
    ]))
}

/// "Creatures your opponents control attack this turn if able." (Bident of
/// Thassa): a plain filtered attack requirement until end of turn.
pub fn parse_sentence_must_attack_filter_this_turn(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::effect::Until;

    let tokens = clause.tokens();
    let Some((suffix_start, (), suffix_rest)) =
        crate::grammar::primitives::find_prefix(tokens, || {
            winnow::combinator::alt((
                crate::grammar::primitives::phrase(&["attack", "this", "turn", "if", "able"]),
                crate::grammar::primitives::phrase(&["attacks", "this", "turn", "if", "able"]),
            ))
        })
    else {
        return Ok(None);
    };
    if !crate::lexer::trim_lexed_commas(suffix_rest).is_empty() {
        return Ok(None);
    }
    let subject_tokens = crate::lexer::trim_lexed_commas(&tokens[..suffix_start]);
    if subject_tokens.is_empty()
        || subject_tokens
            .iter()
            .any(|token| token.is_any_word(&["target", "chosen", "choice"]))
    {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::subject_verb_grant_abilities_all(
        filter,
        vec![crate::cards::builders::GrantedAbilityAst::MustAttack],
        Until::EndOfTurn,
    )]))
}

/// "Creatures your opponents control attack each combat if able and attack a
/// player other than you if able." (Kardur, Doomscourge): the goad requirement
/// (CR 701.38) spelled out for a filtered set of creatures.
pub fn parse_sentence_goad_requirement_filter(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::effect::Until;
    use winnow::prelude::*;

    let tokens = clause.tokens();
    let Some((suffix_start, (), suffix_rest)) =
        crate::grammar::primitives::find_prefix(tokens, || {
            (
                winnow::combinator::alt((
                    crate::grammar::primitives::kw("attack"),
                    crate::grammar::primitives::kw("attacks"),
                )),
                crate::grammar::primitives::phrase(&["each", "combat", "if", "able", "and"]),
                winnow::combinator::alt((
                    crate::grammar::primitives::kw("attack"),
                    crate::grammar::primitives::kw("attacks"),
                )),
                crate::grammar::primitives::phrase(&[
                    "a", "player", "other", "than", "you", "if", "able",
                ]),
            )
                .void()
        })
    else {
        return Ok(None);
    };
    if !crate::lexer::trim_lexed_commas(suffix_rest).is_empty() {
        return Ok(None);
    }
    let subject_tokens = crate::lexer::trim_lexed_commas(&tokens[..suffix_start]);
    if subject_tokens.is_empty()
        || subject_tokens
            .iter()
            .any(|token| token.is_any_word(&["target", "chosen", "choice", "that", "it"]))
    {
        return Ok(None);
    }
    let filter = parse_object_filter(subject_tokens, false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::subject_verb_goad_requirement(
        TargetAst::Object(filter, None, span_from_tokens(subject_tokens)),
        Until::YourNextTurn,
    )]))
}

pub fn parse_sentence_must_attack_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_must_attack_creature_type_choice_shape(clause.tokens())
    else {
        return Ok(None);
    };
    use crate::effect::Until;

    if !shape.trailing_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }
    let filter_clause = SubjectVerbPrimitiveClause::new(shape.filter_subject_tokens).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice attack subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }
    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_grant_abilities_all(
            filter,
            vec![crate::cards::builders::GrantedAbilityAst::MustAttack],
            Until::EndOfTurn,
        ),
    ]))
}

pub fn parse_sentence_put_sticker_on(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(effect) = parse_put_ability_sticker_with_ticket_terms(clause.tokens())? {
        return Ok(Some(vec![effect]));
    }
    let Some(shape) = combat_shapes::parse_put_sticker_shape(clause.tokens()) else {
        return Ok(None);
    };
    let target_clause = SubjectVerbPrimitiveClause::new(shape.target_tokens).trimmed();

    if let Some((sticker_target, aura_target, attachment_filter)) =
        parse_put_sticker_then_becomes_aura(target_clause)?
    {
        return Ok(Some(vec![
            EffectAst::subject_verb_put_sticker(sticker_target, shape.action),
            EffectAst::subject_verb_become_aura_enchantment(
                aura_target,
                attachment_filter,
                crate::effect::Until::Forever,
            ),
        ]));
    }

    // "put up to two name stickers on it" (_____ _____ Rocketship): each
    // sticker is its own (optional, for "up to") placement.
    let repeat = {
        let words = crate::lexer::token_word_refs(clause.tokens());
        match words.as_slice() {
            ["put", "up", "to", amount, ..] => {
                crate::util::parse_number_word_u32(amount).map(|amount| (amount, true))
            }
            ["put", amount, _, "stickers", ..] | ["put", amount, "stickers", ..] => {
                crate::util::parse_number_word_u32(amount)
                    .filter(|amount| *amount > 1)
                    .map(|amount| (amount, false))
            }
            _ => None,
        }
    };
    let repeated = |put: EffectAst| -> EffectAst {
        let Some((amount, optional)) = repeat else {
            return put;
        };
        let placement = if optional {
            EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May {
                effects: vec![put],
            })
        } else {
            put
        };
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
            count: crate::effect::Value::Fixed(i32::try_from(amount).unwrap_or(i32::MAX)),
            effects: vec![placement],
        })
    };

    if shape.target_is_reference {
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![repeated(EffectAst::subject_verb_put_sticker(
            target,
            shape.action,
        ))]));
    }

    let mut filter = parse_object_filter(target_clause.tokens(), false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    Ok(Some(vec![EffectAst::subject_verb_put_sticker(
        TargetAst::Object(filter, None, None),
        shape.action,
    )]))
}

/// "put an ability sticker with ticket cost X or less on it [without paying
/// that sticker's ticket cost]" (Pin Collection).
fn parse_put_ability_sticker_with_ticket_terms(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let view = crate::lexer::TokenWordView::new(tokens);
    let words = view.word_refs();
    let ["put", "an" | "a", "ability", "sticker", "with", "ticket", "cost", _, "or", "less", "on", ..] =
        words.as_slice()
    else {
        return Ok(None);
    };
    let without = words.iter().position(|word| *word == "without");
    let free = match without {
        Some(idx) => {
            let tail = &words[idx..];
            if !(tail.starts_with(&["without", "paying"]) && tail.ends_with(&["ticket", "cost"])) {
                return Ok(None);
            }
            true
        }
        None => false,
    };
    let target_end = without.unwrap_or(words.len());
    if target_end <= 11 {
        return Ok(None);
    }
    let (Some(cap_span), Some(target_span)) = (
        view.token_span_for_words(7, 8),
        view.token_span_for_words(11, target_end),
    ) else {
        return Ok(None);
    };
    let Some((cap, used)) = parse_value(&tokens[cap_span.clone()]) else {
        return Ok(None);
    };
    if used != cap_span.len() {
        return Ok(None);
    }
    let target = parse_target_phrase(&tokens[target_span])?;
    Ok(Some(EffectAst::subject_verb_put_sticker_with_ticket_terms(
        target,
        crate::events::KeywordActionKind::AbilitySticker,
        Some(cap),
        free,
    )))
}

fn parse_put_sticker_then_becomes_aura(
    target_clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(TargetAst, TargetAst, ObjectFilter)>, CardTextError> {
    let Some(shape) = combat_shapes::parse_sticker_aura_shape(target_clause.tokens()) else {
        return Ok(None);
    };

    Ok(Some((
        parse_target_phrase(shape.sticker_target_tokens)?,
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(shape.sticker_target_tokens),
        ),
        parse_object_filter(shape.enchant_filter_tokens, false)?,
    )))
}

pub fn parse_sentence_return_targets_of_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_return_creature_type_choice_shape(clause.tokens())
    else {
        return Ok(None);
    };
    if shape.base_target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before chosen-type qualifier (clause: '{}')",
            clause.text()
        )));
    }
    let mut filter = parse_object_filter(&shape.base_target_tokens, false)?;
    if shape.excluded {
        filter.excluded_chosen_creature_type = true;
    } else {
        filter.chosen_creature_type = true;
    }

    let mut effects = Vec::new();
    if shape.needs_inline_choice_effect {
        effects.push(EffectAst::subject_verb_choose_creature_type(
            PlayerAst::You,
            vec![],
        ));
    }

    if shape.has_explicit_target {
        let mut target = parse_target_phrase(&shape.base_target_tokens)?;
        // Recursively patch `chosen_creature_type` / `excluded_chosen_creature_type`
        // on the ObjectFilter buried inside the TargetAst (may be wrapped in WithCount).
        fn patch_chosen_type(t: &mut TargetAst, chosen: bool, excluded: bool) {
            match t {
                TargetAst::Object(f, _, _) => {
                    f.chosen_creature_type |= chosen;
                    f.excluded_chosen_creature_type |= excluded;
                }
                TargetAst::WithCount(inner, _) => patch_chosen_type(inner, chosen, excluded),
                _ => {}
            }
        }
        patch_chosen_type(
            &mut target,
            filter.chosen_creature_type,
            filter.excluded_chosen_creature_type,
        );
        effects.push(EffectAst::subject_verb_return_to_hand(target, false));
    } else {
        effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
    }

    Ok(Some(effects))
}

pub fn parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    let Some(shape) = combat_shapes::parse_choose_all_zones_to_hand_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut base_filter = parse_object_filter(shape.filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_text
        ))
    })?;
    base_filter.controller = None;

    let mut battlefield_filter = base_filter.clone();
    battlefield_filter.zone = Some(shape.zones[0]);

    let mut graveyard_filter = base_filter;
    graveyard_filter.zone = Some(shape.zones[1]);

    Ok(Some(vec![
        EffectAst::subject_verb_return_all_to_hand(battlefield_filter),
        EffectAst::subject_verb_return_all_to_hand(graveyard_filter),
    ]))
}

pub fn parse_sentence_return_multiple_targets(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // A single printed "return" can coordinate objects with different
    // destinations, as in "Return this card to its owner's hand and [a]
    // creature card ... to the battlefield under your control." Try each
    // conjunction boundary as two complete return clauses before applying
    // the shared-destination grammar below. Reusing `parse_return` keeps each
    // destination's zone and controller typed independently.
    let tokens = clause.tokens();
    if tokens.first().and_then(OwnedLexToken::as_word) == Some("return") {
        for and_idx in 2..tokens.len().saturating_sub(1) {
            if tokens[and_idx].as_word() != Some("and") {
                continue;
            }
            let left = trim_lexed_commas(&tokens[1..and_idx]);
            let mut right = trim_lexed_commas(&tokens[and_idx + 1..]);
            if left.is_empty() || right.is_empty() {
                continue;
            }
            if right.first().and_then(OwnedLexToken::as_word) == Some("return") {
                right = trim_lexed_commas(&right[1..]);
            }
            if let (Ok(left), Ok(right)) = (parse_return(left), parse_return(right)) {
                return Ok(Some(vec![EffectAst::Coordinated {
                    effects: vec![left, right],
                    leading_duration: false,
                    result_conjunction: false,
                }]));
            }
        }
    }

    let Some(shape) = combat_shapes::parse_return_multiple_targets_shape(clause.tokens()) else {
        return Ok(None);
    };
    let targets_clause = SubjectVerbPrimitiveClause::new(shape.targets_tokens).trimmed();
    let destination = shape.destination;

    let mut segments: Vec<SubjectVerbPrimitiveOwnedClause> = Vec::new();
    for segment_clause in targets_clause.trimmed_and_comma_segments() {
        let facts = combat_shapes::parse_return_segment_facts(segment_clause.tokens());
        if !segments.is_empty()
            && !facts.starts_new_target
            && !facts.mentions_target
            && !facts.starts_like_zone_suffix
        {
            let last = segments.last_mut().expect("segments is non-empty");
            last.append_comma_then(segment_clause);
        } else {
            segments.push(SubjectVerbPrimitiveOwnedClause::from_clause(segment_clause));
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| combat_shapes::parse_return_segment_facts(segment.tokens()).quantifier);

    let shared_suffix = segments
        .last()
        .and_then(|segment| combat_shapes::return_zone_suffix_tokens(segment.tokens()))
        .map(<[OwnedLexToken]>::to_vec)
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        let mut facts = combat_shapes::parse_return_segment_facts(segment.tokens());
        if !facts.mentions_zone && !shared_suffix.is_empty() {
            segment.extend_from_slice(&shared_suffix);
            facts = combat_shapes::parse_return_segment_facts(segment.tokens());
        }
        if let Some(quantifier) = shared_quantifier
            && facts.quantifier.is_none()
            && !facts.starts_like_target_reference
            && !facts.mentions_target
        {
            segment.insert_leading_word(quantifier.as_str());
            facts = combat_shapes::parse_return_segment_facts(segment.tokens());
        }
        if facts.quantifier.is_some() {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    clause.text()
                )));
            }
            let filter = parse_object_filter(segment.from_tokens(1), false)?;
            if destination.zone == Zone::Battlefield {
                effects.push(EffectAst::subject_verb_return_all_to_battlefield(
                    filter,
                    destination.tapped,
                    false,
                    ReturnControllerAst::Owner,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
            }
        } else {
            let target = parse_target_phrase(segment.tokens())?;
            if destination.zone == Zone::Battlefield {
                effects.push(EffectAst::subject_verb_return_to_battlefield(
                    target,
                    destination.tapped,
                    false,
                    false,
                    ReturnControllerAst::Preserve,
                    None,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_to_hand(target, false));
            }
        }
    }

    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub fn parse_sentence_for_each_of_target_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_target_objects_shape(clause.tokens()) else {
        return Ok(None);
    };

    let subject_clause = SubjectVerbPrimitiveClause::new(shape.subject_tokens).trimmed();
    let Some(target) = parse_for_each_targeted_object_subject(subject_clause.tokens())? else {
        return Ok(None);
    };

    let effect_clause = SubjectVerbPrimitiveClause::new(shape.effect_tokens).trimmed();
    if effect_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause.text()
        )));
    }
    let mut per_target_effects = parse_effect_chain(effect_clause.tokens())?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        EffectAst::TagAffected {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            effect: Box::new(EffectAst::subject_verb_target_only(target)),
        },
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::It.bind(),
            effects: per_target_effects,
        }),
    ]))
}

pub fn parse_distribute_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    if clause
        .first_word()
        .is_none_or(|word| word != DISTRIBUTE_WORD)
    {
        return Ok(None);
    }

    let amount_clause = clause.from(1);
    if let Some(effect) = parse_distribute_stickers(amount_clause.tokens())? {
        return Ok(Some(effect));
    }
    // "distribute up to that many +1/+1 counters" (Lathiel): the counted
    // prefixes of "put ... counters" ("up to N", "that many") read the same.
    // "distribute a number of +1/+1 counters equal to the number of +1/+1
    // counters on this creature among ..." (Vastwood Hydra): the article
    // is not the amount; the "equal to" value is.
    let amount_words = amount_clause.trimmed_word_refs();
    let equal_to_amount = if amount_words.starts_with(&["a", "number", "of"]) {
        let tokens = amount_clause.tokens();
        let equal_idx = tokens
            .windows(2)
            .position(|pair| pair[0].is_word("equal") && pair[1].is_word("to"));
        let among_idx = tokens.iter().position(|token| token.is_word("among"));
        match (equal_idx, among_idx) {
            (Some(equal_idx), Some(among_idx)) if equal_idx + 2 < among_idx => {
                let value_tokens = &tokens[equal_idx + 2..among_idx];
                parse_value(value_tokens)
                    .filter(|(_, used)| *used == value_tokens.len())
                    .map(|(value, _)| value)
            }
            _ => None,
        }
    } else {
        None
    };
    let (count, used) = match equal_to_amount
        .map(|value| (value, 3))
        .or_else(|| parse_value(amount_clause.tokens()))
    {
        Some(parsed) => parsed,
        None => crate::effect_sentences::zone_counter_helpers::parse_put_counter_count_value(
            amount_clause.tokens(),
        )
        .ok()
        .filter(|(_, used)| *used > 0)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed counter amount (clause: '{}')",
                clause.text()
            ))
        })?,
    };
    let rest_clause = clause.from(1 + used);
    let counter_type = parse_counter_type_from_tokens(rest_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause.text()
        ))
    })?;
    let Some((_before_among, target_clause)) = rest_clause.split_once_on_word("among") else {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target clause after 'among' (clause: '{}')",
            clause.text()
        )));
    };
    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause.text()
        )));
    }
    // "among it and any number of commanders": the leading object is a
    // mandatory member of the division (it always receives at least one
    // counter); the counted phrase names the optional recipients.
    let mut required_recipient = None;
    let mut target_clause = target_clause;
    if let Some((member, rest)) = target_clause.split_once_on_word_trimmed("and")
        && !member.is_empty()
        && !rest.is_empty()
        && {
            let member_words = member.trimmed_word_refs();
            member_words == ["it"] || crate::util::is_source_reference_words(&member_words)
        }
        && parse_counter_target_count_prefix(rest.tokens())?.is_some()
    {
        required_recipient = Some(TargetAst::Source(span_from_tokens(member.tokens())));
        target_clause = rest;
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(target_clause.tokens())?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause.text()
            ))
        })?;
    let target_phrase = target_clause.from(used_count).trimmed();
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause.text()
        )));
    }
    let target = if target_phrase.tokens().iter().any(|token| token.is_word("target")) {
        parse_target_phrase(target_phrase.tokens())?
    } else {
        // "Among any number of creatures" chooses recipients at resolution;
        // it does not declare targets while casting the spell.
        TargetAst::Object(parse_object_filter(target_phrase.tokens(), false)?, None,
            span_from_tokens(target_phrase.tokens()))
    };

    let mut effect = EffectAst::subject_verb_put_counters(
        counter_type,
        count,
        target,
        Some(target_count),
        true,
    );
    if let Some(required) = required_recipient
        && let EffectAst::SubjectVerb(subject) = &mut effect
        && let crate::cards::builders::SubjectVerbActionAst::Counters(
            crate::cards::builders::CounterActionAst::PutCounters {
                required_recipient: slot,
                ..
            },
        ) = &mut subject.action
    {
        *slot = Some(required);
    }
    Ok(Some(effect))
}

/// "distribute up to two art stickers among one or two nonland permanents
/// you own" (Roxi, Publicist to the Stars). Stickers are placed one at a
/// time; choosing fewer recipients is always allowed, so the reachable
/// placements are exactly N (optional, for "up to N") placements of that
/// sticker kind, each on a matching permanent.
fn parse_distribute_stickers(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    let Some(among) = words.iter().position(|word| *word == "among") else {
        return Ok(None);
    };
    if !matches!(words.get(among.saturating_sub(1)), Some(&"sticker" | &"stickers")) {
        return Ok(None);
    }
    let view = crate::lexer::TokenWordView::new(tokens);
    let up_to = words.starts_with(&["up", "to"]);
    let amount_start = if up_to { 2 } else { 0 };
    let Some(amount_token) = view.map_word_to_token_start(amount_start) else {
        return Ok(None);
    };
    let Some((count, used)) = parse_value(&tokens[amount_token..]) else {
        return Ok(None);
    };
    let crate::effect::Value::Fixed(count) = count else {
        return Ok(None);
    };
    let Some(kind_end) = view.map_word_to_token_start(among) else {
        return Ok(None);
    };
    let kind_tokens = &tokens[amount_token + used..kind_end];
    let kind_words = crate::lexer::token_word_refs(kind_tokens);
    let action = match kind_words.as_slice() {
        ["art", "sticker" | "stickers"] => crate::events::KeywordActionKind::ArtSticker,
        ["ability", "sticker" | "stickers"] => crate::events::KeywordActionKind::AbilitySticker,
        ["name", "sticker" | "stickers"] => crate::events::KeywordActionKind::NameSticker,
        ["sticker" | "stickers"] => crate::events::KeywordActionKind::Sticker,
        _ => return Ok(None),
    };
    let target_tokens = &tokens[kind_end + 1..];
    let Some((_recipients, used_count)) = parse_counter_target_count_prefix(target_tokens)? else {
        return Ok(None);
    };
    let filter_tokens = &target_tokens[used_count..];
    if filter_tokens.is_empty() || filter_tokens.iter().any(|token| token.is_word("target")) {
        return Ok(None);
    }
    let mut filter = parse_object_filter(filter_tokens, false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    let put = EffectAst::subject_verb_put_sticker(TargetAst::Object(filter, None, None), action);
    let placement = if up_to {
        EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May {
            effects: vec![put],
        })
    } else {
        put
    };
    Ok(Some(EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
        count: crate::effect::Value::Fixed(count),
        effects: vec![placement],
    })))
}

pub fn parse_sentence_distribute_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let Some(primary) = parse_distribute_counters_sentence(head_clause)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        effects.extend(parse_effect_chain(tail_clause.tokens())?);
    }

    Ok(Some(effects))
}

pub fn parse_sentence_transform_with_followup(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = clause.token(0) else {
        return Ok(None);
    };
    let is_transform = token_is_word(first, TRANSFORM_WORD);
    let is_convert = token_is_word(first, CONVERT_WORD);
    if !is_transform && !is_convert {
        return Ok(None);
    }

    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let target_clause = head_clause.from(1).trimmed();
    // "transform this creature and your life total becomes 3" (Enduring
    // Angel): a self-reference followed by a coordinated instruction. The
    // source reader would otherwise accept the whole tail as "this creature"
    // and drop the second instruction.
    if tail_clause.is_none()
        && let Some(and_idx) = target_clause.tokens().iter().position(|token| token.is_word("and"))
        && and_idx > 0
        && and_idx + 1 < target_clause.tokens().len()
        && matches!(
            crate::grammar::effects::zone_counter_shapes::parse_transform_target_shape(
                &target_clause.tokens()[..and_idx]
            ),
            crate::grammar::effects::zone_counter_shapes::TransformTargetShape::Source { .. }
        )
    {
        let head = &target_clause.tokens()[..and_idx];
        let tail = &target_clause.tokens()[and_idx + 1..];
        let transform = if is_transform { parse_transform(head)? } else { parse_convert(head)? };
        let mut effects = vec![transform];
        effects.extend(parse_effect_chain(tail)?);
        return Ok(Some(effects));
    }
    let trailing_if =
        crate::grammar::structure::split_trailing_if_clause_lexed(target_clause.tokens());
    let transform_target_tokens = trailing_if
        .as_ref()
        .map(|condition| condition.leading_tokens)
        .unwrap_or_else(|| target_clause.tokens());
    let transform = if is_transform {
        parse_transform(transform_target_tokens)?
    } else {
        parse_convert(transform_target_tokens)?
    };
    let transform = if let Some(condition) = trailing_if {
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: condition.predicate,
            effects: vec![transform],
        })
    } else {
        transform
    };
    let Some(tail_clause) = tail_clause else {
        return Ok(Some(vec![transform]));
    };
    if tail_clause.is_empty() {
        return Ok(Some(vec![transform]));
    }

    let mut effects = vec![transform];
    effects.extend(parse_effect_chain(tail_clause.tokens())?);
    Ok(Some(effects))
}

pub fn parse_sentence_cant_effect(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_cant_effect_sentence)
}

pub fn parse_sentence_gain_x_plus_life(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_x_plus_life_sentence)
}

pub fn parse_sentence_for_each_exiled_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_this_way_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.subject_tokens.is_empty() || shape.effect_tokens.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_exiled_this_way_sentence)
}

pub fn parse_sentence_for_each_put_into_graveyard_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_this_way_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.subject_tokens.is_empty() || shape.effect_tokens.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_put_into_graveyard_this_way_sentence)
}

pub fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_each_player_put_permanent_cards_exiled_with_source_sentence)
}

pub fn parse_sentence_for_each_destroyed_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_this_way_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.subject_tokens.is_empty() || shape.effect_tokens.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_destroyed_this_way_sentence)
}

pub fn parse_sentence_search_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_search_library_sentence)
}

pub fn parse_sentence_shuffle_graveyard_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_graveyard_into_library_sentence)
}

pub fn parse_sentence_shuffle_object_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_object_into_library_sentence)
}

pub fn parse_sentence_exile_hand_and_graveyard_bundle(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_hand_and_graveyard_bundle_sentence)
}

pub fn parse_sentence_target_player_exiles_creature_and_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_target_player_exiles_creature_and_graveyard_sentence)
}

pub fn parse_sentence_look_at_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_hand_sentence)
}

pub fn parse_sentence_look_at_top_then_exile_one(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_top_then_exile_one_sentence)
}

pub fn parse_sentence_gain_life_equal_to_age(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_life_equal_to_age_sentence)
}

pub fn parse_sentence_for_each_player_doesnt(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_one_with_lexed(parse_for_each_player_doesnt)
}

pub(super) use delayed_shapes::DelayedTimingStepShape as DelayedNextStepKind;

pub(super) fn delayed_next_step_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize, DelayedNextStepKind, PlayerAst)> {
    delayed_shapes::parse_delayed_timing_marker_shape(clause.tokens())
        .map(|shape| (shape.start_word, shape.end_word, shape.step, shape.player))
}

#[cfg(test)]
mod coordinated_return_tests {
    use super::*;
    use crate::cards::builders::ZoneMoveActionAst;
    use crate::lexer::lex_line;

    #[test]
    fn preserves_distinct_destinations_and_controller_in_coordinated_return() {
        let tokens = lex_line(
            "return this card to its owner's hand and up to one creature card milled this way to the battlefield under your control",
            0,
        )
        .expect("lex coordinated return");
        let effects =
            parse_sentence_return_multiple_targets(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse coordinated return")
                .expect("coordinated return shape");
        let effects = match effects.as_slice() {
            [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
            effects => effects,
        };
        let [hand, battlefield] = effects else {
            panic!("expected two return branches");
        };

        assert!(matches!(
            hand,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. }),
                ..
            })
        ));
        assert!(matches!(
            battlefield,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                    controller: ReturnControllerAst::You,
                    ..
                }),
                ..
            })
        ));
    }

    #[test]
    fn same_destination_coordinated_return_preserves_explicit_target() {
        let tokens = lex_line(
            "return this creature to its owner's hand and return target griffin card from your graveyard to your hand",
            0,
        )
        .expect("lex coordinated return");
        let effects =
            parse_sentence_return_multiple_targets(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse coordinated return")
                .expect("coordinated return shape");
        let effects = match effects.as_slice() {
            [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
            effects => effects,
        };
        let [
            _,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }),
                ..
            }),
        ] = effects
        else {
            panic!("expected a second return-to-hand action");
        };

        assert!(matches!(target, TargetAst::Object(_, Some(_), _)));
    }

    #[test]
    fn same_destination_coordinated_return_accepts_independent_optional_targets() {
        let tokens = lex_line(
            "return up to one target artifact card and up to one target land card from your graveyard to the battlefield",
            0,
        )
        .expect("lex coordinated graveyard return");
        let effects =
            parse_sentence_return_multiple_targets(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse coordinated graveyard return")
                .expect("coordinated return shape");
        let [EffectAst::Coordinated { effects, .. }] = effects.as_slice() else {
            panic!("expected one coordinated return: {effects:#?}");
        };

        assert_eq!(effects.len(), 2, "{effects:#?}");
        assert!(effects.iter().all(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ZoneMoves(
                    ZoneMoveActionAst::ReturnToBattlefield { .. }
                ),
                ..
            })
        )));
    }
}

/// "[Then] for each card you put on top of your library [this way], <effects>"
/// (Starving Revenant, after "surveil N"): iterate the cards the preceding
/// surveil kept on top, which the surveil publishes under its well-known
/// kept-on-top tag.
pub fn parse_sentence_for_each_card_kept_on_top_of_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_for_each_card_kept_on_top_of_library_sentence)
}

fn parse_for_each_card_kept_on_top_of_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::grammar::primitives;
    let tokens = trim_lexed_commas(tokens);
    let tokens = primitives::strip_lexed_prefix_phrase(tokens, &["then"])
        .map(trim_lexed_commas)
        .unwrap_or(tokens);
    let Some(rest) = primitives::strip_lexed_prefix_phrase(
        tokens,
        &["for", "each", "card", "you", "put", "on", "top", "of", "your", "library"],
    ) else {
        return Ok(None);
    };
    let rest = trim_lexed_commas(rest);
    let effect_tokens = primitives::strip_lexed_prefix_phrase(rest, &["this", "way"])
        .unwrap_or(rest);
    let effect_tokens = crate::util::trim_edge_punctuation_tokens(trim_lexed_commas(effect_tokens));
    if effect_tokens.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::CompilerReferenceTag::SurveilKeptOnTop.bind(),
        effects,
    })]))
}
