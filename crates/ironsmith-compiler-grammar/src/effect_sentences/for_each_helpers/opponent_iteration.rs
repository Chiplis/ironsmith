use crate::cards::builders::ForEachEffectAst;
use super::*;

pub(super) fn wrap_opponents(filter: &PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    if *filter == PlayerFilter::Opponent {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
    } else {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            filter: filter.clone(),
            effects,
        })
    }
}

use crate::recognition::ParseOutcome;
#[path = "opponent_iteration/for_each_opponent_readings.rs"]
mod for_each_opponent_readings;

pub fn parse_for_each_opponent_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Voter-relative opponent sets are already represented by an event-
    // populated player tag. Recognize that typed set before the ordinary
    // quantified-opponent path wraps it in a second loop, which would apply
    // the tagged-player action once for every opponent.
    if let Some(mut effects) =
        super::super::dispatch_inner::parse_vote_affinity_subject_verb(tokens)?
    {
        if effects.len() == 1 {
            return Ok(effects.pop());
        }
        return Err(CardTextError::ParseError(
            "voter-relative opponent clause produced multiple outer effects".to_string(),
        ));
    }

    let Some(outer) = for_each_shapes::parse_participant_clause_shape(tokens) else {
        return Ok(None);
    };
    let Some(iteration_filter) = opponent_filter(outer.scope) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let slot_chooser = if outer.participant_is_actor {
        PlayerAst::That
    } else {
        PlayerAst::You
    };
    let input = for_each_opponent_readings::ParticipantClause {
        tokens,
        outer: &outer,
        iteration_filter: iteration_filter.clone(),
        clause_text: &clause_text,
        slot_chooser,
        read_by_cache: Default::default(),
    };
    match for_each_opponent_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    let participant_may = outer.participant_is_actor
        && outer
            .inner_tokens
            .first()
            .is_some_and(|token| token.is_word("may"));
    let participant_chooses = for_each_shapes::starts_choose(outer.inner_tokens);
    let quantified_unless_payment = if outer
        .inner_tokens
        .iter()
        .any(|token| token.is_word("unless"))
    {
        let normalized = if outer.participant_is_actor {
            prepend_that_player_subject(outer.inner_tokens)
        } else {
            outer.inner_tokens.to_vec()
        };
        super::super::parse_sentence_unless_pays(super::super::SubjectVerbPrimitiveClause::new(
            &normalized,
        ))?
    } else {
        None
    };
    let mut effects = if let Some(effects) = quantified_unless_payment {
        effects
    } else if outer.participant_is_actor && !participant_may {
        if let Some(effects) = parse_quantified_participant_actor_program(outer.inner_tokens)? {
            effects
        } else {
            let normalized = prepend_that_player_subject(outer.inner_tokens);
            parse_maybe_effects(&normalized, true, true)?
        }
    } else {
        let normalized = prepend_that_player_life_total_subject(outer.inner_tokens);
        parse_maybe_effects(&normalized, true, outer.participant_is_actor)?
    };
    if !outer.participant_is_actor {
        // The quantified participant is the iteration key, not the actor, in
        // imperative clauses such as "For each opponent, create a token."
        // Resolve the otherwise implicit token controller to the effect
        // controller before lowering enters iterated-player context.
        force_implicit_token_controller_you(&mut effects);
    }
    if participant_chooses {
        if outer.participant_is_actor
            && !outer.inner_tokens.iter().any(|token| token.is_word("you"))
        {
            bind_quantified_participant_actor(&mut effects);
        }
        bind_implicit_choose_chooser(
            &mut effects,
            if outer.participant_is_actor {
                PlayerAst::That
            } else {
                PlayerAst::You
            },
        );
        stabilize_standalone_participant_choice_tag(&mut effects, outer.inner_tokens);
    }
    Ok(Some(wrap_opponents(&iteration_filter, effects)))
}
