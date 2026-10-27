use super::*;
use crate::cards::builders::KeywordActionAst;
use crate::cards::builders::LifeResourceActionAst;
use crate::cards::builders::ObjectChoiceEffectAst;
use crate::cards::builders::PredicateAst;
use crate::cards::builders::TokenActionAst;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;

fn cost_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    crate::word_primitives::sequence_occurs(words, phrase)
}

fn is_exact_per_target_cost_modifier(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(idx, word)| {
        *word == "for"
            && words.get(idx + 1) == Some(&"each")
            && words.get(idx + 2) == Some(&"target")
            && words.get(idx + 3) != Some(&"beyond")
    })
}

/// "for each creature it targets" (Battlefield Thaumaturge): the modifier is
/// applied once per chosen target matching the description (CR 601.2f), not
/// once per matching object on the battlefield.
fn parse_per_matching_target_cost_modifier(
    remaining_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(remaining_tokens);
    let words = TokenWordView::new(tokens);
    let word_refs = words.to_word_refs();
    let Some(each) = crate::word_primitives::parse_sequence_start(&word_refs, &["for", "each"])
    else {
        return Ok(None);
    };
    let Some(targets) = [&["it", "targets"][..], &["that", "spell", "targets"][..]]
        .iter()
        .find_map(|suffix| suffix_word_start(&word_refs, suffix))
    else {
        return Ok(None);
    };
    if targets <= each + 2 {
        return Ok(None);
    }
    let Some(range) = words.token_span_for_words(each + 2, targets) else {
        return Ok(None);
    };
    let Some(mut filter) = parse_complete_cost_count_filter(&tokens[range])? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported per-target cost modifier (clause: '{}')",
            word_refs.join(" ")
        )));
    };
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(filter))
}

fn suffix_word_start(words: &[&str], suffix: &[&str]) -> Option<usize> {
    crate::word_primitives::parse_sequence_suffix(words, suffix).then(|| words.len() - suffix.len())
}

fn apply_anywhere_other_than_hand_origin(filter: &mut ObjectFilter) {
    filter.any_of = [
        (Zone::Hand, Some(PlayerFilter::NotYou)),
        (Zone::Library, None),
        (Zone::Battlefield, None),
        (Zone::Graveyard, None),
        (Zone::Exile, None),
        (Zone::Command, None),
        (Zone::OutsideGame, None),
    ]
    .into_iter()
    .map(|(zone, owner)| {
        let mut branch = ObjectFilter::default();
        branch.zone = Some(zone);
        branch.owner = owner;
        branch
    })
    .collect();
}

/// Parse a shared first-spell subject that receives both a cost reduction and
/// flash timing. The same typed filter drives both capabilities, including the
/// per-turn ordinal, spell type, and caster restriction.
pub fn parse_first_spell_cost_reduction_and_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    const TAIL: [&str; 9] = [
        "and", "can", "be", "cast", "as", "though", "it", "had", "flash",
    ];
    let words = crate::lexer::token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_suffix(&words, &TAIL) {
        return Ok(None);
    }
    // The ordinary reducer deliberately tolerates trailing coordination and
    // already proves the complete first-spell cost shape. Reuse that exact
    // proven reduction rather than maintaining a second token-boundary model.
    let Some(reduction) = parse_spells_cost_modifier_line(tokens)? else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::CostReduction(reduction_spec) = &reduction.payload
    else {
        return Ok(None);
    };
    if !reduction_spec.filter.first_spell_cast_each_turn
        || reduction_spec.filter.cast_by != Some(PlayerFilter::You)
    {
        return Ok(None);
    }
    let flash = StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::flash_timing_for_spells_matching(
            reduction_spec.filter.clone(),
        ),
    );
    Ok(Some(vec![reduction.into(), flash.into()]))
}

/// Lower a source-card graveyard permission with a dynamic generic surcharge
/// into the two reusable static capabilities that enforce it. Keeping these
/// in one static source-line chunk preserves both runtime behavior and the
/// authored single-sentence presentation.
pub fn parse_source_graveyard_dynamic_surcharge_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(fact) =
        permission_graveyard_facts::parse_source_graveyard_dynamic_surcharge_tokens(tokens)
    else {
        return Ok(None);
    };
    let Some(parsed_cost) = parse_leaf_fixed_mana_cost_prefix_tokens(fact.cost_tokens) else {
        return Ok(None);
    };
    if parsed_cost.consumed != fact.cost_tokens.len() {
        return Ok(None);
    }
    let [pip] = parsed_cost.cost.pips() else {
        return Ok(None);
    };
    let [ManaSymbol::Generic(multiplier)] = pip.as_slice() else {
        return Ok(None);
    };
    if *multiplier == 0 {
        return Ok(None);
    }
    let Some(repetitions) = parse_dynamic_cost_modifier_value(fact.repetition_tokens)? else {
        return Ok(None);
    };
    let source_surface = render_token_slice(fact.source_tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if source_surface.is_empty() {
        return Ok(None);
    }

    let permission_filter = ObjectFilter::source_with_surface(
        crate::target::SourceReferenceSurface::ThisPermanentType(source_surface),
    );
    let permission = StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            permission_filter,
            Zone::Graveyard,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    let increase = StaticAbility::new(crate::model::CompilerCostIncrease::new(
        ObjectFilter::source(),
        scale_dynamic_cost_modifier_value(repetitions, i32::from(*multiplier)),
    ));

    Ok(Some(vec![permission.into(), increase.into()]))
}

fn apply_this_spell_cost_increase_condition(
    filter: &mut ObjectFilter,
    condition: crate::static_abilities::ThisSpellCostCondition,
    clause_words: &[&str],
) -> Result<Option<crate::ConditionExpr>, CardTextError> {
    use crate::static_abilities::ThisSpellCostCondition;

    match condition {
        ThisSpellCostCondition::Always => Ok(None),
        ThisSpellCostCondition::YourTurn => Ok(Some(crate::ConditionExpr::YourTurn)),
        ThisSpellCostCondition::NotYourTurn => Ok(Some(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::YourTurn,
        )))),
        // A spell-cost condition is already the bound form; this maps one
        // runtime condition onto another, so it stays on that side.
        ThisSpellCostCondition::ConditionExpr { condition, .. }
        | ThisSpellCostCondition::AsLongAsConditionExpr { condition, .. } => Ok(Some(condition)),
        ThisSpellCostCondition::TargetsPlayer(player) => {
            filter.targets_player = Some(player);
            Ok(None)
        }
        ThisSpellCostCondition::TargetsObject(object) => {
            filter.targets_object = Some(Box::new(object));
            Ok(None)
        }
        other => Err(CardTextError::ParseError(format!(
            "unsupported self-only cost-increase condition (clause: '{}'; condition: {other:?})",
            clause_words.join(" ")
        ))),
    }
}

pub fn parse_cost_modifier_target_spec(
    target_tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<Box<ObjectFilter>>, bool), CardTextError> {
    let alternatives = crate::grammar::primitives::split_lexed_slices_on_or(target_tokens);
    if let [left, right] = alternatives.as_slice() {
        let (left_player, left_object, left_any_of) = parse_cost_modifier_target_spec(left)?;
        let (right_player, right_object, right_any_of) = parse_cost_modifier_target_spec(right)?;
        if !left_any_of && !right_any_of {
            match (left_player, left_object, right_player, right_object) {
                (Some(player), None, None, Some(object))
                | (None, Some(object), Some(player), None) => {
                    return Ok((Some(player), Some(object), true));
                }
                _ => {}
            }
        }
    }

    // "spells that target it" in a static cost modifier: with no other
    // antecedent on the line, the pronoun names the source itself
    // ("As long as Kaervek's Torch is on the stack, spells that target it
    // cost {2} more to cast.").
    if let [only] = target_tokens
        && only.is_word("it")
    {
        return Ok((None, Some(Box::new(ObjectFilter::source())), false));
    }
    // "Spells you cast that target enchanted player" (Infectious Curse): a
    // player Aura's host is a player, never an enchanted permanent.
    if let [first, second] = target_tokens
        && first.is_word("enchanted")
        && second.is_word("player")
    {
        return Ok((
            Some(PlayerFilter::TaggedPlayer(
                (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
            )),
            None,
            false,
        ));
    }

    match static_mid_facts::parse_cost_modifier_target_fact(target_tokens) {
        Some(static_mid_facts::CostTargetFact::You) => Ok((Some(PlayerFilter::You), None, false)),
        Some(static_mid_facts::CostTargetFact::Opponent) => {
            Ok((Some(PlayerFilter::Opponent), None, false))
        }
        Some(static_mid_facts::CostTargetFact::AnyPlayer) => {
            Ok((Some(PlayerFilter::Any), None, false))
        }
        Some(static_mid_facts::CostTargetFact::Object(filter)) => {
            Ok((None, Some(Box::new(filter)), false))
        }
        None => Ok((
            None,
            Some(Box::new(parse_object_filter(target_tokens, false)?)),
            false,
        )),
    }
}

pub fn parse_cost_modifier_prefix_condition(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Result<(Option<PredicateAst>, usize), CardTextError> {
    if let Some(prefix) =
        keyword_static_lines::parse_cost_prefix_condition_tokens(tokens, spells_token_idx)
    {
        match prefix {
            keyword_static_lines::CostPrefixCondition::DuringTurnsOtherThanYours {
                subject_start,
            } => {
                return Ok((
                    Some(PredicateAst::Not(Box::new(PredicateAst::YourTurn))),
                    subject_start,
                ));
            }
            keyword_static_lines::CostPrefixCondition::DuringYourTurn { subject_start } => {
                return Ok((Some(PredicateAst::YourTurn), subject_start));
            }
            keyword_static_lines::CostPrefixCondition::AsLongAs {
                condition_tokens,
                subject_start,
            } => {
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after leading 'as long as' clause (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    )));
                }
                let condition = if crate::grammar::functional_zones::parse_source_command_or_battlefield_condition_tokens(condition_tokens) {
                    PredicateAst::Or(
                        Box::new(PredicateAst::Source(SourcePredicateAst::SourceIsInZone(Zone::Command))),
                        Box::new(PredicateAst::Source(SourcePredicateAst::SourceIsInZone(Zone::Battlefield))),
                    )
                } else { match parse_static_condition_clause(condition_tokens) {
                    Ok(condition) => condition,
                    // Both branches are recognized predicates now.
                    Err(_) => parse_source_tap_status_condition_lexed(condition_tokens)
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unsupported static condition clause (clause: '{}')",
                                crate::lexer::token_word_refs(condition_tokens).join(" ")
                            ))
                        })?,
                } };
                return Ok((Some(condition), subject_start));
            }
        }
    }

    Ok((None, 0))
}

/// "This effect reduces only the amount of colored mana you pay" (or "... of
/// green mana ..."). Without this sentence, the part of a colored reduction a
/// cost doesn't require reduces its generic mana instead (CR 118.7b-c).
pub fn cost_reduction_is_colored_only(tokens: &[OwnedLexToken]) -> bool {
    use winnow::Parser as _;
    crate::grammar::primitives::find_prefix(tokens, || {
        crate::grammar::primitives::phrase(&["reduces", "only", "the", "amount", "of"]).void()
    })
    .is_some()
}

pub fn parse_optional_life_additional_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let additional_words = crate::lexer::token_word_refs(tokens);
    let Some(spec) = static_keyword_cost_shapes::parse_additional_cost_spell_filter(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.spell_filter_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_spell_filter_with_grammar_entrypoint(&subject_tokens);
    let subject_words = crate::lexer::token_word_refs(&subject_tokens);
    if static_keyword_cost_shapes::parse_optional_life_subject_is_permanent(&subject_words) {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    filter.cast_by = Some(PlayerFilter::You);

    let Some(optional_life_shape) =
        static_keyword_cost_shapes::parse_optional_life_reduction_words(&additional_words)
    else {
        return Ok(None);
    };
    let pay_word_idx = optional_life_shape.pay.word;
    let payment_words = &additional_words[pay_word_idx + 1..];
    let Some(life_cost) = payment_words
        .first()
        .and_then(|word| parse_number_word_i32(word))
        .and_then(|amount| crate::util::narrowed_u32(amount))
    else {
        return Ok(None);
    };
    if !optional_life_shape.payment_has_life {
        return Ok(None);
    }

    if !optional_life_shape.those_spells_paid_life_this_way {
        return Ok(None);
    }
    let costs_word_idx = optional_life_shape.costs.word;
    let Some(costs_idx) = static_keyword_shapes::parse_word_token_offset(tokens, costs_word_idx)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[costs_idx + 1..];
    let (_, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let Some((reduction, _)) = parsed_mana_cost else {
        return Ok(None);
    };
    let remaining_words = crate::lexer::token_word_refs(amount_tokens);
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::Less)
        || !static_keyword_cost_shapes::parse_cost_modifier_cast_marker(&remaining_words)
    {
        return Ok(None);
    }

    let label_end = locate_token_kind(tokens, TokenKind::Period)
        .map(|idx| idx + 1)
        .unwrap_or(costs_idx);
    let label = render_token_slice(&tokens[..label_end])
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::new(
        crate::model::CompilerCostReductionManaCost::new(filter, reduction)
            .with_optional_life_additional_cost(label, life_cost)
            .with_colored_only(cost_reduction_is_colored_only(tokens)),
    )))
}

/// The words after "As an additional cost to cast <subject> spells," in a
/// granted pay-any-amount-of-mana entry-counter cost.
const ANY_AMOUNT_OF_MANA_ENTRY_COUNTERS_TAIL: &[&str] = &[
    "you", "may", "pay", "any", "amount", "of", "mana", "if", "you", "do", "that", "creature",
    "enters", "with", "that", "many", "additional", "+1/+1", "counters", "on", "it",
];

/// "As an additional cost to cast creature spells, you may pay any amount of
/// mana. If you do, that creature enters with that many additional +1/+1
/// counters on it." (Chorus of the Conclave): a granted optional additional
/// cost on the matching spells you cast (CR 601.2b, 601.2f), linked to the
/// resulting creature's entry counters (CR 614.1c).
pub fn parse_as_additional_cost_any_amount_of_mana_entry_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(spec) = static_keyword_cost_shapes::parse_additional_cost_spell_filter(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.spell_filter_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let words = crate::lexer::parser_token_word_refs(tokens);
    let subject_word_count = crate::lexer::parser_token_word_refs(spec.spell_filter_tokens).len();
    // "as an additional cost to cast" + subject + "spells"
    let Some(tail) = words.get(6 + subject_word_count + 1..) else {
        return Ok(None);
    };
    if tail != ANY_AMOUNT_OF_MANA_ENTRY_COUNTERS_TAIL {
        return Ok(None);
    }
    let mut filter = parse_spell_filter_with_grammar_entrypoint(&subject_tokens);
    // "that creature enters": the subject names creature spells.
    if filter.card_types != vec![CardType::Creature] {
        return Ok(None);
    }
    filter.cast_by = Some(PlayerFilter::You);
    let display = render_token_slice(tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::grant_spell_keyword(
        filter,
        ironsmith_core::GrantedSpellKeyword::intrinsic(
            ironsmith_core::GrantedSpellKeywordKind::AnyAmountOfManaForEntryCounters,
        ),
        None,
        display,
    )))
}

fn parse_cost_reduction_characteristic_intersection(
    tokens: &[OwnedLexToken],
) -> Result<Option<ironsmith_core::CostReductionCharacteristicIntersection>, CardTextError> {
    // "for each of the chosen colors it is" (Seal of the Guildpact): the
    // spell's colors among the source's chosen colors.
    if (0..tokens.len()).any(|start| {
        crate::grammar::primitives::parse_prefix(
            &tokens[start..],
            crate::grammar::primitives::phrase(&[
                "for", "each", "of", "the", "chosen", "colors", "it", "is",
            ]),
        )
        .is_some()
    }) {
        return Ok(Some(
            ironsmith_core::CostReductionCharacteristicIntersection::source_chosen_colors(),
        ));
    }
    // "for each card with the same name as that spell in your graveyard"
    // (Locket of Yesterdays): one per comparison card named like the spell.
    for start in 0..tokens.len() {
        let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
            &tokens[start..],
            crate::grammar::primitives::phrase(&["for", "each"]),
        ) else {
            continue;
        };
        let Some((name_start, _, after)) = crate::grammar::primitives::find_prefix(rest, || {
            crate::grammar::primitives::phrase(&[
                "with", "the", "same", "name", "as", "that", "spell",
            ])
        }) else {
            continue;
        };
        let mut comparison_tokens = rest[..name_start].to_vec();
        comparison_tokens.extend_from_slice(after);
        let comparison_tokens = trim_commas(&comparison_tokens);
        if comparison_tokens.is_empty() {
            continue;
        }
        let comparison = parse_object_filter(&comparison_tokens, false)?;
        return Ok(Some(
            ironsmith_core::CostReductionCharacteristicIntersection::new(
                ironsmith_core::ObjectCharacteristic::Name,
                comparison,
            )
            .counting_matching_objects()
            .with_comparison_surface(
                render_token_slice(&comparison_tokens)
                    .trim()
                    .trim_end_matches('.')
                    .to_string(),
            ),
        ));
    }
    let characteristic_at = |start: usize| {
        if tokens.get(start).is_some_and(|token| token.is_word("card"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("type") || token.is_word("types"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::CardType, 2));
        }
        if tokens
            .get(start)
            .is_some_and(|token| token.is_word("permanent"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("type") || token.is_word("types"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::PermanentType, 2));
        }
        if tokens
            .get(start)
            .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("type") || token.is_word("types"))
        {
            return Some((
                ironsmith_core::ObjectCharacteristic::Subtype(
                    crate::types::SubtypeFamily::Creature,
                ),
                2,
            ));
        }
        if tokens
            .get(start)
            .is_some_and(|token| token.is_word("color"))
            || tokens
                .get(start)
                .is_some_and(|token| token.is_word("colors"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::Color, 1));
        }
        if tokens.get(start).is_some_and(|token| token.is_word("mana"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("value"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::ManaValue, 2));
        }
        None
    };

    for each_index in 0..tokens.len().saturating_sub(5) {
        if !tokens[each_index].is_word("for") || !tokens[each_index + 1].is_word("each") {
            continue;
        }
        let Some((characteristic, characteristic_len)) = characteristic_at(each_index + 2) else {
            continue;
        };
        let subject_index = each_index + 2 + characteristic_len;
        if !tokens
            .get(subject_index)
            .is_some_and(|token| token.is_word("they") || token.is_word("it"))
            || !tokens
                .get(subject_index + 1)
                .is_some_and(|token| token.is_word("share") || token.is_word("shares"))
            || !tokens
                .get(subject_index + 2)
                .is_some_and(|token| token.is_word("with"))
        {
            continue;
        }
        let comparison_tokens = trim_commas(&tokens[subject_index + 3..]);
        if comparison_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "missing comparison set after shared-characteristic cost reduction".to_string(),
            ));
        }
        let comparison = parse_object_filter(&comparison_tokens, false)?;
        let comparison_surface = render_token_slice(&comparison_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return Ok(Some(
            ironsmith_core::CostReductionCharacteristicIntersection::new(
                characteristic,
                comparison,
            )
            .with_comparison_surface(comparison_surface),
        ));
    }

    Ok(None)
}

pub fn parse_spells_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if spell_additional_life_cost_per_target_amount(tokens).is_some() {
        return Ok(None);
    }
    if let Some(ability) = parse_optional_life_additional_cost_reduction_line(tokens)? {
        return Ok(Some(ability));
    }

    let clause_words = crate::lexer::parser_token_word_refs(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }
    let Some(spells_token_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    // A chosen-option label ("• Dragons — Spells your opponents cast ...",
    // Monastery Siege) is not part of the spell subject; parse the body alone.
    if let Some(label_end) = tokens[..spells_token_idx]
        .iter()
        .rposition(|token| matches!(token.kind, crate::lexer::TokenKind::EmDash))
    {
        return parse_spells_cost_modifier_line(&tokens[label_end + 1..]);
    }

    let first_spell_fact = static_mid_facts::parse_first_spell_each_turn_cost_fact(tokens);
    let second_spell_each_turn = crate::word_primitives::sequence_occurs(
        &clause_words,
        &["second", "spell", "you", "cast", "each", "turn"],
    );

    let (prefix_condition, subject_start) =
        parse_cost_modifier_prefix_condition(tokens, spells_token_idx)?;
    if subject_start > spells_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start..spells_token_idx]);
    let is_this_spell = is_this_subject_reference_lexed(&subject_tokens);

    let Some(cost_token_idx) =
        static_mid_facts::parse_cost_component_boundary(tokens, spells_token_idx + 1)
            .map(|boundary| boundary.cost_token)
    else {
        return Ok(None);
    };
    if cost_token_idx <= spells_token_idx {
        return Ok(None);
    }

    let mut filter = if is_this_spell {
        ObjectFilter::default()
    } else {
        parse_spell_filter_with_grammar_entrypoint(&subject_tokens)
    };
    // "Commander spells you cast cost ..." (Myth Unbound, via the "your
    // commander costs" rewrite): the commander designation restricts the
    // subject and must not be dropped into "spells you cast".
    if !is_this_spell
        && !filter.is_commander
        && crate::lexer::parser_token_word_refs(&subject_tokens)
            .iter()
            .any(|word| matches!(*word, "commander" | "commanders"))
    {
        filter.is_commander = true;
    }
    if first_spell_fact.is_some() && !is_this_spell {
        filter.first_spell_cast_each_turn = true;
    }
    if second_spell_each_turn && !is_this_spell {
        filter.spell_cast_ordinal_each_turn = Some(2);
    }

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    if !is_this_spell {
        let between_fact = static_mid_facts::parse_spell_cost_between_fact(between_tokens);
        let between_words = crate::lexer::parser_token_word_refs(between_tokens);
        for descriptor_tokens in between_fact.descriptor_segments {
            let extra_filter = parse_spell_filter_with_grammar_entrypoint(
                strip_relative_target_clause(descriptor_tokens),
            );
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        // The actor phrase is a separate fact — "with flying you cast" must
        // parse the quality without it, or the filter silently drops it.
        let mut between_for_filter: Vec<OwnedLexToken> =
            strip_relative_target_clause(between_tokens).to_vec();
        for actor_phrase in [
            &["you", "cast"][..],
            &["your", "opponents", "cast"],
            &["an", "opponent", "casts"],
            &["enchanted", "player", "casts"],
        ] {
            let view = TokenWordView::new(&between_for_filter);
            let actor_words = view.to_word_refs();
            if let Some(word_pos) =
                crate::word_primitives::parse_sequence_start(&actor_words, actor_phrase)
                && let (Some(token_start), Some(token_end)) = (
                    view.map_word_to_token_start(word_pos),
                    view.token_index_after_words(word_pos + actor_phrase.len()),
                )
            {
                between_for_filter.drain(token_start..token_end);
                break;
            }
        }
        let between_filter = parse_spell_filter_with_grammar_entrypoint(&between_for_filter);
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        match between_fact.actor {
            Some(static_mid_facts::SpellCastActorFact::You) => {
                filter.cast_by = Some(PlayerFilter::You)
            }
            Some(static_mid_facts::SpellCastActorFact::Opponent) => {
                filter.cast_by = Some(PlayerFilter::Opponent)
            }
            // "Spells with the chosen name enchanted player casts cost {2}
            // more to cast" (Curse of Silence): the Aura's enchanted player.
            None if cost_words_contain_phrase(
                &between_words,
                &["enchanted", "player", "casts"],
            ) =>
            {
                filter.cast_by = Some(PlayerFilter::TaggedPlayer("enchanted".into()));
            }
            None => {}
        }
        if between_fact.from_your_graveyard {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
            // "Spells you cast from your graveyard or from exile" (Doc
            // Aurlock): a second cast origin is a disjunction, and only the
            // graveyard branch carries the "your" ownership.
            if filter.any_of.is_empty()
                && (cost_words_contain_phrase(
                    &between_words,
                    &["from", "your", "graveyard", "or", "from", "exile"],
                ) || cost_words_contain_phrase(
                    &between_words,
                    &["from", "your", "graveyard", "or", "exile"],
                ))
            {
                filter.zone = None;
                filter.owner = None;
                let mut graveyard = ObjectFilter::spell().in_zone(Zone::Graveyard);
                graveyard.owner = Some(PlayerFilter::You);
                filter.any_of = vec![graveyard, ObjectFilter::spell().in_zone(Zone::Exile)];
            }
        }
        // "Spells with the chosen name you cast cost {2} less to cast."
        // (Council of the Absolute) — the name constraint resolves against
        // the as-enters choice at runtime. The "this turn" form is a
        // one-shot effect (Cheering Fanatic) owned by the effect-shape
        // parser, not this static line.
        if (cost_words_contain_phrase(&between_words, &["with", "the", "chosen", "name"])
            || cost_words_contain_phrase(&between_words, &["with", "chosen", "name"]))
            && !cost_words_contain_phrase(&clause_words, &["this", "turn"])
        {
            filter.name = Some("{chosen name}".to_string());
        }
        if cost_words_contain_phrase(
            &between_words,
            &["from", "anywhere", "other", "than", "your", "hand"],
        ) {
            filter.zone = None;
            filter.owner = None;
            apply_anywhere_other_than_hand_origin(&mut filter);
        }
        if cost_words_contain_phrase(&between_words, &["but", "don't", "own"])
            || cost_words_contain_phrase(&between_words, &["but", "dont", "own"])
        {
            filter.owner = Some(PlayerFilter::NotYou);
        }
        if let Some(target_tokens) = between_fact.target_tokens {
            let (target_player, target_object, targets_any_of) =
                parse_cost_modifier_target_spec(target_tokens)?;
            filter.targets_player = target_player;
            filter.targets_object = target_object;
            filter.targets_any_of = targets_any_of;
        }
    }

    let mut amount_start = cost_token_idx + 1;
    // "cost an additional 3 life to cast": the surcharge direction is stated
    // ahead of the amount instead of as a trailing "more".
    let additional_surcharge = matches!(
        tokens.get(amount_start..amount_start + 2),
        Some([first, second]) if first.is_word("an") && second.is_word("additional")
    );
    if additional_surcharge {
        amount_start += 2;
    }
    let amount_tokens = &tokens[amount_start..];
    let (parsed_amount, mut parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let mut parsed_mana_cost_repetitions = None;
    let (mut amount_value, used) = parsed_amount.clone().unwrap_or({
        if let Some((_, used)) = &parsed_mana_cost {
            (Value::Fixed(1), *used)
        } else {
            (Value::Fixed(1), 0)
        }
    });
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::lexer::parser_token_word_refs(remaining_tokens);
    let if_boundary =
        static_keyword_cost_shapes::parse_cost_direction_if_boundary(&remaining_words)
            .map(|boundary| boundary.word);
    let as_long_as_boundary =
        crate::word_primitives::parse_sequence_start(&remaining_words, &["as", "long", "as"]);
    let condition_boundary = match (if_boundary, as_long_as_boundary) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(boundary), None) | (None, Some(boundary)) => Some(boundary),
        (None, None) => None,
    };
    let direction_words = condition_boundary
        .map(|boundary| &remaining_words[..boundary])
        .unwrap_or(&remaining_words);
    let direction = match static_mid_facts::parse_cost_modifier_direction_words(direction_words) {
        Some(direction) => direction,
        None if additional_surcharge
            && crate::word_primitives::sequence_occurs(direction_words, &["to", "cast"]) =>
        {
            CostModifierDirection::More
        }
        None => return Ok(None),
    };
    let is_life_cost_modifier =
        crate::word_primitives::sequence_occurs(&remaining_words, &["life"]);
    if is_life_cost_modifier && direction == CostModifierDirection::More {
        // "cost an additional 3 life to cast" (Terror of the Peaks): a life
        // surcharge on matching spells, never generic mana.
        let Value::Fixed(life) = amount_value else {
            return Err(CardTextError::ParseError(format!(
                "unsupported life cost increase for spells (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if is_this_spell {
            return Err(CardTextError::ParseError(format!(
                "unsupported life cost increase for spells (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
        let mut display = crate::lexer::render_token_slice(tokens).trim().to_string();
        if !crate::string_primitives::ends_with_char(&display, '.') {
            display.push('.');
        }
        return Ok(Some(StaticAbility::cost_increase_life(
            filter,
            u32::try_from(life).unwrap_or(0),
            display,
        )));
    }
    let per_matching_target = if is_life_cost_modifier || is_this_spell {
        None
    } else {
        parse_per_matching_target_cost_modifier(remaining_tokens)?
    };
    let per_target = !is_life_cost_modifier
        && (is_exact_per_target_cost_modifier(&remaining_words) || per_matching_target.is_some());
    if let Some(target_filter) = per_matching_target {
        // The spell must target a matching object for the modifier to apply;
        // the engine counts the chosen targets that match it.
        filter.targets_object = Some(Box::new(target_filter));
    }
    let per_additional_target = cost_words_contain_phrase(
        &remaining_words,
        &["for", "each", "target", "beyond", "the", "first"],
    );
    let characteristic_intersection = if direction == CostModifierDirection::Less {
        parse_cost_reduction_characteristic_intersection(remaining_tokens)?
    } else {
        None
    };

    let compound_this_spell_reduction = if direction == CostModifierDirection::Less
        && is_this_spell
        && condition_boundary.is_none()
        && !per_target
        && characteristic_intersection.is_none()
    {
        parsed_amount
            .as_ref()
            .and_then(|(value, _)| match value {
                Value::Fixed(multiplier) => Some(*multiplier),
                _ => None,
            })
            .map(|multiplier| {
                parse_compound_this_spell_cost_reduction_value(remaining_tokens, multiplier)
            })
            .transpose()?
            .flatten()
    } else {
        None
    };

    // Whether the words after "less/more to cast" were read as part of the
    // amount; otherwise they must be a condition (see below).
    let mut tail_read_by_amount = per_target
        || per_additional_target
        || characteristic_intersection.is_some()
        || static_mid_facts::parse_where_x_clause_tokens(remaining_tokens).is_some()
        || parse_cost_reduction_cap(remaining_tokens).is_some();
    if let Some(compound) = compound_this_spell_reduction {
        tail_read_by_amount = true;
        parsed_mana_cost = None;
        amount_value = compound;
    } else if !per_target
        && characteristic_intersection.is_none()
        && !crate::word_primitives::sequence_occurs(&remaining_words, &["as", "long", "as"])
        // The amount's "for each ..." must come before any trailing condition:
        // "if an opponent has drawn four or more cards this turn" (Even the
        // Score) is a condition, not a per-card repetition.
        && condition_boundary.is_none_or(|boundary| {
            crate::word_primitives::parse_sequence_start(&remaining_words, &["for", "each"])
                .is_some_and(|each| each < boundary)
        })
        && let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)?
    {
        tail_read_by_amount = true;
        // A colored per-count reduction ("costs {U} less to cast for each
        // land ...", Eluge) keeps its mana symbols and repeats them; only a
        // purely generic one folds into a scaled generic amount.
        let colored_mana_cost = parsed_mana_cost.as_ref().is_some_and(|(cost, _)| {
            cost.pips().iter().flatten().any(|symbol| {
                !matches!(symbol, crate::mana::ManaSymbol::Generic(_))
            })
        });
        if parsed_mana_cost.is_some()
            && (is_this_spell || (colored_mana_cost && direction == CostModifierDirection::Less))
        {
            parsed_mana_cost_repetitions = Some(dynamic_value);
        } else {
            if parsed_mana_cost.is_some() {
                parsed_mana_cost = None;
            }
            let multiplier = parsed_amount
                .as_ref()
                .and_then(|(value, _)| match value {
                    Value::Fixed(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(1);
            amount_value = scale_dynamic_cost_modifier_value(dynamic_value, multiplier);
        }
    } else if parsed_amount.is_none() && parsed_mana_cost.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    // Handle trailing "where X is ..." clauses, e.g.
    // "This spell costs {X} less to cast, where X is the number of differently named lands you control."
    if let Some(where_tokens) = static_mid_facts::parse_where_x_clause_tokens(remaining_tokens) {
        let clause = clause_words.join(" ");
        let x_value = parse_value_binding_clause(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in spells-cost modifier (clause: '{clause}')"
            ))
        })?;
        if !value_contains_unbound_x(&amount_value) {
            return Err(CardTextError::ParseError(format!(
                "missing where-x clause in spells-cost modifier (clause: '{clause}')"
            )));
        }
        amount_value = replace_unbound_x_with_value(amount_value, &x_value, &clause)?;
    }
    if direction == CostModifierDirection::Less
        && let Some(cap) = parse_cost_reduction_cap(remaining_tokens)
    {
        amount_value = Value::Min(Box::new(amount_value), Box::new(Value::Fixed(cap)));
    }

    if !is_this_spell {
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
        parse_trailing_candidate_ability_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
        let except_during_controller_turn = [
            &["except", "during", "its", "controller's", "turn"][..],
            &["except", "during", "its", "controllers", "turn"][..],
            &["except", "during", "its", "controller", "s", "turn"][..],
        ]
        .iter()
        .any(|phrase| cost_words_contain_phrase(&remaining_words, phrase));
        if except_during_controller_turn {
            let caster = filter.cast_by.take().unwrap_or(PlayerFilter::Any);
            filter.cast_by = Some(PlayerFilter::excluding(caster, PlayerFilter::Active));
            filter.set_except_during_controller_turn_surface(true);
        }
    }

    let this_spell_condition = if is_this_spell {
        if let Some(condition) =
            parse_trailing_this_spell_cost_condition(remaining_tokens, &clause_words)?
        {
            condition
        } else if let Some(prefix) = &prefix_condition {
            match prefix {
                PredicateAst::YourTurn => crate::static_abilities::ThisSpellCostCondition::YourTurn,
                PredicateAst::Not(inner) if matches!(inner.as_ref(), PredicateAst::YourTurn) => {
                    crate::static_abilities::ThisSpellCostCondition::NotYourTurn
                }
                other => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported leading this-spell cost condition (clause: '{}'; condition: {other:?})",
                        clause_words.join(" ")
                    )));
                }
            }
        } else {
            // CR 601.2f: an unconditional self reduction reads exactly
            // "costs {N} less/more to cast". Any other trailing words are an
            // unrecognized condition, which must not be dropped into an
            // always-on reduction.
            let direction_idx = remaining_words
                .iter()
                .position(|word| matches!(*word, "less" | "more"));
            let trailing = direction_idx
                .map(|idx| &remaining_words[idx + 1..])
                .unwrap_or_default();
            if !tail_read_by_amount && !trailing.is_empty() && trailing != ["to", "cast"] {
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-spell cost condition (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            crate::static_abilities::ThisSpellCostCondition::Always
        }
    } else {
        crate::static_abilities::ThisSpellCostCondition::Always
    };

    let mut non_this_condition = if is_this_spell {
        None
    } else {
        prefix_condition.clone()
    };
    if !is_this_spell && let Some(boundary) = as_long_as_boundary {
        let condition_start =
            static_keyword_shapes::parse_word_token_offset(remaining_tokens, boundary + 3)
                .ok_or_else(|| {
                    CardTextError::ParseError("missing trailing cost condition".to_string())
                })?;
        let condition_tokens = trim_commas(&remaining_tokens[condition_start..]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        non_this_condition = Some(match non_this_condition.take() {
            Some(prefix) => PredicateAst::And(Box::new(prefix), Box::new(condition)),
            None => condition,
        });
    }
    // "Spells your opponents cast during your turn cost {1} more to cast."
    // (Paladin Class): the turn restriction gates the modifier.
    if !is_this_spell
        && first_spell_fact.is_none()
        && crate::word_primitives::sequence_occurs(
            &crate::lexer::parser_token_word_refs(&tokens[spells_token_idx + 1..cost_token_idx]),
            &["cast", "during", "your", "turn"],
        )
    {
        non_this_condition = Some(match non_this_condition.take() {
            Some(existing) => {
                PredicateAst::And(Box::new(existing), Box::new(PredicateAst::YourTurn))
            }
            None => PredicateAst::YourTurn,
        });
    }
    if first_spell_fact.is_some_and(|fact| fact.during_each_of_your_turns) && !is_this_spell {
        non_this_condition = Some(match non_this_condition.take() {
            Some(existing) => {
                PredicateAst::And(Box::new(existing), Box::new(PredicateAst::YourTurn))
            }
            None => PredicateAst::YourTurn,
        });
    }

    if direction == CostModifierDirection::Less {
        // "This spell costs {N} less to cast" is a self-only modifier that should not
        // apply from the permanent on the battlefield after it resolves.
        if is_this_spell && parsed_mana_cost.is_none() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount_value,
                    this_spell_condition,
                ),
            )));
        }
        if is_this_spell && let Some((cost, _)) = parsed_mana_cost.clone() {
            let mut ability = crate::static_abilities::ThisSpellCostReductionManaCost::new(
                cost,
                this_spell_condition,
            )
            .with_colored_only(cost_reduction_is_colored_only(tokens));
            if let Some(repetitions) = parsed_mana_cost_repetitions {
                ability = ability.with_repetitions(repetitions);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        if let Some((cost, _)) = parsed_mana_cost {
            let mut ability = crate::static_abilities::CostReductionManaCost::new(filter, cost)
                .with_colored_only(cost_reduction_is_colored_only(tokens));
            if let Some(repetitions) = parsed_mana_cost_repetitions {
                ability = ability.with_repetitions(repetitions);
            }
            if per_target {
                ability = ability.with_per_target();
            }
            if let Some(condition) = non_this_condition.clone() {
                ability = ability.with_condition(condition);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        let mut ability = crate::static_abilities::CostReduction::new(filter, amount_value);
        if let Some(intersection) = characteristic_intersection {
            ability = ability.with_characteristic_intersection(intersection);
        }
        if per_target {
            ability = ability.with_per_target();
        }
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let source_only_increase = is_this_spell && !is_life_cost_modifier && !per_additional_target;
    let mut source_only_condition = None;
    if source_only_increase {
        filter = ObjectFilter::source();
        source_only_condition = apply_this_spell_cost_increase_condition(
            &mut filter,
            this_spell_condition,
            &clause_words,
        )?;
    }

    if let Some((cost, _)) = parsed_mana_cost {
        let mut ability = crate::static_abilities::CostIncreaseManaCost::new(filter, cost);
        if per_target {
            ability = ability.with_per_target();
        }
        if let Some(condition) = source_only_condition
            .clone()
            .map(|bound| PredicateAst::Bound(Box::new(bound)))
            .or_else(|| non_this_condition.clone())
        {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let mut ability = crate::static_abilities::CostIncrease::new(filter, amount_value);
    if per_target {
        ability = ability.with_per_target();
    }
    if let Some(condition) = source_only_condition
        .map(|bound| PredicateAst::Bound(Box::new(bound)))
        .or_else(|| non_this_condition.clone())
    {
        ability = ability.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(ability)))
}

/// Preserve two executable clauses that share one authored spell subject:
/// `Spells ... cost ... less to cast and can't be countered.`
///
/// The ordinary cost-modifier parser intentionally ignores trailing prose
/// after a complete `less to cast` direction. Claim only the exact terminal
/// countering conjunction here and reuse the typed spell filter from the cost
/// reduction for the restriction.
pub fn parse_spells_cost_reduction_and_cant_be_countered_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_index) =
        crate::slice_primitives::select_last_position(tokens, |token| token.is_word("and"))
    else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(&tokens[and_index + 1..]),
        &["cant", "be", "countered"],
    ) {
        return Ok(None);
    }
    let left = trim_lexed_commas(&tokens[..and_index]);
    let Some(reduction) = parse_spells_cost_modifier_line(left)? else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::CostReduction(payload) = &reduction.payload else {
        return Ok(None);
    };
    let filter = payload.filter.clone();
    let restriction = StaticAbility::restriction(
        crate::effect::Restriction::be_countered(filter.clone()),
        format!("{} can't be countered", filter.description()),
    );
    Ok(Some(vec![reduction, restriction]))
}

/// "This spell costs {1} less to cast if you control an artifact and {1} less
/// to cast if you control an enchantment." (Assassin's Ink): two independent
/// conditional self reductions sharing one subject (CR 601.2f). Each half is
/// read by the ordinary cost-modifier parser as its own this-spell reduction.
pub fn parse_double_conditional_this_spell_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_prefix(&words, &["this", "spell", "costs"]) {
        return Ok(None);
    }
    let Some(costs_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("costs"))
    else {
        return Ok(None);
    };
    let split = (costs_idx + 1..tokens.len()).find(|&idx| {
        tokens[idx].is_word("and")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.mana_group_inner().is_some())
            && crate::word_primitives::sequence_occurs(
                &crate::lexer::parser_token_word_refs(&tokens[idx + 1..]),
                &["to", "cast", "if"],
            )
    });
    // "This spell costs {1} less to cast if you control a Spirit. It also
    // costs {1} less to cast if you control an enchantment.": the second
    // sentence is a second, independent reduction of the same spell.
    let also_sentence = (costs_idx + 1..tokens.len()).find(|&idx| {
        tokens[idx].kind == TokenKind::Period
            && tokens.get(idx + 1).is_some_and(|token| token.is_word("it"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("also"))
            && tokens
                .get(idx + 3)
                .is_some_and(|token| token.is_word("costs"))
    });
    let (left, right) = if let Some(period_idx) = also_sentence {
        let left = trim_commas(&tokens[..period_idx]);
        let mut right = tokens[..=costs_idx].to_vec();
        right.extend_from_slice(&tokens[period_idx + 4..]);
        (left, right)
    } else {
        let Some(and_idx) = split else {
            return Ok(None);
        };
        let left = trim_commas(&tokens[..and_idx]);
        let mut right = tokens[..=costs_idx].to_vec();
        right.extend_from_slice(&tokens[and_idx + 1..]);
        (left, right)
    };
    let (Some(first), Some(second)) = (
        parse_spells_cost_modifier_line(&left)?,
        parse_spells_cost_modifier_line(&right)?,
    ) else {
        return Ok(None);
    };
    let is_conditional_self_reduction = |ability: &StaticAbility| {
        matches!(
            &ability.payload,
            ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction)
                if reduction.condition != crate::static_abilities::ThisSpellCostCondition::Always
        ) || matches!(
            &ability.payload,
            ironsmith_core::StaticAbilityPayload::ThisSpellCostReductionManaCost(reduction)
                if reduction.condition != crate::static_abilities::ThisSpellCostCondition::Always
        )
    };
    if !is_conditional_self_reduction(&first) || !is_conditional_self_reduction(&second) {
        return Err(CardTextError::ParseError(format!(
            "unsupported compound this-spell cost reduction (clause: '{}')",
            words.join(" ")
        )));
    }
    Ok(Some(vec![first, second]))
}

pub fn parse_spell_and_player_activated_ability_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_idx) = static_keyword_cost_shapes::parse_spell_and_abilities_separator(tokens)
        .map(|boundary| boundary.token)
    else {
        // A standalone "Abilities <players> activate [that target ...] cost
        // {N} more to activate" line (Kopala, Warden of Waves).
        return Ok(parse_player_activated_ability_cost_modifier_clause(
            &trim_commas(crate::util::trim_edge_punctuation_tokens(tokens)),
        )?
        .map(|ability| vec![ability]));
    };
    let right_start = and_idx + 1;

    let left_tokens = trim_commas(&tokens[..and_idx]);
    let right_tokens = trim_commas(&tokens[right_start..]);
    let Some(spell_cost_ability) = parse_spells_cost_modifier_line(&left_tokens)? else {
        return Ok(None);
    };
    let Some(mut activated_cost_ability) =
        parse_player_activated_ability_cost_modifier_clause(&right_tokens)?
    else {
        return Ok(None);
    };

    if let Some(spells_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    {
        let (prefix_condition, _) = parse_cost_modifier_prefix_condition(tokens, spells_idx)?;
        if let Some(condition) = prefix_condition {
            activated_cost_ability = activated_cost_ability.with_condition(condition);
        }
    }

    Ok(Some(vec![spell_cost_ability, activated_cost_ability]))
}

pub fn parse_cycling_cost_alternative_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(fact) = static_mid_facts::parse_cycling_cost_alternative_fact(tokens) else {
        return Ok(None);
    };

    let condition = fact
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let replacement_mana_cost = if fact.replacement_cost_tokens.is_empty() {
        ManaCost::new()
    } else {
        let replacement_total_cost = parse_compiler_activation_cost(fact.replacement_cost_tokens)?;
        if replacement_total_cost.has_non_mana_costs() {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-mana cycling alternative cost (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        replacement_total_cost.mana_cost().cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing cycling alternative mana cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let mut filter = ObjectFilter::default().with_ability_marker("cycling");
    filter.zone = Some(Zone::Hand);
    let display = format!(
        "You may pay {} rather than pay cycling costs",
        replacement_mana_cost.to_oracle()
    );
    let mut ability =
        StaticAbility::replace_activated_ability_mana_cost(filter, replacement_mana_cost, display);
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

pub fn parse_player_activated_ability_cost_modifier_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if clause_words.len() < 7 || clause_words.first().is_none_or(|word| *word != "abilities") {
        return Ok(None);
    }

    let Some(cost_words) =
        static_keyword_cost_shapes::parse_player_ability_cost_words(&clause_words)
    else {
        return Ok(None);
    };
    let activate_idx = cost_words.activate.word;
    let clause = LexedClause::new(tokens);
    let Some(activator_clause) = clause.between_word_range(1, activate_idx) else {
        return Ok(None);
    };
    let activator =
        match static_mid_facts::parse_activated_ability_cost_actor(activator_clause.tokens()) {
            Some(static_mid_facts::ActivatedAbilityCostActorFact::You) => PlayerFilter::You,
            Some(static_mid_facts::ActivatedAbilityCostActorFact::Opponent) => {
                PlayerFilter::Opponent
            }
            None => return Ok(None),
        };

    let cost_idx = cost_words.costs.word;
    let cost_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, cost_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map activated-ability cost modifier amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    // "Abilities your opponents activate that target a Merfolk you control
    // cost {2} more to activate." (Kopala): a relative target clause between
    // the activation and the cost gates which activations are taxed.
    let mut target_filter = None;
    let activate_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, activate_idx);
    if let Some(activate_token_idx) = activate_token_idx
        && activate_token_idx + 1 < cost_token_idx
    {
        let relative = &tokens[activate_token_idx + 1..cost_token_idx];
        if relative.len() > 2
            && relative[0].is_word("that")
            && (relative[1].is_word("target") || relative[1].is_word("targets"))
        {
            target_filter = Some(parse_object_filter(&relative[2..], false)?);
        } else {
            return Ok(None);
        }
    }

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (increase, used) = if let Some((mana_cost, used)) = parsed_mana_cost {
        (
            ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(mana_cost),
            used,
        )
    } else if let Some((Value::Fixed(amount), used)) = parsed_amount {
        if amount < 0 {
            return Ok(None);
        }
        let generic = amount.min(u8::MAX as i32) as u8;
        (
            ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(ManaCost::from_symbols(
                vec![ManaSymbol::Generic(generic)],
            )),
            used,
        )
    } else {
        return Ok(None);
    };
    let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
    let remaining_words = crate::lexer::token_word_refs(remaining_tokens);
    let Some(tail_fact) = static_mid_facts::parse_activated_ability_cost_tail(remaining_tokens)
    else {
        return Ok(None);
    };
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::More)
    {
        return Ok(None);
    }

    let mut ability = StaticAbility::increase_activated_ability_costs_for_activator(
        activator,
        increase,
        tail_fact.excludes_mana_abilities,
    );
    if let Some(filter) = target_filter
        && let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease {
            ability_condition,
            ..
        } = &mut ability.payload
    {
        *ability_condition = Some(
            crate::static_abilities::ActivatedAbilityCostCondition::TargetsExactly {
                count: 1,
                filter,
            },
        );
    }
    Ok(Some(ability))
}

pub fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = static_keyword_cost_shapes::parse_relative_target_clause(tokens)
        .map(|boundary| boundary.token)
    else {
        return tokens;
    };

    &tokens[..target_clause_idx]
}

pub fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let Some(fact) = static_mid_facts::parse_trailing_target_condition(remaining_tokens) else {
        return Ok(());
    };
    if fact.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (targets_player, targets_object, targets_any_of) =
        parse_cost_modifier_target_spec(fact.target_tokens)?;
    filter.targets_player = targets_player;
    filter.targets_object = targets_object;
    filter.targets_any_of = targets_any_of;
    Ok(())
}

fn parse_trailing_candidate_ability_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let remaining_words = crate::lexer::token_word_refs(remaining_tokens);
    let Some(if_idx) =
        static_keyword_cost_shapes::parse_trailing_cost_condition_if(&remaining_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(());
    };
    let Some(keyword_words) = remaining_words.get(if_idx + 3..) else {
        return Ok(());
    };
    if !crate::word_primitives::parse_sequence_prefix(
        &remaining_words[if_idx + 1..],
        &["it", "has"],
    ) {
        return Ok(());
    }
    let Some((constraints, _, consumed)) =
        crate::util::parse_filter_keyword_constraint_list_words(keyword_words)
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported candidate-spell ability condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if constraints.is_empty() || consumed != keyword_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported candidate-spell ability condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    for constraint in constraints {
        crate::util::apply_filter_keyword_constraint(filter, constraint, false);
    }
    filter.set_trailing_candidate_ability_condition_surface(true);
    Ok(())
}

pub fn parse_flashback_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::parser_token_word_refs(tokens);
    let Some((kind, consumed)) = parse_alternative_cast_words(&clause_words) else {
        return Ok(None);
    };
    if clause_words.len() < consumed + 5 {
        return Ok(None);
    }
    if clause_words.get(consumed).copied() != Some("costs") {
        return Ok(None);
    }
    let Some(cost_idx) =
        static_keyword_cost_shapes::parse_last_cost_verb(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::lexer::parser_token_word_refs(remaining_tokens);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
    else {
        return Ok(None);
    };
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }
    // Anything after "less"/"more" must be owned, never dropped: "{1} less for
    // each time you've cast your commander from the command zone this game"
    // (Henzie) scales the amount.
    let direction_idx = remaining_words
        .iter()
        .position(|word| matches!(*word, "less" | "more"))
        .unwrap_or(0);
    let direction_tail = &remaining_words[direction_idx + 1..];
    let amount_value = if direction_tail.is_empty() {
        amount_value
    } else {
        let Some((per_each, used)) = parse_for_each_count_value_words(direction_tail) else {
            return Ok(None);
        };
        if used != direction_tail.len() {
            return Ok(None);
        }
        match amount_value {
            Value::Fixed(1) => per_each,
            Value::Fixed(amount) => Value::Scaled(Box::new(per_each), amount),
            _ => return Ok(None),
        }
    };

    let mut filter = ObjectFilter::default();
    filter.alternative_cast = Some(kind);
    match static_mid_facts::parse_alternative_cost_payer(tokens) {
        Some(static_mid_facts::AlternativeCostPayerFact::You) => {
            filter.cast_by = Some(PlayerFilter::You)
        }
        Some(static_mid_facts::AlternativeCostPayerFact::Opponent) => {
            filter.cast_by = Some(PlayerFilter::Opponent)
        }
        None => {}
    }

    if direction == CostModifierDirection::Less {
        return Ok(Some(StaticAbility::new(
            crate::model::CompilerCostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::model::CompilerCostIncrease::new(filter, amount_value),
    )))
}

pub fn parse_equip_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(head) = keyword_static_lines::parse_equip_cost_modifier_head_tokens(tokens) else {
        return Ok(None);
    };
    let cost_idx = head.cost_token;

    let amount_tokens = &tokens[cost_idx + 1..];
    let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Ok(None);
    };
    let Value::Fixed(amount) = amount_value else {
        return Ok(None);
    };
    if amount < 0 {
        return Ok(None);
    }

    let remaining_words = crate::lexer::token_word_refs(&amount_tokens[used..]);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
    else {
        return Ok(None);
    };

    use keyword_static_lines::EquipCostTarget;
    // The ability being activated must be an equip ability, and for "that
    // target ..." one with such a target (CR 601.2f, 602.2b). Both are facts
    // about the activation, not about the Equipment, which may have other
    // activated abilities.
    let targeting = match head.target {
        EquipCostTarget::Any => None,
        EquipCostTarget::Source => Some(ObjectFilter::source()),
        EquipCostTarget::EnchantedCreature | EquipCostTarget::EquippedCreature => {
            let mut filter = ObjectFilter::creature();
            filter.with_attached_object = Some(Box::new(ObjectFilter::source()));
            Some(filter)
        }
        EquipCostTarget::Unsupported => {
            return Err(CardTextError::ParseError(format!(
                "unsupported equip cost modifier target (clause: '{}')",
                crate::lexer::token_word_refs(tokens).join(" ")
            )));
        }
    };
    let mut filter = if head.source_relative_equipment {
        ObjectFilter::source()
    } else {
        ObjectFilter::default()
    };
    if head.other_equipment {
        filter.subtypes.push(crate::types::Subtype::Equipment);
        filter.other = true;
    }
    match head.payer {
        keyword_static_lines::EquipCostPayer::You => filter.controller = Some(PlayerFilter::You),
        keyword_static_lines::EquipCostPayer::Opponent => {
            filter.controller = Some(PlayerFilter::Opponent)
        }
        keyword_static_lines::EquipCostPayer::Unspecified => {}
    }

    if direction == CostModifierDirection::Less {
        let amount_text = format!("{{{amount}}}");
        let display = if head.source_relative_equipment {
            format!("This Equipment's equip abilities cost {amount_text} less to activate")
        } else if head.target == EquipCostTarget::Source {
            format!(
                "Equip abilities you activate that target this creature cost {amount_text} less to activate"
            )
        } else if head.target == EquipCostTarget::EnchantedCreature {
            format!(
                "Equip abilities you activate that target enchanted creature cost {amount_text} less to activate"
            )
        } else if head.target == EquipCostTarget::EquippedCreature {
            format!(
                "Equip abilities you activate that target equipped creature cost {amount_text} less to activate"
            )
        } else if head.other_equipment {
            format!(
                "Equip abilities you activate of other Equipment cost {amount_text} less to activate"
            )
        } else if head.equip_abilities_head && filter.controller == Some(PlayerFilter::You) {
            format!("Equip abilities you activate cost {amount_text} less to activate")
        } else if filter.controller == Some(PlayerFilter::Opponent) {
            format!("Equip costs your opponents pay cost {amount_text} less")
        } else {
            format!("Equip costs you pay cost {amount_text} less")
        };
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                filter,
                amount as u32,
                None,
                display,
            )
            .with_activated_ability_cost_condition(
                crate::static_abilities::ActivatedAbilityCostCondition::EquipAbility { targeting },
            ),
        ));
    }
    let mut filter = filter.with_ability_marker("equip");
    if let Some(targeting) = targeting {
        filter.targets_object = Some(Box::new(targeting));
    }

    let increase = ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(
        ManaCost::from_symbols(vec![ManaSymbol::Generic(amount.min(u8::MAX as i32) as u8)]),
    );
    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, increase,
    )))
}

/// "Plotting cards from your hand costs {2} less." (Doc Aurlock, Grizzled
/// Genius): a generic reduction of the plot special action (CR 702.170a).
pub fn parse_plotting_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if !words.starts_with(&["plotting", "cards", "from", "your", "hand", "costs"]) {
        return Ok(None);
    }
    let Some(costs_index) = tokens.iter().position(|token| token.is_word("costs")) else {
        return Ok(None);
    };
    let Some((Value::Fixed(amount), consumed)) =
        parse_cost_modifier_amount(&tokens[costs_index + 1..])
    else {
        return Err(CardTextError::ParseError(
            "plot special-action reduction needs a fixed generic cost".into(),
        ));
    };
    let mut tail = &tokens[costs_index + 1 + consumed..];
    if tail.last().is_some_and(|token| token.is_period()) {
        tail = &tail[..tail.len() - 1];
    }
    if amount < 0 || crate::lexer::token_word_refs(tail).as_slice() != ["less"] {
        return Err(CardTextError::ParseError(
            "unsupported plot special-action modifier syntax".into(),
        ));
    }
    Ok(Some(StaticAbility::plot_special_action_modifier(amount as u32)))
}

/// "All morph costs cost {2} more." (Exiled Doomsayer): a generic increase
/// of every player's turn-face-up special action paid with a morph or
/// megamorph cost (CR 116.2b, 702.37). The reminder "(This doesn't affect
/// the cost to cast creature spells face down.)" is stripped upstream.
pub fn parse_morph_costs_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if !words.starts_with(&["all", "morph", "costs", "cost"]) {
        return Ok(None);
    }
    let Some(cost_index) = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_word("cost"))
        .map(|(index, _)| index)
        .next()
    else {
        return Ok(None);
    };
    let Some((Value::Fixed(amount), consumed)) =
        parse_cost_modifier_amount(&tokens[cost_index + 1..])
    else {
        return Err(CardTextError::ParseError(
            "morph cost increase needs a fixed generic cost".into(),
        ));
    };
    let mut tail = &tokens[cost_index + 1 + consumed..];
    if tail.last().is_some_and(|token| token.is_period()) {
        tail = &tail[..tail.len() - 1];
    }
    if amount <= 0 || crate::lexer::token_word_refs(tail).as_slice() != ["more"] {
        return Err(CardTextError::ParseError(
            "unsupported morph cost modifier syntax".into(),
        ));
    }
    Ok(Some(StaticAbility::morph_cost_increase(amount as u32)))
}

/// "Unlock costs you pay cost {1} less." (Inquisitive Glimmer): the
/// controller's door-unlocking special action costs that much less generic
/// mana (CR 709.5c).
pub fn parse_unlock_costs_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if !words.starts_with(&["unlock", "costs", "you", "pay", "cost"]) {
        return Ok(None);
    }
    let Some(cost_index) = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.is_word("cost"))
        .map(|(index, _)| index)
        .next()
    else {
        return Ok(None);
    };
    let Some((Value::Fixed(amount), consumed)) =
        parse_cost_modifier_amount(&tokens[cost_index + 1..])
    else {
        return Err(CardTextError::ParseError(
            "unlock cost reduction needs a fixed generic cost".into(),
        ));
    };
    let mut tail = &tokens[cost_index + 1 + consumed..];
    if tail.last().is_some_and(|token| token.is_period()) {
        tail = &tail[..tail.len() - 1];
    }
    if amount <= 0 || crate::lexer::token_word_refs(tail).as_slice() != ["less"] {
        return Err(CardTextError::ParseError(
            "unsupported unlock cost modifier syntax".into(),
        ));
    }
    Ok(Some(StaticAbility::unlock_cost_reduction(amount as u32)))
}

/// "Once each turn, you may play a card from exile with a collection counter
/// on it if it was exiled by an ability you controlled, and you may spend
/// mana as though it were mana of any color to cast it." (Evelyn, the
/// Covetous): one source-wide play permission over exiled cards carrying the
/// counter whose exile link names a source you controlled
/// ([`ironsmith_core::tag::EXILED_BY_YOU_TAG`]), with a once-each-turn budget
/// and a permission-local any-color mana rider (CR 609.4b).
pub fn parse_once_each_turn_play_countered_exiled_card_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    const HEAD: &[&str] = &[
        "once", "each", "turn", "you", "may", "play", "a", "card", "from", "exile", "with", "a",
    ];
    const TAIL: &[&str] = &[
        "counter", "on", "it", "if", "it", "was", "exiled", "by", "an", "ability", "you",
        "controlled", "and", "you", "may", "spend", "mana", "as", "though", "it", "were", "mana",
        "of", "any", "color", "to", "cast", "it",
    ];
    let words = parser_token_word_refs(tokens);
    let Some(rest) = words.strip_prefix(HEAD) else {
        return Ok(None);
    };
    let Some(counter_at) = rest.iter().position(|word| *word == "counter") else {
        return Ok(None);
    };
    let (kind, tail) = rest.split_at(counter_at);
    if kind.is_empty() || tail != TAIL {
        return Ok(None);
    }
    let mut kind_words = kind.to_vec();
    kind_words.push("counter");
    let Some(counter_type) = crate::grammar::filters::parse_counter_type_words(&kind_words) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.with_counter = Some(crate::filter::CounterConstraint::Typed(counter_type));
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: ironsmith_core::tag::TagKey::from(ironsmith_core::tag::EXILED_BY_YOU_TAG).into(),
        relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        filter,
        Zone::Exile,
    )
    .with_beneficiary(PlayerFilter::You)
    .with_usage_limit(crate::grant::GrantUsageLimit::OnceEachTurn);
    spec.cast_mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::AnyColor;
    let rendered = crate::lexer::render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    Ok(Some(StaticAbility::grants(spec)))
}

pub fn parse_foretelling_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    let Some(fact) = static_mid_facts::parse_foretell_cost_modifier_fact(tokens) else {
        return Ok(None);
    };

    if fact.direction != CostModifierDirection::Less || !fact.during_any_players_turn {
        return Ok(None);
    }

    let Some(costs_index) = tokens.iter().position(|token| token.is_word("costs")) else {
        return Ok(None);
    };
    let Some((Value::Fixed(amount), consumed)) = parse_cost_modifier_amount(&tokens[costs_index + 1..]) else {
        return Err(CardTextError::ParseError("foretell special-action reduction needs a fixed generic cost".into()));
    };
    let prefix = &tokens[..=costs_index];
    let mut tail = &tokens[costs_index + 1 + consumed..];
    if tail.last().is_some_and(|token| token.is_period()) { tail = &tail[..tail.len() - 1]; }
    let tail_words = crate::lexer::parser_token_word_refs(tail);
    let exact_tail = matches!(tail_words.as_slice(),
        ["less", "and", "can", "be", "done", "on", "any", "players", "turn"]
        | ["less", "and", "can", "be", "done", "on", "any", "player", "s", "turn"]
        | ["less", "and", "can", "be", "done", "on", "any", "player", "turn"]);
    if amount < 0 || !prefix.iter().all(|token| token.kind == TokenKind::Word)
        || !tail.iter().all(|token| matches!(token.kind, TokenKind::Word | TokenKind::Apostrophe))
        || !exact_tail
    {
        return Err(CardTextError::ParseError("unsupported trailing or non-generic foretell special-action modifier syntax".into()));
    }
    Ok(Some(StaticAbility::foretell_special_action_modifier(amount as u32, true)))
}

/// "The first card you foretell each turn costs {0} to foretell." (Ranar the
/// Ever-Watchful): the foretell special action (CR 702.143a, normally {2})
/// is free for its controller's first foretell each turn.
pub fn parse_first_foretell_each_turn_free_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(costs_index) = tokens.iter().position(|token| token.is_word("costs")) else {
        return Ok(None);
    };
    let prefix_words = crate::lexer::parser_token_word_refs(&tokens[..costs_index]);
    if prefix_words.as_slice() != ["the", "first", "card", "you", "foretell", "each", "turn"] {
        return Ok(None);
    }
    let Some((Value::Fixed(0), consumed)) = parse_cost_modifier_amount(&tokens[costs_index + 1..])
    else {
        return Ok(None);
    };
    let mut tail = &tokens[costs_index + 1 + consumed..];
    if tail.last().is_some_and(|token| token.is_period()) {
        tail = &tail[..tail.len() - 1];
    }
    if crate::lexer::parser_token_word_refs(tail).as_slice() != ["to", "foretell"]
        || !tail.iter().all(|token| token.kind == TokenKind::Word)
    {
        return Ok(None);
    }
    Ok(Some(StaticAbility::foretell_first_each_turn_free()))
}

pub fn parse_cost_modifier_amount(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let first_token = tokens.first()?;
    let group = mana_pips_from_token(first_token)?;
    if group.len() != 1 {
        return None;
    }
    let symbol = group[0];
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    if symbol == ManaSymbol::X {
        return Some((Value::X, 1));
    }
    None
}

pub fn parse_cost_modifier_mana_cost(
    tokens: &[OwnedLexToken],
) -> Option<(crate::mana::ManaCost, usize)> {
    let parsed = parse_leaf_fixed_mana_cost_prefix_tokens(tokens)?;
    Some((parsed.cost, parsed.consumed))
}

pub fn parse_cost_modifier_components(
    amount_tokens: &[OwnedLexToken],
) -> (
    Option<(Value, usize)>,
    Option<(crate::mana::ManaCost, usize)>,
) {
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let parsed_mana_cost = parse_cost_modifier_mana_cost(amount_tokens);

    let amount_used = parsed_amount.as_ref().map(|(_, used)| *used).unwrap_or(0);
    let mana_used = parsed_mana_cost
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(0);

    // Prefer mana-symbol parsing when it consumes a longer contiguous mana sequence
    // (e.g. "{2}{U}{U}" should stay a single mana-cost reduction component).
    if mana_used > amount_used {
        return (None, parsed_mana_cost);
    }

    (parsed_amount, None)
}

fn parse_compound_this_spell_cost_reduction_value(
    tokens: &[OwnedLexToken],
    first_multiplier: i32,
) -> Result<Option<Value>, CardTextError> {
    let mut boundaries = Vec::new();
    for (and_index, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let Some((Value::Fixed(multiplier), amount_used)) =
            parse_cost_modifier_amount(&tokens[and_index + 1..])
        else {
            continue;
        };
        let second_start = and_index + 1 + amount_used;
        if !tokens
            .get(second_start)
            .is_some_and(|token| token.is_word("less"))
            || !tokens
                .get(second_start + 1)
                .is_some_and(|token| token.is_word("to"))
            || !tokens
                .get(second_start + 2)
                .is_some_and(|token| token.is_word("cast"))
        {
            continue;
        }
        boundaries.push((and_index, multiplier, second_start));
    }
    if boundaries.is_empty() {
        return Ok(None);
    }

    let first_end = boundaries[0].0;
    let Some(first_value) = parse_dynamic_cost_modifier_value(&tokens[..first_end])? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported first dynamic term in compound cost reduction (clause: '{}')",
            parser_token_word_refs(tokens).join(" ")
        )));
    };
    let mut value = scale_dynamic_cost_modifier_value(first_value, first_multiplier);

    for (index, (_, multiplier, segment_start)) in boundaries.iter().copied().enumerate() {
        let segment_end = boundaries
            .get(index + 1)
            .map(|boundary| boundary.0)
            .unwrap_or(tokens.len());
        let Some(term) = parse_dynamic_cost_modifier_value(&tokens[segment_start..segment_end])?
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing dynamic term in compound cost reduction (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        };
        value = Value::Add(
            Box::new(value),
            Box::new(scale_dynamic_cost_modifier_value(term, multiplier)),
        );
    }

    Ok(Some(value))
}

pub fn parse_cost_reduction_cap(tokens: &[OwnedLexToken]) -> Option<i32> {
    for idx in 2..tokens.len().saturating_sub(1) {
        if !tokens[idx - 2].is_word("by")
            || !tokens[idx - 1].is_word("more")
            || !tokens[idx].is_word("than")
        {
            continue;
        }
        let group = mana_pips_from_token(tokens.get(idx + 1)?)?;
        if group.len() != 1 {
            return None;
        }
        return match group[0] {
            ManaSymbol::Generic(amount) => Some(amount as i32),
            _ => None,
        };
    }
    None
}

/// Parse the object description counted by a "for each <description>" cost
/// modifier, refusing words the description parser cannot model.
///
/// CR 601.2f: the modifier counts exactly the objects matching the whole
/// description. The object-filter parser is tolerant and skips words it
/// doesn't understand ("sacrificed this turn", "it targets", "counter on"),
/// which silently widens the count to every object of the head noun. When
/// dropping a trailing part of the phrase (a postmodifier such as a history
/// or relative clause) yields the very same filter, that part contributed
/// nothing, so the phrase is rejected instead.
///
/// Returns `Ok(None)` when the phrase isn't an object description at all, so
/// callers keep their other readings, and an error when it is one that loses
/// words.
pub(crate) fn parse_complete_cost_count_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Ok(filter) = parse_object_filter(tokens, false) else {
        return Ok(None);
    };
    let is_filler = |token: &OwnedLexToken| {
        token.mana_group_inner().is_none()
            && token
                .as_word()
                .is_none_or(|_| token.is_any_word(&["a", "an", "the"]))
    };
    let unmodeled = |part: &[OwnedLexToken]| {
        Err(CardTextError::ParseError(format!(
            "unsupported qualifier '{}' in cost-modifier count (clause: '{}')",
            parser_token_word_refs(part).join(" "),
            parser_token_word_refs(tokens).join(" ")
        )))
    };
    for split in 1..tokens.len() {
        let (head, tail) = tokens.split_at(split);
        if !tail.iter().all(is_filler)
            && parse_object_filter(crate::util::trim_edge_punctuation_tokens(head), false)
                .is_ok_and(|shorter| shorter == filter)
        {
            return unmodeled(tail);
        }
    }
    Ok(Some(filter))
}

pub fn parse_dynamic_cost_modifier_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    use keyword_static_lines::{
        CounterReferenceKind, DynamicCostValueShape, DynamicPlayerKind, DynamicThisWayMetric,
        SpellCastDynamicKind,
    };

    let for_each_value_tokens = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
        .and_then(|boundary| tokens.get(boundary.token.saturating_add(1)..));
    let history_tokens = for_each_value_tokens.unwrap_or(tokens);
    let with_for_each_surface = |value: Value| {
        if for_each_value_tokens.is_some() {
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
        } else {
            value
        }
    };
    // "for each {E} you've paid or lost this turn" (Blaster Hulk).
    if for_each_value_tokens.is_some() {
        let mut count_words = vec!["for", "each"];
        count_words.extend(crate::lexer::token_word_refs(history_tokens));
        if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
            && matches!(
                value.unhinted(),
                Value::TurnHistoryCount(
                    ironsmith_core::TurnHistoryCount::PlayerCountersRemoved { .. }
                )
            )
        {
            return Ok(Some(value));
        }
    }
    let parsed_shape = keyword_static_lines::parse_dynamic_cost_value_shape_tokens(tokens);
    // A card-types-among aggregate can contain a historical qualifier in its
    // object scope ("permanents you've sacrificed this turn"). Classify the
    // outer aggregate before the generic history parser sees that nested
    // phrase and incorrectly turns the whole value into a sacrifice count.
    match parsed_shape {
        Some(DynamicCostValueShape::CardTypesAmong { scope_tokens }) => {
            // "card type among permanents you've sacrificed this turn"
            // (Korvold, Gleeful Glutton) reads the sacrifice history, not the
            // permanents on the battlefield.
            if let Some(Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Sacrificed {
                player,
                filter,
            })) = crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(
                scope_tokens,
            )
            .map(|value| value.unhinted().clone())
            {
                return Ok(Some(with_for_each_surface(Value::TurnHistoryCount(
                    ironsmith_core::TurnHistoryCount::SacrificedCardTypes { player, filter },
                ))));
            }
            let Some(filter) = parse_complete_cost_count_filter(scope_tokens)? else {
                return Ok(None);
            };
            return Ok(Some(with_for_each_surface(Value::CardTypesAmong(filter))));
        }
        Some(DynamicCostValueShape::UnsupportedCardTypesAmong) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported card-types-among dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        _ => {}
    }
    if let Some(value) =
        crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(history_tokens)
    {
        return Ok(Some(with_for_each_surface(value)));
    }
    let Some(shape) = parsed_shape else {
        return Ok(None);
    };
    let player_filter = |player| match player {
        DynamicPlayerKind::You => PlayerFilter::You,
        DynamicPlayerKind::Opponent => PlayerFilter::Opponent,
        DynamicPlayerKind::Any => PlayerFilter::Any,
    };
    let value = match shape {
        // Your own draws are one player's total; the opponents' draws are
        // summed over every opponent (Heliod, the Warped Eclipse).
        DynamicCostValueShape::CardsDrawn(DynamicPlayerKind::You) => {
            with_for_each_surface(Value::MaxCardsDrawnThisTurn(PlayerFilter::You))
        }
        DynamicCostValueShape::CardsDrawn(player) => {
            with_for_each_surface(Value::TurnHistoryCount(
                ironsmith_core::TurnHistoryCount::CardsDrawn(player_filter(player)),
            ))
        }
        DynamicCostValueShape::LifeGained(player) => {
            with_for_each_surface(Value::LifeGainedThisTurn(player_filter(player)))
        }
        DynamicCostValueShape::KickCount => Value::KickCount,
        DynamicCostValueShape::CreaturesDiedThisTurn => Value::CreaturesDiedThisTurn,
        DynamicCostValueShape::OpponentsLifeLostThisTurn => {
            Value::LifeLostThisTurn(PlayerFilter::Opponent)
        }
        DynamicCostValueShape::ControlledCreaturesDiedThisTurn => {
            Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You)
        }
        DynamicCostValueShape::PlayersBeingAttacked => {
            with_for_each_surface(Value::PlayersBeingAttacked)
        }
        DynamicCostValueShape::SpellCast { player, kind } => {
            let player = player_filter(player);
            match kind {
                SpellCastDynamicKind::CardTypes => {
                    let mut filter = ObjectFilter::spell();
                    filter.cast_by = Some(player);
                    Value::CardTypesAmong(filter)
                }
                SpellCastDynamicKind::OtherThanFirst => Value::Add(
                    Box::new(Value::SpellsCastThisTurn(player)),
                    Box::new(Value::Fixed(-1)),
                ),
                SpellCastDynamicKind::MatchingTypes {
                    instant,
                    sorcery,
                    exclude_source,
                } => {
                    let mut filter = ObjectFilter::spell();
                    filter.card_types = match (instant, sorcery) {
                        (true, true) => vec![CardType::Instant, CardType::Sorcery],
                        (true, false) => vec![CardType::Instant],
                        (false, true) => vec![CardType::Sorcery],
                        (false, false) => Vec::new(),
                    };
                    Value::SpellsCastThisTurnMatching {
                        player,
                        filter,
                        exclude_source,
                    }
                }
                SpellCastDynamicKind::Simple => Value::SpellsCastThisTurn(player),
            }
        }
        DynamicCostValueShape::CardTypesInGraveyard(player) => {
            Value::CardTypesInGraveyard(player_filter(player))
        }
        DynamicCostValueShape::ColorsSpentToCastThisSpell => {
            Value::ColorsOfManaSpentToCastThisSpell
        }
        DynamicCostValueShape::PartySize => Value::PartySize(PlayerFilter::You),
        DynamicCostValueShape::AggregateScope => {
            let each_idx = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
                .map(|boundary| boundary.token)
                .unwrap_or(0);
            let filter_tokens = tokens.get(each_idx + 1..).unwrap_or_default();
            let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) else {
                return Ok(None);
            };
            value
        }
        DynamicCostValueShape::CardTypesAmong { .. }
        | DynamicCostValueShape::UnsupportedCardTypesAmong => {
            unreachable!("card-types-among values are handled before history fallbacks")
        }
        DynamicCostValueShape::CountersRemovedThisWay(counter_type) => Value::PendingPriorEffectMetric(
            ironsmith_core::PriorEffectMetricQuery::new(
                EffectMetricSource::Outcome,
                EffectMetric::Count,
            )
            .with_action(ironsmith_core::PriorEffectAction::Removed)
            .with_counter_type(counter_type),
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
        DynamicCostValueShape::EnergyPaidThisWay => Value::PendingPriorEffectMetric(
            ironsmith_core::PriorEffectMetricQuery::new(
                EffectMetricSource::Outcome,
                EffectMetric::Count,
            )
            .with_action(ironsmith_core::PriorEffectAction::Removed)
            .with_counter_type(Some(crate::object::CounterType::Energy)),
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::EnergyPaidThisWay),
        DynamicCostValueShape::PlayerCounters(counter_type) => {
            Value::PlayerCounters(PlayerFilter::You, counter_type)
        }
        DynamicCostValueShape::ThisWayMetric(metric) => match metric {
            DynamicThisWayMetric::Destroyed => {
                let mut count_words = vec!["for", "each"];
                count_words.extend(parser_token_word_refs(history_tokens));
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                    && matches!(value.unhinted(), Value::PendingPriorEffectMetric(_))
                {
                    value
                } else {
                    Value::PendingEffectMetric {
                        source: EffectMetricSource::AffectedObjects,
                        metric: EffectMetric::Count,
                    }
                }
            }
            DynamicThisWayMetric::Sacrificed => {
                let words = parser_token_word_refs(history_tokens);
                let mut count_words = vec!["for", "each"];
                count_words.extend(words.iter().copied());
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                    && matches!(value.unhinted(), Value::PendingPriorEffectMetric(_))
                {
                    with_for_each_surface(value)
                } else {
                    let kind = if words
                        .iter()
                        .any(|word| matches!(*word, "creature" | "creatures"))
                    {
                        ironsmith_core::SacrificedObjectKind::Creature
                    } else if words
                        .iter()
                        .any(|word| matches!(*word, "artifact" | "artifacts"))
                    {
                        ironsmith_core::SacrificedObjectKind::Artifact
                    } else if words
                        .iter()
                        .any(|word| matches!(*word, "enchantment" | "enchantments"))
                    {
                        ironsmith_core::SacrificedObjectKind::Enchantment
                    } else {
                        ironsmith_core::SacrificedObjectKind::Permanent
                    };
                    with_for_each_surface(
                        Value::PendingEffectMetric {
                            source: EffectMetricSource::AffectedObjects,
                            metric: EffectMetric::Count,
                        }
                        .with_surface_hint(
                            ironsmith_core::ValueSurfaceHint::PermanentsSacrificedThisWay,
                        )
                        .with_surface_hint(
                            ironsmith_core::ValueSurfaceHint::SacrificedObject(kind),
                        ),
                    )
                }
            }
            DynamicThisWayMetric::Discarded => {
                let all_words = parser_token_word_refs(tokens);
                let count_words =
                    crate::word_primitives::parse_sequence_start(&all_words, &["for", "each"])
                        .map(|start| all_words[start..].to_vec())
                        .unwrap_or_else(|| {
                            let mut words = vec!["for", "each"];
                            words.extend(parser_token_word_refs(history_tokens));
                            words
                        });
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                    && matches!(value.unhinted(), Value::PendingPriorEffectMetric(_))
                {
                    value
                } else {
                    Value::PendingEffectMetric {
                        source: EffectMetricSource::Outcome,
                        metric: EffectMetric::Count,
                    }
                }
            }
            DynamicThisWayMetric::Exiled => {
                let action_idx =
                    crate::slice_primitives::select_position(history_tokens, |token| {
                        token.is_word("exiled")
                    });
                let mut filter = action_idx
                    .and_then(|idx| {
                        let subject = &history_tokens[..idx];
                        (!subject.is_empty())
                            .then(|| {
                                crate::grammar::primitives::probe_shape(parse_object_filter(
                                    subject, false,
                                ))
                            })
                            .flatten()
                    })
                    .unwrap_or_default();
                filter.zone = Some(Zone::Exile);
                filter = filter.match_tagged(
                    crate::tag::CompilerReferenceTag::SourceExiled.bind(),
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                );
                Value::Count(filter)
            }
            DynamicThisWayMetric::Tapped => {
                // "This spell costs {1} less to cast for each creature tapped
                // this way" counts the permanents the spell's own tap cost
                // tapped (CR 601.2b, 601.2f); the engine binds it once that
                // cost is paid, exactly like "sacrificed this way".
                let action_idx =
                    crate::slice_primitives::select_position(history_tokens, |token| {
                        token.is_word("tapped")
                    });
                let Some(subject) = action_idx
                    .map(|idx| &history_tokens[..idx])
                    .filter(|subject| !subject.is_empty())
                else {
                    return Ok(None);
                };
                let Some(mut filter) =
                    crate::grammar::primitives::probe_shape(parse_object_filter(subject, false))
                else {
                    return Ok(None);
                };
                filter.zone = Some(Zone::Battlefield);
                with_for_each_surface(Value::PendingPriorEffectMetric(
                    ironsmith_core::PriorEffectMetricQuery::new(
                        EffectMetricSource::AffectedObjects,
                        EffectMetric::Count,
                    )
                    .with_filter(filter)
                    .with_action(ironsmith_core::PriorEffectAction::Tapped),
                ))
            }
        },
        DynamicCostValueShape::RevealedPublic => Value::Count(ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::PublicRevealed.bind(),
        )),
        DynamicCostValueShape::RevealedOther => {
            let words = parser_token_word_refs(tokens);
            let Some((value, used_words)) = parse_for_each_count_value_words(&words) else {
                return Ok(None);
            };
            if used_words != words.len() {
                return Ok(None);
            }
            value
        }
        DynamicCostValueShape::CounterReference(reference) => {
            let counter_type = reference.counter_type;
            let value = match reference.reference_kind {
                CounterReferenceKind::Source => match counter_type {
                    Some(counter_type) => Value::CountersOnSource(counter_type),
                    None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
                },
                CounterReferenceKind::Tagged => Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(
                        (crate::tag::CompilerReferenceTag::It.bind()).into(),
                    )),
                    counter_type,
                ),
                CounterReferenceKind::Other => {
                    let words = parser_token_word_refs(reference.reference_tokens);
                    if let Some(surface) = source_reference_surface_for_words(&words) {
                        Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            counter_type,
                        )
                    } else if let Some(object_words) = words
                        .strip_suffix(&["it", "targets"][..])
                        .map(|object_words| {
                            object_words
                                .strip_prefix(&["the"][..])
                                .unwrap_or(object_words)
                        })
                        .filter(|object_words| !object_words.is_empty())
                    {
                        // "each +1/+1 counter on the creature it targets"
                        // (Warrior's Blades): counters on the chosen target.
                        let object_tokens =
                            crate::lexer::synthetic_word_tokens(object_words.iter().copied());
                        let Some(filter) = parse_complete_cost_count_filter(&object_tokens)? else {
                            return Ok(None);
                        };
                        Value::CountersOn(
                            Box::new(ChooseSpec::target(ChooseSpec::Object(filter))),
                            counter_type,
                        )
                    } else {
                        // "each +1/+1 counter on creatures you control"
                        // (Deepwood Denizen): counters summed over every
                        // matching permanent.
                        let Some(mut filter) =
                            parse_complete_cost_count_filter(reference.reference_tokens)?
                        else {
                            return Ok(None);
                        };
                        if filter.zone.is_none() {
                            filter.zone = Some(Zone::Battlefield);
                        }
                        Value::CountersOn(Box::new(ChooseSpec::All(filter)), counter_type)
                    }
                }
            };
            with_for_each_surface(value)
        }
        DynamicCostValueShape::UnsupportedThisWay => {
            return Err(CardTextError::ParseError(format!(
                "unsupported this-way dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        DynamicCostValueShape::Other { filter_tokens } => {
            if let Some(value) =
                crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(
                    filter_tokens,
                )
            {
                with_for_each_surface(value)
            } else if matches!(
                parser_token_word_refs(filter_tokens).as_slice(),
                ["time" | "times", "it's" | "its", "been", "cast", "from", "the", "command", "zone", "this", "game"]
                    | ["time" | "times", "it", "has" | "s", "been", "cast", "from", "the", "command", "zone", "this", "game"]
            ) {
                // "for each time it's been cast from the command zone this
                // game" (Myth Unbound): within a cost modifier "it" is the
                // spell being priced, the iterated object of that context.
                with_for_each_surface(Value::CommanderCastCountOf(Box::new(ChooseSpec::Iterated)))
            } else if parse_commander_cast_count_player(filter_tokens).is_some()
                && let Some(value) =
                    crate::grammar::shared_util::value_helper_shapes::parse_commander_cast_count_value(
                        &parser_token_word_refs(filter_tokens),
                    )
            {
                with_for_each_surface(value)
            } else {
                let filter_words = parser_token_word_refs(filter_tokens);
                let mut count_words = vec!["for", "each"];
                count_words.extend(filter_words.iter().copied());
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                {
                    value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
                } else if let Some(filter) = parse_complete_cost_count_filter(filter_tokens)? {
                    with_for_each_surface(Value::Count(filter))
                } else {
                    return Ok(None);
                }
            }
        }
    };
    Ok(Some(value))
}

pub fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if keyword_static_lines::parse_that_much_value_marker_tokens(tokens) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    // "add twice that much {G}" (Fangorn, Tree Shepherd)
    if tokens.first().is_some_and(|token| token.is_word("twice"))
        && keyword_static_lines::parse_that_much_value_marker_tokens(&tokens[1..])
    {
        return Some(Value::Scaled(
            Box::new(Value::EventValue(EventValueSpec::Amount)),
            2,
        ));
    }
    None
}

pub fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let tokens = super::super::grammar::effects::split_labeled_effect_prefix_lexed(&tokens)
        .unwrap_or(&tokens);
    if let Some(fact) = type_and_color_facts::parse_skip_your_upkeep_tokens(tokens) {
        let mut ability = StaticAbility::player_skips_upkeep(crate::target::PlayerFilter::You);
        match fact.tail {
            type_and_color_facts::SkipYourUpkeepTail::None => {}
            type_and_color_facts::SkipYourUpkeepTail::Condition(condition_tokens) => {
                let condition = parse_static_condition_clause(condition_tokens)?;
                ability = ability.with_condition(condition);
            }
            type_and_color_facts::SkipYourUpkeepTail::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip-upkeep tail (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
        }
        return Ok(Some(ability));
    }
    if is_players_skip_upkeep_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub fn parse_players_skip_extra_turns_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = crate::lexer::parser_token_word_refs(&tokens);
    let player = if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "an", "opponent", "would", "begin", "an", "extra", "turn", "that", "player",
            "skips", "that", "turn", "instead",
        ],
    ) {
        crate::target::PlayerFilter::Opponent
    } else if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "a", "player", "would", "begin", "an", "extra", "turn", "that", "player",
            "skips", "that", "turn", "instead",
        ],
    ) {
        crate::target::PlayerFilter::Any
    } else if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "you", "would", "begin", "an", "extra", "turn", "skip", "that", "turn", "instead",
        ],
    ) {
        crate::target::PlayerFilter::You
    } else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::players_skip_extra_turns(player)))
}

/// "If you would begin your turn while this artifact is tapped, you may skip
/// that turn instead. If you do, untap this artifact." (Time Vault).
///
/// CR 614.10: a replacement of the turn's beginning, offered before the turn
/// starts. It is a static ability of the permanent, never a resolution-time
/// instruction.
pub fn parse_may_skip_turn_while_source_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(period) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Period)
    else {
        return Ok(None);
    };
    let (replacement, followup) = tokens.split_at(period + 1);
    if !crate::grammar::effects::followup_shapes::is_skip_tapped_source_turn_replacement(
        replacement,
    ) || !crate::grammar::effects::followup_shapes::is_if_did_untap_source_followup(followup)
    {
        return Ok(None);
    }
    Ok(Some(
        StaticAbility::may_skip_turn_while_source_tapped_then_untap(),
    ))
}

/// "If you would begin your draw step, you may skip that step instead. If you
/// do, you gain 2 life." (Fasting).
///
/// CR 614.10: an optional replacement of the step's beginning, offered by the
/// turn runner before the draw step begins. The "if you do" sentence is the
/// program run when the player skips.
pub fn parse_may_skip_draw_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(period) = tokens
        .iter()
        .position(|token| token.kind == TokenKind::Period)
    else {
        return Ok(None);
    };
    let (replacement, followup) = tokens.split_at(period + 1);
    if !crate::grammar::effects::followup_shapes::is_may_skip_draw_step_replacement(replacement) {
        return Ok(None);
    }
    let Some(body) = crate::grammar::effects::followup_shapes::if_you_do_followup_body(followup)
    else {
        return Ok(None);
    };
    let body = crate::util::trim_edge_punctuation_tokens(body);
    if body.is_empty() || body.iter().any(|token| token.kind == TokenKind::Period) {
        return Ok(None);
    }
    let Ok(effects) = crate::clause_support::parse_effect_sentences_lexed(&body) else {
        return Ok(None);
    };
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(StaticAbility::may_skip_draw_step_with_followup(
        effects,
        crate::lexer::render_token_slice(tokens),
    )))
}

pub fn parse_skip_your_draw_step_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_skip_your_draw_step_line_lexed(tokens) {
        return Ok(Some(StaticAbility::player_skips_draw_step(
            crate::target::PlayerFilter::You,
        )));
    }
    Ok(None)
}

pub fn parse_legend_rule_doesnt_apply_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(scope) = keyword_static_lines::parse_legend_rule_doesnt_apply_tokens(tokens) {
        return Ok(Some(match scope {
            keyword_static_lines::LegendRuleScopeShape::Global => {
                StaticAbility::legend_rule_doesnt_apply()
            }
            keyword_static_lines::LegendRuleScopeShape::Controller => {
                // "The legend rule doesn't apply to Spiders you control"
                // (Spider-Verse): a typed object scope between "apply to"
                // and "you control" restricts the exemption to it.
                match legend_rule_controlled_object_scope(tokens)? {
                    Some(filter) => {
                        StaticAbility::legend_rule_doesnt_apply_to_controller_matching(filter)
                    }
                    None => StaticAbility::legend_rule_doesnt_apply_to_controller(),
                }
            }
            keyword_static_lines::LegendRuleScopeShape::ControllerCreatures => {
                StaticAbility::legend_rule_doesnt_apply_to_controller_matching(
                    ObjectFilter::creature(),
                )
            }
            keyword_static_lines::LegendRuleScopeShape::ControllerTokens => {
                StaticAbility::legend_rule_doesnt_apply_to_tokens_you_control()
            }
        }));
    }
    Ok(None)
}

/// The object noun phrase of "... doesn't apply to <objects> you control",
/// when it names something narrower than permanents.
fn legend_rule_controlled_object_scope(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words: Vec<(usize, &str)> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.as_word().map(|word| (idx, word)))
        .collect();
    let Some(apply_idx) = words.iter().position(|(_, word)| *word == "apply") else {
        return Ok(None);
    };
    if words.get(apply_idx + 1).map(|(_, word)| *word) != Some("to") || words.len() < apply_idx + 5 {
        return Ok(None);
    }
    let len = words.len();
    if words[len - 2].1 != "you" || words[len - 1].1 != "control" {
        return Ok(None);
    }
    let start = words[apply_idx + 2].0;
    let end = words[len - 2].0;
    if start >= end {
        return Ok(None);
    }
    let noun_words: Vec<&str> = words[apply_idx + 2..len - 2].iter().map(|(_, word)| *word).collect();
    if matches!(noun_words.as_slice(), ["permanents"] | ["permanent"]) {
        return Ok(None);
    }
    let Ok(mut filter) = parse_object_filter(&tokens[start..end], false) else {
        return Ok(None);
    };
    if filter == ObjectFilter::default() {
        return Ok(None);
    }
    filter.controller = None;
    Ok(Some(filter))
}

pub fn parse_all_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_all_permanents_colorless_line_lexed(tokens) {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub fn parse_subject_are_card_types_in_addition_to_their_other_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if crate::grammar::abilities::parse_source_is_chosen_type_in_addition_line_lexed(tokens)
        .is_some()
    {
        // The source-scoped chosen-type production owns its complete line.
        return Ok(None);
    }
    let Some(fact) = type_and_color_facts::parse_subject_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    if fact.chosen_type {
        let Some(filter) = complete_characteristic_subject(fact.subject_tokens)? else {
            return Ok(None);
        };
        // The authored family wins over recipient types. A land or land/creature
        // union may receive the chosen creature type without choosing a land type.
        let explicit_creature_type = fact.descriptor_tokens.iter().any(|token| token.is_word("creature"));
        fn includes_land(filter: &ObjectFilter) -> bool {
            filter.card_types.contains(&CardType::Land)
                || filter.any_of.iter().any(includes_land)
        }
        if !explicit_creature_type && includes_land(&filter) {
            return Ok(Some(vec![StaticAbility::add_chosen_basic_land_type(
                filter,
                render_token_slice(tokens),
            )]));
        }
        return Ok(Some(vec![StaticAbility::add_chosen_creature_type(
            filter,
            render_token_slice(tokens),
        )]));
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }

        let Some(subtype) = parse_subtype_flexible(descriptor) else {
            return Ok(None);
        };
        crate::slice_primitives::push_unique(&mut subtypes, subtype);
    }
    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    // A bare "it" subject of a permanent's own static ("It is a land in
    // addition to its other types") names that permanent; it is not an
    // unconstrained object filter (which would read as "all permanents").
    let filter = if matches!(parser_token_word_refs(fact.subject_tokens).as_slice(), ["it"]) {
        ObjectFilter::source()
    } else {
        parse_object_filter_lexed(fact.subject_tokens, false)?
    };

    let mut abilities = Vec::new();
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

/// "Clandestine Chameleon has all abilities of ability stickers on other
/// permanents you own and cards in your graveyard.": the source gains the
/// abilities of the ability stickers on every object the trailing complete
/// nominal names.
pub fn parse_source_has_ability_sticker_abilities_of_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clean = trim_edge_punctuation(tokens);
    let view = crate::lexer::TokenWordView::new(&clean);
    let words = view.word_refs();
    const PHRASE: &[&str] = &["has", "all", "abilities", "of", "ability", "stickers", "on"];
    let Some(has) = words.windows(PHRASE.len()).position(|window| window == PHRASE) else {
        return Ok(None);
    };
    if has == 0 || !is_source_reference_words(&words[..has]) {
        return Ok(None);
    }
    let Some(objects_start) = view.map_word_to_token_start(has + PHRASE.len()) else {
        return Ok(None);
    };
    let object_tokens = &clean[objects_start..];
    if object_tokens.is_empty() {
        return Ok(None);
    }
    let words_after = parser_token_word_refs(object_tokens);
    if crate::util::is_source_reference_words(&words_after) {
        // "... on this Equipment" belongs to the attached-object production.
        return Ok(None);
    }
    let filter = match complete_characteristic_subject(object_tokens)? {
        Some(filter) => filter,
        None => parse_object_filter_lexed(object_tokens, false)?,
    };
    Ok(Some(StaticAbility::source_has_ability_sticker_abilities_of(
        filter,
        render_token_slice(&clean).trim().to_string(),
    )))
}

/// "Creatures you control are the chosen type. The same is true for creature
/// spells you control and creature cards you own that aren't on the
/// battlefield." (Conspiracy): without "in addition to their other types" the
/// chosen creature type replaces each object's creature types (CR 205.1a),
/// for the first subject and every subject the second sentence extends it to.
pub fn parse_subject_are_chosen_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clean = trim_edge_punctuation(tokens);
    let sentences: Vec<&[OwnedLexToken]> = clean
        .split(|token| token.is_period())
        .filter(|sentence| !sentence.is_empty())
        .collect();
    let Some((first, rest)) = sentences.split_first() else {
        return Ok(None);
    };
    if rest.len() > 1 {
        return Ok(None);
    }
    let view = crate::lexer::TokenWordView::new(first);
    let words = view.word_refs();
    let tail_len = match words.as_slice() {
        [.., "are" | "is", "the", "chosen", "type"] => 4,
        [.., "are" | "is", "the", "chosen", "creature", "type"] => 5,
        _ => return Ok(None),
    };
    if words.len() <= tail_len {
        return Ok(None);
    }
    let Some(subject_end) = view.map_word_to_token_start(words.len() - tail_len) else {
        return Ok(None);
    };
    let Some(first_subject) = complete_characteristic_subject(&first[..subject_end])? else {
        return Ok(None);
    };
    if first_subject.source {
        return Ok(None);
    }
    // A land subject's "chosen type" is a chosen land type ("Enchanted land
    // is the chosen type."), never a creature type.
    if tail_len == 4 && first_subject.card_types.contains(&CardType::Land) {
        return Ok(None);
    }
    let mut subjects = vec![first_subject];
    if let Some(second) = rest.first() {
        let second = trim_edge_punctuation(second);
        let words = parser_token_word_refs(&second);
        if !words.starts_with(&["the", "same", "is", "true", "for"]) || second.len() <= 5 {
            return Ok(None);
        }
        let Some(extended) = complete_characteristic_subject(&second[5..])? else {
            return Ok(None);
        };
        if extended.source {
            return Ok(None);
        }
        if extended.any_of.is_empty() || extended.controller.is_some() || extended.owner.is_some() {
            subjects.push(extended);
        } else {
            subjects.extend(extended.any_of);
        }
    }
    let filter = if subjects.len() == 1 {
        subjects.remove(0)
    } else {
        let mut union = ObjectFilter::default();
        union.any_of = subjects;
        union
    };
    Ok(Some(vec![StaticAbility::set_chosen_creature_type(
        filter,
        render_token_slice(&clean).trim().to_string(),
    )]))
}

/// Whole nominal subjects for the bounded copular characteristic productions.
/// No suffix recovery: unrelated words cannot become an ignored qualifier.
pub fn complete_characteristic_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if !tokens.iter().all(|token| token.as_word().is_some()) { return Ok(None); }
    let words = parser_token_word_refs(tokens);
    if words.first().is_some_and(|word| matches!(*word, "and" | "or" | "and/or"))
        || words.windows(2).any(|pair| pair.iter().all(|word| matches!(*word, "and" | "or" | "and/or"))) {
        return Ok(None);
    }
    if words.contains(&"instead") || words.last().is_some_and(|word| matches!(*word, "and" | "or" | "and/or")) {
        return Ok(None);
    }
    if is_source_reference_words(&words) || anthem_grant_grammar::is_source_it_subject(tokens) {
        return Ok(Some(ObjectFilter::source()));
    }
    if matches!(words.as_slice(), ["enchanted" | "equipped", "creature" | "permanent" | "artifact" | "land" | "vehicle"]) {
        return parse_object_filter_lexed(tokens, false).map(Some);
    }
    let tokens = if tokens.first().is_some_and(|token| token.is_any_word(&["all", "each"])) {
        &tokens[1..]
    } else { tokens };
    let mut filter = if let Some(filter) =
        crate::grammar::filters::parse_simple_object_filter_lexed(tokens, false)
    {
        filter
    } else if let Some(filter) = complete_characteristic_extended_subject(tokens)? {
        filter
    } else {
        return Ok(None);
    };
    if filter.zone.is_none() && filter.any_of.is_empty()
        && !filter.has_explicit_card_noun() && filter.stack_kind.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(filter))
}

/// Two compositional extensions of a complete simple nominal: a terminal
/// nonbattlefield card domain, or independently scoped nominal set arms.
/// A bare type list is still owned by the simple reader (not split into sets).
fn complete_characteristic_extended_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    for split in 1..tokens.len() {
        let tail = parser_token_word_refs(&tokens[split..]);
        if matches!(tail.as_slice(),
            ["that", "aren't" | "arent" | "isn't" | "isnt", "on", "the", "battlefield"]
            | ["that", "are" | "is", "not", "on", "the", "battlefield"])
        {
            if let Some(base) = crate::grammar::filters::parse_simple_object_filter_lexed(
                &tokens[..split], false,
            ) {
                // The simple reader infers Battlefield from a permanent card
                // type even with a card noun. That default is not an authored
                // zone and must yield to the explicit nonbattlefield suffix.
                let explicit_zone = tokens[..split].iter()
                    .any(|token| token.is_any_word(&["in", "on", "from"]));
                if base.has_explicit_card_noun() && !explicit_zone
                    && matches!(base.zone, None | Some(Zone::Battlefield))
                    && base.stack_kind.is_none() && base.any_of.is_empty()
                {
                    // Only this fully validated single nominal uses the existing
                    // seven-zone expansion. No independently scoped arms are
                    // reparsed through the broad union reader.
                    return parse_object_filter_lexed(tokens, false).map(Some);
                }
            }
            return Ok(None);
        }
        if !tokens[split].is_any_word(&["and", "or", "and/or"]) {
            continue;
        }
        // Requiring a scoped complete left arm keeps an internal type list
        // (e.g. artifact and enchantment creatures) out of this production.
        let Some(left) = complete_characteristic_subject(&tokens[..split])? else {
            continue;
        };
        if left.source || (left.controller.is_none() && left.owner.is_none()
            && left.any_of.is_empty()) {
            continue;
        }
        let Some(right) = complete_characteristic_subject(&tokens[split + 1..])? else {
            continue;
        };
        if !right.source && (right.controller.is_some() || right.owner.is_some()
            || !right.any_of.is_empty()) {
            let mut union = ObjectFilter::default();
            union.any_of = vec![left, right];
            if tokens[split].is_word("and") {
                union.set_conjunctive_set_surface(true);
            } else if tokens[split].is_word("and/or") {
                union.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
            }
            return Ok(Some(union));
        }
    }
    Ok(None)
}

/// Unconditional additive subtype assertions use the existing source-zone and
/// origin owners to distinguish printed characteristic definitions from grants.
pub fn parse_subject_is_also_subtypes_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subject_card_type_identity_tokens(tokens) else {
        return Ok(None);
    };
    if !fact.descriptor_tokens.first().is_some_and(|token| token.is_word("also")) { return Ok(None); }
    let Some(filter) = complete_characteristic_subject(fact.subject_tokens)? else { return Ok(None); };
    if !fact.descriptor_tokens.last().and_then(OwnedLexToken::as_word)
        .is_some_and(|word| parse_subtype_flexible(word).is_some()) {
        return Ok(None);
    }
    let mut subtypes = Vec::new();
    for token in &fact.descriptor_tokens[1..] {
        if token.is_comma() { continue; }
        let Some(word) = token.as_word() else { return Ok(None); };
        if matches!(word, "a" | "an" | "and") { continue; }
        let Some(subtype) = parse_subtype_flexible(word) else { return Ok(None); };
        crate::slice_primitives::push_unique(&mut subtypes, subtype);
    }
    if subtypes.is_empty() { return Ok(None); }
    Ok(Some(StaticAbility::add_subtypes(filter, subtypes)))
}

pub fn parse_subject_is_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_source_is_chosen_color_tokens(tokens).is_some() {
        // Preserve the established source owner and its canonical display.
        return Ok(None);
    }
    let Some(fact) = type_and_color_facts::parse_subject_card_type_identity_tokens(tokens) else {
        return Ok(None);
    };
    if !fact.descriptor_tokens.iter().all(|token| token.as_word().is_some()) { return Ok(None); }
    let words = parser_token_word_refs(fact.descriptor_tokens);
    if !matches!(words.as_slice(), ["the", "chosen", "color"] | ["chosen", "color"]) {
        return Ok(None);
    }
    let Some(filter) = complete_characteristic_subject(fact.subject_tokens)? else { return Ok(None); };
    Ok(Some(StaticAbility::set_chosen_color(filter, render_token_slice(tokens))))
}

pub fn parse_subject_is_card_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subject_card_type_identity_tokens(tokens) else {
        return Ok(None);
    };

    let mut card_types = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "a" | "an" | "and") {
            continue;
        }
        let Some(card_type) = parse_card_type(word) else {
            return Ok(None);
        };
        crate::slice_primitives::push_unique(&mut card_types, card_type);
    }
    if card_types.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(fact.subject_tokens)?;
    let mut filter = anthem_subject_filter(&subject);
    if matches!(subject, AnthemSubjectAst::Source) {
        let subject_words = parser_token_word_refs(fact.subject_tokens);
        if subject_words.len() > 1
            && let Some(surface) = source_reference_surface_for_words(&subject_words)
        {
            filter = filter.with_source_surface(surface);
        }
    }

    Ok(Some(StaticAbility::set_card_types(filter, card_types)))
}

/// "Black and/or red permanents and spells are colorless sources of damage."
/// (Ghostly Flame): matching permanents and spells are colorless only as the
/// source of damage, read by every damage-source color check.
pub fn parse_colorless_damage_sources_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let tail = ["permanents", "and", "spells", "are", "colorless", "sources", "of", "damage"];
    let Some(prefix) = words.as_slice().strip_suffix(&tail[..]) else {
        return Ok(None);
    };
    let mut colors = crate::color::ColorSet::new();
    for word in prefix {
        if matches!(*word, "and" | "or" | "and/or") {
            continue;
        }
        let Some(color) = crate::util::parse_color(word) else {
            return Ok(None);
        };
        colors = colors.union(color);
    }
    if colors.is_empty() {
        return Ok(None);
    }
    let names = crate::color::Color::ALL
        .into_iter()
        .filter(|color| colors.contains(*color))
        .map(|color| color.name())
        .collect::<Vec<_>>()
        .join(" and/or ");
    let mut display = format!("{names} permanents and spells are colorless sources of damage");
    if let Some(first) = display.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    let mut filter = ObjectFilter::default();
    filter.colors = Some(colors);
    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::DamageSourcesAreColorless(filter),
        display,
    )))
}

pub fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_all_cards_spells_permanents_colorless_tokens(tokens) {
        let mut filter = ObjectFilter::default();
        filter.set_global_characteristic_domain_surface(Some(
            ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents,
        ));
        return Ok(Some(StaticAbility::make_colorless(filter)));
    }
    Ok(None)
}

/// Swirl the Mists: a layer-3 substitution of every other color word by the
/// source's chosen color in each spell and permanent (CR 612.2).
pub fn parse_all_color_words_changed_to_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_all_color_words_changed_to_chosen_color_tokens(tokens) {
        return Ok(Some(StaticAbility::all_color_words_become_chosen_color()));
    }
    Ok(None)
}

pub fn parse_all_cards_spells_permanents_add_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if type_and_color_facts::parse_all_cards_chosen_color_addition_tokens(tokens).is_some() {
        return Ok(Some(StaticAbility::add_chosen_color(
            ObjectFilter::default(),
            render_token_slice(tokens),
        )));
    }

    Ok(None)
}

pub fn parse_conjoined_subject_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let subject_tokens = trim_lexed_commas(tokens);
    if let Some(filter) =
        crate::activation_and_restrictions::parse_type_adjective_conjunction_filter(subject_tokens)?
    {
        return Ok(filter);
    }
    let subject_segments = split_lexed_slices_on_and(subject_tokens);
    if subject_segments.len() <= 1 {
        return parse_object_filter_lexed(subject_tokens, false);
    }

    let mut branches = Vec::with_capacity(subject_segments.len());
    for segment in subject_segments {
        let segment = trim_lexed_commas(segment);
        if segment.is_empty() {
            return parse_object_filter_lexed(subject_tokens, false);
        }
        branches.push(parse_object_filter_lexed(segment, false)?);
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Ok(filter)
}

pub fn parse_all_are_pt_color_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_power_toughness_type_addition_tokens(tokens)
    else {
        return Ok(None);
    };
    // Both grammars recognize a sized creature addition. Use the same
    // ordering and recipient filter so the registry sees one interpretation.
    if let Some(abilities) = parse_lands_are_pt_creatures_still_lands_line(tokens)? {
        return Ok(Some(abilities));
    }
    let mut colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if is_article(descriptor) || matches!(descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(color) = parse_color(descriptor) {
            colors = colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in pt-color-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if colors.is_empty() && card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_conjoined_subject_filter(fact.subject_tokens)?;

    let mut abilities = Vec::new();
    if !colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), colors));
    }
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter.clone(), subtypes));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter,
        fact.power,
        fact.toughness,
    ));
    Ok(Some(abilities))
}

pub fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_color_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), fact.color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub fn parse_all_creatures_are_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // The dedicated colorless productions own their complete lines (CR
    // 105.2c); reading them again as a set-colors effect is a second,
    // non-equivalent interpretation of the same sentence.
    if is_all_permanents_colorless_line_lexed(tokens)
        || keyword_static_lines::parse_all_cards_spells_permanents_colorless_tokens(tokens)
    {
        return Ok(None);
    }
    let Some(fact) = type_and_color_facts::parse_subject_color_tokens(tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(fact.subject_tokens, false)?;
    // CR 109.2: an unqualified type/subtype noun ("Slivers") means
    // permanents. Preserve source references and explicit card/zone domains.
    if !filter.source && filter.zone.is_none() && filter.any_of.is_empty()
        && !filter.has_explicit_card_noun()
    {
        filter.zone = Some(Zone::Battlefield);
    }

    if fact.exclude_from_color_identity {
        if !filter.is_source_only() { return Ok(None); }
        return Ok(Some(StaticAbility::set_colors_without_color_identity(filter, fact.color)));
    }
    Ok(Some(StaticAbility::set_colors(filter, fact.color)))
}

pub fn parse_subjects_are_basic_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subjects_are_basic_tokens(tokens) else {
        return Ok(None);
    };

    let subject_segments = split_lexed_slices_on_and(fact.subject_tokens);
    let filter = if subject_segments.len() > 1 {
        let mut branches = Vec::with_capacity(subject_segments.len());
        for segment in subject_segments {
            let segment = trim_lexed_commas(segment);
            if segment.is_empty() {
                return Ok(None);
            }
            branches.push(parse_object_filter_lexed(segment, false)?);
        }
        let mut filter = ObjectFilter::default();
        filter.any_of = branches;
        filter
    } else {
        parse_object_filter_lexed(fact.subject_tokens, false)?
    };

    Ok(Some(StaticAbility::add_supertypes(
        filter,
        vec![Supertype::Basic],
    )))
}

pub fn parse_nonbasic_lands_are_basic_land_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_basic_land_subtype_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(StaticAbility::set_land_subtypes(
        filter,
        vec![fact.subtype],
    )))
}

pub fn parse_remove_snow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_remove_snow_line_lexed(tokens) {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

pub fn parse_land_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_land_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    match fact {
        type_and_color_facts::LandTypeAdditionFact::EveryBasic { subject_tokens } => {
            let filter = parse_object_filter_lexed(subject_tokens, false)?;
            Ok(Some(StaticAbility::add_subtypes(
                filter,
                vec![
                    Subtype::Plains,
                    Subtype::Island,
                    Subtype::Swamp,
                    Subtype::Mountain,
                    Subtype::Forest,
                ],
            )))
        }
        type_and_color_facts::LandTypeAdditionFact::One {
            subject_tokens,
            subtype,
        } => Ok(Some(StaticAbility::add_subtypes(
            parse_object_filter_lexed(subject_tokens, false)?,
            vec![subtype],
        ))),
    }
}

pub fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_land_animation_tokens(tokens) else {
        return Ok(None);
    };
    let Some(filter) = complete_characteristic_subject(fact.subject_tokens)? else {
        return Ok(None);
    };
    let mut colors = ColorSet::new();
    let mut subtypes = Vec::new();
    let mut creature = false;
    for token in fact.descriptor_tokens {
        let Some(word) = token.as_word() else { return Ok(None); };
        if matches!(word, "a" | "an" | "and") { continue; }
        if matches!(word, "creature" | "creatures") { creature = true; }
        else if let Some(color) = parse_color(word) { colors = colors.union(color); }
        else if let Some(subtype) = parse_subtype_flexible(word).filter(Subtype::is_creature_type) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
        } else { return Ok(None); }
    }
    if !creature { return Ok(None); }
    let mut abilities = vec![StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature])];
    if !subtypes.is_empty() { abilities.push(StaticAbility::add_subtypes(filter.clone(), subtypes)); }
    if !colors.is_empty() { abilities.push(StaticAbility::set_colors(filter.clone(), colors)); }
    abilities.push(StaticAbility::set_base_power_toughness(filter, fact.power, fact.toughness));
    Ok(Some(abilities))
}

pub fn parse_static_base_power_toughness_value_tail(
    tail_tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    if !keyword_static_lines::parse_iterated_mana_value_base_pt_tail_tokens(tail_tokens) {
        return None;
    }
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    Some((value.clone(), value))
}

/// Conditional copular creature descriptors with optional base size. Both the
/// leading condition and a trailing "as long as" are owned in full. This reader
/// does not absorb grants, durations, or quoted abilities.
pub fn parse_conditional_copular_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    // This is an ownership probe, never a committed parse: even a match is
    // deferred to the established sized-addition owner below. Its speculative
    // subject recovery must not taint a later, unrelated successful reading.
    // Nested capture preserves the caller's diagnostics and error behavior;
    // the eventual committed owner remains responsible for its own losses.
    let (sized_addition, _) = crate::parse_loss::capture(|| {
        parse_filter_is_pt_creature_in_addition_line(tokens)
    });
    if sized_addition?.is_some() {
        // The established complete sized-addition bundle owns its ordering,
        // condition representation and canonical rendering.
        return Ok(None);
    }
    let tokens = crate::grammar::document_shapes::parse_statement_label_strip_tokens(tokens).body_tokens;
    let tokens = trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Period | TokenKind::Semicolon)) {
        return Ok(None);
    }
    let (body, condition_tokens) = if let Some(split) = split_as_long_as_condition_prefix_lexed(tokens) {
        (split.remainder_tokens, split.condition_tokens)
    } else if let Some(index) = tokens.windows(3).position(|part| {
        part[0].is_word("as") && part[1].is_word("long") && part[2].is_word("as")
    }) {
        (&tokens[..index], &tokens[index + 3..])
    } else { return Ok(None); };
    let Some(be) = static_keyword_line_shapes::parse_animation_copula(body) else { return Ok(None); };
    let contracted = be.token == 0 && body[0].is_any_word(&["it's", "it’s", "its"]);
    let subject_tokens = if contracted { &body[..1] } else { &body[..be.token] };
    let source_pronoun = contracted || matches!(parser_token_word_refs(subject_tokens).as_slice(), ["it"]);
    let predicate = &body[be.token + 1..];
    let addition = predicate.windows(3).position(|part| {
        part[0].is_word("in") && part[1].is_word("addition") && part[2].is_word("to")
    });
    let descriptor = if let Some(index) = addition {
        if type_and_color_facts::parse_other_type_addition_tail_tokens(&predicate[index..]).is_none() {
            return Ok(None);
        }
        &predicate[..index]
    } else { predicate };
    if !descriptor.iter().all(|token| token.as_word().is_some() || token.kind == TokenKind::Number) {
        return Ok(None);
    }
    let words = parser_token_word_refs(descriptor);
    let words = strip_leading_article_word_refs(&words);
    let fixed = words.first().and_then(|word| crate::grammar::primitives::probe_shape(parse_pt_modifier(word)));
    let type_words = if fixed.is_some() { &words[1..] } else { words };
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for word in type_words {
        if let Some(card_type) = parse_card_type(word) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
        } else if let Some(subtype) = parse_subtype_flexible(word).filter(Subtype::is_creature_type) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
        } else { return Ok(None); }
    }
    // "As long as this creature is on the battlefield, it's a land in addition
    // to its other types." (Awakened Skyclave): an explicit additive card type
    // on the source pronoun is the same layer-4 addition for any card type.
    let additive_source_type = addition.is_some() && source_pronoun && !card_types.is_empty();
    if !card_types.contains(&CardType::Creature) && !additive_source_type { return Ok(None); }
    // CR 205.1b: becoming an artifact creature retains existing types and
    // subtypes even without "in addition". Other replacements are not this rule.
    if addition.is_none() && card_types != [CardType::Artifact, CardType::Creature] {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(condition_tokens)?;
    // Explicit off-battlefield functioning belongs to its separate owner;
    // a battlefield-only conditional would otherwise be a vacuous success.
    if copular_condition_has_source_zone(&condition)
        && !copular_condition_is_source_on_battlefield(&condition)
    {
        return Err(CardTextError::ParseError("unsupported source-zone condition in copular static".into()));
    }
    let filter = if source_pronoun {
        match infer_attached_subject_filter_from_condition_expr(Some(&condition)) {
            Some(filter) => filter,
            None if condition_tokens.iter().any(|token| token.is_any_word(&["enchanted", "equipped", "fortified"])) => {
                // Do not silently bind an attachment-dependent pronoun to the
                // granting permanent when a compound antecedent is not modeled.
                return Err(CardTextError::ParseError("unsupported compound attachment antecedent in copular static".into()));
            }
            None => ObjectFilter::source(),
        }
    } else {
        let Some(filter) = complete_characteristic_subject(subject_tokens)? else { return Ok(None); };
        filter
    };
    let mut result = vec![StaticAbility::add_card_types(filter.clone(), card_types).with_condition(condition.clone()).into()];
    if !subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(filter.clone(), subtypes).with_condition(condition.clone()).into());
    }
    if let Some((power, toughness)) = fixed {
        result.push(StaticAbility::set_base_power_toughness(filter, power, toughness).with_condition(condition).into());
    }
    Ok(Some(result))
}

/// "As long as this creature is on the battlefield, ..." names the zone the
/// static already functions in (CR 611.3b), so the condition holds whenever the
/// ability applies; unlike an off-battlefield condition it is not vacuous.
fn copular_condition_is_source_on_battlefield(condition: &PredicateAst) -> bool {
    match condition {
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(zone)) => *zone == Zone::Battlefield,
        PredicateAst::CountComparison { count: AnthemCountExpression::MatchingFilter(filter), comparison, .. } =>
            filter.source
                && filter.zone == Some(Zone::Battlefield)
                && matches!(comparison, crate::effect::Comparison::GreaterThanOrEqual(1)),
        _ => false,
    }
}

fn copular_condition_has_source_zone(condition: &PredicateAst) -> bool {
    match condition {
        PredicateAst::Source(SourcePredicateAst::SourceIsInZone(_)) => true,
        PredicateAst::CountComparison { count: AnthemCountExpression::MatchingFilter(filter), .. } =>
            filter.source && filter.zone.is_some(),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) =>
            copular_condition_has_source_zone(left) || copular_condition_has_source_zone(right),
        PredicateAst::Not(inner) => copular_condition_has_source_zone(inner),
        _ => false,
    }
}

pub fn parse_filter_is_pt_creature_in_addition_and_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = LexedClause::new(tokens).word_refs();
    let Some(animation_verbs) = static_keyword_line_shapes::parse_animation_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = animation_verbs.be.token;
    let has_idx = animation_verbs.has.token;

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let contracted_source_subject = subject_start == be_idx
        && tokens.get(be_idx).is_some_and(|token| {
            token.is_word("it's") || token.is_word("it’s") || token.is_word("its")
        });
    let subject_tokens = if contracted_source_subject {
        tokens[be_idx..be_idx + 1].to_vec()
    } else {
        trim_commas(&tokens[subject_start..be_idx])
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut subject = if contracted_source_subject {
        AnthemSubjectAst::Source
    } else {
        match parse_anthem_subject(&subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };
    // Animation-subject grammar intentionally consumes leading distributive
    // quantifiers before it builds the semantic filter. Preserve that authored
    // surface on this compound animation bundle so both lowered continuous
    // abilities render with the same subject.
    if let AnthemSubjectAst::Filter(filter) = &mut subject {
        filter.set_set_quantifier_surface(leading_set_quantifier_surface(&subject_tokens));
    }
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    // In a copular animation bundle, the follow-up grant may repeat the
    // animated subject as a pronoun: "... is a 0/0 creature in addition to
    // its other types and it has annihilator 2." The pronoun belongs to the
    // grant head, not to the preceding type-addition predicate.
    let before_has_end = if has_idx >= 2
        && tokens[has_idx - 2].is_word("and")
        && tokens[has_idx - 1].is_word("it")
    {
        has_idx - 2
    } else {
        has_idx
    };
    let before_has = trim_commas(&tokens[be_idx + 1..before_has_end]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let before_has_clause = LexedClause::new(&before_has);
    let raw_before_has_words = before_has_clause.word_refs();
    let before_has_words = strip_leading_article_word_refs(&raw_before_has_words);
    let skipped_article_words = raw_before_has_words
        .len()
        .saturating_sub(before_has_words.len());
    let Some(creature_idx) =
        static_keyword_line_shapes::parse_animation_creature_word(before_has_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let granted_tail_tokens = &tokens[has_idx + 1..];
    let (base_power_toughness, subtype_start_word, granted_tail) = match before_has_words
        .first()
        .and_then(|word| crate::grammar::primitives::probe_shape(parse_pt_modifier(word)))
    {
        Some((power, toughness)) => {
            if creature_idx == 0 {
                return Ok(None);
            }
            let parsed_tail = parse_heterogeneous_granted_tail(
                granted_tail_tokens,
                &clause_words,
                attached_subject,
            )?;
            // Quotation marks are presentation delimiters around a granted
            // triggered ability, not part of its trigger grammar. A quoted
            // trigger at the end of a heterogeneous grant list can make the
            // segmenter retain the delimiters while the equivalent unquoted
            // trigger parses normally. Retry the same generic tail after
            // removing only quote tokens when a quote directly introduces a
            // trigger; activated abilities keep their quote-aware comma
            // grouping.
            let parsed_tail =
                if parsed_tail.is_none() && quoted_trigger_intro_present(granted_tail_tokens) {
                    let unquoted = granted_tail_tokens
                        .iter()
                        .filter(|token| token.kind != TokenKind::Quote)
                        .cloned()
                        .collect::<Vec<_>>();
                    parse_heterogeneous_granted_tail(&unquoted, &clause_words, attached_subject)?
                } else {
                    parsed_tail
                };
            let Some(granted_tail) = parsed_tail else {
                return Ok(None);
            };
            (
                (Value::Fixed(power), Value::Fixed(toughness)),
                1usize,
                granted_tail,
            )
        }
        None => {
            let Some((power, toughness)) =
                parse_static_base_power_toughness_value_tail(&tokens[has_idx + 1..])
            else {
                return Ok(None);
            };
            ((power, toughness), 0usize, ParsedGrantedTailAst::default())
        }
    };
    let subtype_words = &before_has_words[subtype_start_word..creature_idx];
    let mut subtypes = Vec::new();
    for word in subtype_words {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }
    let tail_start_word = skipped_article_words + creature_idx + 1;
    let mut tail_end_word = skipped_article_words + before_has_words.len();
    let tail_ends_with_and = before_has_words[creature_idx + 1..]
        .last()
        .is_some_and(|word| *word == "and");
    if tail_ends_with_and {
        tail_end_word = tail_end_word.saturating_sub(1);
    }
    if type_and_color_facts::parse_other_type_addition_tail_tokens(
        before_has_clause
            .between_word_range(tail_start_word, tail_end_word)
            .map(|tail_clause| tail_clause.tokens())
            .unwrap_or_default(),
    )
    .is_none()
    {
        return Ok(None);
    }

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some(base_power_toughness),
            granted_tail,
        },
    )))
}

/// Parse a persistent copular animation with no following granted ability,
/// such as “it's a 1/1 Insect creature in addition to its other types.”
/// Compound variants with an `and has ...` tail remain owned by
/// `parse_filter_is_pt_creature_in_addition_and_has_line` above.
pub fn parse_filter_is_pt_creature_in_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(be) = static_keyword_line_shapes::parse_animation_copula(tokens) else {
        return Ok(None);
    };
    let be_idx = be.token;
    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let contracted_source_subject = subject_start == be_idx
        && tokens.get(be_idx).is_some_and(|token| {
            token.is_word("it's") || token.is_word("it’s") || token.is_word("its")
        });
    let subject_tokens = if contracted_source_subject {
        tokens[be_idx..be_idx + 1].to_vec()
    } else {
        trim_commas(&tokens[subject_start..be_idx])
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = if contracted_source_subject {
        AnthemSubjectAst::Source
    } else {
        match parse_anthem_subject(&subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };

    let predicate_tokens = trim_edge_punctuation_tokens(&tokens[be_idx + 1..]);
    let predicate_clause = LexedClause::new(predicate_tokens);
    let raw_words = predicate_clause.word_refs();
    let words = strip_leading_article_word_refs(&raw_words);
    let skipped_article_words = raw_words.len().saturating_sub(words.len());
    let Some((power, toughness)) = words
        .first()
        .and_then(|word| crate::grammar::primitives::probe_shape(parse_pt_modifier(word)))
    else {
        return Ok(None);
    };
    let Some(creature_idx) = static_keyword_line_shapes::parse_animation_creature_word(words)
        .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    if creature_idx == 0 {
        return Ok(None);
    }

    let mut subtypes = Vec::new();
    for word in &words[1..creature_idx] {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let tail_start_word = skipped_article_words + creature_idx + 1;
    let tail_end_word = skipped_article_words + words.len();
    let Some(tail_clause) = predicate_clause.between_word_range(tail_start_word, tail_end_word)
    else {
        return Ok(None);
    };
    if type_and_color_facts::parse_other_type_addition_tail_tokens(tail_clause.tokens()).is_none() {
        return Ok(None);
    }

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some((Value::Fixed(power), Value::Fixed(toughness))),
            granted_tail: ParsedGrantedTailAst::default(),
        },
    )))
}

fn quoted_trigger_intro_present(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().enumerate().any(|(idx, token)| {
        if token.kind != TokenKind::Quote {
            return false;
        }
        matches!(
            tokens.get(idx + 1).and_then(OwnedLexToken::as_word),
            Some("when" | "whenever")
        ) || (tokens.get(idx + 1).and_then(OwnedLexToken::as_word) == Some("at")
            && tokens.get(idx + 2).and_then(OwnedLexToken::as_word) == Some("the"))
    })
}

pub fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens =
        if let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens) {
            if document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_some() {
                tokens
            } else {
                body_tokens
            }
        } else {
            tokens
        };
    let Some(grant_verbs) = static_keyword_line_shapes::parse_subtype_grant_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = grant_verbs.be.token;
    let with_idx = grant_verbs.with.token;

    let (_condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    let type_tokens = trim_commas(&tokens[be_idx + 1..with_idx]);
    if type_tokens.is_empty() {
        return Ok(None);
    }
    let type_words = LexedClause::new(&type_tokens).word_refs();
    let type_words = strip_leading_article_word_refs(&type_words);
    if type_words.is_empty() {
        return Ok(None);
    }
    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let mut after_with = trim_commas(&tokens[with_idx + 1..]).to_vec();
    if after_with.is_empty() {
        return Ok(None);
    }

    if let Some(loses) = type_and_color_facts::find_loses_other_creature_types_tokens(&after_with) {
        after_with.truncate(loses.marker_token);
    }

    let after_with = trim_edge_punctuation_tokens(&after_with);
    let Some(base) = type_and_color_facts::parse_base_power_toughness_grant_tokens(after_with)
    else {
        return Ok(None);
    };
    let after_with_words = parser_token_word_refs(after_with);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(base.ability_tokens, &after_with_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition: _condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::ReplaceCreatureTypes,
            base_power_toughness: Some((Value::Fixed(base.power), Value::Fixed(base.toughness))),
            granted_tail,
        },
    )))
}

pub fn parse_creatures_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if is_creatures_cant_block_line_lexed(tokens) {
        // This is a rule imposed by the source, not an ability granted to
        // every creature. Use the same typed restriction as the complete
        // generic reader, including its live source lifetime.
        let Some(abilities) = parse_cant_clauses(tokens)? else { return Ok(None); };
        let [ability]: [StaticAbility; 1] = abilities.try_into().map_err(|_| {
            CardTextError::ParseError("a complete creatures-cant-block clause must contain one restriction".into())
        })?;
        return Ok(Some(StaticAbilityAst::Static(ability)));
    }
    Ok(None)
}

pub fn parse_prevent_all_damage_dealt_to_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_dealt_to_creatures_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub fn parse_prevent_damage_to_other_creature_you_control_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::prevent_damage_to_other_creature_you_control_put_counters_instead(
            crate::object::CounterType::PlusOnePlusOne,
            display_text_for_tokens(tokens, true),
        ),
    ))
}

pub fn parse_damage_source_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut words = strip_leading_article_word_refs(words).to_vec();
    // "a source an opponent controls" / "a source you control"
    let controller = if words.ends_with(&["an", "opponent", "controls"]) {
        words.truncate(words.len() - 3);
        Some(PlayerFilter::Opponent)
    } else if words.ends_with(&["you", "control"]) {
        words.truncate(words.len() - 2);
        Some(PlayerFilter::You)
    } else {
        None
    };
    if word_slice_last_is_any(&words, &["source", "sources"]) {
        words.pop();
    }
    let mut filter = ObjectFilter::default();
    filter.controller = controller;
    if words.is_empty() {
        return Some(filter);
    }

    let mut colors: Option<ColorSet> = None;
    for word in words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = Some(colors.unwrap_or_default().union(color));
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            filter.card_types.push(card_type);
            continue;
        }
        return None;
    }
    if let Some(colors) = colors {
        filter.colors = Some(colors);
    }
    Some(filter)
}

pub fn parse_damage_source_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = LexedClause::new(tokens).word_refs();
    parse_damage_source_filter_words(&words)
}

pub fn parse_prevent_damage_to_you_from_source_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_prevent_damage_to_you_tokens(tokens) else {
        return Ok(None);
    };
    let Some(source_filter) = parse_damage_source_filter_tokens(spec.source_tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);

    Ok(Some(
        StaticAbility::prevent_damage_to_you_from_source_filter(
            spec.amount,
            source_filter,
            display,
        ),
    ))
}

/// Read a prevention event followed by its recipient's owner shuffling that recipient.
/// The event selectors and the additional effects remain independently executable.
pub fn parse_damage_prevention_with_owner_shuffle_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    if words.first() != Some(&"if") {
        return Ok(None);
    }
    let Some(would) = tokens.iter().position(|t| t.as_word() == Some("would")) else {
        return Ok(None);
    };
    let Some(comma) = tokens.iter().position(|t| t.kind == TokenKind::Comma) else {
        return Ok(None);
    };
    if would <= 1 || comma <= would {
        return Ok(None);
    }
    let source_tokens = &tokens[1..would];
    let source_words = crate::lexer::token_word_refs(source_tokens);
    let source_filter = if is_source_reference_words(&source_words) {
        let mut filter = ObjectFilter::source();
        filter.source_surface = source_reference_surface_for_words(&source_words);
        filter
    } else {
        let Ok(filter) = parse_object_filter_lexed(source_tokens, false) else {
            return Ok(None);
        };
        filter
    };
    let damage_words = crate::lexer::token_word_refs(&tokens[would..comma]);
    let (combat_only, prefix_len) =
        if damage_words.starts_with(&["would", "deal", "combat", "damage", "to"]) {
            (Some(true), 5)
        } else if damage_words.starts_with(&["would", "deal", "noncombat", "damage", "to"]) {
            (Some(false), 5)
        } else if damage_words.starts_with(&["would", "deal", "damage", "to"]) {
            (None, 4)
        } else {
            return Ok(None);
        };
    // This bounded head contains only words, so token and word offsets coincide.
    let target_tokens = &tokens[would + prefix_len..comma];
    let target_words = crate::lexer::token_word_refs(target_tokens);
    let Some(noun) = target_words.last().copied() else {
        return Ok(None);
    };
    if !matches!(noun, "creature" | "permanent" | "planeswalker" | "battle") {
        return Ok(None);
    }
    let tail = crate::lexer::token_word_refs(&tokens[comma + 1..]);
    let possessive = format!("{noun}'s");
    if tail
        != [
            "prevent",
            "that",
            "damage",
            "and",
            "that",
            possessive.as_str(),
            "owner",
            "shuffles",
            "it",
            "into",
            "their",
            "library",
        ]
    {
        return Ok(None);
    }
    let target_filter = parse_object_filter_lexed(target_tokens, false)?;
    let recipient = crate::tag::TagRef::of("__prevented_damage_recipient__");
    let shuffle = crate::cards::builders::EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::ItsOwner,
        SubjectVerbActionAst::Library(
            crate::cards::builders::LibraryActionAst::ShuffleObjectsIntoLibrary {
                target: TargetAst::Tagged(recipient.clone(), None),
                all: false,
                owner_library_destination: true,
                possessive_owner_subject: true,
                shuffle_subject_library: false,
            },
        ),
    );
    Ok(Some(StaticAbility::damage_prevention_with_follow_up(
        source_filter,
        target_filter,
        combat_only,
        recipient.key.clone(),
        vec![shuffle],
    )))
}

pub fn parse_replace_damage_with_counters_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !keyword_static_lines::parse_noncombat_damage_minus_counter_replacement_tokens(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::replace_damage_with_counters_instead(
        CounterType::MinusOneMinusOne,
        ObjectFilter::default().controlled_by(PlayerFilter::You),
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        Some(false),
        display_text_for_tokens(tokens, true),
    )))
}

pub fn parse_double_counters_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_counter_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::CounterReplacementShape::ActorScopedDouble { filter_tokens, counter_type } => {
            let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            let mut ability = StaticAbility::double_counters_replacement(
                filter, counter_type, display_text_for_tokens(tokens, true),
            );
            if let ironsmith_core::StaticAbilityPayload::DoubleCountersReplacement { actor, .. } = &mut ability.payload {
                *actor = Some(PlayerFilter::You);
            }
            ability
        }
        keyword_static_lines::CounterReplacementShape::ActorAnyKindAdd {
            filter_tokens,
            includes_player,
            additional,
        } => {
            let self_tail = filter_tokens.len() >= 3
                && filter_tokens[filter_tokens.len() - 3..]
                    .iter()
                    .zip(["or", "on", "yourself"])
                    .all(|(token, word)| token.is_word(word));
            if self_tail != includes_player {
                return Ok(None);
            }
            let object_tokens = if self_tail {
                &filter_tokens[..filter_tokens.len() - 3]
            } else {
                filter_tokens
            };
            let mut filter = parse_object_filter_lexed(object_tokens, false)?;
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            StaticAbility::actor_counters_addition_replacement(
                filter,
                includes_player.then_some(PlayerFilter::You),
                PlayerFilter::You,
                None,
                additional,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::AnyKindDouble { filter_tokens } => {
            let mut filter = parse_object_filter_lexed(filter_tokens, false)?;
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            StaticAbility::double_counters_replacement(
                filter,
                None,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::CounterAdjustment {
            filter_tokens,
            counter_type,
            adjustment,
        } => StaticAbility::add_counters_placement_replacement(
            parse_object_filter_lexed(filter_tokens, false)?,
            Some(counter_type),
            adjustment,
            display_text_for_tokens(tokens, true),
        ),
        keyword_static_lines::CounterReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_counters_replacement(
                ObjectFilter::permanent().controlled_by(PlayerFilter::You),
                None,
                display_text_for_tokens(tokens, true),
            )
            .effect_caused_counters_only()
        }
        keyword_static_lines::CounterReplacementShape::EnergyYouGet => {
            StaticAbility::double_player_counters_replacement(
                PlayerFilter::You,
                Some(CounterType::Energy),
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::EnergyYouGetPlus { additional } => {
            StaticAbility::add_player_counters_placement_replacement(
                PlayerFilter::You,
                Some(CounterType::Energy),
                additional,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::ActorAnyKindMultiply { opponent, halve } => {
            StaticAbility::actor_counter_multiplier_replacement(
                if opponent {
                    PlayerFilter::Opponent
                } else {
                    PlayerFilter::You
                },
                halve,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlusOneAdd {
            filter_tokens,
            additional,
            ability_kind,
        } => {
            let ability = StaticAbility::add_counters_placement_replacement(
                parse_object_filter_lexed(filter_tokens, false)?,
                Some(CounterType::PlusOnePlusOne),
                additional,
                display_text_for_tokens(tokens, true),
            );
            match ability_kind {
                Some(ability_kind) => ability.with_counter_placement_cause(
                    ironsmith_core::CauseFilter::any().with_ability_kind(ability_kind),
                ),
                None => ability,
            }
        }
        keyword_static_lines::CounterReplacementShape::AnyKindAdd {
            filter_tokens,
            additional,
        } => StaticAbility::add_counters_placement_replacement(
            parse_object_filter_lexed(filter_tokens, false)?,
            None,
            additional,
            display_text_for_tokens(tokens, true),
        ),
        keyword_static_lines::CounterReplacementShape::PlayerAnyKindAdd { additional } => {
            StaticAbility::add_player_counters_placement_replacement(
                PlayerFilter::You,
                None,
                additional,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlusOneDouble { filter_tokens } => {
            StaticAbility::double_counters_replacement(
                parse_object_filter_lexed(filter_tokens, false)?,
                Some(CounterType::PlusOnePlusOne),
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlayerCounterPerTurnLimit {
            counter_type,
            maximum,
        } => StaticAbility::player_counter_per_turn_limit_replacement(
            PlayerFilter::You,
            counter_type,
            maximum,
            display_text_for_tokens(tokens, true),
        ),
    }))
}

/// Token descriptor words ("creature", "Food") as a token filter; `None`
/// for an empty descriptor (every token), `Some(None)` never, and an outer
/// `None` when a word is neither a card type nor a subtype.
fn token_descriptor_filter(descriptor_tokens: &[OwnedLexToken]) -> Option<Option<ObjectFilter>> {
    let words = parser_token_word_refs(descriptor_tokens);
    if words.is_empty() {
        return Some(None);
    }
    let mut token_filter = ObjectFilter::default().token();
    for word in words {
        if let Some(card_type) = parse_card_type(word) {
            token_filter = token_filter.with_type(card_type);
        } else if let Some(subtype) = parse_subtype_flexible(word) {
            token_filter = token_filter.with_subtype(subtype);
        } else {
            return None;
        }
    }
    Some(Some(token_filter))
}

/// Reuse the full token-definition grammar rather than naming a finite set of
/// additional token kinds. Existing complete specialized readings keep ownership.
pub fn parse_token_creation_templates_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if parse_double_token_creation_replacement_line(tokens)?.is_some() {
        return Ok(None);
    }
    let Some(shape) = keyword_static_lines::parse_token_template_replacement(tokens) else {
        return Ok(None);
    };
    let Some(filter) = token_descriptor_filter(shape.source_descriptor) else {
        return Ok(None);
    };
    let token_filter = filter.unwrap_or_else(|| ObjectFilter::default().token());
    let mut templates = Vec::new();
    for descriptor in shape.templates {
        // "tokens that are copies of enchanted permanent": one copy per
        // replaced token (CR 707.2).
        let descriptor_words = parser_token_word_refs(descriptor);
        let copy_of = match descriptor_words.as_slice() {
            ["tokens", "that", "are", "copies", "of", ..] => Some(5),
            ["token", "thats", "a", "copy", "of", ..] | ["token", "that's", "a", "copy", "of", ..] => Some(5),
            _ => None,
        };
        let mut recipe = if let Some(prefix_words) = copy_of {
            let start = crate::lexer::TokenWordView::new(descriptor)
                .token_span_for_words(0, prefix_words)
                .map(|span| span.end)
                .ok_or_else(|| CardTextError::ParseError("token copy template".to_string()))?;
            let mut recipe = crate::lexer::lex_line("create a token that's a copy of", 0)?;
            // "choose a creature other than Esix and create that many tokens
            // that are copies of that creature": the copied permanent is the
            // chosen one, a choice among the permanents the description
            // matches as the replacement applies.
            if let Some(chosen) = shape.chosen_copy_source {
                if !matches!(
                    parser_token_word_refs(&descriptor[start..]).as_slice(),
                    ["that", "creature" | "permanent" | "artifact" | "land" | "enchantment"]
                        | ["it"]
                ) {
                    return Ok(None);
                }
                recipe.extend_from_slice(chosen);
                templates.extend(crate::clause_support::parse_effect_sentences_lexed(&recipe)?);
                continue;
            }
            recipe.extend_from_slice(&descriptor[start..]);
            templates.extend(crate::clause_support::parse_effect_sentences_lexed(&recipe)?);
            continue;
        } else {
            vec![
                OwnedLexToken::word("create", TextSpan::synthetic()),
                OwnedLexToken::word("one", TextSpan::synthetic()),
            ]
        };
        recipe.extend_from_slice(descriptor);
        templates.extend(crate::clause_support::parse_effect_sentences_lexed(
            &recipe,
        )?);
    }
    if shape.chosen_copy_source.is_some() && templates.is_empty() {
        return Ok(None);
    }
    let ability = StaticAbilityAst::TokenCreationTemplates {
        controller: PlayerFilter::You,
        token_filter,
        templates,
        mode: shape.mode,
        choose_one: shape.choose_one,
        optional: shape.optional,
        display: display_text_for_tokens(tokens, true),
    };
    if !shape.first_time_each_turn {
        return Ok(Some(ability));
    }
    // CR 614.1: only the first token creation event of the turn qualifies,
    // so no token has been created under your control this turn yet.
    let first_time = crate::cards::builders::PredicateAst::ValueComparison {
        left: crate::effect::Value::TurnHistoryCount(
            ironsmith_core::TurnHistoryCount::TokensCreated(PlayerFilter::You),
        ),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: crate::effect::Value::Fixed(0),
    };
    // "... during each of your turns": and only on your own turns.
    let condition = if shape.first_time_each_of_your_turns {
        crate::cards::builders::PredicateAst::And(
            Box::new(first_time),
            Box::new(crate::cards::builders::PredicateAst::YourTurn),
        )
    } else {
        first_time
    };
    Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
        ability: Box::new(ability),
        condition,
    }))
}

pub fn parse_double_token_creation_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_token_creation_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::TokenCreationReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_token_creation_replacement(
                PlayerFilter::You,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::TokenCreationReplacementShape::GenericMultiplied {
            descriptor_tokens,
            under_your_control,
            factor,
        } => {
            let controller = if under_your_control {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            let Some(token_filter) = token_descriptor_filter(descriptor_tokens) else {
                return Ok(None);
            };
            if factor == 2 && token_filter.is_none() {
                StaticAbility::double_token_creation_replacement(
                    controller,
                    display_text_for_tokens(tokens, true),
                )
            } else {
                StaticAbility::multiply_token_creation_replacement(
                    controller,
                    token_filter,
                    factor,
                    display_text_for_tokens(tokens, true),
                )
            }
        }
        keyword_static_lines::TokenCreationReplacementShape::OneOfEach { kind_tokens } => {
            let mut kinds = Vec::new();
            for word in parser_token_word_refs(kind_tokens) {
                match word {
                    "clue" => kinds.push(ironsmith_core::AdditionalTokenKind::Clue),
                    "food" => kinds.push(ironsmith_core::AdditionalTokenKind::Food),
                    "treasure" => kinds.push(ironsmith_core::AdditionalTokenKind::Treasure),
                    "or" | "and" => {}
                    _ => return Ok(None),
                }
            }
            if kinds.len() < 2 {
                return Ok(None);
            }
            StaticAbility::create_one_of_each_token_replacement(
                kinds,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::TokenCreationReplacementShape::AddNamedToken {
            descriptor_tokens,
            additional_kind_word,
            per_created,
        } => {
            let Some(token_filter) = token_descriptor_filter(descriptor_tokens) else {
                return Ok(None);
            };
            let additional_token = match additional_kind_word {
                "food" => ironsmith_core::AdditionalTokenKind::Food,
                "treasure" => ironsmith_core::AdditionalTokenKind::Treasure,
                "squirrel" => ironsmith_core::AdditionalTokenKind::Squirrel,
                _ => return Ok(None),
            };
            let token_filter = token_filter.unwrap_or_else(|| ObjectFilter::default().token());
            if per_created {
                StaticAbility::add_token_per_created_replacement(
                    PlayerFilter::You,
                    token_filter,
                    additional_token,
                    display_text_for_tokens(tokens, true),
                )
            } else {
                StaticAbility::add_token_creation_replacement(
                    PlayerFilter::You,
                    token_filter,
                    additional_token,
                    1,
                    display_text_for_tokens(tokens, true),
                )
            }
        }
        keyword_static_lines::TokenCreationReplacementShape::AddTreasure { descriptor_tokens } => {
            let mut token_filter = ObjectFilter::default().token();
            for word in parser_token_word_refs(descriptor_tokens) {
                if let Some(card_type) = parse_card_type(word) {
                    token_filter = token_filter.with_type(card_type);
                } else if let Some(subtype) = parse_subtype_flexible(word) {
                    token_filter = token_filter.with_subtype(subtype);
                } else {
                    return Ok(None);
                }
            }
            StaticAbility::add_token_creation_replacement(
                PlayerFilter::You,
                token_filter,
                ironsmith_core::AdditionalTokenKind::Treasure,
                1,
                display_text_for_tokens(tokens, true),
            )
        }
    }))
}

pub fn parse_prevent_all_combat_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_combat_damage_to_source_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_combat_damage_to_self()));
    }

    Ok(None)
}

/// "Prevent all damage that would be dealt to attacking creatures you
/// control." (Iroas, God of Victory): a filtered prevention of combat and
/// noncombat damage alike. Self, player, bare-creature, and attached-object
/// subjects keep their dedicated rules.
pub fn parse_prevent_all_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // "If damage would be dealt to this creature, prevent that damage."
    // (Phyrexian Vindicator) is the replacement spelling of the same rule.
    if let Some(head) = crate::grammar::keyword_static_lines::parse_passive_damage_prevention_head(tokens)
        && !head.combat_only
        && !head.noncombat_only
        && matches!(
            parser_token_word_refs(head.damaged_tokens).as_slice(),
            ["this", "creature" | "permanent" | "artifact" | "enchantment" | "land" | "planeswalker" | "battle" | "token"]
        )
    {
        return Ok(Some(StaticAbility::prevent_all_damage_to_self()));
    }
    let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&[
            "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
        ]),
    ) else {
        return Ok(None);
    };
    let target_tokens = trim_edge_punctuation(rest);
    let words = parser_token_word_refs(&target_tokens);
    if matches!(
        words.as_slice(),
        [
            "this",
            "creature"
                | "permanent"
                | "artifact"
                | "enchantment"
                | "land"
                | "planeswalker"
                | "battle"
                | "token"
        ]
    ) {
        return Ok(Some(StaticAbility::prevent_all_damage_to_self()));
    }
    if words.len() < 2
        || matches!(
            words[0],
            "you" | "this" | "it" | "that" | "enchanted" | "equipped" | "each" | "any"
        )
        || words
            .iter()
            .any(|word| matches!(*word, "by" | "and" | "or"))
        // "... to target creature this turn" is a one-shot prevention shield
        // (Indestructible Aura), never a static over every creature.
        || words
            .iter()
            .any(|word| matches!(*word, "target" | "turn" | "until"))
    {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    if filter.zone != Some(Zone::Battlefield) {
        return Ok(None);
    }
    Ok(Some(
        StaticAbility::prevent_all_damage_to_permanents_matching(filter),
    ))
}

pub fn parse_prevent_all_combat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_all_combat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }
    let Some(prevention) =
        static_keyword_replacement_shapes::parse_combat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "prevent-all combat damage static line missing target filter (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    // "... dealt to and dealt by enchanted creature": the permanent is both
    // the recipient and the source of the prevented damage.
    if target_tokens.len() > 3
        && target_tokens[0].is_word("and")
        && target_tokens[1].is_word("dealt")
        && target_tokens[2].is_word("by")
    {
        let filter = parse_object_filter_lexed(&trim_commas(&target_tokens[3..]), false)?;
        return Ok(Some(
            StaticAbility::prevent_all_combat_damage_to_and_by_permanents_matching(filter),
        ));
    }
    if let Some(by_idx) =
        crate::slice_primitives::select_position(target_tokens.as_slice(), |token| {
            token.is_word("by")
        })
    {
        let source_tokens = trim_commas(&target_tokens[by_idx + 1..]);
        let word_positions = source_tokens
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| token.as_word().map(|word| (idx, word)))
            .collect::<Vec<_>>();
        let (source_end, source_relation) = if word_positions.len() >= 2
            && word_positions[word_positions.len() - 2].1 == "blocking"
            && word_positions[word_positions.len() - 1].1 == "it"
        {
            (
                word_positions[word_positions.len() - 2].0,
                ironsmith_core::StaticDamageSourceRelation::BlockingStaticSource,
            )
        } else {
            (
                source_tokens.len(),
                ironsmith_core::StaticDamageSourceRelation::Any,
            )
        };
        let source_filter_tokens = trim_commas(&source_tokens[..source_end]);
        if source_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "prevent-all combat damage static line missing source filter (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        let source_filter = parse_object_filter_lexed(&source_filter_tokens, false)?;
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_from_sources_matching(
                ironsmith_core::PreventAllDamageToSelfFromSourcesMatchingSpec {
                    source_filter,
                    combat_only: true,
                    source_relation,
                    display: display_text_for_tokens(tokens, true),
                },
            ),
        ));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_combat_damage_to_permanents_matching(filter),
    ))
}

pub fn parse_during_your_turn_prevent_all_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_during_your_turn_prevent_all_damage_to_source_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self().with_condition(
                PredicateAst::ActivationTiming(crate::ability::ActivationTiming::DuringYourTurn),
            ),
        ));
    }

    Ok(None)
}

pub fn parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_noncombat_damage_to_other_creatures_you_control(),
        ));
    }

    Ok(None)
}

pub fn parse_prevent_all_noncombat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        // The other-creatures-you-control production owns its complete line.
        return Ok(None);
    }
    if !is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }

    let Some(prevention) =
        static_keyword_replacement_shapes::parse_noncombat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all noncombat damage target filter: {}",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_noncombat_damage_to_permanents_matching(filter),
    ))
}

pub fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_to_source_by_creatures_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_by_creatures(),
        ));
    }
    Ok(None)
}

pub fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_may_choose_not_untap_tokens(tokens) else {
        return Ok(None);
    };
    let subject_words = parser_token_word_refs(fact.subject_tokens);
    if !fact.simple_source_subject && !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let subject = render_token_slice(fact.subject_tokens);
    let subject = source_reference_surface_for_words(&subject_words)
        .map(|surface| surface.display_text())
        .unwrap_or(subject);
    Ok(Some(
        StaticAbility::may_choose_not_to_untap_during_untap_step(subject),
    ))
}

pub fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in other-players untap ability (clause: '{}')",
            render_token_slice(tokens)
        )));
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = render_token_slice(&subject_tokens);
    Ok(Some(if spec.opponents_only {
        StaticAbility::untap_during_each_opponents_untap_step(
            filter,
            format!("Untap all {subject_text} during each opponent's untap step"),
        )
    } else {
        StaticAbility::untap_during_each_other_players_untap_step(
            filter,
            format!("Untap all {subject_text} during each other player's untap step"),
        )
    }))
}

/// "Prevent all damage that would be dealt to you." (Solitary Confinement)
pub fn parse_prevent_all_damage_to_you_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    if words.as_slice()
        != [
            "prevent", "all", "damage", "that", "would", "be", "dealt", "to", "you",
        ]
    {
        return Ok(None);
    }
    Ok(Some(StaticAbility::prevent_all_damage_to_you()))
}

/// Flagbearer: "While an opponent is choosing targets as part of casting a
/// spell they control or activating an ability they control, that player must
/// choose at least one Flagbearer on the battlefield if able."
pub fn parse_opponents_must_target_flagbearers_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(trim_edge_punctuation_tokens(tokens));
    let expected = [
        "while",
        "an",
        "opponent",
        "is",
        "choosing",
        "targets",
        "as",
        "part",
        "of",
        "casting",
        "a",
        "spell",
        "they",
        "control",
        "or",
        "activating",
        "an",
        "ability",
        "they",
        "control",
        "that",
        "player",
        "must",
        "choose",
        "at",
        "least",
        "one",
        "flagbearer",
        "on",
        "the",
        "battlefield",
        "if",
        "able",
    ];
    if words.as_slice() != expected {
        return Ok(None);
    }
    Ok(Some(StaticAbility::opponents_must_target_flagbearers()))
}

/// "The attacking player chooses how each creature blocks each combat."
/// (Invasion Plans): the attacking player makes every block declaration.
pub fn parse_attacking_player_chooses_blocks_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(trim_edge_punctuation_tokens(tokens));
    if !matches!(
        words.as_slice(),
        [
            "the", "attacking", "player", "chooses", "how", "each", "creature", "blocks", "each",
            "combat"
        ] | [
            "the", "attacking", "player", "chooses", "how", "each", "creature", "blocks"
        ]
    ) {
        return Ok(None);
    }
    Ok(Some(StaticAbility::attacking_player_chooses_blocks(
        "The attacking player chooses how each creature blocks each combat",
    )))
}

/// "If an opponent would search a library, that player searches the top four
/// cards of that library instead." (Aven Mindcensor)
pub fn parse_search_limited_to_top_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((subject, rest)) = crate::word_primitives::strip_any_prefix(
        &words,
        &[&["if", "an", "opponent"], &["if", "a", "player"]],
    ) else {
        return Ok(None);
    };
    let Some(rest) = rest.strip_prefix(
        &[
            "would", "search", "a", "library", "that", "player", "searches", "the", "top",
        ][..],
    ) else {
        return Ok(None);
    };
    let Some((count_word, rest)) = rest.split_first() else {
        return Ok(None);
    };
    let Some(count) = crate::util::parse_number_word_u32(count_word) else {
        return Ok(None);
    };
    if rest != ["cards", "of", "that", "library", "instead"] {
        return Ok(None);
    }
    let (searcher, subject_text) = if subject == ["if", "an", "opponent"] {
        (PlayerFilter::Opponent, "an opponent")
    } else {
        (PlayerFilter::Any, "a player")
    };
    Ok(Some(StaticAbility::search_limited_to_top_cards(
        searcher,
        count,
        format!(
            "If {subject_text} would search a library, that player searches the top {count_word} cards of that library instead"
        ),
    )))
}

/// "Players can't untap more than one land during their untap steps."
/// (Winter Orb), "... more than one nonbasic land ..." (Winter Moon), "You
/// can't untap more than two permanents during your untap step."
/// "You can't choose an untapped creature as this spell's target as you cast
/// it." (Enthralling Hold, Treachery-style Auras): a restriction on this
/// spell's own target announcement (CR 601.2c) that the resolution recheck
/// (CR 608.2b) does not read.
pub fn parse_cast_target_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&trimmed);
    if crate::word_primitives::strip_any_prefix(
        &words,
        &[&["you", "cant", "choose"], &["you", "can't", "choose"], &["you", "cannot", "choose"]],
    )
    .is_none()
    {
        return Ok(None);
    }
    let Some(choose_idx) = trimmed.iter().position(|token| token.is_word("choose")) else {
        return Ok(None);
    };
    let Some(as_offset) = trimmed[choose_idx + 1..]
        .iter()
        .position(|token| token.is_word("as"))
    else {
        return Ok(None);
    };
    let as_idx = choose_idx + 1 + as_offset;
    let tail_words = parser_token_word_refs(&trimmed[as_idx..]);
    let tail_ok = matches!(
        tail_words.as_slice(),
        [
            "as",
            "this",
            "spell's" | "spells" | "spell’s",
            "target",
            "as",
            "you",
            "cast",
            "it"
        ]
    );
    let filter_tokens = &trimmed[choose_idx + 1..as_idx];
    if !tail_ok || filter_tokens.is_empty() {
        return Ok(None);
    }
    let forbidden = parse_object_filter_lexed(filter_tokens, false)?;
    Ok(Some(StaticAbility::cast_target_restriction(
        forbidden,
        crate::lexer::render_token_slice(&trimmed),
    )))
}

/// "Spells cost an additional \"Sacrifice a Swamp\" to cast for each black
/// mana symbol in their mana costs." / "Activated abilities cost an
/// additional \"Sacrifice a Swamp\" to activate for each black mana symbol
/// in their activation costs." (Drought)
pub fn parse_additional_sacrifice_per_mana_symbol_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    // Quote marks around the cost are punctuation here.
    let word_tokens: Vec<OwnedLexToken> = trimmed
        .iter()
        .filter(|token| token.as_word().is_some())
        .cloned()
        .collect();
    let words = crate::lexer::token_word_refs(&word_tokens);
    let (activated, rest) = match words.as_slice() {
        ["spells", "cost", "an", "additional", "sacrifice", rest @ ..] => (false, rest),
        ["activated", "abilities", "cost", "an", "additional", "sacrifice", rest @ ..] => {
            (true, rest)
        }
        _ => return Ok(None),
    };
    let Some(to) = rest.iter().position(|word| *word == "to") else {
        return Ok(None);
    };
    let (verb, color, cost_kind) = match &rest[to..] {
        ["to", verb, "for", "each", color, "mana", "symbol", "in", "their", kind, "costs"] => {
            (*verb, *color, *kind)
        }
        _ => return Ok(None),
    };
    let expected = if activated {
        ("activate", "activation")
    } else {
        ("cast", "mana")
    };
    if (verb, cost_kind) != expected {
        return Ok(None);
    }
    let symbol = match color {
        "white" => crate::mana::ManaSymbol::White,
        "blue" => crate::mana::ManaSymbol::Blue,
        "black" => crate::mana::ManaSymbol::Black,
        "red" => crate::mana::ManaSymbol::Red,
        "green" => crate::mana::ManaSymbol::Green,
        _ => return Ok(None),
    };
    let prefix = if activated { 6 } else { 5 };
    let filter_tokens = &word_tokens[prefix..prefix + to];
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut sacrifice = parse_object_filter_lexed(filter_tokens, false)?;
    sacrifice.zone = Some(crate::zone::Zone::Battlefield);
    sacrifice.controller = Some(PlayerFilter::You);
    Ok(Some(StaticAbility::additional_sacrifice_per_mana_symbol(
        activated,
        symbol,
        sacrifice,
        crate::lexer::render_token_slice(&trimmed),
    )))
}

pub fn parse_untap_step_limit_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let Some((subject, rest)) = crate::word_primitives::strip_any_prefix(
        &words,
        &[&["players"], &["each", "player"], &["you"], &["your", "opponents"], &["each", "opponent"]],
    ) else {
        return Ok(None);
    };
    let player = if subject == ["you"] {
        PlayerFilter::You
    } else if subject.contains(&"opponents") || subject.contains(&"opponent") {
        // "Your opponents can't untap more than two permanents during their
        // untap steps." (Dovin Baan's emblem)
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    };
    let Some(rest) =
        crate::word_primitives::strip_any_prefix(rest, &[&["cant"], &["can't"], &["cannot"]])
            .map(|(_, rest)| rest)
    else {
        return Ok(None);
    };
    let Some(rest) = rest.strip_prefix(&["untap", "more", "than"][..]) else {
        return Ok(None);
    };
    let Some((count_word, rest)) = rest.split_first() else {
        return Ok(None);
    };
    let Some(max) = crate::util::parse_number_word_u32(count_word) else {
        return Ok(None);
    };
    let suffixes: &[&[&str]] = if player == PlayerFilter::You {
        &[&["during", "your", "untap", "step"]]
    } else {
        &[
            &["during", "their", "untap", "steps"],
            &["during", "their", "untap", "step"],
        ]
    };
    let Some((suffix, filter_words)) = crate::word_primitives::strip_any_suffix(rest, suffixes)
    else {
        return Ok(None);
    };
    if filter_words.is_empty() {
        return Ok(None);
    }
    let view = TokenWordView::new(tokens);
    let filter_start_word = words.len() - rest.len();
    let filter_end_word = filter_start_word + filter_words.len();
    let (Some(filter_start), Some(filter_end)) = (
        view.map_word_to_token_start(filter_start_word),
        view.map_word_to_token_start(filter_end_word),
    ) else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&tokens[filter_start..filter_end]);
    let filter = parse_object_filter(&filter_tokens, false)?;
    let subject_text = match player {
        PlayerFilter::You => "You",
        PlayerFilter::Opponent => "Your opponents",
        _ => "Players",
    };
    Ok(Some(StaticAbility::untap_step_limit(
        player,
        filter,
        max,
        format!(
            "{subject_text} can't untap more than {count_word} {} {}",
            render_token_slice(&filter_tokens),
            suffix.join(" ")
        ),
    )))
}

pub fn parse_attacked_during_controllers_last_turn(
    tokens: &[OwnedLexToken],
    filter: ObjectFilter,
) -> Option<PredicateAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let matches = [
        &["it", "attacked", "during", "your", "last", "turn"][..],
        &[
            "it",
            "attacked",
            "during",
            "its",
            "controllers",
            "last",
            "turn",
        ],
        &[
            "it",
            "attacked",
            "during",
            "its",
            "controller's",
            "last",
            "turn",
        ],
        &[
            "it",
            "attacked",
            "during",
            "its",
            "controller",
            "last",
            "turn",
        ],
        &[
            "it",
            "attacked",
            "during",
            "its",
            "controller",
            "s",
            "last",
            "turn",
        ],
    ]
    .iter()
    .any(|phrase| crate::word_primitives::parse_sequence_complete(&words, phrase));
    matches.then(|| {
        PredicateAst::Bound(Box::new(crate::ConditionExpr::TurnHistory(
            ironsmith_core::TurnHistoryCondition::ObjectAttackedDuringControllersLastTurn(filter),
        )))
    })
}

/// A shared attached subject can impose both untap and activation restrictions.
pub fn parse_attached_untap_and_activation_restrictions_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(DoesntUntapDuringUntapStepSpec::Attached { subject_tokens, tail_tokens, .. }) =
        parse_doesnt_untap_during_untap_step_spec_lexed(tokens)
    else { return Ok(None); };
    let beam_prohibition = match crate::lexer::parser_token_word_refs(tail_tokens).as_slice() {
        ["and", "its", "activated", "abilities", "cant", "be", "activated"] => false,
        ["and", "cant" | "can't", "be", participle @ ..]
            if crate::keyword_forms::words_spell(participle, crate::keyword_forms::BEAMED_UP) => true,
        _ => return Ok(None),
    };
    let Some(and_index) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    let Some(untap) = parse_doesnt_untap_during_untap_step_line(&tokens[..and_index])? else {
        return Ok(None);
    };
    let tag = if subject_tokens.first().is_some_and(|token| token.is_word("equipped")) {
        crate::tag::CompilerReferenceTag::Equipped
    } else {
        crate::tag::CompilerReferenceTag::Enchanted
    };
    let filter = ObjectFilter::permanent_card().in_zone(Zone::Battlefield).match_tagged(
        tag.bind(), crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    );
    let (restriction, text) = if beam_prohibition {
        (crate::effect::Restriction::ObjectRule {
            objects: filter,
            rule: ironsmith_core::ObjectRule::CantBeBeamedUp,
        }, format!("{} can't be beamed up", render_token_slice(subject_tokens)))
    } else {
        (crate::effect::Restriction::activate_abilities_of(filter),
            format!("{} activated abilities can't be activated", render_token_slice(subject_tokens)))
    };
    let activation = StaticAbilityAst::Static(StaticAbility::restriction(restriction, text));
    Ok(Some(vec![untap, activation]))
}

pub fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    match parse_doesnt_untap_during_untap_step_spec_lexed(tokens) {
        Some(DoesntUntapDuringUntapStepSpec::Source { tail_tokens }) => {
            let clause_display = render_token_slice(tokens);
            let tail_tokens = trim_edge_punctuation(&trim_commas(tail_tokens));
            if tail_tokens.is_empty() {
                return Ok(Some(
                    StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
                ));
            }
            if tail_tokens.first().is_some_and(|token| token.is_word("if")) {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step if-clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition = match parse_attacked_during_controllers_last_turn(
                    &condition_tokens,
                    ObjectFilter::source(),
                ) {
                    Some(condition) => condition,
                    None => parse_static_condition_clause(&condition_tokens)?,
                };
                return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                    condition,
                }));
            }
            // "Bombur doesn't untap during your untap step unless you have an
            // enduring story": the restriction applies while the condition
            // is false.
            if tail_tokens.first().is_some_and(|token| token.is_word("unless")) {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step unless-clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition = parse_static_condition_clause(&condition_tokens)?;
                return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                    condition: PredicateAst::Not(Box::new(condition)),
                }));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported trailing untap-step clause (clause: '{}')",
                clause_display
            )))
        }
        Some(DoesntUntapDuringUntapStepSpec::Attached {
            subject_tokens,
            tail_tokens,
            during_your_untap_step,
        }) => {
            if matches!(crate::lexer::parser_token_word_refs(tail_tokens).as_slice(),
                ["and", "its", "activated", "abilities", "cant", "be", "activated"]
                | ["and", "cant" | "can't", "be", "beamed", "up"])
            {
                // The complete compound reader owns both restrictions.
                return Ok(None);
            }
            let subject = render_token_slice(subject_tokens);
            let step_owner = if during_your_untap_step {
                "your"
            } else {
                "its controller's"
            };
            let mut text = format!("{subject} doesn't untap during {step_owner} untap step");
            // The authored trailing gate ("if this Aura has a net counter on
            // it", "unless that player is the monarch") stays part of the
            // rule's own sentence; the typed condition below gates it.
            if !tail_tokens.is_empty() {
                text = format!("{text} {}", render_token_slice(&trim_edge_punctuation(tail_tokens)));
            }
            let condition = if tail_tokens.is_empty() {
                None
            } else {
                let clause_display = render_token_slice(tokens);
                if tail_tokens
                    .first()
                    .is_none_or(|token| !token.is_any_word(&["unless", "if"]))
                {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing attached untap-step clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after attached untap-step unless-clause (clause: '{}')",
                        clause_display
                    )));
                }
                let tag = if subject_tokens
                    .first()
                    .is_some_and(|token| token.is_word("equipped"))
                {
                    crate::tag::CompilerReferenceTag::Equipped
                } else {
                    crate::tag::CompilerReferenceTag::Enchanted
                };
                let mut predicate = match parse_attacked_during_controllers_last_turn(
                    &condition_tokens,
                    ObjectFilter::tagged(tag.bind()),
                ) {
                    Some(predicate) => predicate,
                    None => parse_static_condition_clause(&condition_tokens)?,
                };
                // A possessive pronoun in the attached subject's condition
                // refers to that permanent, not to the Aura or Equipment.
                if condition_tokens
                    .first()
                    .is_some_and(|token| token.is_word("it"))
                    && let PredicateAst::CountComparison {
                        count:
                            crate::static_abilities::AnthemCountExpression::CountersOnSource(counter),
                        comparison: crate::effect::Comparison::GreaterThanOrEqual(minimum),
                        ..
                    } = &predicate
                {
                    predicate = PredicateAst::ValueComparison {
                        left: Value::CountersOn(
                            Box::new(crate::target::ChooseSpec::Tagged(tag.bind().into())),
                            Some(*counter),
                        ),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(*minimum),
                    };
                }
                Some(if tail_tokens[0].is_word("unless") {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                })
            };
            // DoesntUntap applies during the affected permanent's natural untap.
            // For an Aura's "your" step, grant it only on the Aura controller's turn.
            let condition = if during_your_untap_step {
                Some(match condition {
                    Some(condition) => {
                        PredicateAst::And(Box::new(condition), Box::new(PredicateAst::YourTurn))
                    }
                    None => PredicateAst::YourTurn,
                })
            } else {
                condition
            };
            // "Enchanted creature doesn't untap ..." is an effect of the Aura
            // or Equipment on the permanent it's attached to, not an ability
            // that permanent gains: removing the permanent's abilities can't
            // end it (CR 303.4, 613.1).
            let tag = if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("equipped"))
            {
                crate::tag::CompilerReferenceTag::Equipped
            } else {
                crate::tag::CompilerReferenceTag::Enchanted
            };
            let attached = ObjectFilter::permanent_card().in_zone(Zone::Battlefield).match_tagged(
                tag.bind(),
                crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            );
            let restriction = StaticAbilityAst::Static(StaticAbility::restriction_with_display(
                crate::effect::Restriction::untap(attached),
                text,
            ));
            Ok(Some(match condition {
                Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(restriction),
                    condition,
                },
                None => restriction,
            }))
        }
        None => Ok(None),
    }
}

pub fn parse_flying_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(match parse_flying_block_restriction_line_lexed(tokens) {
        Some(FlyingBlockRestrictionKind::FlyingOnly) => {
            Some(StaticAbility::flying_only_restriction())
        }
        Some(FlyingBlockRestrictionKind::FlyingOrReach) => {
            Some(StaticAbility::flying_restriction())
        }
        None => None,
    })
}

pub fn parse_can_block_only_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_block_only_flying_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub fn parse_can_block_subtype_as_though_reach_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(parse_can_block_subtype_as_though_reach_line_lexed(tokens)
        .map(StaticAbility::can_block_subtype_as_though_reach))
}

pub fn parse_assign_damage_as_unblocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_may_assign_damage_as_unblocked_line_lexed(tokens) {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }
    if crate::grammar::abilities::is_unblocked_may_assign_damage_to_defending_creature_line_lexed(
        tokens,
    ) {
        return Ok(Some(
            StaticAbility::may_assign_unblocked_damage_to_defending_creature(),
        ));
    }
    if crate::grammar::abilities::is_may_divide_combat_damage_among_defending_player_and_creatures_line_lexed(
        tokens,
    ) {
        return Ok(Some(
            StaticAbility::may_divide_combat_damage_among_defending_player_and_creatures(),
        ));
    }

    Ok(None)
}

/// "<play permission> by <cost> in addition to paying their other costs."
/// (Dawnhand Dissident: "During your turn, you may cast creature spells from
/// among cards you own exiled with this creature by removing three counters
/// from among creatures you control ..."; Into the Pit, Falco Spara: "... from
/// the top of your library by ..."): the ordinary play permission, each cast
/// through it paying the card's mana cost plus the stated cost. Graveyard
/// permissions keep their own reader.
pub fn parse_play_permission_with_additional_cast_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    const SUFFIX: &[&str] = &["in", "addition", "to", "paying", "their", "other", "costs"];
    let tokens = trim_edge_punctuation(tokens);
    let positions = crate::lexer::parser_token_word_positions(&tokens);
    let words = positions.iter().map(|(_, word)| *word).collect::<Vec<_>>();
    if !words.ends_with(SUFFIX)
        || !words.iter().any(|word| *word == "cast")
        || words.iter().any(|word| *word == "graveyard")
    {
        return Ok(None);
    }
    let word_positions = positions.iter().map(|(index, _)| *index).collect::<Vec<_>>();
    let suffix_start = words.len() - SUFFIX.len();
    let Some(by) = words[..suffix_start].iter().rposition(|word| *word == "by") else {
        return Ok(None);
    };
    if by + 1 >= suffix_start {
        return Ok(None);
    }
    let cast = words.iter().position(|word| *word == "cast").unwrap_or(0);
    if cast >= by || words.get(cast + 1) == Some(&"this") {
        return Ok(None);
    }
    let permission_tokens = &tokens[..word_positions[by]];
    let cost_tokens = &tokens[word_positions[by + 1]..word_positions[suffix_start]];
    let Some(costs) = crate::permission_helpers::parse_graveyard_cast_additional_cost_tokens(
        trim_lexed_commas(cost_tokens),
    )?
    else {
        return Ok(None);
    };
    let Some(mut abilities) = parse_static_ability_ast_line_lexed(permission_tokens)? else {
        return Ok(None);
    };

    fn add_costs(
        ability: &mut StaticAbility,
        costs: &[crate::model::CompilerCost],
    ) -> bool {
        match &mut ability.payload {
            ironsmith_core::StaticAbilityPayload::Grants(spec)
                if matches!(spec.grantable, ironsmith_core::Grantable::PlayFrom)
                    && spec.zone != Zone::Graveyard =>
            {
                spec.grantable = ironsmith_core::Grantable::DerivedAlternativeCast(
                    ironsmith_core::DerivedAlternativeCast::ZoneCastFromCardManaCost {
                        zone: spec.zone,
                        additional_costs: costs.to_vec(),
                    },
                );
                true
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                add_costs(ability, costs)
            }
            _ => false,
        }
    }
    fn add_ast_costs(ability: &mut StaticAbilityAst, costs: &[crate::model::CompilerCost]) -> bool {
        match ability {
            StaticAbilityAst::Static(ability) => add_costs(ability, costs),
            StaticAbilityAst::ConditionalStaticAbility { ability, .. }
            | StaticAbilityAst::LabeledConditionalStaticAbility { ability, .. } => {
                add_ast_costs(ability, costs)
            }
            _ => false,
        }
    }
    let mut converted = 0;
    for ability in &mut abilities {
        if add_ast_costs(ability, &costs) {
            converted += 1;
        }
    }
    if converted != 1 {
        return Ok(None);
    }
    Ok(Some(abilities))
}

pub fn parse_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::mana_value_as_generic_from_hand(),
            filter,
            Zone::Hand,
        ),
    )))
}

pub fn parse_life_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_life_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    let usage_limit = match spec.usage_limit {
        keyword_static_lines::LifeManaValueGrantUsageLimit::OnceDuringEachOfYourTurns => {
            crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
        }
    };
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::life_equal_mana_value_from_hand(Some(usage_limit)),
            filter,
            Zone::Hand,
        ),
    )))
}

pub fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_fixed_mana_cost_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::cast_from_hand_for_alternative_mana_cost_matching(
            filter,
            spec.mana_cost,
        ),
    )))
}

pub fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // The complete free-cast rule already includes the flash grant; reading
    // only its timing suffix would discard its payment and beneficiary.
    if matches!(parse_player_may_cast_spells_free_and_flash_line(tokens), Ok(Some(_))) {
        return Ok(None);
    }
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            mut spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec.zone == Zone::Stack
            && matches!(&spec.grantable,
            crate::model::CompilerGrantableCore::Ability(ability) if ability.id() == crate::static_abilities::StaticAbilityId::Flash) =>
        {
            let Some(beneficiary) = static_grant_beneficiary(player) else {
                return Ok(None);
            };
            spec.beneficiary = beneficiary;
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub fn static_grant_beneficiary(player: crate::cards::builders::PlayerAst) -> Option<PlayerFilter> {
    match player {
        crate::cards::builders::PlayerAst::You | crate::cards::builders::PlayerAst::Implicit => {
            Some(PlayerFilter::You)
        }
        crate::cards::builders::PlayerAst::Any => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_counter_permission_tokens(tokens) else {
        return Ok(None);
    };
    let is_play_lands_and_cast_noncreature_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayLandsAndCastNoncreatureCardsExiledBySource
    );
    let is_play_owned_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayLandsAndCastSpellsOwnedInExile
    );
    let is_play_not_owned_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayCardsNotOwnedInExile
    );
    let owner = match spec.owner {
        keyword_static_lines::ExileCounterPermissionOwner::Any => None,
        keyword_static_lines::ExileCounterPermissionOwner::Opponent => Some(PlayerFilter::Opponent),
        keyword_static_lines::ExileCounterPermissionOwner::You => Some(PlayerFilter::You),
        keyword_static_lines::ExileCounterPermissionOwner::NotYou => Some(PlayerFilter::NotYou),
    };

    let uses_snow_sources = matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::SnowSources
    );

    let mut base_filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner,
        with_counter: Some(crate::filter::CounterConstraint::Typed(spec.counter_type)),
        ..ObjectFilter::default()
    };
    if is_play_lands_and_cast_noncreature_family {
        base_filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::ExiledByYou.bind()).into(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    let mut filter = if is_play_lands_and_cast_noncreature_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Creature, CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else if is_play_owned_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else if is_play_not_owned_family {
        base_filter.clone()
    } else {
        ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..base_filter
        }
    };
    filter.has_mana_cost = false;

    let mut grant = StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            filter.clone(),
            Zone::Exile,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    if spec.during_your_turn {
        grant = grant.with_condition(PredicateAst::ActivationTiming(
            crate::ability::ActivationTiming::DuringYourTurn,
        ));
    }
    if matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::None
    ) {
        return Ok(Some(vec![grant]));
    }
    let permission = if uses_snow_sources {
        crate::effect::ManaSpendPermission::any_color_from_sources_for_casting_matching(
            PlayerFilter::You,
            filter,
            ObjectFilter::default().with_supertype(Supertype::Snow),
        )
    } else if matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::AnyTypeCanBeSpent
    ) {
        crate::effect::ManaSpendPermission::any_type_for_casting_matching(PlayerFilter::You, filter)
    } else {
        crate::effect::ManaSpendPermission::any_color_for_casting_matching(
            PlayerFilter::You,
            filter,
        )
    };
    let mana_permission =
        StaticAbility::mana_spend_permission(permission, render_token_slice(tokens));

    Ok(Some(vec![grant, mana_permission]))
}

pub fn parse_surveilled_graveyard_play_life_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if !late_static_facts::is_surveilled_graveyard_play_life_cost(tokens) {
        return Ok(None);
    }

    let base_filter = ObjectFilter {
        zone: Some(Zone::Graveyard),
        owner: Some(PlayerFilter::You),
        surveilled_this_turn: true,
        ..ObjectFilter::default()
    };
    let mut spell_filter = base_filter.clone();
    spell_filter.excluded_card_types.push(CardType::Land);

    Ok(Some(vec![
        StaticAbility::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::play_from(),
                base_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
        StaticAbility::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::life_equal_mana_value_from_zone(
                    Zone::Graveyard,
                    None,
                ),
                spell_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
    ]))
}

/// "During each of your turns, you may play a land and cast a permanent spell
/// of each permanent type from your graveyard." Each permanent type is an
/// independent once-per-your-turn budget: one static permission per type
/// (land play; artifact, creature, enchantment, planeswalker, battle spells).
/// The source-line marker gives every member its own ability slot, so each
/// budget has a distinct permission identity. A card with several permanent
/// types matches several budgets; the exact permission selected as it is
/// played is the type chosen for it (CR 110.4 permanent types).
pub fn parse_land_and_each_permanent_type_graveyard_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    const SHAPE: &[&str] = &[
        "during", "each", "of", "your", "turns", "you", "may", "play", "a", "land", "and",
        "cast", "a", "permanent", "spell", "of", "each", "permanent", "type", "from", "your",
        "graveyard",
    ];
    let tokens = trim_edge_punctuation_tokens(tokens);
    if parser_token_word_refs(tokens).as_slice() != SHAPE {
        return Ok(None);
    }
    let rendered = render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    let surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    let budget = |card_type: CardType| {
        let mut filter = ObjectFilter {
            zone: Some(Zone::Graveyard),
            owner: Some(PlayerFilter::You),
            card_types: vec![card_type],
            ..ObjectFilter::default()
        };
        if card_type != CardType::Land {
            filter.excluded_card_types.push(CardType::Land);
        }
        let mut spec = crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            filter,
            Zone::Graveyard,
        )
        .with_beneficiary(PlayerFilter::You)
        .with_usage_limit(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns);
        spec.filtered_zone_surface = surface.clone();
        StaticAbility::grants(spec)
    };
    Ok(Some(
        [
            CardType::Land,
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
            CardType::Battle,
        ]
        .into_iter()
        .map(budget)
        .collect(),
    ))
}

/// "[During your turn,] you may play cards exiled with <source>. If you cast
/// a spell this way, pay life equal to its mana value rather than pay its
/// mana cost."
pub fn parse_source_exiled_play_life_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    use winnow::Parser as _;
    let (during_your_turn, rest) = match crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["during", "your", "turn"]),
    ) {
        Some((_, rest)) => (true, trim_lexed_commas(rest)),
        None => (false, tokens),
    };
    // "During your turn, you may cast cards exiled with Azula and you may
    // cast them as though they had flash. Mana of any type can be spent to
    // cast those spells." (Azula, Cunning Usurper)
    if let Some((_, cast_rest)) = crate::grammar::primitives::parse_prefix(
        rest,
        crate::grammar::primitives::phrase(&["you", "may", "cast"]),
    ) && let Some((reference, tail)) =
        crate::grammar::permission_facts::source_exiled::parse_cards_from_source_exiled_tokens(
            cast_rest,
        )
    {
        if crate::grammar::primitives::parse_all(
            tail,
            (
                winnow::combinator::opt(crate::grammar::primitives::comma()),
                crate::grammar::primitives::phrase(&[
                    "and", "you", "may", "cast", "them", "as", "though", "they", "had", "flash",
                ]),
                crate::grammar::primitives::period(),
                crate::grammar::primitives::phrase(&[
                    "mana", "of", "any", "type", "can", "be", "spent", "to", "cast", "those",
                    "spells",
                ]),
                winnow::combinator::opt(crate::grammar::primitives::period()),
            )
                .void(),
            "source-exiled flash cast with any mana type",
        )
        .is_err()
        {
            return Ok(None);
        }
        let mut pool = ObjectFilter::default().in_zone(Zone::Exile);
        pool.excluded_card_types.push(CardType::Land);
        pool.tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        let mut spec = crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            pool,
            Zone::Exile,
        )
        .with_beneficiary(PlayerFilter::You)
        .with_source_exiled_surface(crate::grant::SourceExiledGrantSurface {
            mana_rider: None,
            source: reference.surface,
            plural_spell_subject: false,
            generic_card_pool: true,
            generic_cast_this_way_subject: true,
        });
        spec.instant_timing = true;
        spec.cast_mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::AnyType;
        if during_your_turn {
            spec = spec.with_usage_limit(crate::grant::GrantUsageLimit::DuringYourTurns);
        }
        return Ok(Some(vec![StaticAbility::grants(spec)]));
    }
    let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
        rest,
        crate::grammar::primitives::phrase(&["you", "may", "play"]),
    ) else {
        return Ok(None);
    };
    let Some((reference, tail)) =
        crate::grammar::permission_facts::source_exiled::parse_cards_from_source_exiled_tokens(
            rest,
        )
    else {
        return Ok(None);
    };
    if crate::grammar::primitives::parse_all(
        tail,
        (
            winnow::combinator::opt(crate::grammar::primitives::period()),
            crate::grammar::primitives::phrase(&["if", "you", "cast", "a", "spell", "this", "way"]),
            winnow::combinator::opt(crate::grammar::primitives::comma()),
            crate::grammar::primitives::phrase(&[
                "pay", "life", "equal", "to", "its", "mana", "value", "rather", "than", "pay",
                "its", "mana", "cost",
            ]),
            winnow::combinator::opt(crate::grammar::primitives::period()),
        )
            .void(),
        "source-exiled play life cost",
    )
    .is_err()
    {
        return Ok(None);
    }
    let usage_limit = during_your_turn.then_some(crate::grant::GrantUsageLimit::DuringYourTurns);
    let surface = crate::grant::SourceExiledGrantSurface {
            mana_rider: None,
        source: reference.surface,
        plural_spell_subject: false,
        generic_card_pool: true,
        generic_cast_this_way_subject: true,
    };
    let mut pool = ObjectFilter::default();
    pool.tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
    let mut spells = pool.clone();
    spells.excluded_card_types.push(CardType::Land);
    let mut play = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        pool,
        Zone::Exile,
    )
    .with_beneficiary(PlayerFilter::You)
    .with_source_exiled_surface(surface.clone());
    let mut life = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::life_equal_mana_value_from_zone(
            Zone::Exile,
            usage_limit,
        ),
        spells,
        Zone::Exile,
    )
    .with_beneficiary(PlayerFilter::You)
    .with_source_exiled_surface(surface);
    if let Some(limit) = usage_limit {
        play = play.with_usage_limit(limit);
        life = life.with_usage_limit(limit);
    }
    Ok(Some(vec![
        StaticAbility::grants(play),
        StaticAbility::grants(life),
    ]))
}

/// "During each player's turn, that player may play a land or cast a spell
/// from among cards exiled with this enchantment, and they may spend mana as
/// though it were mana of any color to cast that spell. When they do, exile
/// the top card of their library." (Share the Spoils)
///
/// One standing play permission over the source's exiled cards whose
/// beneficiary is the active player (CR 102.1), with a permission-local mana
/// rider and a reflexive instruction run when it is used. The reflexive
/// instruction is controlled by the player who used the permission, so
/// "their library" is that player's own.
fn parse_turn_player_source_exiled_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::model::CompilerGrantSpecCore>, CardTextError> {
    let joined = parser_token_word_refs(tokens)
        .into_iter()
        .map(|word| word.replace('\'', ""))
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" player s turn ", " players turn ");
    let words = joined.split(' ').collect::<Vec<_>>();
    const HEAD: &[&str] = &[
        "during", "each", "players", "turn", "that", "player", "may", "play", "a", "land", "or",
        "cast", "a", "spell", "from", "among", "cards", "exiled", "with", "this",
    ];
    const RIDER: &[&str] = &[
        "and", "they", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color", "to", "cast", "that", "spell", "when", "they", "do", "exile", "the", "top",
        "card", "of", "their", "library",
    ];
    let Some([noun, rider @ ..]) = words.strip_prefix(HEAD) else {
        return Ok(None);
    };
    if rider != RIDER {
        return Ok(None);
    }
    // The reflexive instruction's controller is the player who used the
    // permission; "their library" is that player's ("your" in its program).
    let follow_up =
        crate::lexer::synthetic_word_tokens(["exile", "the", "top", "card", "of", "your", "library"]);
    let on_use_effects = crate::clause_support::parse_effect_sentences_lexed(&follow_up)?;
    if on_use_effects.is_empty() {
        return Ok(None);
    }
    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
        relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        filter,
        Zone::Exile,
    )
    .with_beneficiary(PlayerFilter::Active);
    spec.cast_mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::AnyColor;
    spec.on_use_effects = on_use_effects;
    let rendered = crate::lexer::render_token_slice(tokens);
    let surface = rendered.trim().trim_end_matches('.');
    let mut chars = surface.chars();
    spec.filtered_zone_surface = chars
        .next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    let _ = noun;
    Ok(Some(spec))
}

/// "You may play lands from among cards exiled with this enchantment." and
/// "You may cast spells from among cards exiled with this enchantment. You
/// can't cast more than one spell this way each turn.": the source-linked
/// exile pool split into a land-only play permission and a once-each-turn
/// spell permission (the budget belongs to that permission, not to the
/// player's other spells).
fn parse_source_exiled_lands_or_budgeted_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    use crate::grammar::primitives;
    if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["you", "may", "play", "lands", "from", "among"]),
    ) && let Some((reference, tail)) =
        crate::grammar::permission_facts::source_exiled::parse_cards_from_source_exiled_tokens(rest)
        && primitives::probe_all(tail, primitives::sentence_end(), "source-linked land permission")
            .is_some()
    {
        let mut filter = ObjectFilter::default()
            .in_zone(Zone::Exile)
            .with_type(CardType::Land);
        filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
        let spec = crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            filter,
            Zone::Exile,
        )
        .with_source_exiled_surface(crate::grant::SourceExiledGrantSurface {
            mana_rider: None,
            source: reference.surface,
            plural_spell_subject: false,
            generic_card_pool: true,
            generic_cast_this_way_subject: true,
        });
        return Ok(Some(vec![StaticAbility::grants(spec)]));
    }
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let [permission, budget] = sentences.as_slice() else {
        return Ok(None);
    };
    let budget_words = parser_token_word_refs(budget);
    let budget_words = budget_words.as_slice();
    if !matches!(
        budget_words,
        ["you", "can't" | "cant", "cast", "more", "than", "one", "spell", "this", "way", "each", "turn"]
    ) || !parser_token_word_refs(permission).starts_with(&["you", "may", "cast", "spells"])
    {
        return Ok(None);
    }
    let Some(mut abilities) = parse_you_may_static_grant_line(permission)? else {
        return Ok(None);
    };
    let [ability] = abilities.as_mut_slice() else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::Grants(spec) = &mut ability.payload else {
        return Ok(None);
    };
    if spec.usage_limit.is_some()
        || !matches!(spec.grantable, crate::model::CompilerGrantableCore::PlayFrom)
    {
        return Ok(None);
    }
    spec.usage_limit = Some(crate::grant::GrantUsageLimit::OnceEachTurn);
    Ok(Some(abilities))
}

/// "Each player may [look at cards they exiled with this <source>, and they
/// may] play lands and cast spells from among [those] cards [they exiled with
/// this <source> this turn]": one permission per player over the source's
/// linked cards that player exiled (Shared Fate, Uba Mask).
pub fn parse_each_player_source_exiled_by_them_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some((reference, may_look, this_turn)) =
        crate::grammar::permission_facts::source_exiled::parse_each_player_source_exiled_by_them_tokens(tokens)
    else {
        return Ok(None);
    };
    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
        relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    let by_them = if this_turn {
        ironsmith_core::tag::SOURCE_EXILED_BY_YOU_THIS_TURN_TAG
    } else {
        ironsmith_core::tag::SOURCE_EXILED_BY_YOU_TAG
    };
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: ironsmith_core::tag::TagKey::from(by_them).into(),
        relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(), filter, Zone::Exile,
    ).with_source_exiled_surface(crate::grant::SourceExiledGrantSurface {
        mana_rider: None,
        source: reference.surface,
        plural_spell_subject: true,
        generic_card_pool: true,
        generic_cast_this_way_subject: true,
    });
    spec.beneficiary = PlayerFilter::Any;
    spec.may_look_at_linked_exile = may_look;
    Ok(Some(StaticAbility::grants(spec)))
}

pub fn parse_you_may_static_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    // A complete zone-specific free-cast line on a permanent is a standing
    // permission. The document dispatcher gives spell statements precedence
    // on instants/sorceries; activated and triggered bodies use effect parsing.
    if let Some((_, rest)) = crate::grammar::primitives::parse_prefix(
        tokens,
        crate::grammar::primitives::phrase(&["you", "may", "cast"]),
    ) && let Some(fact) = crate::grammar::permission_facts::zone_free_cast::parse_free_cast_from_zone_tokens(rest)
        && matches!(fact.zone, Zone::Graveyard | Zone::Exile)
        && fact.mana_value.is_none()
        && let Some(mut filter) = crate::grammar::permission_facts::subject_filters::parse_permission_subject_filter_tokens(fact.filter_tokens)?
    {
        filter.zone = None;
        filter.stack_kind = None;
        if !filter.excluded_card_types.contains(&crate::types::CardType::Land) {
            filter.excluded_card_types.push(crate::types::CardType::Land);
        }
        if fact.zone == Zone::Graveyard {
            filter.owner = Some(PlayerFilter::You);
        }
        let method = ironsmith_core::AlternativeCastingMethod::<
            EffectAst, crate::model::CompilerCost, ironsmith_core::ThisSpellCostCondition,
        >::cast_from_zone_with_total_cost(
            "Cast without paying mana cost", fact.zone,
            ironsmith_core::TotalCost::<crate::model::CompilerCost>::free(), None, false,
        );
        return Ok(Some(vec![StaticAbility::grants(crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::AlternativeCast(method), filter, fact.zone,
        ))]));
    }

    if let Some(abilities) = parse_source_exiled_lands_or_budgeted_spells_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(spec) = parse_turn_player_source_exiled_play_line(tokens)? {
        return Ok(Some(vec![StaticAbility::grants(spec)]));
    }
    if let Some(reference) = crate::grammar::permission_facts::source_exiled::parse_look_source_exiled_tokens(tokens) {
        return Ok(Some(vec![StaticAbility::look_at_source_exiled_cards(reference.surface)]));
    }
    let inspected = crate::grammar::permission_facts::source_exiled::parse_look_and_play_source_exiled_tokens(tokens);
    let may_look = inspected.is_some();
    let inline = crate::grammar::permission_facts::source_exiled::parse_play_source_exiled_inline_mana_tokens(tokens);
    let rider_surface = if inline.is_some() { ironsmith_core::SourceExiledManaRiderSurface::InlineCastSpells }
        else { ironsmith_core::SourceExiledManaRiderSurface::ConditionalCast };
    let converted = inline.or_else(|| crate::grammar::permission_facts::source_exiled::parse_play_source_exiled_with_mana_tokens(tokens));
    let mode = converted.as_ref().map_or(ironsmith_core::value_model::ManaSpendMode::Normal, |(_, mode)| *mode);
    if let Some(reference) = converted.map(|(reference, _)| reference).or(inspected).or_else(|| crate::grammar::permission_facts::source_exiled::
        parse_play_lands_and_spells_from_source_exiled_tokens(tokens))
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
        // "you may cast the exiled card" (Null Summoner): a cast-only
        // permission never lets a land be played from the pool.
        let words = parser_token_word_refs(tokens);
        if words.windows(3).any(|window| window == ["you", "may", "cast"])
            && !words.contains(&"play")
        {
            filter.excluded_card_types.push(crate::types::CardType::Land);
        }
        let mut spec = crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(), filter, Zone::Exile,
        ).with_source_exiled_surface(crate::grant::SourceExiledGrantSurface {
            mana_rider: (!mode.is_normal()).then_some(rider_surface),
            source: reference.surface,
            plural_spell_subject: parser_token_word_refs(tokens).windows(3)
                .any(|words| words == ["and", "cast", "spells"]),
            generic_card_pool: true, generic_cast_this_way_subject: true,
        });
        spec.requires_linked_exile_pair = true;
        spec.may_look_at_linked_exile = may_look;
        spec.cast_mana_spend_mode = mode;
        return Ok(Some(vec![StaticAbility::grants(spec)]));
    }
    // The dedicated land permission owns this exact surface. Its canonical
    // grant must not compete with the broader play-from permission parser.
    if is_play_lands_from_graveyard_line_lexed(tokens) {
        return Ok(None);
    }
    if let Some(bundle) = crate::permission_helpers::parse_top_look_and_permission(tokens)? {
        return Ok(Some(bundle));
    }
    if late_static_facts::is_source_linked_exile_cast_with_any_mana(tokens) {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.owner = Some(PlayerFilter::NotYou);
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        let grant = StaticAbility::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::play_from(),
                filter.clone(),
                Zone::Exile,
            )
            // "During each player's turn, that player may cast ...": only the
            // player whose turn it is holds the permission.
            .with_beneficiary(PlayerFilter::Active),
        );
        let mana_permission = StaticAbility::mana_spend_permission(
            crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                PlayerFilter::Any,
                filter,
            ),
            "Mana of any type can be spent to cast it",
        );
        return Ok(Some(vec![grant, mana_permission]));
    }

    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => {
            let singular_spell = late_static_facts::contains_singular_cast_spell(tokens);
            // A once-per-turn budget makes the singular wording a standing
            // permission rather than a one-shot resolution.
            if singular_spell
                && spec.usage_limit.is_none()
                && spec.additional_zones.is_empty()
                && spec.zone == Zone::Hand
                && matches!(
                    &spec.grantable,
                    crate::model::CompilerGrantableCore::AlternativeCast(method)
                        if method.cast_from_zone() == Zone::Hand
                            && method.mana_cost().is_none()
                            && method.non_mana_costs().is_empty()
                )
            {
                return Ok(None);
            }
            Ok(static_grant_beneficiary(player)
                .map(|beneficiary| vec![StaticAbility::grants(spec.with_beneficiary(beneficiary))]))
        }
        _ => Ok(None),
    }
}

pub fn parse_as_you_cascade_land_drop_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_cascade_land_drop_tokens(tokens) {
        return Ok(Some(StaticAbility::cascade_land_drop()));
    }
    Ok(None)
}

pub fn parse_play_from_permission_with_haste_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(permission_sentence) =
        late_static_facts::parse_play_permission_with_haste_followup(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
        ) && spec.filter.card_types.len() == 1
            && spec.filter.card_types.first() == Some(&CardType::Creature) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::haste()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub fn parse_play_from_permission_with_enter_counter_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(parsed) = keyword_static_lines::parse_play_permission_enter_counter_tokens(tokens)
    else {
        return Ok(None);
    };
    // "During your turn, you may cast creature spells ... from your
    // graveyard. If you cast a spell this way, that creature enters with a
    // finality counter on it." (Leonardo, Sewer Samurai): the leading turn
    // window is the permission's usage limit.
    let (during_your_turn, permission_tokens) = match crate::grammar::primitives::parse_prefix(
        parsed.permission_tokens,
        crate::grammar::primitives::phrase(&["during", "your", "turn"]),
    ) {
        Some((_, rest)) => (true, trim_lexed_commas(rest)),
        None => (false, parsed.permission_tokens),
    };

    match parse_permission_clause_spec(permission_tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
                | crate::model::CompilerGrantableCore::DerivedAlternativeCast(_)
        ) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                let count = if parsed.additional {
                    Value::Fixed(1)
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
                } else {
                    Value::Fixed(1)
                };
                let mut spec = spec.with_beneficiary(beneficiary).with_cast_this_way_grant(
                    StaticAbility::enters_with_counters_value(parsed.counter_type, count),
                );
                if during_your_turn && spec.usage_limit.is_none() {
                    spec.usage_limit = Some(crate::grant::GrantUsageLimit::DuringYourTurns);
                }
                if let Some(filter) = parsed.cast_this_way_filter {
                    spec = spec.with_cast_this_way_filter(filter);
                }
                StaticAbility::grants(spec)
            }))
        }
        _ => Ok(None),
    }
}

pub fn parse_play_from_permission_with_enter_tapped_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(permission_sentence) =
        late_static_facts::parse_play_permission_with_enter_tapped_followup(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
                | crate::model::CompilerGrantableCore::AlternativeCast(_)
                | crate::model::CompilerGrantableCore::DerivedAlternativeCast(_)
        ) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::enters_tapped_ability()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub fn parse_you_may_look_top_card_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_top_card_any_time_line_lexed(tokens) {
        return Ok(Some(StaticAbility::look_at_top_card_of_library()));
    }
    Ok(None)
}

pub fn parse_you_may_look_face_down_creatures_you_dont_control_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // CR 708.6: a permission of this source's controller, modeled as a rule
    // effect that reveals the matching face-down creatures to that player.
    if let Some(objects) = crate::grammar::abilities::face_down_look_permission_objects_lexed(tokens) {
        let rule = ironsmith_core::ObjectRule::FaceDownVisibleTo(PlayerFilter::You);
        let display = {
            let text = rule.describe(&objects);
            let mut chars = text.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        };
        return Ok(Some(StaticAbility::restriction_with_display(
            crate::effect::Restriction::object_rule(objects, rule),
            display,
        )));
    }
    Ok(None)
}

pub fn parse_players_play_top_card_libraries_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_play_top_card_libraries_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ));
    }
    Ok(None)
}

pub fn parse_play_top_card_your_library_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_top_card_your_library_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ));
    }
    Ok(None)
}

pub fn parse_self_or_global_hands_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    Ok(
        if crate::word_primitives::parse_sequence_complete(
            &words,
            &["play", "with", "your", "hand", "revealed"],
        ) {
            Some(StaticAbility::controller_plays_with_hand_revealed())
        } else if crate::word_primitives::parse_sequence_complete(
            &words,
            &["players", "play", "with", "their", "hands", "revealed"],
        ) {
            Some(StaticAbility::players_play_with_hands_revealed())
        } else {
            None
        },
    )
}

pub fn parse_your_opponents_play_with_hands_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_your_opponents_play_with_hands_revealed_line_lexed(tokens) {
        return Ok(Some(StaticAbility::opponents_play_with_hands_revealed()));
    }
    Ok(None)
}

pub fn parse_control_opponents_while_searching_libraries_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_control_opponents_while_searching(tokens) {
        return Ok(Some(
            StaticAbility::control_opponents_while_searching_libraries(),
        ));
    }
    Ok(None)
}

pub fn parse_opponent_search_exile_found_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_opponent_search_exile_found_cards(tokens) {
        return Ok(Some(StaticAbility::opponent_search_exile_found_cards()));
    }
    Ok(None)
}

pub fn parse_cast_this_card_from_library_while_searching_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_cast_this_card_from_library_while_searching(tokens) {
        return Ok(Some(
            StaticAbility::cast_this_card_from_library_while_searching(),
        ));
    }
    Ok(None)
}

/// Trailing ordinary state predicates share the leading-condition owner.
/// Target-dependent, paid and X-dependent timing keep their specialized rules.
pub fn parse_conditional_self_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(index) = tokens.iter().position(|token| token.is_word("if")) else {
        return Ok(None);
    };
    if !is_cast_this_spell_as_though_it_had_flash_line_lexed(&tokens[..index]) {
        return Ok(None);
    }
    let tail = trim_edge_punctuation(&tokens[index + 1..]);
    if tail.first().is_some_and(|token| token.is_word("x"))
        || (tail.first().is_some_and(|token| token.is_word("it"))
            && tail.get(1).is_some_and(|token| token.is_word("targets")))
    {
        return Ok(None);
    }
    // A conjunction of complete existential combat-state clauses keeps each
    // witness independent; it does not require one creature to do both.
    let combat_states: Option<Vec<bool>> = {
        use winnow::prelude::*;
        crate::grammar::primitives::probe_all(
            &tail,
            winnow::combinator::separated(
                1..,
                (
                    crate::grammar::primitives::phrase(&["a", "creature", "is"]),
                    winnow::combinator::alt((
                        crate::grammar::primitives::kw("attacking").value(true),
                        crate::grammar::primitives::kw("blocking").value(false),
                    )),
                )
                    .map(|(_, attacking)| attacking),
                crate::grammar::primitives::kw("and"),
            ),
            "existential-combat-state-flash",
        )
    };
    let condition = if let Some(states) = combat_states {
        states
            .into_iter()
            .map(|attacking| {
                let mut filter = ObjectFilter::creature();
                filter.attacking = attacking;
                filter.blocking = !attacking;
                PredicateAst::CountComparison {
                    count: crate::static_abilities::AnthemCountExpression::MatchingFilter(filter),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    display: None,
                }
            })
            .reduce(|left, right| PredicateAst::And(Box::new(left), Box::new(right)))
            .expect("nonempty grammar")
    } else {
        let Ok(condition) = parse_static_condition_clause(&tail) else {
            return Ok(None);
        };
        condition
    };
    fn board_state(predicate: &PredicateAst) -> bool {
        match predicate {
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                board_state(left) && board_state(right)
            }
            PredicateAst::Not(inner) => board_state(inner),
            PredicateAst::CountComparison {
                count: crate::static_abilities::AnthemCountExpression::MatchingFilter(filter),
                ..
            }
            | PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControls {
                filter,
                ..
            }) => matches!(filter.zone, None | Some(Zone::Battlefield)),
            _ => false,
        }
    }
    if !board_state(&condition)
        || static_condition_references_source_outside_battlefield(&condition)
    {
        return Ok(None);
    }
    Ok(Some(StaticAbilityAst::LabeledConditionalStaticAbility {
        ability: Box::new(StaticAbilityAst::from(StaticAbility::flash())),
        condition,
        label: crate::lexer::render_token_slice(&trim_edge_punctuation(tokens)),
    }))
}

pub fn parse_cast_this_spell_as_though_it_had_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_cast_this_spell_as_though_it_had_flash_line_lexed(tokens) {
        return Ok(Some(StaticAbility::flash()));
    }
    if let Some(index) = tokens.iter().position(|token| token.is_word("if"))
        && is_cast_this_spell_as_though_it_had_flash_line_lexed(&tokens[..index])
    {
        let tail = trim_edge_punctuation(&tokens[index + 1..]);
        if tail.len() > 2 && tail[0].is_word("it") && tail[1].is_word("targets") {
            let filter = parse_object_filter_lexed(&tail[2..], false)?;
            return Ok(Some(StaticAbility::flash_if_targets_matching(filter)));
        }
        // Decision-dependent timing (CR 601.2b-c): "if X is 3 or less"
        // (Silver Scrutiny), "if it's cast using teamwork" (Quantum
        // Reduction).
        let words: Vec<String> = crate::lexer::token_word_refs(&tail)
            .into_iter()
            .map(|word| word.replace(['\'', '’'], ""))
            .collect();
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        let choice = match words.as_slice() {
            ["x", "is", count, "or", "less"] => crate::util::parse_number_word_u32(count)
                .map(ironsmith_core::FlashCastChoice::XAtMost),
            ["its" | "it", "cast", "using", label] | ["it", "is", "cast", "using", label]
                if *label == "teamwork" =>
            {
                Some(ironsmith_core::FlashCastChoice::OptionalCostPaid("Teamwork".into()))
            }
            _ => None,
        };
        if let Some(choice) = choice {
            return Ok(Some(StaticAbility::flash_if_cast_choice(choice)));
        }
    }
    Ok(None)
}

pub fn parse_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if super::combat_requirements::owns_combat_requirement_line(tokens) {
        let parsed = super::combat_requirements::parse_combat_requirement_static_line(tokens)?;
        return Ok(parsed.and_then(|mut abilities| {
            (abilities.len() == 1).then(|| abilities.remove(0))
        }));
    }

    if let Some(unless) = tokens.iter().position(|token| token.is_word("unless"))
        && let Some(ability) = parse_attacks_each_combat_if_able_line(&tokens[..unless])?
    {
        let condition = parse_static_condition_clause(&tokens[unless + 1..])?;
        return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(ability),
            condition: PredicateAst::Not(Box::new(condition)),
        }));
    }
    // "Creatures your opponents control attack each combat if able and attack
    // a player other than you if able." (Kardur, Doomscourge) is the goad
    // requirement stated in full.
    if let Some(goad_tail) = crate::slice_primitives::find_window_by(tokens, 10, |window| {
        window[0].is_word("and")
            && window[1].is_word("attack")
            && window[2].is_word("a")
            && window[3].is_word("player")
            && window[4].is_word("other")
            && window[5].is_word("than")
            && window[6].is_word("you")
            && window[7].is_word("if")
            && window[8].is_word("able")
            && window[9].kind == crate::lexer::TokenKind::Period
    }) {
        let head = &tokens[..goad_tail];
        if let Some(late_static_facts::AttackEachCombatFact::Subject(subject_tokens)) =
            late_static_facts::parse_attack_each_combat_if_able_tokens(head)
        {
            let subject_tokens = trim_commas(subject_tokens);
            if !subject_tokens.is_empty() {
                let filter = parse_object_filter_lexed(&subject_tokens, false)?;
                return Ok(Some(StaticAbilityAst::Static(
                    StaticAbility::goad_matching(filter),
                )));
            }
        }
    }
    let Some(fact) = late_static_facts::parse_attack_each_combat_if_able_tokens(tokens) else {
        return Ok(None);
    };
    if matches!(
        fact,
        late_static_facts::AttackEachCombatFact::AttachedController
    ) {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::all_creatures_attack_attached_controller_each_combat_if_able(),
        )));
    }
    let late_static_facts::AttackEachCombatFact::Subject(subject_tokens) = fact else {
        unreachable!("attached-controller fact returned above")
    };
    let subject_tokens = trim_commas(subject_tokens);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => {
            Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())))
        }
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
            condition: None,
        })),
    }
}

pub fn parse_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(count) = late_static_facts::parse_additional_land_play_count(tokens) else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::additional_land_plays(count)]))
}

pub fn parse_play_lands_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_lands_from_graveyard_line_lexed(tokens) {
        let spec = crate::model::CompilerGrantSpecCore::play_lands_from_graveyard();
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub fn parse_graveyard_cards_have_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_retrace_grant_tokens(tokens) else {
        return Ok(None);
    };
    if fact.during_your_turn {
        // The turn-scoped grant is the conditional rule's
        // (`parse_during_your_turn_graveyard_cards_have_retrace_line`).
        return Ok(None);
    }
    Ok(Some(graveyard_cards_have_retrace_ability(fact)))
}

fn graveyard_cards_have_retrace_ability(
    fact: late_static_facts::RetraceGrantFact,
) -> StaticAbility {
    let mut filter = if fact.nonland_permanents {
        ObjectFilter::permanent_card().without_type(CardType::Land)
    } else {
        ObjectFilter {
            card_types: fact.card_types,
            subtypes: fact.subtypes,
            ..ObjectFilter::default()
        }
    };
    filter.owner = Some(PlayerFilter::You);
    filter.zone = Some(Zone::Graveyard);
    let spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::retrace_from_cards_mana_cost(),
        filter,
        Zone::Graveyard,
    );
    StaticAbility::grants(spec)
}

/// "During your turn, nonland permanent cards in your graveyard have
/// retrace." (Six): the retrace grant scoped to the controller's turn.
pub fn parse_during_your_turn_graveyard_cards_have_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<crate::cards::builders::StaticAbilityAst>, CardTextError> {
    let Some(fact) = late_static_facts::parse_retrace_grant_tokens(tokens) else {
        return Ok(None);
    };
    if !fact.during_your_turn {
        return Ok(None);
    }
    Ok(Some(
        crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(crate::cards::builders::StaticAbilityAst::Static(
                graveyard_cards_have_retrace_ability(fact),
            )),
            condition: crate::cards::builders::PredicateAst::YourTurn,
        },
    ))
}

/// "You have protection from each of your opponents." (Absolute Virtue):
/// CR 702.16b/c/e/j — opponents' sources can't target, enchant or damage you.
pub fn parse_you_have_protection_from_opponents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(tokens);
    if words.as_slice() != ["you", "have", "protection", "from", "each", "of", "your", "opponents"] {
        if !words.starts_with(&["you", "have", "protection", "from"]) {
            return Ok(None);
        }
        let Some(actions) = crate::clause_support::parse_protection_chain(&tokens[2..]) else {
            return Ok(None);
        };
        let mut abilities = Vec::new();
        for action in actions {
            let Some(filter) = super::granted_protection_source_filter(
                &StaticAbilityAst::KeywordAction(action),
            ) else { return Ok(None); };
            abilities.push(StaticAbility::player_protection_from(
                PlayerFilter::You, filter, render_token_slice(tokens),
            ));
        }
        return Ok(Some(abilities));
    }
    Ok(Some(vec![StaticAbility::player_protection_from(
        PlayerFilter::You,
        ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
        "You have protection from each of your opponents",
    )]))
}

/// "Any player may cast creature spells with mana value 3 or less without
/// paying their mana costs and as though they had flash." (Aluren): a free
/// alternative price and flash timing for the same spells.
pub fn parse_player_may_cast_spells_free_and_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    const FREE: &[&str] = &["without", "paying", "their", "mana", "costs"];
    const FLASH: &[&str] = &["as", "though", "they", "had", "flash"];
    let tokens = trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(tokens);
    let (beneficiary, rest_start) = match words.as_slice() {
        ["any", "player", "may", "cast", ..] => (PlayerFilter::Any, 4),
        ["you", "may", "cast", ..] => (PlayerFilter::You, 3),
        _ => return Ok(None),
    };
    let Some(without) = words[rest_start..]
        .iter()
        .position(|word| *word == "without")
    else {
        return Ok(None);
    };
    let tail = &words[rest_start + without..];
    let with_flash = if tail == [FREE, &["and"], FLASH].concat().as_slice() {
        true
    } else if tail == FREE {
        false
    } else {
        return Ok(None);
    };
    let view = TokenWordView::new(tokens);
    let (Some(start), Some(end)) = (
        view.map_word_to_token_boundary(rest_start),
        view.map_word_to_token_boundary(rest_start + without),
    ) else {
        return Ok(None);
    };
    let spell_words = &words[rest_start..rest_start + without];
    if !spell_words.contains(&"spells") || spell_words.contains(&"from") {
        return Ok(None);
    }
    let Ok(mut filter) = parse_object_filter_lexed(&tokens[start..end], false) else {
        return Ok(None);
    };
    // The permission applies to cards being cast, not to objects already on
    // the stack.
    filter.zone = None;
    filter.stack_kind = None;
    // No origin is named: this replaces the price of any independently
    // authorized cast, including graveyard/exile permissions.
    let mut free = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::AlternativePrice {
            costs: Vec::new(),
            origin: None,
        },
        filter.clone(),
        Zone::Hand,
    );
    free.beneficiary = beneficiary.clone();
    // Preserve this price's authored label independently of the flash grant.
    let price_end = view.map_word_to_token_boundary(rest_start + without + FREE.len())
        .unwrap_or(tokens.len());
    let surface = crate::lexer::render_token_slice(&tokens[..price_end]);
    let mut chars = surface.trim().trim_end_matches('.').chars();
    free.filtered_zone_surface = chars.next()
        .map(|first| format!("{}{}", first.to_uppercase(), chars.as_str()));
    let mut abilities = vec![StaticAbility::grants(free)];
    if with_flash {
        let mut flash =
            crate::model::CompilerGrantSpecCore::flash_timing_for_spells_matching(filter);
        flash.beneficiary = beneficiary;
        abilities.push(StaticAbility::grants(flash));
    }
    Ok(Some(abilities))
}

pub fn parse_cast_spells_from_hand_without_paying_mana_costs_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::contains_singular_cast_spell(tokens) {
        return Ok(None);
    }
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec.zone == Zone::Hand
            && matches!(
                &spec.grantable,
                crate::model::CompilerGrantableCore::AlternativeCast(method)
                    if method.cast_from_zone() == Zone::Hand
                        && method.mana_cost().is_none()
                        && method.non_mana_costs().is_empty()
            ) =>
        {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power_str = strip_leading_plus_char(power_raw);
    let toughness_str = strip_leading_plus_char(toughness_raw);
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

pub fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = split_signed_pt_component(trimmed);

    if pt_component_is_x(value_text) {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

pub fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power = parse_signed_pt_component(power_raw)?;
    let toughness = parse_signed_pt_component(toughness_raw)?;
    Ok((power, toughness))
}

pub fn split_pt_modifier_components(raw: &str) -> Result<(&str, &str), CardTextError> {
    static_keyword_shapes::parse_pt_components(raw)
        .map(|components| (components.power, components.toughness))
        .ok_or_else(|| CardTextError::ParseError("missing power/toughness modifier".to_string()))
}

pub fn strip_leading_plus_char(raw: &str) -> &str {
    let trimmed = raw.trim();
    let mut chars = trimmed.chars();
    if chars.next().is_some_and(|ch| ch == '+') {
        chars.as_str()
    } else {
        trimmed
    }
}

pub fn split_signed_pt_component(trimmed: &str) -> (i32, &str) {
    let mut chars = trimmed.chars();
    match chars.next() {
        Some('+') => (1, chars.as_str()),
        Some('-' | '−') => (-1, chars.as_str()),
        _ => (1, trimmed),
    }
}

pub fn pt_component_is_x(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|ch| matches!(ch, 'x' | 'X')) && chars.next().is_none()
}

pub fn parse_devotion_increase_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let prefix = ["your", "devotion", "to", "each", "color", "and", "each", "combination", "of", "colors", "is", "increased", "by"];
    if tokens.len() <= prefix.len()
        || !tokens.iter().zip(prefix).all(|(token, word)| token.is_word(word))
    {
        return Ok(None);
    }
    let Some((amount, used)) = parse_number(&tokens[prefix.len()..]) else {
        return Ok(None);
    };
    if used != tokens.len() - prefix.len() {
        return Ok(None);
    }
    let amount = u32::try_from(amount).map_err(|_| CardTextError::ParseError("devotion increase exceeds the supported range".into()))?;
    Ok(Some(StaticAbility::devotion_increase(amount)))
}

pub fn parse_no_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(player) = crate::grammar::abilities::no_maximum_hand_size_player(tokens) {
        return Ok(Some(if player == PlayerFilter::You {
            StaticAbility::no_maximum_hand_size()
        } else {
            StaticAbility::no_maximum_hand_size_for(player)
        }));
    }
    Ok(None)
}

pub fn parse_can_be_your_commander_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_be_your_commander_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_be_commander()));
    }
    Ok(None)
}

pub fn parse_reduced_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_hand_size_line_tokens(tokens) else {
        return Ok(None);
    };
    let player = match spec.player {
        keyword_static_lines::HandSizePlayerKind::You => PlayerFilter::You,
        keyword_static_lines::HandSizePlayerKind::Opponent => PlayerFilter::Opponent,
        keyword_static_lines::HandSizePlayerKind::Any => PlayerFilter::Any,
        keyword_static_lines::HandSizePlayerKind::Chosen => PlayerFilter::ChosenPlayer,
    };
    let min_card_types_condition = if let Some(condition_tokens) = spec.condition_tokens {
        let Some((metric, threshold)) =
            parse_graveyard_metric_threshold_condition(condition_tokens)?
        else {
            return Ok(None);
        };
        if metric != crate::static_abilities::GraveyardCountMetric::CardTypes {
            return Ok(None);
        }
        threshold
    } else {
        0
    };
    Ok(Some(match spec.operation {
        keyword_static_lines::HandSizeOperation::Reduce(amount) => {
            StaticAbility::reduce_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Increase(amount) => {
            StaticAbility::increase_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Set(amount) => {
            StaticAbility::set_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::SourceCounters(counter_type) => {
            StaticAbility::maximum_hand_size_from_source_counters(player, counter_type)
        }
        keyword_static_lines::HandSizeOperation::SevenMinusGraveyardCardTypes => {
            StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                player,
                min_card_types_condition,
            )
        }
    }))
}

/// "If a spell or ability an opponent controls causes you to discard a card,
/// you may reveal that card and put it on top of your library instead of
/// putting it anywhere else." (Nephalia Academy)
pub fn parse_opponent_effect_discard_to_library_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let trimmed = trim_edge_punctuation(tokens);
    let words = crate::lexer::token_word_refs(&trimmed);
    let matched = matches!(
        words.as_slice(),
        [
            "if", "a", "spell", "or", "ability", "an", "opponent", "controls", "causes", "you",
            "to", "discard", "a", "card", "you", "may", "reveal", "that", "card", "and", "put",
            "it", "on", "top", "of", "your", "library", "instead", "of", "putting", "it",
            "anywhere", "else",
        ]
    );
    Ok(matched.then(StaticAbility::opponent_effect_discard_to_library_replacement))
}

pub fn parse_effect_discard_to_library_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_effect_discard_to_library_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::effect_discard_to_library_replacement()));
    }

    if is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::opponent_effect_discard_this_to_battlefield_replacement(),
        ));
    }

    if let Some((counter_type, count)) =
        parse_opponent_discard_this_to_battlefield_with_counters(tokens)
    {
        return Ok(Some(
            StaticAbility::opponent_effect_discard_this_to_battlefield_with_counters_replacement(
                counter_type,
                count,
            ),
        ));
    }

    Ok(None)
}

/// "If a spell or ability an opponent controls causes you to discard this
/// card, put it onto the battlefield with two +1/+1 counters on it instead of
/// putting it into your graveyard." (Dodecapod)
fn parse_opponent_discard_this_to_battlefield_with_counters(
    tokens: &[OwnedLexToken],
) -> Option<(crate::object::CounterType, u32)> {
    const HEAD: &[&str] = &[
        "if", "a", "spell", "or", "ability", "an", "opponent", "controls", "causes", "you", "to",
        "discard", "this", "card", "put", "it", "onto", "the", "battlefield", "with",
    ];
    const TAIL: &[&str] = &[
        "on", "it", "instead", "of", "putting", "it", "into", "your", "graveyard",
    ];
    let words = tokens
        .iter()
        .filter_map(|token| token.as_word().map(|_| token.parser_text()))
        .collect::<Vec<_>>();
    let middle = words.strip_prefix(HEAD)?.strip_suffix(TAIL)?;
    let [count, counter, "counter" | "counters"] = middle else {
        return None;
    };
    let count = match *count {
        "a" | "an" => 1,
        other => crate::util::parse_number_word_u32(other).or_else(|| other.parse().ok())?,
    };
    let counter_type = crate::util::parse_counter_type_word(counter)?;
    Some((counter_type, count))
}

pub fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // A flavor-word label ("Binding Contract — If you would draw a card, ...")
    // names the ability; it is not part of the replacement's event.
    let tokens =
        crate::grammar::document_shapes::parse_statement_label_strip_tokens(tokens).body_tokens;
    if is_draw_replace_exile_top_face_down_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub fn parse_draw_replacement_exile_top_and_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(count) = late_static_facts::parse_draw_replacement_exile_top_and_play_count(tokens)
    else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::draw_replacement_exile_top_and_play(
        count,
    )))
}

pub fn parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) =
        static_keyword_replacement_shapes::parse_draw_reveal_matching_rest_bottom(tokens)
    else {
        return Ok(None);
    };
    let Some(card_type) = parse_card_type(spec.card_type_word) else {
        return Ok(None);
    };
    let order = match spec.order {
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Chosen => {
            ironsmith_core::LibraryBottomOrder::ChooserChooses
        }
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Random => {
            ironsmith_core::LibraryBottomOrder::Random
        }
    };

    let mut filter = ObjectFilter::default();
    filter.card_types.push(card_type);

    Ok(Some(
        StaticAbility::draw_replacement_reveal_top_matching_to_hand_rest_bottom(
            spec.count,
            filter,
            order,
            render_token_slice(tokens),
        ),
    ))
}

pub fn parse_draw_replacement_double_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_double_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_double()));
    }

    Ok(None)
}

pub fn parse_draw_replacement_skip_empty_library_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_skip_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_skip_empty_library()));
    }

    Ok(None)
}

/// "If you would draw one or more cards, you draw that many cards plus one
/// instead." (Quantum Riddler)
pub fn parse_if_opponent_would_draw_redirect_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_redirect_draw_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let drawer = if fact.drawer_is_opponent {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    };
    Ok(Some(StaticAbility::redirect_draw_replacement(
        drawer,
        fact.except_first_of_draw_step,
        render_token_slice(tokens),
    )))
}

/// "If you would draw a card, instead look at the top three cards of your
/// library, then put one into your hand and the rest into your graveyard."
/// (Underrealm Lich): the draw is replaced by the stated effects.
pub fn parse_if_you_would_draw_instead_effects_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_draw_replacement_program(tokens) else {
        return Ok(None);
    };
    // Preserve the existing specialized readings' ownership (and their exact
    // semantics such as a count modification versus an executable program).
    if matches!(parse_conditional_draw_replacement_line(tokens), Ok(Some(_)))
        || matches!(parse_draw_replacement_double_line(tokens), Ok(Some(_)))
        || matches!(parse_draw_replace_exile_top_face_down_line(tokens), Ok(Some(_)))
        || matches!(parse_draw_extra_cards_replacement_line(tokens), Ok(Some(_)))
        || matches!(
            parse_if_opponent_would_draw_redirect_line(tokens),
            Ok(Some(_))
        )
        || matches!(
            parse_draw_replacement_skip_empty_library_line(tokens),
            Ok(Some(_))
        )
        // "If you would draw a card, draw two cards instead" is the
        // dedicated doubling replacement (CR 614.1a).
        || matches!(parse_draw_replacement_double_line(tokens), Ok(Some(_)))
        || matches!(
            parse_draw_replacement_exile_top_and_play_line(tokens),
            Ok(Some(_))
        )
        || matches!(
            parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(tokens),
            Ok(Some(_))
        )
    {
        return Ok(None);
    }
    let body = draw_replacement_drawn_card_body(&shape.body).unwrap_or_else(|| shape.body.clone());
    let effects = if shape.skip {
        Vec::new()
    } else {
        super::super::clause_support::parse_effect_sentences_lexed(&body)?
    };
    let display = render_token_slice(tokens);
    if let Some(source_tokens) = shape.cycling_source.as_ref() {
        // "If a cycling ability of another nonland card would cause you to
        // draw a card, instead ...": scoped to draws a resolving cycling
        // ability of a matching source proposes.
        if shape.optional || shape.skip {
            return Ok(None);
        }
        let cycling_source = parse_object_filter(source_tokens, false)?;
        return Ok(Some(
            StaticAbility::draw_replacement_with_effects_for_cycling_source(
                PlayerFilter::You,
                cycling_source,
                effects,
                display,
            ),
        ));
    }
    if shape.optional || shape.empty_library || shape.during_your_draw_step {
        let condition = if shape.empty_library {
            PredicateAst::ValueComparison {
                left: Value::CardsInLibrary(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            }
        } else if shape.during_your_draw_step {
            // "If you would draw a card during your draw step" (Island
            // Sanctuary): every draw proposed during your own draw step.
            PredicateAst::ActivationTiming(crate::ability::ActivationTiming::DuringYourDrawStep)
        } else {
            PredicateAst::ValueComparison {
                left: Value::Fixed(1),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(1),
            }
        };
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                condition,
                effects,
                shape.optional,
                display,
            ),
        ));
    }
    let drawer = match shape.player {
        keyword_static_lines::DrawReplacementPlayer::You => PlayerFilter::You,
        keyword_static_lines::DrawReplacementPlayer::Opponent => PlayerFilter::Opponent,
        keyword_static_lines::DrawReplacementPlayer::Any => PlayerFilter::Any,
    };
    Ok(Some(StaticAbility::draw_replacement_with_effects(
        drawer,
        shape.except_first_of_draw_step,
        effects,
        display,
    )))
}

/// The card a replaced draw would have drawn is the top card of the drawer's
/// library (CR 121.1, 614.6): "that player exiles that card face up instead"
/// (Uba Mask) reads as "that player exiles the top card of their library".
/// Exile is face up by default (CR 406.3), so "face up" adds nothing.
fn draw_replacement_drawn_card_body(body: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let words = parser_token_word_refs(body);
    let verb = words.iter().position(|word| matches!(*word, "exiles" | "exile"))?;
    let rest = match &words[verb + 1..] {
        ["that", "card", rest @ ..] | ["it", rest @ ..] => rest,
        _ => return None,
    };
    let rest = match rest {
        ["face", "up", rest @ ..] => rest,
        rest => rest,
    };
    if !rest.is_empty() {
        return None;
    }
    let subject = &words[..verb];
    let possessive = if subject.is_empty() || subject == ["you"] { "your" } else { "their" };
    let mut rewritten = crate::lexer::synthetic_word_tokens(subject);
    rewritten.extend(crate::lexer::synthetic_word_tokens([words[verb], "the", "top", "card", "of", possessive, "library"]));
    Some(rewritten)
}

pub fn parse_draw_extra_cards_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // The exact "If you would draw a card, draw two cards instead." line has
    // its own static (Thought Reflection); claiming it here too makes the
    // registry reject the line as ambiguous.
    if is_draw_replacement_double_line_lexed(tokens) {
        return Ok(None);
    }
    let Some(fact) = late_static_facts::parse_draw_extra_cards_replacement_tokens(tokens) else {
        return Ok(None);
    };
    if fact.extra == 0 {
        return Ok(None);
    }
    Ok(Some(
        StaticAbility::draw_extra_cards_replacement_with_options(
            fact.extra,
            fact.except_first_of_draw_step,
            fact.per_instruction,
            render_token_slice(tokens),
        ),
    ))
}

/// "The first time you would draw a card each turn[ except the first card you
/// draw during each of your draw steps], <effects> instead." (Reed Richards;
/// Scion of Halaster's granted "..., instead look at ...").
///
/// The replacement applies to your first draw of the turn: no card has been
/// drawn this turn yet. With the draw-step exception, the first card of your
/// draw step is never replaced and does not use up the replacement, so on
/// your turn one earlier draw is still allowed. A draw the replacement
/// itself makes is not replaced again (CR 614.5).
fn parse_first_draw_each_turn_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    const HEADER: &[&str] = &[
        "the", "first", "time", "you", "would", "draw", "a", "card", "each", "turn",
    ];
    const DRAW_STEP_EXCEPTION: &[&str] = &[
        "except", "the", "first", "card", "you", "draw", "during", "each", "of", "your", "draw",
        "steps",
    ];
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let word_at = |index: usize, word: &str| tokens.get(index).is_some_and(|token| token.is_word(word));
    if !HEADER.iter().enumerate().all(|(index, word)| word_at(index, word)) {
        return Ok(None);
    }
    let mut index = HEADER.len();
    let except_first_of_draw_step = DRAW_STEP_EXCEPTION
        .iter()
        .enumerate()
        .all(|(offset, word)| word_at(index + offset, word));
    if except_first_of_draw_step {
        index += DRAW_STEP_EXCEPTION.len();
    }
    if !tokens.get(index).is_some_and(|token| token.kind == TokenKind::Comma) {
        return Ok(None);
    }
    let mut body = trim_lexed_commas(&tokens[index + 1..]).to_vec();
    if body.first().is_some_and(|token| token.is_word("instead")) {
        body.remove(0);
    } else {
        let end = body
            .iter()
            .rposition(|token| !token.is_period())
            .map(|last| last + 1)
            .unwrap_or(0);
        if end == 0 || !body[end - 1].is_word("instead") {
            return Ok(None);
        }
        body.remove(end - 1);
    }
    if body.is_empty() {
        return Ok(None);
    }
    let effects = super::super::clause_support::parse_effect_sentences_lexed(&body)?;
    if effects.is_empty() {
        return Ok(None);
    }
    let drawn = |operator, amount| PredicateAst::ValueComparison {
        left: Value::MaxCardsDrawnThisTurn(PlayerFilter::You),
        operator,
        right: Value::Fixed(amount),
    };
    let none_drawn = drawn(crate::effect::ValueComparisonOperator::Equal, 0);
    let condition = if except_first_of_draw_step {
        PredicateAst::Or(
            Box::new(none_drawn),
            Box::new(PredicateAst::And(
                Box::new(PredicateAst::YourTurn),
                Box::new(drawn(crate::effect::ValueComparisonOperator::LessThanOrEqual, 1)),
            )),
        )
    } else {
        none_drawn
    };
    let ability =
        StaticAbility::conditional_draw_replacement(condition, effects, render_token_slice(tokens));
    Ok(Some(if except_first_of_draw_step {
        ability.with_draw_step_first_draw_exception()
    } else {
        ability
    }))
}

pub fn parse_conditional_draw_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let always = || PredicateAst::ValueComparison {
        left: Value::Fixed(1),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(1),
    };

    if let Some(ability) = parse_first_draw_each_turn_replacement_line(tokens)? {
        return Ok(Some(ability));
    }

    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if",
            "you",
            "would",
            "draw",
            "a",
            "card",
            "you",
            "may",
            "put",
            "a",
            "study",
            "counter",
            "on",
            "this",
            "enchantment",
            "instead",
        ],
    ) {
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                always(),
                vec![EffectAst::subject_verb_put_counters(
                    CounterType::Study,
                    Value::Fixed(1),
                    TargetAst::Source(None),
                    None,
                    false,
                )],
                true,
                render_token_slice(tokens),
            ),
        ));
    }

    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "you", "would", "draw", "a", "card", "you", "may", "instead", "search", "your",
            "library", "for", "a", "card", "put", "that", "card", "into", "your", "hand", "then",
            "shuffle",
        ],
    ) {
        let mut card = ObjectFilter::default();
        card.set_explicit_card_noun(true);
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                always(),
                vec![EffectAst::subject_verb_search_library(
                    card,
                    Zone::Hand,
                    PlayerAst::You,
                    PlayerAst::You,
                    crate::effect::SearchSelectionMode::Exact,
                    false,
                    None,
                    true,
                    ChoiceCount::exactly(1),
                    None,
                    None,
                    crate::effect::SearchResultReferenceSurface::ThatCard,
                    false,
                    false,
                    false,
                )],
                true,
                render_token_slice(tokens),
            ),
        ));
    }

    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "you", "would", "draw", "a", "card", "you", "may", "instead", "choose", "land",
            "or", "nonland", "and", "reveal", "cards", "from", "the", "top", "of", "your",
            "library", "until", "you", "reveal", "a", "card", "of", "the", "chosen", "kind", "put",
            "that", "card", "into", "your", "hand", "and", "put", "all", "other", "cards",
            "revealed", "this", "way", "on", "the", "bottom", "of", "your", "library", "in", "any",
            "order",
        ],
    ) {
        let mode = |label: &str, filter: ObjectFilter, suffix: &str| {
            let all_tag = crate::tag::CompilerIndexedTag::DrawReplacementAll.key_in_scope(suffix);
            let match_tag =
                crate::tag::CompilerIndexedTag::DrawReplacementMatch.key_in_scope(suffix);
            ChooseOneModeAst {
                description: label.to_string(),
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::You,
                        LibraryConsultModeAst::Reveal,
                        filter,
                        LibraryConsultStopRuleAst::FirstMatch,
                        all_tag.clone(),
                        match_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(match_tag.clone(), None),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        all_tag,
                        Some(match_tag),
                        LibraryBottomOrderAst::ChooserChooses,
                        PlayerAst::You,
                    ),
                ],
            }
        };
        let mut land = ObjectFilter {
            card_types: vec![CardType::Land],
            ..Default::default()
        };
        land.set_explicit_card_type_noun(Some(CardType::Land));
        let mut nonland = ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        };
        nonland.set_explicit_card_noun(true);
        let choose_kind = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf {
            chooser: crate::target::PlayerFilter::You,
            modes: vec![
                mode("land", land, "land"),
                mode("nonland", nonland, "nonland"),
            ],
        });
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                always(),
                vec![choose_kind],
                true,
                render_token_slice(tokens),
            ),
        ));
    }

    if is_draw_replacement_win_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::conditional_draw_replacement(
            PredicateAst::ValueComparison {
                left: Value::CardsInLibrary(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            },
            vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
            render_token_slice(tokens),
        )));
    }

    if let Some(body) =
        late_static_facts::parse_empty_library_draw_effect_replacement_tokens(tokens)
    {
        let effects = super::super::clause_support::parse_effect_sentences_lexed(body)?;
        return Ok(Some(StaticAbility::conditional_draw_replacement(
            PredicateAst::ValueComparison {
                left: Value::CardsInLibrary(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            },
            effects,
            render_token_slice(tokens),
        )));
    }

    let Some(fact) = late_static_facts::parse_conditional_draw_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let Some(no_cards_condition) =
        crate::grammar::conditions::parse_player_cards_in_hand_condition(fact.condition_tokens)
    else {
        return Ok(None);
    };
    if no_cards_condition.player != PlayerAst::You || !no_cards_condition.is_no_cards_in_hand() {
        return Ok(None);
    }

    let draw_count = fact.draw_count;
    let mut replacement_effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
            count: Value::Fixed(draw_count as i32),
        }),
    )];
    if let Some(amount) = fact.life_loss {
        replacement_effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife {
                amount: Value::Fixed(amount as i32),
            }),
        ));
    }

    let draw_amount_text = match draw_count {
        1 => "a".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        _ => draw_count.to_string(),
    };
    let draw_card_text = if draw_count == 1 { "card" } else { "cards" };
    let mut display = format!(
        "If you would draw a card while you have no cards in hand, instead you draw {draw_amount_text} {draw_card_text}"
    );
    if let Some(amount) = fact.life_loss {
        display.push_str(&format!(" and you lose {amount} life"));
    }
    display.push('.');

    Ok(Some(StaticAbility::conditional_draw_replacement(
        PredicateAst::Not(Box::new(PredicateAst::CardsInHandOrMore(1))),
        replacement_effects,
        display,
    )))
}

pub fn parse_lose_game_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = TokenWordView::new(tokens);
    if !words.parses_prefix(&["if", "you", "would", "lose"])
        || words.parse_any_word_position_from(&["game"], 4).is_none()
    {
        return Ok(None);
    }
    let Some(instead_word) = words.parse_any_word_position_from(&["instead"], 4) else {
        return Ok(None);
    };
    let Some(effect_start) = words.token_index_after_words(instead_word + 1) else {
        return Ok(None);
    };
    let effect_tokens = trim_lexed_commas(&tokens[effect_start..]);
    if effect_tokens.is_empty() {
        return Ok(None);
    }
    let effects = super::super::clause_support::parse_effect_sentences_lexed(effect_tokens)?;
    let optional = (4..instead_word).any(|idx| words.get(idx) == Some("may"));
    Ok(Some(StaticAbilityAst::LoseGameReplacement {
        effects,
        optional,
        display: render_token_slice(tokens),
    }))
}

pub fn parse_keyword_action_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_keyword_action_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);
    Ok(Some(match shape {
        keyword_static_lines::KeywordActionReplacementShape::ProliferateYouTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::You),
                vec![EffectAst::subject_verb_proliferate(Value::Fixed(2))],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ProliferateOpponentTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                vec![EffectAst::subject_verb_proliferate(Value::Fixed(2))],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    EffectAst::subject_verb_explore(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        None,
                    )),
                    EffectAst::subject_verb_explore(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        None,
                    )),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreAfterScry { value_tokens } => {
            let value_words = parser_token_word_refs(value_tokens);
            let (count, used) = parse_value_expr_words(&value_words).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                ))
            })?;
            if used != value_words.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::You,
                        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { count }),
                    ),
                    EffectAst::subject_verb_explore(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.bind(),
                        None,
                    )),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ConniveAfterDraw => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Connive,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::You,
                        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                            count: Value::Fixed(1),
                        }),
                    ),
                    EffectAst::subject_verb_connive(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                        Value::Fixed(1),
                    ),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::AssembleRiggerTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::AssembleContraption,
                ObjectFilter::default()
                    .with_subtype(crate::types::Subtype::Rigger)
                    .controlled_by(PlayerFilter::You),
                vec![EffectAst::subject_verb_emit_keyword_action(
                    crate::events::KeywordActionKind::AssembleContraption,
                    2,
                )],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::PlaneswalkAfterPlanarDeckChoice {
            count,
        } => StaticAbility::keyword_action_replacement_for_player(
            crate::events::KeywordActionKind::Planeswalk,
            PlayerFilter::You,
            vec![
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::You,
                    SubjectVerbActionAst::ReorderTopPlanarDeck { count },
                ),
                EffectAst::subject_verb_emit_keyword_action(
                    crate::events::KeywordActionKind::Planeswalk,
                    1,
                ),
            ],
            display,
        ),
        keyword_static_lines::KeywordActionReplacementShape::LearnReturnThisFromGraveyard => {
            StaticAbility::keyword_action_replacement_for_player_with_optional(
                crate::events::KeywordActionKind::Learn,
                PlayerFilter::You,
                vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Source(None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Owner,
                    false,
                    None,
                )],
                true,
                display,
            )
        }
    }))
}

pub fn parse_exile_to_countered_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::exile_to_countered_exile_instead_of_graveyard(
            spec.player,
            spec.counter_type,
        ),
    ))
}

#[cfg(test)]
mod optional_draw_replacement_regression_tests {
    use super::*;
    use crate::cards::builders::TokenActionAst;
    use crate::lexer::lex_line;

    fn parse(text: &str) -> Option<StaticAbility> {
        let tokens = lex_line(text, 0).expect("draw replacement should lex");
        parse_conditional_draw_replacement_line(&tokens)
            .expect("draw replacement parser should not error")
    }

    #[test]
    fn strict_optional_draw_replacement_families_lower_to_event_replacements() {
        for text in [
            "If you would draw a card, you may put a study counter on this enchantment instead.",
            "If you would draw a card, you may instead search your library for a card, put that card into your hand, then shuffle.",
            "If you would draw a card, you may instead choose land or nonland and reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and put all other cards revealed this way on the bottom of your library in any order.",
        ] {
            let ability =
                parse(text).unwrap_or_else(|| panic!("expected typed replacement: {text}"));
            let ironsmith_core::StaticAbilityPayload::ConditionalDrawReplacement {
                optional,
                replacement_effects,
                ..
            } = ability.payload
            else {
                panic!("expected draw replacement payload: {ability:#?}");
            };
            assert!(optional, "{text}");
            assert!(!replacement_effects.is_empty(), "{text}");
        }
    }

    #[test]
    fn similar_nonreplacement_instructions_are_not_claimed() {
        for text in [
            "Whenever you draw a card, you may put a study counter on this enchantment.",
            "If you would draw a card, search your library for a card, put that card into your hand, then shuffle.",
            "If you would draw a card, you may instead choose land and reveal the top card of your library.",
        ] {
            assert!(parse(text).is_none(), "near miss was overclaimed: {text}");
        }
    }
}

pub fn parse_exile_to_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_to_graveyard_replacement_tokens(tokens)
    else {
        return Ok(None);
    };
    let graveyard_owner = match spec.graveyard_owner {
        keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
        keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
        keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
    };
    let filter = match spec.filter_kind {
        keyword_static_lines::ExileGraveyardFilterKind::Source => ObjectFilter::source(),
        keyword_static_lines::ExileGraveyardFilterKind::AnyCardOrToken => {
            let mut filter = ObjectFilter::default();
            filter.set_explicit_card_noun(true);
            filter
        }
        // A token isn't a card (CR 108.2, 111.1): "a card" graveyard
        // replacements leave dying tokens alone.
        keyword_static_lines::ExileGraveyardFilterKind::AnyCard => {
            let mut filter = ObjectFilter::default().nontoken();
            filter.set_explicit_card_noun(true);
            filter
        }
        keyword_static_lines::ExileGraveyardFilterKind::CardYouDidntControl => {
            // Cards outside the battlefield and stack are controlled by their
            // owners, so only a permanent or spell you controlled is spared.
            let mut filter = ObjectFilter::default().nontoken();
            filter.set_explicit_card_noun(true);
            filter.controller = Some(PlayerFilter::NotYou);
            filter
        }
        // "a creature card": a token isn't a card (CR 108.2, 111.1).
        keyword_static_lines::ExileGraveyardFilterKind::CreatureCard => {
            let mut filter = ObjectFilter::creature().nontoken();
            filter.set_explicit_card_noun(true);
            filter
        }
        keyword_static_lines::ExileGraveyardFilterKind::CyclingCard => {
            let mut filter = ObjectFilter::default()
                .with_ability_marker("cycling")
                .nontoken();
            filter.set_explicit_card_noun(true);
            filter
        }
        keyword_static_lines::ExileGraveyardFilterKind::ObjectFilter => {
            match parse_object_filter(spec.filter_tokens, false) {
                // "... card" subjects exclude tokens (CR 108.2) unless the
                // filter explicitly names tokens or has alternatives.
                Ok(mut filter) => {
                    if filter.has_explicit_card_noun() && !filter.token && filter.any_of.is_empty()
                    {
                        filter.nontoken = true;
                    }
                    filter
                }
                Err(_) if crate::lexer::is_bare_card_name_phrase(spec.filter_tokens) => {
                    ObjectFilter::source()
                }
                Err(error) => return Err(error),
            }
        }
    };
    let ability = if spec.exclude_cycled {
        StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(filter, graveyard_owner)
    } else {
        StaticAbility::exile_to_exile_instead_of_graveyard(filter, graveyard_owner)
    };
    Ok(Some(ability))
}

pub fn parse_exile_would_die_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_would_die_tokens(tokens) else {
        return Ok(None);
    };
    let ability = match spec {
        keyword_static_lines::ExileWouldDieSpec::NontokenCreature {
            controller,
            exile_counter,
            follow_up_token,
            follow_up_tokens,
        } => {
            let matched_filter = match controller {
                keyword_static_lines::ReplacementPlayerKind::Any => {
                    ObjectFilter::creature().nontoken()
                }
                keyword_static_lines::ReplacementPlayerKind::You => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::You),
                keyword_static_lines::ReplacementPlayerKind::Opponent => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::Opponent),
            };
            let exile_with_counters = exile_counter
                .map(|counter_type| vec![(counter_type, 1)])
                .unwrap_or_default();
            let mut follow_up = follow_up_token
                .map(build_replacement_creature_token)
                .map(|token| vec![token])
                .unwrap_or_default();
            if !follow_up_tokens.is_empty() {
                follow_up.extend(super::super::clause_support::parse_effect_sentences_lexed(
                    &follow_up_tokens,
                )?);
            }
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                matched_filter,
                None,
                exile_with_counters,
                follow_up,
            )
        }
        keyword_static_lines::ExileWouldDieSpec::DamagedBy { victim, damaged_by } => {
            let victim = match victim {
                keyword_static_lines::ExileWouldDieVictimKind::Creature => ObjectFilter::creature(),
                keyword_static_lines::ExileWouldDieVictimKind::Permanent => {
                    ObjectFilter::permanent()
                }
            };
            StaticAbility::exile_would_die_instead_with_damage_source(victim, Some(damaged_by))
        }
        keyword_static_lines::ExileWouldDieSpec::DamagedByFilter {
            victim,
            damager_filter_tokens,
        } => {
            let victim = match victim {
                keyword_static_lines::ExileWouldDieVictimKind::Creature => ObjectFilter::creature(),
                keyword_static_lines::ExileWouldDieVictimKind::Permanent => {
                    ObjectFilter::permanent()
                }
            };
            let damager_filter = if keyword_static_lines::parse_you_controlled_source_filter_tokens(
                &damager_filter_tokens,
            ) {
                ObjectFilter::default().controlled_by(PlayerFilter::You)
            } else {
                parse_object_filter_lexed(&damager_filter_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported filtered damage source in would-die replacement (clause: '{}')",
                        render_token_slice(&damager_filter_tokens)
                    ))
                })?
            };
            StaticAbility::exile_would_die_instead_with_damage_filter_surface(
                victim,
                damager_filter,
                Some(render_token_slice(&damager_filter_tokens)),
            )
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleSource(kind) => {
            let filter = match kind {
                keyword_static_lines::SimpleSourceReplacementKind::Any => ObjectFilter::source(),
                keyword_static_lines::SimpleSourceReplacementKind::Creature => {
                    ObjectFilter::source().with_type(CardType::Creature)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Artifact => {
                    ObjectFilter::source().with_type(CardType::Artifact)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Enchantment => {
                    ObjectFilter::source().with_type(CardType::Enchantment)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Permanent => {
                    ObjectFilter::source()
                }
            };
            StaticAbility::exile_would_die_instead(filter)
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleCreature {
            controller: player,
            other,
            follow_up_tokens,
        } => {
            let player = match player {
                keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
                keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
            };
            let mut filter = ObjectFilter::creature().controlled_by(player);
            if other {
                filter = filter.other();
            }
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                filter,
                None,
                Vec::new(),
                super::super::clause_support::parse_effect_sentences_lexed(&follow_up_tokens)?,
            )
        }
    };
    Ok(Some(ability))
}

pub fn build_replacement_creature_token(
    shape: crate::model::token_definition::CreatureTokenShape,
) -> EffectAst {
    let name = shape.name.clone();
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
            name,
            definition: crate::model::token_definition::TokenDefinitionSpec::Creature(shape),
            count: Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Implicit,
            actor_surface_explicit: false,
            attached_to: None,
            tapped: false,
            attacking: false,
            attack_target_player: None,
            combat_entry: Default::default(),
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        }),
    )
}

pub fn parse_discard_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(
        static_keyword_replacement_shapes::parse_discard_or_redirect_replacement(tokens).map(
            |shape| {
                StaticAbility::discard_or_redirect_replacement(
                    ObjectFilter::default().with_type(shape.discard_type),
                    shape.redirect_zone,
                )
            },
        ),
    )
}

pub fn parse_sacrifice_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) =
        static_keyword_replacement_shapes::parse_sacrifice_or_redirect_replacement(tokens)
    else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.filter_tokens, false)?;
    Ok(Some(StaticAbility::sacrifice_or_redirect_replacement(
        filter,
        shape.count,
        shape.redirect_zone,
    )))
}

/// "As this land enters, choose a basic land type. Then you may pay 2 life. If
/// you don't, it enters tapped." (Multiversal Passage): an as-enters land type
/// choice followed by the shock-land life gate.
pub fn parse_choose_basic_land_type_then_pay_life_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [choice_tokens, then_tokens, tail_tokens] = sentences.as_slice() else {
        return Ok(None);
    };
    let Some((then, after_then)) = then_tokens.split_first() else {
        return Ok(None);
    };
    if !then.is_word("then") {
        return Ok(None);
    }
    let Some(choice) =
        crate::keyword_static::parse_choose_basic_land_type_as_enters_line(choice_tokens)?
    else {
        return Ok(None);
    };
    // The gate shares the choice sentence's "As this land enters" subject.
    let Some(enters) = choice_tokens
        .iter()
        .position(|token| token.is_any_word(&["enters", "enter"]))
    else {
        return Ok(None);
    };
    let mut gate_tokens = choice_tokens[..=enters].to_vec();
    gate_tokens.extend(after_then.iter().cloned());
    gate_tokens.extend(tail_tokens.iter().cloned());
    let Some(gate) = parse_pay_life_or_enter_tapped_line(&gate_tokens)? else {
        return Ok(None);
    };
    Ok(Some(vec![choice, gate]))
}

/// "If you would gain life, you gain twice that much life instead." (Boon
/// Reflection) / "If an opponent would lose life during your turn, they lose
/// twice that much life instead." (Bloodletter of Aclazotz)
pub fn parse_if_player_would_change_life_double_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_double_life_change_tokens(tokens) else {
        return Ok(None);
    };
    let player = match fact.player {
        late_static_facts::LifeChangePlayerFact::You => PlayerFilter::You,
        late_static_facts::LifeChangePlayerFact::Opponent => PlayerFilter::Opponent,
        late_static_facts::LifeChangePlayerFact::Any => PlayerFilter::Any,
    };
    let ability = StaticAbility::double_life_change_replacement(
        player,
        fact.loss,
        render_token_slice(tokens),
    );
    Ok(Some(if fact.during_your_turn {
        ability.with_condition(PredicateAst::YourTurn)
    } else {
        ability
    }))
}

/// "If a land is tapped for two or more mana, it produces {C} instead of any
/// other type and amount." (Damping Sphere)
pub fn parse_if_source_tapped_for_mana_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) =
        crate::grammar::effects::parse_tapped_for_amount_mana_replacement_spec_lexed(tokens)
    else {
        return Ok(None);
    };
    let mut source_filter = parse_object_filter(spec.source_tokens, false)?;
    if source_filter.zone.is_none() {
        source_filter.zone = Some(Zone::Battlefield);
    }
    parser_trace("parse_static:tapped-for-mana-replacement:matched", tokens);
    Ok(Some(StaticAbility::mana_production_replacement(
        source_filter,
        spec.minimum_amount,
        vec![spec.replacement_mana],
        render_token_slice(tokens),
    )))
}

/// "If you tap a permanent for mana, it produces three times as much of that
/// mana instead." (Nyxbloom Ancient, Mana Reflection)
pub fn parse_if_you_tap_for_mana_multiplier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = crate::grammar::effects::parse_mana_multiplier_replacement_spec_lexed(tokens)
    else {
        return Ok(None);
    };
    let mut source_filter = parse_object_filter(spec.source_tokens, false)?;
    if source_filter.zone.is_none() {
        source_filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(StaticAbility::mana_production_multiplier_replacement(
        source_filter,
        spec.factor,
        render_token_slice(tokens),
    )))
}

/// "If a nontoken creature would enter and it wasn't cast, exile it instead."
/// (Containment Priest)
pub fn parse_redirect_would_enter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_redirect_would_enter_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter(spec.filter_tokens, false)?;
    parser_trace("parse_static:redirect-would-enter:matched", tokens);
    if spec.or_no_mana_spent {
        return Ok(Some(StaticAbility::redirect_would_enter_unless_paid_cast(
            filter,
            spec.destination,
            render_token_slice(tokens),
        )));
    }
    Ok(Some(StaticAbility::redirect_would_enter(
        filter,
        spec.not_cast,
        spec.destination,
        render_token_slice(tokens),
    )))
}

/// "As this land enters, you may reveal a Plains or Island card from your
/// hand. If you don't, this land enters tapped." (Port Town, Frostboil Snarl).
pub fn parse_reveal_card_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    // The behold gate (Theorist's Sanctum) is the same two-sentence compound
    // with "choose a <subtype> you control or reveal one" as its action.
    if let Some(ability) = parse_behold_or_enters_tapped_line(tokens) {
        return Ok(Some(ability));
    }
    let Some(fact) = late_static_facts::parse_reveal_card_or_enter_tapped_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter(fact.filter_tokens, false)?;
    parser_trace("parse_static:reveal-card-etb:matched", tokens);
    Ok(Some(StaticAbility::reveal_card_or_enter_tapped(
        filter,
        fact.subject,
        fact.tail_subject,
    )))
}

pub fn parse_pay_life_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let fact = match late_static_facts::parse_pay_life_or_enter_tapped_tokens(tokens) {
        Ok(Some(fact)) => fact,
        Ok(None) => return Ok(None),
        Err(error) => {
            let clause = parser_token_word_refs(tokens).join(" ");
            let detail = match error {
                late_static_facts::PayLifeOrEnterTappedError::MissingPay => {
                    "missing 'pay' keyword in pay-life ETB clause"
                }
                late_static_facts::PayLifeOrEnterTappedError::UnsupportedPrefix => {
                    "unsupported pay-life ETB prefix"
                }
                late_static_facts::PayLifeOrEnterTappedError::MissingAmount => {
                    "missing life payment amount in pay-life ETB clause"
                }
                late_static_facts::PayLifeOrEnterTappedError::MissingIfYouDont => {
                    "unsupported pay-life ETB trailing clause (expected 'if you don't ...')"
                }
                late_static_facts::PayLifeOrEnterTappedError::UnsupportedTail => {
                    "unsupported pay-life ETB trailing clause"
                }
            };
            return Err(CardTextError::ParseError(format!(
                "{detail} (clause: '{clause}')"
            )));
        }
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(fact.amount)))
}

/// "Koh has all activated and triggered abilities of the last chosen card."
/// (CR 613.1f): two copy grants on this object. "The [last] chosen <noun>"
/// is the object this source most recently chose and remembered.
pub fn parse_copy_activated_and_triggered_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = parser_token_word_refs(&tokens);
    const MARKER: &[&str] = &["all", "activated", "and", "triggered", "abilities", "of"];
    let Some(has_idx) = words
        .iter()
        .position(|word| matches!(*word, "has" | "have"))
    else {
        return Ok(None);
    };
    if has_idx == 0 || words.get(has_idx + 1..has_idx + 1 + MARKER.len()) != Some(MARKER) {
        return Ok(None);
    }
    if crate::util::source_reference_surface_for_words(&words[..has_idx]).is_none() {
        return Ok(None);
    }
    let filter_words = &words[has_idx + 1 + MARKER.len()..];
    let filter = match filter_words {
        ["the", "last", "chosen", _] | ["the", "chosen", _] => {
            let mut filter = ObjectFilter::default();
            filter.tagged_constraints.push(crate::filter::TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::ChosenObjects.bind().into(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
            filter
        }
        _ => {
            let Some(span) = crate::lexer::TokenWordView::new(&tokens)
                .token_span_for_words(has_idx + 1 + MARKER.len(), words.len())
            else {
                return Ok(None);
            };
            match parse_object_filter(&tokens[span], false) {
                Ok(filter) => filter,
                Err(_) => return Ok(None),
            }
        }
    };
    let display = words.join(" ");
    let activated = crate::static_abilities::CopyActivatedAbilities::new(filter.clone())
        .with_exclude_source_id(true)
        .with_display(display.clone());
    let triggered =
        crate::static_abilities::CopyTriggeredAbilities::new(filter).with_display(display);
    Ok(Some(vec![
        StaticAbilityAst::Static(StaticAbility::copy_activated_abilities(activated)),
        StaticAbilityAst::Static(StaticAbility::copy_triggered_abilities(triggered)),
    ]))
}

pub fn parse_copy_activated_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(fact) = late_static_facts::parse_copy_activated_abilities_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, fact.marker_token)
    {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..fact.marker_token]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = if crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(&subject_tokens),
        &["this"],
    ) {
        AnthemSubjectAst::Source
    } else {
        match parse_anthem_subject(&subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };

    let filter_tokens =
        trim_edge_punctuation(&tokens[fact.filter_start_token..fact.filter_end_token]);
    let filter_tokens =
        strip_leading_token_words_any(&filter_tokens, &["all", "each"]).to_vec();
    let exclude_mana_abilities = fact.exclude_mana_abilities;
    let force_once_each_turn = fact.once_each_turn_word_start.is_some();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let source_reference = crate::util::source_reference_surface_for_words(
        &parser_token_word_refs(&filter_tokens),
    )
    .is_some();
    let mut filter = if source_reference {
        // "the loyalty abilities of Kasmina": the donor is this object.
        ObjectFilter::source()
    } else {
        match parse_object_filter(&filter_tokens, false) {
            Ok(filter) => filter,
            Err(_) => return Ok(None),
        }
    };
    if fact.exclude_source_name {
        // "that don't have the same name as this creature" is carried by
        // `exclude_source_name`; a name relation left on the filter (whose
        // polarity the filter grammar does not keep) would contradict it.
        filter.tagged_constraints.retain(|constraint| {
            !matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    | crate::filter::TaggedOpbjectRelation::DifferentNameFromTagged
            )
        });
    }

    let counter = match filter.with_counter {
        Some(crate::filter::CounterConstraint::Typed(counter_type)) => Some(counter_type),
        _ => None,
    };

    let display_words = copy_activated_abilities_display_words(&clause_words);
    // A grant renders its own recipients, so its ability text is only the
    // half after the `has`/`have` marker. A source-subject line keeps the
    // complete authored clause.
    let display_head_start = match &subject {
        AnthemSubjectAst::Source => 0,
        AnthemSubjectAst::Filter(_) => copy_activated_display_index_for_original_word(
            &clause_words,
            fact.marker_word_start + 1,
        ),
    };
    let display_words = display_words.get(display_head_start..).unwrap_or_default();
    let display = if force_once_each_turn {
        let display_tail_start = fact
            .once_each_turn_word_start
            .map(|start| copy_activated_display_index_for_original_word(&clause_words, start))
            .and_then(|start| start.checked_sub(display_head_start));
        if let Some(start) = display_tail_start {
            format!(
                "{}. You may activate each of those abilities only once each turn",
                display_words[..start].join(" ").trim()
            )
        } else {
            display_words.join(" ")
        }
    } else {
        display_words.join(" ")
    };

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(fact.exclude_source_name)
        .with_exclude_source_id(true)
        .with_exclude_mana_abilities(exclude_mana_abilities)
        .with_display(display);
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if fact.only_loyalty {
        ability = ability.with_only_loyalty();
    }
    if force_once_each_turn {
        ability = ability.with_once_each_turn();
    }

    let ability = StaticAbility::copy_activated_abilities(ability);
    let ast = match subject {
        AnthemSubjectAst::Source => match condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition,
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(subject_filter) => StaticAbilityAst::GrantStaticAbility {
            filter: subject_filter,
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition,
        },
    };

    Ok(Some(ast))
}

pub fn copy_activated_abilities_display_words<'a>(clause_words: &[&'a str]) -> Vec<&'a str> {
    let mut display_words = Vec::with_capacity(clause_words.len());
    for (idx, word) in clause_words.iter().copied().enumerate() {
        if copy_activated_should_skip_display_word(clause_words, idx, word) {
            continue;
        }
        display_words.push(word);
    }
    display_words
}

pub fn copy_activated_display_index_for_original_word(
    clause_words: &[&str],
    original_idx: usize,
) -> usize {
    clause_words
        .iter()
        .enumerate()
        .take(original_idx)
        .filter(|(idx, word)| !copy_activated_should_skip_display_word(clause_words, *idx, word))
        .count()
}

pub fn copy_activated_should_skip_display_word(
    clause_words: &[&str],
    idx: usize,
    word: &str,
) -> bool {
    idx >= 2
        && word == clause_words[idx - 1]
        && clause_words[idx - 2] == "this"
        && copy_activated_display_source_noun(word)
}

pub fn copy_activated_display_source_noun(word: &str) -> bool {
    matches!(word, "card" | "permanent" | "source" | "spell")
        || parse_card_type(word).is_some()
        || parse_subtype_flexible(word).is_some()
}

/// CR 609.4b: the permission changes only how mana may be spent to cast the
/// matching spells.
fn mana_spend_any_color_to_cast_permission(
    player: keyword_static_lines::ManaSpendPlayerKind,
    filter_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(crate::effect::ManaSpendPermission, String), CardTextError> {
    let player = match player {
        keyword_static_lines::ManaSpendPlayerKind::You => PlayerFilter::You,
        keyword_static_lines::ManaSpendPlayerKind::Any => PlayerFilter::Any,
    };
    let mut filter = parse_object_filter(filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported mana spend cast filter (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    filter.zone = None;
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    Ok((
        crate::effect::ManaSpendPermission::any_color_for_casting_matching(player, filter),
        clause_words.join(" "),
    ))
}

/// Only mana of `symbol` converts, and only for the source's own
/// activation costs (CR 609.4b).
fn mana_spend_symbol_for_source_activation_permission(
    symbol: ManaSymbol,
    clause_words: &[&str],
) -> (crate::effect::ManaSpendPermission, String) {
    let mut permission =
        crate::effect::ManaSpendPermission::any_color_for_activation(PlayerFilter::You, ObjectFilter::source());
    permission.mode = ironsmith_core::value_model::ManaSpendMode::Normal;
    permission.any_color_mana_symbol = Some(symbol);
    (permission, clause_words.join(" "))
}

pub fn parse_spend_mana_as_any_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_mana_spend_permission_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);
    let (permission, display) = match shape {
        keyword_static_lines::ManaSpendPermissionShape::SymbolAsAnyColorOtherAsColorless {
            symbol,
        } => {
            let symbol_text = match symbol {
                ManaSymbol::White => "white",
                ManaSymbol::Blue => "blue",
                ManaSymbol::Black => "black",
                ManaSymbol::Red => "red",
                ManaSymbol::Green => "green",
                _ => unreachable!("typed grammar only returns colored mana symbols"),
            };
            (
                crate::effect::ManaSpendPermission::mana_symbol_as_any_color_other_as_colorless(
                    PlayerFilter::You,
                    symbol,
                ),
                format!(
                    "You may spend {symbol_text} mana as though it were mana of any color. You may spend other mana only as though it were colorless mana"
                ),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyTypeToCast { filter_tokens } => {
            let filter = parse_object_filter(filter_tokens, false)
                .map(|mut filter| {
                    filter.zone = None;
                    filter.stack_kind = None;
                    filter.has_mana_cost = false;
                    filter
                })
                .map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported mana spend cast filter (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
            (
                crate::effect::ManaSpendPermission::any_type_for_casting_matching(
                    PlayerFilter::You,
                    filter,
                ),
                clause_words.join(" "),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyTypeForSourceActivation { subject_tokens } => {
            let subject = parser_token_word_refs(subject_tokens);
            if crate::util::source_reference_surface_for_possessive_words(&subject).is_none() {
                return Ok(None);
            }
            let mut permission = crate::effect::ManaSpendPermission::any_color_for_activation(
                PlayerFilter::Any, ObjectFilter::source(),
            );
            permission.mode = ironsmith_core::value_model::ManaSpendMode::AnyType;
            (permission, clause_words.join(" "))
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyColorToCast {
            player,
            filter_tokens,
        } => mana_spend_any_color_to_cast_permission(player, filter_tokens, &clause_words)?,
        keyword_static_lines::ManaSpendPermissionShape::SymbolAsAnyColorForSourceActivation {
            symbol,
        } => mana_spend_symbol_for_source_activation_permission(symbol, &clause_words),
        keyword_static_lines::ManaSpendPermissionShape::SymbolAsSymbol { symbol, as_symbol } => (
            crate::effect::ManaSpendPermission::mana_symbol_as(PlayerFilter::You, symbol, as_symbol),
            clause_words.join(" "),
        ),
        keyword_static_lines::ManaSpendPermissionShape::AnyColor {
            player,
            activation_filter_tokens,
            source_activation_only,
        } => {
            let player = match player {
                keyword_static_lines::ManaSpendPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ManaSpendPlayerKind::Any => PlayerFilter::Any,
            };
            let permission = if source_activation_only {
                crate::effect::ManaSpendPermission::any_color_for_activation(
                    player.clone(),
                    ObjectFilter::source(),
                )
            } else if let Some(filter_tokens) = activation_filter_tokens {
                let filter = match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => filter,
                    Err(_) => return Ok(None),
                };
                crate::effect::ManaSpendPermission::any_color_for_activation(player.clone(), filter)
            } else {
                crate::effect::ManaSpendPermission::any_color(player.clone())
            };
            let display = if player == PlayerFilter::Any {
                "Players may spend mana as though it were mana of any color".to_string()
            } else {
                clause_words.join(" ")
            };
            (permission, display)
        }
    };

    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::mana_spend_permission(permission, display),
    )))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn passive_any_type_permission_is_limited_to_source_activations() {
        let tokens = lex_line("Mana of any type can be spent to activate this creature's abilities.", 0)
            .expect("permission should lex");
        let ability = parse_spend_mana_as_any_color_line(&tokens)
            .expect("permission should not hard-error")
            .expect("permission should parse");
        let StaticAbilityAst::Static(ability) = ability else {
            panic!("expected a static permission");
        };
        let ironsmith_core::StaticAbilityPayload::ManaSpendPermission { permission, .. } = ability.payload else {
            panic!("expected typed mana spending permission");
        };
        assert_eq!(permission.mode, ironsmith_core::value_model::ManaSpendMode::AnyType);
        assert_eq!(permission.player, PlayerFilter::Any);
        assert_eq!(permission.scope, ironsmith_core::value_model::ManaSpendScope::ActivationCostsOf(ObjectFilter::source()));

        let unrelated = lex_line("Mana of any type can be spent to activate other creatures' abilities.", 0)
            .expect("unrelated permission should lex");
        assert!(parse_spend_mana_as_any_color_line(&unrelated).unwrap().is_none());
    }

    #[test]
    fn spell_reduction_and_cant_be_countered_keeps_both_typed_clauses() {
        let tokens = lex_line(
            "Spells with flash you cast cost {1} less to cast and can't be countered.",
            0,
        )
        .expect("compound spell rule should lex");
        let abilities = parse_spells_cost_reduction_and_cant_be_countered_line(&tokens)
            .expect("compound spell rule should not hard-error")
            .expect("compound spell rule should parse");
        let [reduction, restriction] = abilities.as_slice() else {
            panic!("expected reduction plus restriction, got {abilities:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &reduction.payload
        else {
            panic!("expected typed cost reduction, got {reduction:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::RuleRestriction {
            restriction: crate::effect::Restriction::BeCountered(protected),
            additional_restrictions,
            ..
        } = &restriction.payload
        else {
            panic!("expected typed counter restriction, got {restriction:#?}");
        };
        assert!(additional_restrictions.is_empty());
        assert_eq!(protected, &reduction.filter);

        let unrelated = lex_line(
            "Spells with flash you cast cost {1} less to cast and have ward {1}.",
            0,
        )
        .expect("unrelated rule should lex");
        assert!(
            parse_spells_cost_reduction_and_cant_be_countered_line(&unrelated)
                .expect("unrelated rule should not hard-error")
                .is_none()
        );
    }

    #[test]
    fn source_graveyard_dynamic_surcharge_lowers_to_permission_and_counted_tax() {
        let tokens = lex_line(
            "You may cast this creature from your graveyard if you pay {1} more to cast it for each other creature card in your graveyard.",
            0,
        )
        .expect("source graveyard surcharge should lex");
        let abilities = parse_source_graveyard_dynamic_surcharge_line(&tokens)
            .expect("source graveyard surcharge should not hard-error")
            .expect("source graveyard surcharge should parse");
        let [
            StaticAbilityAst::Static(permission),
            StaticAbilityAst::Static(tax),
        ] = abilities.as_slice()
        else {
            panic!("expected a permission and source tax, got {abilities:#?}");
        };

        let ironsmith_core::StaticAbilityPayload::Grants(grant) = &permission.payload else {
            panic!("expected typed grant, got {permission:#?}");
        };
        assert!(matches!(
            &grant.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
        ));
        assert_eq!(grant.zone, Zone::Graveyard);
        assert_eq!(grant.beneficiary, PlayerFilter::You);
        assert!(grant.filter.source);
        assert!(matches!(
            grant.filter.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(ref surface))
                if surface == "this creature"
        ));

        let ironsmith_core::StaticAbilityPayload::CostIncrease(increase) = &tax.payload else {
            panic!("expected dynamic cost increase, got {tax:#?}");
        };
        assert!(increase.filter.source);
        let Value::Count(counted) = increase.amount.unhinted() else {
            panic!("expected counted dynamic tax, got {:#?}", increase.amount);
        };
        assert_eq!(counted.zone, Some(Zone::Graveyard));
        assert_eq!(counted.owner, Some(PlayerFilter::You));
        assert_eq!(counted.card_types, vec![CardType::Creature]);
        assert!(counted.other);

        let plain = lex_line("You may cast this creature from your graveyard.", 0)
            .expect("plain permission should lex");
        assert!(
            parse_source_graveyard_dynamic_surcharge_line(&plain)
                .expect("plain permission should not hard-error")
                .is_none()
        );
    }

    #[test]
    fn global_colorless_line_lowers_with_typed_multi_domain_surface() {
        let tokens = lex_line(
            "All cards that aren't on the battlefield, spells, and permanents are colorless.",
            0,
        )
        .expect("global colorless line should lex");
        let ability = parse_all_cards_spells_permanents_colorless_line(&tokens)
            .expect("global colorless line should not hard-error")
            .expect("global colorless line should parse");
        let ironsmith_core::StaticAbilityPayload::MakeColorless(filter) = &ability.payload else {
            panic!("expected MakeColorless payload, got {ability:#?}");
        };
        assert_eq!(
            filter.global_characteristic_domain_surface(),
            Some(
                ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents
            )
        );
        assert_eq!(filter, &ObjectFilter::default());
    }

    #[test]
    fn animation_bundle_accepts_a_quoted_granted_trigger() {
        let tokens = lex_line(
            "During your turn, each non-Equipment artifact and non-Aura enchantment you control \
             with mana value 4 or greater is a 4/4 Elemental creature in addition to its other \
             types and has indestructible, haste, and \"Whenever this creature deals combat \
             damage to a player, draw a card.\"",
            0,
        )
        .expect("compound animation should lex");
        let abilities = parse_filter_is_pt_creature_in_addition_and_has_line(&tokens)
            .expect("compound animation should not hard-error")
            .expect("quoted granted trigger should not mask the animation parser");

        assert_eq!(abilities.len(), 6, "{abilities:#?}");
        let debug = format!("{abilities:#?}");
        assert!(debug.contains("Triggered"), "{debug}");
        assert!(debug.contains("action: Draw("), "{debug}");
    }

    #[test]
    fn conditioned_source_animation_without_a_grant_is_structural() {
        let tokens = lex_line(
            "As long as this isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.",
            0,
        )
        .expect("source animation should lex");
        let abilities = parse_filter_is_pt_creature_in_addition_line(&tokens)
            .expect("source animation should not hard-error")
            .expect("source animation should parse");

        assert_eq!(abilities.len(), 3, "{abilities:#?}");
        let debug = format!("{abilities:#?}");
        assert!(debug.contains("AddCardTypes"), "{debug}");
        assert!(debug.contains("SetBasePowerToughness"), "{debug}");
        assert!(debug.contains("Insect"), "{debug}");
        assert!(debug.contains("SourceIsInZone"), "{debug}");
        assert!(debug.contains("Not("), "{debug}");
    }

    #[test]
    fn plural_creature_type_addition_is_a_static_grant() {
        let tokens = lex_line(
            "Creatures you control are Slivers in addition to their other creature types.",
            0,
        )
        .expect("creature-type addition should lex");
        let abilities = parse_subject_are_card_types_in_addition_to_their_other_types_line(&tokens)
            .expect("static type addition should not hard-error")
            .expect("static type addition should parse");

        assert_eq!(abilities.len(), 1, "{abilities:#?}");
        assert_eq!(abilities[0].id(), StaticAbilityId::AddSubtypes);
        let ironsmith_core::StaticAbilityPayload::AddSubtypes { filter, subtypes } =
            &abilities[0].payload
        else {
            panic!("expected a typed add-subtypes payload, got {abilities:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(subtypes, &[crate::types::Subtype::Sliver]);
    }

    #[test]
    fn supported_keyword_marker_uses_token_shapes_for_crew_markers() {
        for line in [
            "This creature crews Vehicles using its toughness rather than its power.",
            "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
            "You may remove a loyalty counter from a planeswalker you control rather than pay this creature's crew cost.",
        ] {
            let tokens = lex_line(line, 0).expect("marker line should lex");
            assert!(
                supported_keyword_marker_tokens(&tokens),
                "{line} should be recognized through token shapes"
            );
        }
    }

    #[test]
    fn pt_modifier_parsers_use_char_signs() {
        assert_eq!(parse_pt_modifier("+2/-1").unwrap(), (2, -1));
        assert_eq!(parse_pt_modifier("2/+3").unwrap(), (2, 3));
        assert_eq!(
            parse_pt_modifier_values("−X/+2").unwrap(),
            (Value::XTimes(-1), Value::Fixed(2))
        );
    }

    #[test]
    fn early_static_ability_parser_uses_parser_token_words() {
        for line in [
            "X can't be greater than the number of players in the game.",
            "This creature can't attack unless you've cast a creature spell this turn.",
            "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated.",
        ] {
            let tokens = lex_line(line, 0).expect("static line should lex");
            assert!(
                parse_static_ability_ast_line_early_lexed(&tokens)
                    .expect("early static line should parse")
                    .is_some(),
                "{line} should match through parser token words"
            );
        }
    }

    #[test]
    fn play_permission_keeps_cast_this_way_entry_counter_rider() {
        let tokens = lex_line(
            "You may play lands and cast Mutant, Ninja, or Turtle spells from the top of your library. If you cast a creature spell this way, that creature enters with an additional +1/+1 counter on it.",
            0,
        )
        .expect("play permission should lex");
        let ability = parse_play_from_permission_with_enter_counter_this_way_line(&tokens)
            .expect("play permission should not hard-error")
            .expect("play permission should keep its entry-counter rider");
        let debug = format!("{ability:#?}");

        assert!(debug.contains("PlayFrom"), "{debug}");
        assert!(debug.contains("cast_this_way_grants"), "{debug}");
        assert!(debug.contains("cast_this_way_filter"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    }

    #[test]
    fn source_linked_exile_permission_keeps_owner_subtype_and_finality_rider() {
        let tokens = lex_line(
            "You may cast Dinosaur creature spells from among cards you own exiled with this creature. If you cast a spell this way, that creature enters with a finality counter on it.",
            0,
        )
        .expect("source-linked exile permission should lex");
        let ability = parse_play_from_permission_with_enter_counter_this_way_line(&tokens)
            .expect("source-linked exile permission should not hard-error")
            .expect("source-linked exile permission should keep its finality rider");
        let ironsmith_core::StaticAbilityPayload::Grants(spec) = &ability.payload else {
            panic!("expected a grant ability, got {ability:#?}");
        };

        assert_eq!(spec.zone, crate::zone::Zone::Exile);
        assert_eq!(spec.filter.zone, Some(crate::zone::Zone::Exile));
        assert_eq!(spec.filter.owner, Some(PlayerFilter::You));
        assert_eq!(spec.filter.card_types, vec![CardType::Creature]);
        assert!(
            spec.filter
                .subtypes
                .contains(&crate::types::Subtype::Dinosaur)
        );
        assert!(spec.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(spec.cast_this_way_grants.iter().any(|grant| {
            grant.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters
        }));
        assert_eq!(
            spec.display(),
            "You may cast Dinosaur creature spells from among cards you own exiled with this creature. If you cast a spell this way, that creature enters with a finality counter on it"
        );
    }

    #[test]
    fn scorched_ruins_entry_payment_is_a_typed_sacrifice_replacement() {
        let definition = ironsmith_compiler_lowering::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Scorched Ruins",
        )
            .card_types(vec![CardType::Land])
            .parse_text(
                "If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.\n{T}: Add {C}{C}{C}{C}.",
            )
            .expect("Scorched Ruins should parse through the static replacement family");
        let replacement = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::SacrificeOrRedirectReplacement =>
                {
                    Some(static_ability)
                }
                _ => None,
            })
            .expect("Scorched Ruins should retain the typed replacement");
        let ironsmith_core::StaticAbilityPayload::SacrificeOrRedirectReplacement {
            filter,
            count,
            redirect_zone,
        } = &replacement.payload
        else {
            panic!("expected sacrifice-or-redirect payload: {replacement:#?}");
        };

        assert_eq!(*count, 2);
        assert_eq!(*redirect_zone, Zone::Graveyard);
        assert_eq!(filter.card_types, vec![CardType::Land]);
        assert!(filter.untapped);
        assert_eq!(
            replacement.display(),
            "sacrifice or redirect replacement",
            "the core label remains an implementation label; runtime rendering owns Oracle text"
        );
        assert!(definition.abilities.iter().any(|ability| {
            matches!(&ability.kind, crate::ability::AbilityKind::Activated(activated)
                if format!("{activated:#?}").matches("Colorless").count() >= 4)
        }));
    }

    #[test]
    fn parse_keyword_action_replacement_static_line() {
        let tokens =
            lex_line("If you would proliferate, proliferate twice instead.", 0).expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("keyword-action replacement parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|ability| ability.id() == StaticAbilityId::KeywordActionReplacement),
            "expected keyword-action replacement static ability, got {parsed:?}"
        );
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("static ability line parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|abilities| abilities.iter().any(
                    |ability| matches!(ability, StaticAbilityAst::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::KeywordActionReplacement)
                )),
            "expected static line parser to preserve keyword-action replacement, got {parsed:?}"
        );

        let tokens = lex_line(
            "If a Rigger you control would assemble a Contraption, it assembles two Contraptions instead.",
            0,
        )
        .expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("assemble replacement parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|ability| ability.id() == StaticAbilityId::KeywordActionReplacement),
            "expected typed assemble replacement static ability, got {parsed:?}"
        );

        let tokens = lex_line(
            "If you would planeswalk, instead look at the top two cards of your planar deck, put one on the bottom of your planar deck and the other on top, then planeswalk.",
            0,
        )
        .expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("planeswalk replacement parser should not hard-error")
            .expect("planeswalk replacement should lower");
        let debug = format!("{parsed:?}");
        assert_eq!(parsed.id(), StaticAbilityId::KeywordActionReplacement);
        assert!(debug.contains("performer_filter: Some(You)"), "{debug}");
        assert!(debug.contains("ReorderTopPlanarDeck"), "{debug}");
        assert!(debug.contains("Planeswalk"), "{debug}");
    }

    #[test]
    fn dynamic_cost_other_shapes_prefer_typed_turn_history_counts() {
        for (text, expected_query) in [
            (
                "less to cast for each opponent who was dealt damage this turn.",
                "PlayersDealtDamage",
            ),
            (
                "less to cast for each card you've cycled or discarded this turn.",
                "DiscardedOrCycled",
            ),
            (
                "less to cast for each creature you attacked with this turn.",
                "CreaturesAttackedWith",
            ),
            (
                "for each of your opponents who lost life this turn.",
                "PlayersLostLife",
            ),
        ] {
            let tokens = lex_line(text, 0).expect("dynamic cost text should lex");
            let value = parse_dynamic_cost_modifier_value(&tokens)
                .expect("dynamic cost should not hard-error")
                .expect("dynamic cost should produce a value");
            let debug = format!("{value:?}");
            assert!(
                debug.contains("TurnHistoryCount")
                    && debug.contains(expected_query)
                    && debug.contains("ForEach"),
                "expected typed for-each turn-history value for {text}, got {debug}"
            );
        }
    }

    #[test]
    fn dynamic_for_each_commander_cast_count_precedes_generic_object_count() {
        let tokens = lex_line(
            "for each time you've cast your commander from the command zone this game.",
            0,
        )
        .expect("commander-count text should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic count should not hard-error")
            .expect("dynamic count should produce a value");
        assert_eq!(
            value,
            Value::CommanderCastCount(PlayerFilter::You),
            "the narrow command-zone history value must win before generic object counting"
        );
    }

    #[test]
    fn dynamic_attacked_opponent_cost_uses_player_count_value() {
        let tokens = lex_line(
            "This spell costs {1} less to cast for each opponent you're attacking.",
            0,
        )
        .expect("attacked-opponent reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("attacked-opponent reduction should not hard-error")
            .expect("attacked-opponent reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
            &ability.payload
        else {
            panic!("expected this-spell reduction payload: {ability:#?}");
        };

        assert!(
            reduction.amount.has_surface_hint(ValueSurfaceHint::ForEach),
            "{:#?}",
            reduction.amount
        );
        assert_eq!(reduction.amount.unhinted(), &Value::PlayersBeingAttacked);
    }

    #[test]
    fn dynamic_discarded_this_way_cost_retains_typed_action() {
        let direct_words = ["for", "each", "card", "discarded", "this", "way"];
        let (direct, used) = parse_for_each_count_value_words(&direct_words)
            .expect("shared count grammar should parse discarded-this-way");
        assert_eq!(used, direct_words.len());
        assert!(
            matches!(direct.unhinted(), Value::PendingPriorEffectMetric(_)),
            "shared count grammar erased the discarded action: {direct:#?}"
        );

        let tokens =
            lex_line("for each card discarded this way.", 0).expect("discard count should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic count should not hard-error")
            .expect("dynamic count should produce a value");
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected a typed discarded-card action count, got {value:#?}");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Discarded)
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
        assert!(
            query
                .filter
                .as_ref()
                .is_some_and(|filter| { filter.union_surface.explicit_card_noun() })
        );
    }

    #[test]
    fn dynamic_sacrificed_this_way_cost_retains_typed_action_and_for_each_surface() {
        let tokens = lex_line("for each permanent sacrificed this way.", 0)
            .expect("sacrifice count should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic count should not hard-error")
            .expect("dynamic count should produce a value");
        assert!(value.has_surface_hint(ValueSurfaceHint::ForEach));
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected a typed sacrificed-permanent count, got {value:#?}");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Sacrificed)
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
    }

    #[test]
    fn compound_this_spell_reduction_keeps_each_scaled_dynamic_basis() {
        let tokens = lex_line(
            "This spell costs {2} less to cast for each permanent sacrificed this way and {2} less to cast for each other artifact or creature you've sacrificed this turn.",
            0,
        )
        .expect("compound reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("compound reduction should not hard-error")
            .expect("compound reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
            &ability.payload
        else {
            panic!("expected this-spell reduction payload: {ability:#?}");
        };
        let debug = format!("{:#?}", reduction.amount);
        assert!(
            debug.contains("PendingPriorEffectMetric")
                && debug.contains("Sacrificed")
                && debug.contains("TurnHistoryCount"),
            "expected both sacrifice bases to remain typed, got {debug}"
        );
        assert_eq!(
            debug.matches("PendingPriorEffectMetric").count(),
            2,
            "the first {{2}} reduction should scale its prior-effect basis twice: {debug}"
        );
        assert_eq!(
            debug.matches("TurnHistoryCount").count(),
            2,
            "the second {{2}} reduction should scale its turn-history basis twice: {debug}"
        );
    }

    #[test]
    fn specialized_card_types_among_cost_value_precedes_history_fallback() {
        let tokens = lex_line(
            "less to cast for each card type among permanents you've sacrificed this turn.",
            0,
        )
        .expect("card-types-among cost text should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic cost should not hard-error")
            .expect("dynamic cost should produce a value");
        assert!(
            matches!(
                value.unhinted(),
                Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SacrificedCardTypes {
                    player: PlayerFilter::You,
                    ..
                })
            ),
            "expected specialized card-types-among value, got {value:?}"
        );
    }

    #[test]
    fn shared_characteristic_cost_reduction_keeps_candidate_intersection() {
        let tokens = lex_line(
            "Spells you cast cost {1} less to cast for each card type they share with cards exiled with this creature.",
            0,
        )
        .expect("shared-characteristic reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("shared-characteristic reduction should not hard-error")
            .expect("shared-characteristic reduction should parse");
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("expected shared cost-reduction payload: {ability:#?}");
        };
        let intersection = reduction
            .characteristic_intersection
            .as_ref()
            .expect("expected typed characteristic intersection");
        assert_eq!(
            intersection.characteristic,
            ironsmith_core::ObjectCharacteristic::CardType
        );
        assert_eq!(
            intersection.comparison_surface.as_deref(),
            Some("cards exiled with this creature")
        );
        assert!(
            intersection
                .comparison
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::SourceExiled.as_str()),
            "{:#?}",
            intersection.comparison
        );
    }

    #[test]
    fn self_cost_reductions_keep_for_each_player_and_disjoint_zone_counts() {
        let opponent_tokens = lex_line(
            "This spell costs {1} less to cast for each opponent you have.",
            0,
        )
        .expect("opponent-count reduction should lex");
        let opponent_ability = parse_spells_cost_modifier_line(&opponent_tokens)
            .expect("opponent-count reduction should not hard-error")
            .expect("opponent-count reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(opponent_reduction) =
            &opponent_ability.payload
        else {
            panic!("expected this-spell reduction payload: {opponent_ability:#?}");
        };
        assert!(
            opponent_reduction
                .amount
                .has_surface_hint(ValueSurfaceHint::ForEach)
        );
        assert_eq!(
            opponent_reduction.amount.unhinted(),
            &Value::CountPlayers(PlayerFilter::Opponent)
        );

        let cave_tokens = lex_line(
            "This spell costs {1} less to cast for each Cave you control and each Cave card in your graveyard.",
            0,
        )
        .expect("multi-zone Cave reduction should lex");
        let cave_ability = parse_spells_cost_modifier_line(&cave_tokens)
            .expect("multi-zone Cave reduction should not hard-error")
            .expect("multi-zone Cave reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(cave_reduction) =
            &cave_ability.payload
        else {
            panic!("expected this-spell reduction payload: {cave_ability:#?}");
        };
        assert!(
            cave_reduction
                .amount
                .has_surface_hint(ValueSurfaceHint::ForEach),
            "{:#?}",
            cave_reduction.amount
        );
        let Value::Count(cave_filter) = cave_reduction.amount.unhinted() else {
            panic!("expected a Cave object count: {:#?}", cave_reduction.amount);
        };
        assert_eq!(cave_filter.any_of.len(), 2, "{cave_filter:#?}");
        assert!(
            cave_filter
                .any_of
                .iter()
                .any(|arm| arm.zone == Some(Zone::Battlefield)),
            "{cave_filter:#?}"
        );
        assert!(
            cave_filter
                .any_of
                .iter()
                .any(|arm| arm.zone == Some(Zone::Graveyard)),
            "{cave_filter:#?}"
        );
    }

    fn parsed_spell_cost_filter(line: &str) -> ObjectFilter {
        let tokens = lex_line(line, 0).expect("spell-cost line should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("spell-cost parser should not hard-error")
            .expect("spell-cost line should be recognized");
        match ability.payload {
            ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => reduction.filter,
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => {
                reduction.filter
            }
            other => panic!("expected a shared spell-cost reduction, got {other:?}"),
        }
    }

    #[test]
    fn chosen_type_spell_cost_filters_survive_actor_word_order_variants() {
        for (line, creature_only) in [
            (
                "Spells you cast of the chosen type cost {1} less to cast.",
                false,
            ),
            (
                "Creature spells you cast of the chosen type cost {1} less to cast.",
                true,
            ),
            (
                "Spells of the chosen type you cast cost {W}{U}{B}{R}{G} less to cast.",
                false,
            ),
            (
                "Creature spells of the chosen type cost {2} less to cast.",
                true,
            ),
        ] {
            let filter = parsed_spell_cost_filter(line);
            assert!(filter.chosen_creature_type, "{line}: {filter:#?}");
            assert_eq!(
                filter.card_types.contains(&CardType::Creature),
                creature_only,
                "{line}: {filter:#?}"
            );
            if line.contains("you cast") {
                assert_eq!(filter.cast_by, Some(PlayerFilter::You), "{line}");
            }
        }
    }

    #[test]
    fn explicit_chosen_card_type_cost_filter_and_unrestricted_control_stay_distinct() {
        let chosen = parsed_spell_cost_filter(
            "Spells of the chosen card type you cast cost {1} less to cast.",
        );
        assert!(chosen.chosen_card_type, "{chosen:#?}");
        assert_eq!(chosen.cast_by, Some(PlayerFilter::You));

        let unrestricted =
            parsed_spell_cost_filter("Creature spells you cast cost {1} less to cast.");
        assert!(!unrestricted.chosen_creature_type, "{unrestricted:#?}");
        assert!(!unrestricted.chosen_card_type, "{unrestricted:#?}");
        assert_eq!(unrestricted.card_types, vec![CardType::Creature]);
        assert_eq!(unrestricted.cast_by, Some(PlayerFilter::You));
    }

    #[test]
    fn permanent_spell_cost_filter_keeps_adventure_as_a_relative_characteristic() {
        let filter = parsed_spell_cost_filter(
            "Permanent spells you cast that have an Adventure cost {1} less to cast.",
        );

        assert!(filter.has_all_permanent_card_types(), "{filter:#?}");
        assert_eq!(filter.subtypes, [Subtype::Adventure]);
        assert_eq!(filter.cast_by, Some(PlayerFilter::You));

        let adventure_only =
            parsed_spell_cost_filter("Adventure spells you cast cost {1} less to cast.");
        assert!(adventure_only.card_types.is_empty(), "{adventure_only:#?}");
        assert_eq!(adventure_only.subtypes, [Subtype::Adventure]);
    }

    #[test]
    fn colored_spell_cost_modifiers_preserve_exact_per_target_scaling() {
        let reduction_tokens =
            lex_line("Spells you cast cost {W} less to cast for each target.", 0)
                .expect("colored reduction should lex");
        let reduction = parse_spells_cost_modifier_line(&reduction_tokens)
            .expect("colored reduction should not hard-error")
            .expect("colored reduction should parse");
        let reduction = match reduction.payload {
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => reduction,
            other => panic!("expected a mana-symbol reduction, got {other:?}"),
        };
        assert!(reduction.per_target);
        assert_eq!(reduction.cost.to_oracle(), "{W}");

        let increase_tokens = lex_line(
            "Spells your opponents cast cost {U} more to cast for each target.",
            0,
        )
        .expect("colored increase should lex");
        let increase = parse_spells_cost_modifier_line(&increase_tokens)
            .expect("colored increase should not hard-error")
            .expect("colored increase should parse");
        let increase = match increase.payload {
            ironsmith_core::StaticAbilityPayload::CostIncreaseManaCost(increase) => increase,
            other => panic!("expected a mana-symbol increase, got {other:?}"),
        };
        assert!(increase.per_target);
        assert_eq!(increase.cost.to_oracle(), "{U}");
    }

    #[test]
    fn non_hand_origin_filter_includes_cards_owned_by_another_player_in_hand() {
        let filter = parsed_spell_cost_filter(
            "Spells you cast from anywhere other than your hand cost {1} less to cast.",
        );

        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Hand) && branch.owner == Some(PlayerFilter::NotYou)
        }));
    }

    #[test]
    fn extra_turn_skip_static_rule_preserves_player_scope_and_rejects_trigger_wording() {
        for (text, expected) in [
            (
                "If an opponent would begin an extra turn, that player skips that turn instead.",
                PlayerFilter::Opponent,
            ),
            (
                "If a player would begin an extra turn, that player skips that turn instead.",
                PlayerFilter::Any,
            ),
            (
                "If you would begin an extra turn, skip that turn instead.",
                PlayerFilter::You,
            ),
        ] {
            let tokens = lex_line(text, 0).expect("extra-turn skip line should lex");
            let ability = parse_players_skip_extra_turns_line(&tokens)
                .expect("extra-turn skip parser should not fail")
                .unwrap_or_else(|| {
                    panic!(
                        "exact extra-turn skip line should parse: {:?}",
                        crate::lexer::parser_token_word_refs(&tokens)
                    )
                });
            assert!(matches!(
                ability.payload,
                ironsmith_core::StaticAbilityPayload::PlayersSkipExtraTurns { player }
                    if player == expected
            ));
        }

        let near_miss = lex_line(
            "Whenever an opponent begins an extra turn, that player skips that turn.",
            0,
        )
        .expect("near-miss line should lex");
        assert!(
            parse_players_skip_extra_turns_line(&near_miss)
                .expect("near-miss parser should not fail")
                .is_none()
        );
    }

    #[test]
    fn cost_modifier_target_spec_preserves_player_or_controlled_permanent_union() {
        let tokens =
            lex_line("you or a permanent you control", 0).expect("cost-modifier target should lex");
        let (player, object, targets_any_of) =
            parse_cost_modifier_target_spec(&tokens).expect("cost-modifier target should parse");

        assert_eq!(player, Some(PlayerFilter::You));
        assert!(targets_any_of);
        let object = object.expect("permanent target branch should be retained");
        assert_eq!(object.zone, Some(Zone::Battlefield));
        assert_eq!(object.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn fixed_reduction_with_as_long_as_draw_threshold_keeps_amount_and_condition_distinct() {
        let tokens = lex_line(
            "This spell costs {2} less to cast as long as you've drawn two or more cards this turn.",
            0,
        )
        .expect("conditional fixed reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("conditional reduction should not error")
            .unwrap_or_else(|| {
                panic!(
                    "conditional reduction should parse: {:?}",
                    crate::lexer::parser_token_word_refs(&tokens)
                )
            });
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
            ability.payload
        else {
            panic!("expected a typed self reduction: {ability:#?}");
        };
        assert!(matches!(reduction.amount.unhinted(), Value::Fixed(2)));
        let crate::static_abilities::ThisSpellCostCondition::AsLongAsConditionExpr {
            condition,
            display,
        } = reduction.condition
        else {
            panic!("expected an as-long-as condition: {reduction:#?}");
        };
        assert_eq!(display, "you've drawn two or more cards this turn");
        assert!(matches!(
            condition,
            crate::ConditionExpr::ValueComparison {
                left,
                operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
                right,
            } if matches!(left.unhinted(), Value::MaxCardsDrawnThisTurn(PlayerFilter::You))
                && matches!(right.unhinted(), Value::Fixed(2))
        ));

        let ordinary = lex_line(
            "This spell costs {2} less to cast if you've drawn two or more cards this turn.",
            0,
        )
        .expect("ordinary conditional should lex");
        let ordinary = parse_spells_cost_modifier_line(&ordinary)
            .expect("ordinary condition should not error")
            .expect("ordinary condition should parse");
        assert!(matches!(
            ordinary.payload,
            ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(
                ironsmith_core::ThisSpellCostReduction {
                    condition: crate::static_abilities::ThisSpellCostCondition::ConditionExpr { .. },
                    ..
                }
            )
        ));
    }

    #[test]
    fn first_x_spell_counter_discount_retains_both_constraints() {
        let tokens = lex_line("The first spell you cast with {X} in its mana cost each turn costs {1} less to cast for each +1/+1 counter on this creature.", 0).unwrap();
        let ability = parse_spells_cost_modifier_line(&tokens).unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("{ability:#?}")
        };
        assert!(reduction.filter.has_x_in_cost, "{reduction:#?}");
        assert!(reduction.filter.first_spell_cast_each_turn);
        assert!(
            format!("{:?}", reduction.amount).contains("PlusOnePlusOne"),
            "{reduction:#?}"
        );
    }

    #[test]
    fn first_matching_spell_during_each_own_turn_is_typed_and_conditioned() {
        let tokens = lex_line(
            "The first non-Lemur creature spell with flying you cast during each of your turns costs {1} less to cast.",
            0,
        )
        .expect("first-spell reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("first-spell reduction should not error")
            .expect("first-spell reduction should parse");
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("expected typed cost reduction: {ability:#?}");
        };
        assert!(
            reduction.filter.first_spell_cast_each_turn,
            "{reduction:#?}"
        );
        assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
        assert_eq!(reduction.filter.card_types, [CardType::Creature]);
        assert_eq!(reduction.filter.excluded_subtypes, [Subtype::Lemur]);
        assert_eq!(reduction.filter.static_abilities, [StaticAbilityId::Flying]);
        assert_eq!(reduction.condition, Some(PredicateAst::YourTurn));
    }

    #[test]
    fn second_spell_each_turn_cost_reduction_keeps_exact_ordinal() {
        let tokens = lex_line(
            "The second spell you cast each turn costs {2} less to cast.",
            0,
        )
        .expect("second-spell reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("second-spell reduction should not error")
            .expect("second-spell reduction should parse");
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("expected typed cost reduction: {ability:#?}");
        };
        assert_eq!(reduction.filter.spell_cast_ordinal_each_turn, Some(2));
        assert!(!reduction.filter.first_spell_cast_each_turn);
        assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    }

    #[test]
    fn first_spell_cost_reduction_and_flash_share_one_typed_filter() {
        let tokens = lex_line(
            "The first creature spell you cast each turn costs {2} less to cast and can be cast as though it had flash.",
            0,
        )
        .expect("compound first-spell line should lex");
        let parsed = parse_first_spell_cost_reduction_and_flash_line(&tokens)
            .expect("compound first-spell line should not error")
            .expect("compound first-spell line should parse");
        let [
            StaticAbilityAst::Static(reduction),
            StaticAbilityAst::Static(flash),
        ] = parsed.as_slice()
        else {
            panic!("expected two typed static capabilities: {parsed:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &reduction.payload
        else {
            panic!("expected cost reduction: {reduction:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::Grants(flash) = &flash.payload else {
            panic!("expected hand-zone flash grant: {flash:#?}");
        };
        assert!(reduction.filter.first_spell_cast_each_turn);
        assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
        assert_eq!(flash.filter, reduction.filter);
        assert_eq!(flash.zone, Zone::Stack);

        let near_miss = lex_line(
            "The first creature spell you cast each turn costs {2} less to cast and has flash.",
            0,
        )
        .expect("near miss should lex");
        assert!(
            parse_first_spell_cost_reduction_and_flash_line(&near_miss)
                .expect("near miss should not error")
                .is_none(),
            "a granted keyword is not the authored cast-as-though permission"
        );
    }

    #[test]
    fn would_die_replacement_keeps_filtered_damage_source_history() {
        let tokens = lex_line(
            "If a creature dealt damage this turn by a source you controlled would die, exile it instead.",
            0,
        )
        .expect("filtered-damager replacement should lex");
        let ability = parse_exile_would_die_instead_line(&tokens)
            .expect("filtered-damager replacement should not error")
            .expect("filtered-damager replacement should parse");
        let ironsmith_core::StaticAbilityPayload::ExileWouldDieInstead {
            filter,
            damaged_by,
            damager_filter,
            damager_filter_surface,
            ..
        } = &ability.payload
        else {
            panic!("expected typed would-die replacement: {ability:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(*damaged_by, None);
        assert_eq!(
            damager_filter
                .as_ref()
                .and_then(|filter| filter.controller.clone()),
            Some(PlayerFilter::You)
        );
        assert_eq!(
            damager_filter_surface.as_deref(),
            Some("a source you controlled")
        );
    }
}

#[cfg(test)]
mod attached_your_untap_tests {
    use super::*;
    #[test]
    fn attached_your_untap_full_card_dispatch() {
        use ironsmith_compiler::ParseCardText;
        let (result, trace) = crate::parse_trace::capture(|| {
            ironsmith_compiler_lowering::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cocoon")
                .card_types(vec![crate::types::CardType::Enchantment])
                .subtypes(vec![crate::types::Subtype::Aura])
                .parse_text("Enchant creature\nEnchanted creature doesn't untap during your untap step if this Aura has a pupa counter on it.")
        });
        let definition = result.unwrap();
        assert!(
            !format!("{:?}", definition.spell_effect).contains("UntapEffect"),
            "{}\n{:#?}",
            trace.render(),
            definition.spell_effect
        );
    }

    #[test]
    fn attached_your_untap_retains_step_owner_and_counter_condition() {
        let tokens = crate::lexer::lex_line("Enchanted creature doesn't untap during your untap step if this Aura has a pupa counter on it.", 0).unwrap();
        let parsed = parse_doesnt_untap_during_untap_step_line(&tokens)
            .unwrap()
            .expect("conditional attached restriction");
        let StaticAbilityAst::ConditionalStaticAbility {
            condition,
            ability,
        } = parsed
        else {
            panic!("conditional attached restriction required");
        };
        assert!(matches!(ability.as_ref(), StaticAbilityAst::Static(ability)
            if matches!(&ability.payload, ironsmith_core::StaticAbilityPayload::RuleRestriction { .. })));
        let text = format!("{condition:?}");
        assert!(text.contains("YourTurn"), "{text}");
        assert!(text.contains("pupa"), "{text}");
    }
}

/// "You may activate abilities of creatures you control as though those
/// creatures had haste." (Tyvar, Jubilant Brawler; Thousand-Year Elixir)
pub fn parse_activate_abilities_as_though_haste_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    let Some(rest) = crate::word_primitives::strip_prefix(
        &words,
        &["you", "may", "activate", "abilities", "of"],
    ) else {
        return Ok(None);
    };
    let Some(as_though) = crate::word_primitives::parse_sequence_start(rest, &["as", "though"])
    else {
        return Ok(None);
    };
    let tail = &rest[as_though + 2..];
    if !matches!(
        tail,
        ["those", "creatures", "had", "haste"]
            | ["they", "had", "haste"]
            | ["it", "had", "haste"]
            | ["those", "permanents", "had", "haste"]
    ) {
        return Ok(None);
    }
    let view = crate::lexer::TokenWordView::new(tokens);
    let filter_start_word = 5;
    let filter_end_word = filter_start_word + as_though;
    let Some(filter_start) = view.token_start_indices().get(filter_start_word).copied() else {
        return Ok(None);
    };
    let Some(filter_end) = view.token_start_indices().get(filter_end_word).copied() else {
        return Ok(None);
    };
    let filter = parse_object_filter(&tokens[filter_start..filter_end], false)?;
    let display = crate::lexer::render_token_slice(tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::activate_abilities_as_though_haste(
        filter, display,
    )))
}

/// "You may play lands and cast spells from the top of your library. If you
/// cast a spell this way, pay life equal to its mana value rather than pay its
/// mana cost." (Bolas's Citadel): lands are played from the top of the library
/// normally; spells are cast from there only through the life-payment
/// alternative cost.
pub fn parse_play_from_top_pay_life_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let [first, second] = sentences.as_slice() else {
        return Ok(None);
    };
    let first_words = crate::lexer::token_word_refs(first);
    let second_words = crate::lexer::token_word_refs(second);
    if first_words.as_slice()
        != [
            "you", "may", "play", "lands", "and", "cast", "spells", "from", "the", "top", "of",
            "your", "library",
        ]
        || second_words.as_slice()
            != [
                "if", "you", "cast", "a", "spell", "this", "way", "pay", "life", "equal", "to",
                "its", "mana", "value", "rather", "than", "pay", "its", "mana", "cost",
            ]
    {
        return Ok(None);
    }
    let land_grant = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::play_from(),
        ObjectFilter {
            card_types: vec![CardType::Land],
            ..ObjectFilter::default()
        },
        Zone::Library,
    );
    let mut spell_filter = ObjectFilter::default();
    spell_filter.excluded_card_types.push(CardType::Land);
    let spell_grant = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::DerivedAlternativeCast(
            ironsmith_core::DerivedAlternativeCast::LifeEqualManaValueFromZone {
                zone: Zone::Library,
                usage_limit: None,
            },
        ),
        spell_filter,
        Zone::Library,
    );
    Ok(Some(vec![
        StaticAbility::grants(land_grant)
            .with_text("You may play lands from the top of your library"),
        StaticAbility::grants(spell_grant).with_text(
            "You may cast spells from the top of your library by paying life equal to their mana value rather than paying their mana costs",
        ),
    ]))
}

/// A scoped permission changes loyalty timing without changing activation limits.
pub fn parse_loyalty_abilities_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let prefix = crate::grammar::abilities::split_as_long_as_condition_prefix_lexed(tokens);
    let body = prefix
        .as_ref()
        .map_or(tokens, |prefix| prefix.remainder_tokens);
    let view = crate::lexer::TokenWordView::new(body);
    let words = view.to_word_refs();
    const START: &[&str] = &["you", "may", "activate"];
    const END: &[&str] = &["any", "time", "you", "could", "cast", "an", "instant"];
    const ANY_PLAYER_TURN: &[&[&str]] = &[
        &["on", "any", "players", "turn"],
        &["on", "any", "player", "s", "turn"],
        &["on", "any", "player", "turn"],
    ];
    if !crate::word_primitives::parse_sequence_prefix(&words, START)
        || !crate::word_primitives::parse_sequence_suffix(&words, END)
    {
        return Ok(None);
    }
    let timing_start = words.len() - END.len();
    let subject_and_turn = &words[START.len()..timing_start];
    // This optional clause belongs to the timing permission, not its object
    // selector. Both forms grant the same unrestricted instant timing.
    let subject_len = ANY_PLAYER_TURN
        .iter()
        .find_map(|suffix| suffix_word_start(subject_and_turn, suffix))
        .unwrap_or(subject_and_turn.len());
    let subject_end = START.len() + subject_len;
    let subject = &words[START.len()..subject_end];
    let mut filter = match subject {
        ["its" | "her" | "his" | "their", "loyalty", "abilities"] => ObjectFilter::source(),
        ["loyalty", "abilities", "of", rest @ ..] if !rest.is_empty() => {
            if let Some(surface) = crate::util::source_reference_surface_for_words(rest) {
                ObjectFilter::source_with_surface(surface)
            } else {
                let Some(range) = view.token_span_for_words(START.len() + 3, subject_end) else {
                    return Ok(None);
                };
                // Require the complete selector grammar: the permissive
                // filter parser can discard an unsupported turn restriction
                // and accidentally grant permission on every turn.
                let Some(filter) = crate::grammar::filters::parse_simple_object_filter_lexed(
                    &body[range],
                    false,
                ) else {
                    return Ok(None);
                };
                filter
            }
        }
        _ => return Ok(None),
    };
    if let Some(prefix) = prefix {
        // A source predicate can be carried by the permission's source filter.
        // Other conditions need a conditional permission representation instead.
        let condition = parse_static_condition_clause(prefix.condition_tokens)?;
        let PredicateAst::CountComparison {
            count: AnthemCountExpression::MatchingFilter(condition_filter),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            ..
        } = condition
        else {
            return Ok(None);
        };
        if !filter.source || !condition_filter.source {
            return Ok(None);
        }
        filter = condition_filter;
    }
    Ok(Some(StaticAbility::loyalty_abilities_any_time(filter)))
}

#[cfg(test)]
mod loyalty_timing_tests {
    use super::*;

    #[test]
    fn loyalty_permission_parses_scoped_and_conditional_filters() {
        for (line, is_source, entered) in [
            (
                "You may activate its loyalty abilities any time you could cast an instant.",
                true,
                false,
            ),
            (
                "You may activate its loyalty abilities on any player's turn any time you could cast an instant.",
                true,
                false,
            ),
            (
                "You may activate loyalty abilities of this on any player's turn any time you could cast an instant.",
                true,
                false,
            ),
            (
                "You may activate loyalty abilities of this planeswalker on any player’s turn any time you could cast an instant.",
                true,
                false,
            ),
            (
                "You may activate loyalty abilities of planeswalkers you control any time you could cast an instant.",
                false,
                false,
            ),
            (
                "You may activate loyalty abilities of planeswalkers you control on any player's turn any time you could cast an instant.",
                false,
                false,
            ),
            (
                "As long as this entered this turn, you may activate its loyalty abilities any time you could cast an instant.",
                true,
                true,
            ),
            (
                "As long as this entered this turn, you may activate its loyalty abilities on any player's turn any time you could cast an instant.",
                true,
                true,
            ),
        ] {
            let tokens = crate::lexer::lex_line(line, 0).unwrap();
            let ability = parse_loyalty_abilities_any_time_line(&tokens)
                .unwrap()
                .expect(line);
            let ironsmith_core::StaticAbilityPayload::LoyaltyAbilitiesAnyTime { filter } =
                ability.payload
            else {
                panic!("{line}");
            };
            assert_eq!(filter.source, is_source, "{line}");
            assert_eq!(filter.entered_battlefield_this_turn, entered, "{line}");
            if !is_source {
                assert!(filter.card_types.contains(&CardType::Planeswalker));
                assert_eq!(filter.controller, Some(PlayerFilter::You));
            }
        }
    }

    #[test]
    fn loyalty_permission_preserves_normalized_named_source() {
        for (name, line) in [
            (
                "Teferi, Master of Time",
                "You may activate loyalty abilities of Teferi on any player's turn any time you could cast an instant.",
            ),
            (
                "Chronicle Adept",
                "You may activate loyalty abilities of Chronicle Adept on any player's turn any time you could cast an instant.",
            ),
        ] {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), name)
                .card_types(vec![CardType::Planeswalker]);
            let tokens = crate::lexer::lex_line(line, 0).unwrap();
            let normalized =
                crate::document_parser::normalize_named_source_tokens_for_builder(&card, &tokens)
                    .expect("the card's own name should become a source reference");
            let ability = parse_loyalty_abilities_any_time_line(&normalized)
                .unwrap()
                .expect(line);
            let ironsmith_core::StaticAbilityPayload::LoyaltyAbilitiesAnyTime { filter } =
                ability.payload
            else {
                panic!("{line}");
            };
            assert!(filter.source, "{line}");
            assert!(filter.controller.is_none(), "{line}");
            assert!(filter.subtypes.is_empty(), "{line}");
        }
    }

    #[test]
    fn loyalty_permission_keeps_generic_filter_scope_before_turn_qualifier() {
        let line = "You may activate loyalty abilities of legendary planeswalkers your opponents control on any player's turn any time you could cast an instant.";
        let tokens = crate::lexer::lex_line(line, 0).unwrap();
        let ability = parse_loyalty_abilities_any_time_line(&tokens)
            .unwrap()
            .expect(line);
        let ironsmith_core::StaticAbilityPayload::LoyaltyAbilitiesAnyTime { filter } = ability.payload
        else {
            panic!("{line}");
        };
        assert!(!filter.source);
        assert_eq!(filter.card_types, vec![CardType::Planeswalker]);
        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert_eq!(filter.supertypes, vec![crate::types::Supertype::Legendary]);
    }

    #[test]
    fn loyalty_permission_rejects_unmodeled_turn_restrictions() {
        for subject in [
            "its loyalty abilities",
            "loyalty abilities of this planeswalker",
            "loyalty abilities of planeswalkers you control",
        ] {
            for qualifier in [
                "on your turn",
                "only on your turn",
                "during your turn",
                "on any opponent's turn",
                "on any player's next turn",
                "on any player's end step",
                "on any player's turn this turn",
                "on your turn on any player's turn",
            ] {
                let line = format!(
                    "You may activate {subject} {qualifier} any time you could cast an instant."
                );
                let tokens = crate::lexer::lex_line(&line, 0).unwrap();
                assert!(
                    !matches!(parse_loyalty_abilities_any_time_line(&tokens), Ok(Some(_))),
                    "unsupported restriction was widened: {line}"
                );
            }
        }
    }
}

#[cfg(test)]
mod scoped_counter_replacement_tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn actor_scoped_counter_doubling_preserves_kind_and_recipient() {
        for (text, kind, card_type) in [
            ("If you would put one or more counters on a creature you control, put twice that many of each of those kinds of counters on that creature instead.", None, CardType::Creature),
            ("If you would put one or more charge counters on an artifact you control, put twice that many charge counters on that artifact instead.", Some(CounterType::Charge), CardType::Artifact),
        ] {
            let ability = parse_double_counters_replacement_line(&lex_line(text, 0).unwrap()).unwrap().unwrap();
            let ironsmith_core::StaticAbilityPayload::DoubleCountersReplacement { filter, counter_type, actor, effect_only, .. } = ability.payload else { panic!() };
            assert_eq!(counter_type, kind);
            assert_eq!(actor, Some(PlayerFilter::You));
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert_eq!(filter.card_types, vec![card_type]);
            assert!(!effect_only);
            let invalid = format!("{text} Draw a card.");
            assert!(!matches!(parse_double_counters_replacement_line(&lex_line(&invalid, 0).unwrap()), Ok(Some(_))));
        }
        let mismatched = "If you would put one or more charge counters on an artifact you control, put twice that many shield counters on that artifact instead.";
        assert!(!matches!(parse_double_counters_replacement_line(&lex_line(mismatched, 0).unwrap()), Ok(Some(_))));
    }
    #[test]
    fn counter_addition_keeps_actor_and_permanent_player_scopes() {
        let text = "If you would put one or more counters on a creature or planeswalker you control or on yourself, put that many plus one of each of those kinds of counters on that permanent or player instead.";
        let ability = parse_double_counters_replacement_line(&lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap();
        let ironsmith_core::StaticAbilityPayload::ActorCountersAddition {
            filter,
            player_filter,
            actor,
            counter_type,
            additional,
            ..
        } = ability.payload
        else {
            panic!()
        };
        assert_eq!(actor, PlayerFilter::You);
        assert_eq!(player_filter, Some(PlayerFilter::You));
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(counter_type, None);
        assert_eq!(additional, 1);
        for invalid in [
            text.replace("or on yourself", "or on an opponent"),
            text.replace("or player instead", "instead"),
            format!("{text} Draw a card."),
        ] {
            assert!(
                !matches!(
                    parse_double_counters_replacement_line(&lex_line(&invalid, 0).unwrap()),
                    Ok(Some(_))
                ),
                "{invalid}"
            );
        }
    }
    #[test]
    fn any_counter_multiplier_preserves_mixed_type_subtype_disjunction() {
        let text = "If one or more counters would be put on a creature, Spacecraft, or Planet you control, twice that many of each of those kinds of counters are put on it instead.";
        let ability = parse_double_counters_replacement_line(&lex_line(text, 0).unwrap())
            .unwrap()
            .unwrap();
        let ironsmith_core::StaticAbilityPayload::DoubleCountersReplacement {
            filter,
            counter_type,
            actor,
            effect_only,
            ..
        } = ability.payload
        else {
            panic!()
        };
        assert_eq!(counter_type, None);
        assert_eq!(actor, None);
        assert!(!effect_only);
        assert_eq!(
            filter.any_of.len(),
            3,
            "mixed type/subtype alternatives must not become an intersection"
        );
    }
}

#[cfg(test)]
mod scoped_hand_size_tests {
    use super::*;
    #[test]
    fn complete_player_scope_and_source_counter_are_typed_without_ignored_tails() {
        for text in [
            "Players have no maximum hand size.",
            "The chosen player's maximum hand size is four.",
            "Your maximum hand size is equal to the number of hour counters on this enchantment.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                parse_no_maximum_hand_size_line(&tokens).unwrap().is_some()
                    || parse_reduced_maximum_hand_size_line(&tokens)
                        .unwrap()
                        .is_some(),
                "{text}"
            );
        }
        for text in [
            "Players have no maximum hand size and draw a card.",
            "Your maximum hand size is equal to the number of hour counters on target artifact.",
            "The chosen player's maximum hand size is four and you gain 3 life.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(parse_no_maximum_hand_size_line(&tokens).unwrap().is_none());
            assert!(
                parse_reduced_maximum_hand_size_line(&tokens)
                    .unwrap()
                    .is_none()
            );
        }
        let tokens = crate::lexer::lex_line(
            "Your maximum hand size is equal to the number of hour counters on this enchantment.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_reduced_maximum_hand_size_line(&tokens)
                .unwrap()
                .unwrap()
                .payload,
            ironsmith_core::StaticAbilityPayload::MaximumHandSizeFromSourceCounters {
                counter_type: crate::object::CounterType::Hour,
                ..
            }
        ));
    }
}

pub(crate) struct ParsedManaOutputRewrite {
    pub rule: ironsmith_core::ManaOutputRewrite,
    pub target: Option<crate::model::TargetAst>,
    pub mode: Option<crate::effects::ReplacementApplyMode>,
    pub display: String,
}
/// Semantic owner shared by static lines and resolving registration sentences.
/// The named grammar must consume the whole clause before its filter is read.
pub(crate) fn parse_mana_output_rewrite_definition(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedManaOutputRewrite>, CardTextError> {
    let Some(shape) = crate::grammar::effects::parse_mana_output_rewrite_shape(tokens) else {
        return Ok(None);
    };
    let target = shape
        .source_tokens
        .filter(|tokens| tokens.first().is_some_and(|token| token.is_word("target")))
        .map(crate::util::parse_target_phrase)
        .transpose()?;
    let mut source_filter = if let Some(tokens) = shape.source_tokens {
        let tokens = if target.is_some() {
            &tokens[1..]
        } else {
            tokens
        };
        parse_object_filter(tokens, false)?
    } else if shape.tapped_for_mana {
        ObjectFilter::land()
    } else {
        ObjectFilter::default()
    };
    if shape.tapped_for_mana {
        source_filter.zone = Some(Zone::Battlefield);
    }
    Ok(Some(ParsedManaOutputRewrite {
        rule: ironsmith_core::ManaOutputRewrite {
            source_filter,
            controller: shape.controller,
            tapped_for_mana: shape.tapped_for_mana,
            input: shape.input,
            output: shape.output,
            quantity: shape.quantity,
        },
        target,
        mode: shape.mode,
        display: render_token_slice(tokens),
    }))
}
pub fn parse_mana_output_rewrite_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(symbol) = crate::grammar::effects::parse_unspent_mana_conversion(tokens) {
        return Ok(Some(StaticAbility::convert_unspent_mana(
            PlayerFilter::You,
            symbol,
        )));
    }
    let Some(parsed) = parse_mana_output_rewrite_definition(tokens)? else {
        return Ok(None);
    };
    if parsed.mode.is_some() || parsed.target.is_some() {
        return Ok(None);
    }
    if parsed.rule.output == ironsmith_core::ManaRewriteOutput::ChosenColor {
        return Err(CardTextError::ParseError("static chosen-color mana rewriting requires a live choice owner; this reader only captures chosen colors in resolving registrations".into()));
    }
    Ok(Some(StaticAbility::mana_production_rewrite(
        parsed.rule,
        parsed.display,
    )))
}

#[cfg(test)]
mod generic_flash_permission_tests {
    use super::*;
    #[test]
    fn any_player_type_union_is_spell_timing_and_self_state_is_labeled() {
        for text in [
            "Any player may cast Sliver spells as though they had flash.",
            "Any player may cast spells as though they had flash.",
            "Any player may cast creature and enchantment spells as though they had flash.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let parsed = parse_grant_flash_to_noncreature_spells_line(&tokens)
                .unwrap()
                .unwrap();
            let ironsmith_core::StaticAbilityPayload::Grants(spec) = parsed.payload else {
                panic!();
            };
            assert_eq!(spec.zone, Zone::Stack);
            assert_eq!(spec.beneficiary, PlayerFilter::Any);
            assert!(matches!(
                spec.grantable,
                crate::model::CompilerGrantableCore::Ability(_)
            ));
        }
        for text in [
            "You may cast this spell as though it had flash if you control a Human.",
            "You may cast this spell as though it had flash if you control an attacking legendary creature.",
            "You may cast this spell as though it had flash if a creature is attacking and a creature is blocking.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            let parsed = parse_conditional_self_flash_line(&tokens).unwrap().unwrap();
            assert!(matches!(
                parsed,
                StaticAbilityAst::LabeledConditionalStaticAbility { .. }
            ));
        }
    }
    #[test]
    fn specialized_and_unknown_self_flash_tails_remain_outside_the_state_reader() {
        for text in [
            "You may cast this spell as though it had flash if X is 3 or less.",
            "You may cast this spell as though it had flash if it targets a commander.",
            "You may cast this spell as though it had flash if you pay {2}.",
            "You may cast this spell as though it had flash if a creature is attacking and draw a card.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(
                parse_conditional_self_flash_line(&tokens)
                    .unwrap()
                    .is_none(),
                "{text}"
            );
        }
    }
}

#[cfg(test)]
mod foretell_special_action_modifier_tests {
    use super::*;
    #[test]
    fn complete_modifier_shape_keeps_amount_and_rejects_unconsumed_syntax() {
        for amount in [0, 1, 3] {
            let tokens = crate::lexer::lex_line(&format!("Foretelling cards from your hand costs {{{amount}}} less and can be done on any player's turn."), 0).unwrap();
            let parsed = parse_foretelling_cards_cost_modifier_line(&tokens).unwrap().unwrap();
            assert!(matches!(parsed.payload, ironsmith_core::StaticAbilityPayload::ForetellSpecialActionModifier {
                generic_reduction, any_players_turn: true, ..
            } if generic_reduction == amount));
        }
        for line in [
            "Foretelling cards from your hand costs {R} less and can be done on any player's turn.",
            "Foretelling cards from your hand costs {1}{R} less and can be done on any player's turn.",
            "Foretelling cards from your hand costs {1}{1} less and can be done on any player's turn.",
            "Foretelling cards from your hand costs {1} less and can be done on any player's turn if you control an Island.",
            "Foretelling cards from your hand costs {1} less and can be done on any player's turn {R}.",
        ] {
            let tokens = crate::lexer::lex_line(line, 0).unwrap();
            assert!(parse_foretelling_cards_cost_modifier_line(&tokens).is_err(), "{line}");
        }
    }
}

#[cfg(test)]
mod static_color_subject_tests {
    use super::*;

    #[test]
    fn nominal_slivers_mean_permanents_and_literal_source_colorless_is_not_devoid() {
        let tokens = crate::lexer::lex_line("All Slivers are colorless.", 0).unwrap();
        let ability = parse_all_creatures_are_color_line(&tokens).unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::SetColors { filter, colors, .. } = ability.payload else {
            panic!("literal color statement must use SetColors");
        };
        assert_eq!(colors, crate::color::ColorSet::COLORLESS);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.subtypes, vec![Subtype::Sliver]);
        assert!(filter.card_types.is_empty());
        assert!(filter.controller.is_none() && !filter.source && !filter.other);
        let tokens = crate::lexer::lex_line("This spell is colorless.", 0).unwrap();
        let ability = parse_all_creatures_are_color_line(&tokens).unwrap().unwrap();
        let ironsmith_core::StaticAbilityPayload::SetColors { filter, colors, .. } = ability.payload else {
            panic!("literal colorless is not the Devoid keyword");
        };
        assert!(filter.is_source_only());
        assert_eq!(colors, crate::color::ColorSet::COLORLESS);
    }
}

#[cfg(test)]
#[path = "copular_characteristic_tests.rs"]
mod copular_characteristic_tests;

#[cfg(test)]
#[path = "copular_probe_loss_tests.rs"]
mod copular_probe_loss_tests;

#[cfg(test)]
mod attached_compound_lock_tests {
    use super::*;

    #[test]
    fn attached_untap_and_activation_lock_keeps_both_restrictions() {
        let tokens = crate::lexer::lex_line("Enchanted permanent doesn't untap during its controller's untap step and its activated abilities can't be activated.", 0).unwrap();
        let abilities = parse_attached_untap_and_activation_restrictions_line(&tokens).unwrap().unwrap();
        assert_eq!(abilities.len(), 2);
        assert!(matches!(&abilities[0], StaticAbilityAst::AttachedStaticAbilityGrant { condition: None, .. }));
        let debug = format!("{abilities:?}");
        assert!(debug.contains("DoesntUntap"), "{debug}");
        assert!(debug.contains("ActivateAbilities"), "{debug}");
        assert!(debug.to_lowercase().contains("enchanted"), "{debug}");
        assert!(parse_doesnt_untap_during_untap_step_line(&tokens).unwrap().is_none());
        assert!(crate::activation_and_restrictions::parse_cant_clauses(&tokens).unwrap().is_none());
        let complete = crate::keyword_static::parse_static_ability_ast_line_lexed(&tokens).unwrap().unwrap();
        assert_eq!(complete.len(), 2);
        let invalid = crate::lexer::lex_line("Enchanted permanent doesn't untap during its controller's untap step and its activated abilities can't be activated unless you pay {1}.", 0).unwrap();
        assert!(parse_attached_untap_and_activation_restrictions_line(&invalid).unwrap().is_none());
    }
}
