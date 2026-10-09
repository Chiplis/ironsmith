//! "At the beginning of the next end step, destroy all non-Wall creatures
//! that player controls that didn't attack this turn. Ignore this effect for
//! each creature the player didn't control continuously since the beginning
//! of the turn." (Siren's Call): a trailing "Ignore this effect for each
//! <object>" sentence removes the objects it describes from the set the
//! immediately preceding instruction acts on. Those objects are simply not
//! affected (CR 608.2c: the instruction is read with the text that modifies
//! it). The exclusion is folded into the preceding instruction's object
//! filter, whatever that instruction is (destroy, exile, sacrifice, damage to
//! each, tap/untap all, pump all, ability grants to all, ...).

use crate::cards::builders::{
    CardTextError, DamageActionAst, EffectAst, GrantActionAst, PermanentStateActionAst,
    StatChangeActionAst, SubjectVerbActionAst, SubjectVerbEffectAst, ZoneMoveActionAst,
};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;
use crate::target::{ObjectFilter, PlayerFilter};

const IGNORE_EFFECT_PREFIX: &[&str] = &["ignore", "this", "effect", "for", "each"];

/// Split off a final "Ignore this effect for each <object filter>." sentence.
/// Returns the preceding instructions and the exclusion's filter tokens.
pub(super) fn split_ignore_effect_exclusion(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    for idx in 1..tokens.len() {
        if !tokens[idx - 1].is_period() || tokens[idx].is_period() {
            continue;
        }
        let Some(((), rest)) =
            primitives::parse_prefix(&tokens[idx..], primitives::phrase(IGNORE_EFFECT_PREFIX))
        else {
            continue;
        };
        let end = rest
            .iter()
            .position(|token| token.is_period())
            .unwrap_or(rest.len());
        // Only a final sentence: anything after it would change the order.
        if end == 0 || rest[end..].iter().any(|token| !token.is_period()) {
            return None;
        }
        return Some((&tokens[..idx], &rest[..end]));
    }
    None
}

pub(super) fn parse_ignore_effect_exclusion(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    crate::object_filters::parse_object_filter_lexed(tokens, false)
}

/// Fold the exclusion into the immediately preceding instruction. A wrapper
/// (a delayed trigger, a sentence marker, ...) hands it to its last nested
/// instruction.
pub(super) fn attach_ignore_effect_exclusion(
    effects: &mut [EffectAst],
    exception: &ObjectFilter,
) -> Result<(), CardTextError> {
    let attached = match effects.last_mut() {
        Some(effect) => fold_into_effect(effect, exception)?,
        None => false,
    };
    if attached {
        Ok(())
    } else {
        Err(CardTextError::ParseError(format!(
            "'Ignore this effect for each ...' has no preceding instruction over a set of objects (exception: '{}')",
            exception.description()
        )))
    }
}

fn fold_into_effect(
    effect: &mut EffectAst,
    exception: &ObjectFilter,
) -> Result<bool, CardTextError> {
    if let Some(filter) = affected_set_filter_mut(effect) {
        exclude_matching_objects(filter, exception)?;
        return Ok(true);
    }
    let mut attached = false;
    let mut error = None;
    crate::model::visit::for_each_nested_effects_mut(effect, false, |nested| {
        if error.is_some() {
            return;
        }
        if let Some(last) = nested.last_mut() {
            match fold_into_effect(last, exception) {
                Ok(branch_attached) => attached |= branch_attached,
                Err(branch_error) => error = Some(branch_error),
            }
        }
    });
    match error {
        Some(error) => Err(error),
        None => Ok(attached),
    }
}

/// The object filter describing the set an instruction acts on.
fn affected_set_filter_mut(effect: &mut EffectAst) -> Option<&mut ObjectFilter> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
            filter,
            target: None,
            ..
        })
        | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { filter, .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { filter, .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { filter, .. }) => {
            Some(filter)
        }
        _ => None,
    }
}

/// Narrow `base` so it no longer matches `exception`. A controller qualifier
/// on a noun that restates the affected set ("each creature you control"
/// after "all creatures") excludes that controller; every other exclusion
/// (types, subtypes, tokens, names, the continuous-control relation of
/// CR 302.6) is the shared "except for ..." exclusion transport, which
/// refuses any qualifier it cannot represent.
fn exclude_matching_objects(
    base: &mut ObjectFilter,
    exception: &ObjectFilter,
) -> Result<(), CardTextError> {
    let mut residual = exception.clone();
    let Some(controller) = residual.controller.take() else {
        return super::zone_handlers::apply_except_filter_exclusions(base, exception);
    };
    let restates_base_noun = residual
        .card_types
        .iter()
        .all(|card_type| base.card_types.contains(card_type))
        && residual.subtypes.is_empty()
        && residual.all_card_types.is_empty();
    residual.card_types.clear();
    residual.zone = None;
    residual.union_surface = Default::default();
    residual.name_surface = Default::default();
    let complement = match (controller, base.controller.as_ref()) {
        (PlayerFilter::You, None) => Some(PlayerFilter::NotYou),
        _ => None,
    };
    match complement {
        Some(complement) if restates_base_noun && residual == ObjectFilter::default() => {
            base.controller = Some(complement);
            Ok(())
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported 'ignore this effect' exclusion (exception: '{}')",
            exception.description()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{ConditionalEffectAst, PlayerAst, PlayerPredicateAst, PredicateAst};

    #[test]
    fn an_exclusion_applies_to_both_conditional_branches() {
        let mut effects = vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::Player(PlayerPredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter: ObjectFilter::creature(),
            }),
            if_true: vec![EffectAst::subject_verb_destroy_all(ObjectFilter::creature())],
            if_false: vec![EffectAst::subject_verb_exile_all(ObjectFilter::creature(), false)],
        })];
        let mut exception = ObjectFilter::creature();
        exception.controller = Some(PlayerFilter::You);
        attach_ignore_effect_exclusion(&mut effects, &exception).unwrap();
        let EffectAst::Conditionals(ConditionalEffectAst::Conditional { if_true, if_false, .. }) =
            &mut effects[0]
        else { panic!("conditional wrapper must remain intact") };
        for branch in [if_true, if_false] {
            let filter = affected_set_filter_mut(&mut branch[0]).unwrap();
            assert_eq!(filter.controller, Some(PlayerFilter::NotYou));
            assert_eq!(filter.card_types, exception.card_types);
        }
    }
}
