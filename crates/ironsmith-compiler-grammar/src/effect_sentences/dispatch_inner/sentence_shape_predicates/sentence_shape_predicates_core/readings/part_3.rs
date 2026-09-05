//! Sentence readings 45–66, in rank order.

use super::super::*;
use super::Sentence;

pub(super) fn read_choice_complement(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let dispatch_shape = effect_grammar::labeled_dispatch::parse_labeled_dispatch_shape(tokens);
    if dispatch_shape.each_player_choose
        && let Some(effect) = parse_choice_complement_subject_verb(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_cast_or_play_tagged(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = crate::permission_helpers::parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_create_token_then_copy_spell_chain(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This complete producer/copy chain must precede the intentionally
    // tolerant standalone copy parser below. That parser can locate a later
    // `copy that spell` clause inside surrounding text, but claiming this
    // exact chain there would discard the token-producing first arm.
    if let Some(effects) = parse_create_token_then_copy_spell_chain(tokens)? {
        crate::parse_trace::event(
            "effect-route: create-token-then-copy after punctuation normalization",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_copy_spell(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // A complete spell-copy clause can contain an `except that the copy is`
    // characteristic modifier.  Route that typed action before broad
    // subject/verb recognition sees only the modifier's final `is` verb and
    // turns the source spell itself into a continuous color-setting effect.
    if let Some(effect) =
        super::super::super::super::clause_pattern_helpers::parse_copy_spell_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_scaled_target_power(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::super::super::subject_verb_special_recognizers::parse_scaled_target_power_sentence(tokens)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_next_spell_grant_sentence(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_matching_spell_cost_reduction(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Matching-spell cost reductions can also be phrased with a leading
    // duration ("Until your next turn, ... spells ... cost ... less"). Give
    // that typed shape precedence over the generic chain parser, which can
    // otherwise reinterpret the spell restriction as a static ability grant
    // to an inferred object.
    if let Some(effect) = lower_matching_spell_cost_reduction_sentence(tokens) {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Cost subject=spell recognizer=matching-spell-reduction",
        );
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_manifest_dread_graveyard_card_to_hand(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_manifest_dread_graveyard_card_to_hand(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_spell_cast_this_way_tax(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = effect_grammar::parse_spell_cast_this_way_tax_tokens(tokens) {
        let mut spell_filter = ObjectFilter::spell().without_type(crate::types::CardType::Land);
        spell_filter.zone = None;
        if let Some(caster) = shape.taxed_caster {
            spell_filter.cast_by = Some(caster);
        }
        return Ok(Some(vec![EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
            crate::model::CompilerGrantableCore::Ability(
                crate::model::CompilerStaticAbilityCore::new(
                    crate::model::CompilerCostIncreaseManaCost::new(
                        spell_filter,
                        shape.additional_cost,
                    ),
                ),
            ),
            crate::grant::GrantDuration::Forever,
        )]));
    }
    Ok(None)
}
pub(super) fn read_attack_or_block_then_prohibition(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_attack_or_block_then_prohibition_sentence(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_optional_companion_fanout(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::super::super::optional_companion_fanout::parse_optional_companion_fanout_sentence(tokens)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_controller_and_defending_player_discard_or_sacrifice(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::super::super::player_subject_sequences::parse_controller_and_defending_player_discard_or_sacrifice(
                tokens,
            )
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_explicit_player_subject_clauses(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(clauses) =
        super::super::super::super::player_subject_sequences::split_explicit_player_subject_clauses(
            tokens,
        )
    {
        let mut effects = Vec::new();
        for clause in clauses {
            effects.extend(parse_effect_sentence_lexed_inner(clause)?);
        }
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_target_relative_combat_set(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_target_relative_combat_set_sentence(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_conjoined_must_be_blocked(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_conjoined_must_be_blocked_sentence(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_destroy_then_temporary_cant_attack_block_chain(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::parse_destroy_then_temporary_cant_attack_block_chain_lexed(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_cant_gain_life_replacement(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // "If <player refs> would gain life this turn, that player gains no life
    // instead." == a can't-gain-life window for those players (Flames of the
    // Blood Hand). Intercept before leading-if splitting since the would-gain
    // predicate isn't a state condition.
    if sentence_shapes::parses_cant_gain_life_replacement_tokens(tokens) {
        return Ok(Some(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::gain_life(crate::target::PlayerFilter::DamagedPlayer),
            crate::effect::Until::EndOfTurn,
            None,
        )]));
    }
    Ok(None)
}
pub(super) fn read_reveal_source_exiled_permanents_sentence(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_reveal_source_exiled_permanents_sentence_lexed(tokens) {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_put_cards_from_single_graveyard_on_bottom_owner_library(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
        parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(tokens)
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_vote_affinity(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Preserve voter-relative player predicates before the generic player
    // subject machinery rewrites `each opponent` to an iterated `that
    // player` action and discards the qualifying vote relationship.
    if let Some(effects) = parse_vote_affinity_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-affinity",
        );
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_vote(input: &Sentence<'_>) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) = parse_vote_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-procedure",
        );
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_keyword_mechanic(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Numeric die-result branches also have the surface shape
    // "for each <noun phrase>, <effect>".  Route the typed keyword shape
    // before the generic object iterator so "odd/even result" is not sent to
    // object-filter lowering.
    if matches!(
        effect_grammar::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens),
        Some(effect_grammar::clause_pattern_shapes::KeywordMechanicShape::OddEvenResult { .. })
    ) && let Some(effect) = parse_keyword_mechanic_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
