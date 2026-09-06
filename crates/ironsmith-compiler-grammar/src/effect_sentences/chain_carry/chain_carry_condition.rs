use crate::cards::builders::TurnEventPredicateAst;
use crate::cards::builders::PlayerPredicateAst;
use super::*;

pub(super) fn parse_carried_cant_effects(
    tokens: &[OwnedLexToken],
    duration: &Until,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(restrictions) =
        super::super::super::activation_and_restrictions::parse_cant_restrictions(tokens)?
    else {
        return Ok(None);
    };

    let mut target = None;
    let mut effects = Vec::with_capacity(restrictions.len() + 1);
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target
                && existing != &parsed_target
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mixed carried restriction targets (clause: '{}')",
                    token_word_refs(tokens).join(" ")
                )));
            }
            target = Some(parsed_target);
        }
        effects.push(EffectAst::subject_verb_cant(
            parsed.restriction,
            duration.clone(),
            None,
        ));
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::subject_verb_target_only(target));
    }
    Ok(Some(effects))
}

pub fn parse_effect_clause_with_trailing_if_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    // Some clauses contain an authored trailing condition inside a larger
    // typed procedure, such as a face-down return followed by turning the
    // returned permanent face up. Let the clause parser preserve that
    // multi-effect structure before this generic splitter treats everything
    // after the first `if` as predicate text.
    let words = token_word_refs(tokens);
    let comma_then = tokens.iter().enumerate().any(|(index, token)| {
        token.is_comma()
            && tokens
                .get(index + 1)
                .is_some_and(|next| next.is_word("then"))
    });
    let may_have_embedded_followup = crate::word_primitives::sequence_occurs(&words, &["if"])
        && comma_then
        && crate::word_primitives::sequence_occurs(&words, &["turn"])
        && crate::word_primitives::sequence_occurs(&words, &["face", "up"]);
    if may_have_embedded_followup
        && let Ok(effect) = parse_effect_clause_lexed(tokens)
        && matches!(
            &effect,
            EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { effects, .. }) if effects.len() > 1
        )
    {
        return Ok(effect);
    }

    let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) else {
        return parse_effect_clause_lexed(tokens);
    };
    if crate::grammar::effects::control_flow::is_anaphoric_destroy_battlefield_guard(tokens) {
        // The destroy parser owns this complete shape so it can fold the
        // authored guard into the referenced object's battlefield filter.
        // Wrapping the leading destroy here would turn the pronoun into a
        // condition on the resolving source instead.
        return parse_effect_clause_lexed(tokens);
    }
    let mut predicate = trailing_if.predicate;
    if !trailing_if_predicate_supported(&predicate) {
        return parse_effect_clause_lexed(tokens);
    }

    // Equality is executable independently of its authored wording. Retain
    // the exact-comparison surface on the numeric operand only when the
    // predicate itself (after the trailing `if`) contains `exactly`; a count
    // in the leading effect must not leak into the condition's presentation.
    let exact_predicate_surface =
        crate::slice_primitives::select_last_position(tokens, |token| token.is_word("if"))
            .is_some_and(|if_index| {
                tokens[if_index + 1..]
                    .iter()
                    .any(|token| token.is_word("exactly"))
            });
    if exact_predicate_surface
        && let PredicateAst::ValueComparison {
            operator: ironsmith_core::ValueComparisonOperator::Equal,
            right,
            ..
        } = &mut predicate
    {
        *right = right
            .clone()
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ExactComparison);
    }

    let base_effect = if let Ok(effect) = parse_effect_clause_lexed(trailing_if.leading_tokens) {
        effect
    } else if let Some(effect) = parse_simple_lose_ability_clause_lexed(trailing_if.leading_tokens)?
    {
        effect
    } else if let Some(effect) = parse_simple_gain_ability_clause_lexed(trailing_if.leading_tokens)?
    {
        effect
    } else {
        return parse_effect_clause_lexed(tokens);
    };

    let predicate = bind_trailing_it_predicate_to_explicit_effect_target(predicate, &base_effect);
    Ok(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
        predicate,
        effects: vec![base_effect],
    }))
}

pub(super) fn trailing_if_predicate_supported(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::ItMatches(_)
            | PredicateAst::ItMatchedLastKnown(_)
            | PredicateAst::TargetMatches(_)
            | PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanYou { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerControls { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerControlsExactly { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeastWithDifferentPowers { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreLifeThanYou { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasNoOpponentWithMoreLifeThan { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerIsMonarch { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasInitiative { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasCitysBlessing { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasMoreCardsInHandThanYou { .. })
            | PredicateAst::Player(PlayerPredicateAst::PlayerHasCardTypesInGraveyardOrMore { .. })
            | PredicateAst::YouControlMoreCreaturesThanTargetSpellController
            | PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(_))
            | PredicateAst::ValueComparison { .. }
    ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if crate::tag::CompilerReferenceTag::Enchanted.matches(tag))
}

pub fn parse_effect_clause_with_trailing_if(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause_with_trailing_if_lexed(tokens)
}
