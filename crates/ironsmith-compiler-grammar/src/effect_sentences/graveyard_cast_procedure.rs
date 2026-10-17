//! A graveyard-cast permission and the exile replacement on its spell.
//!
//! "You may cast target instant or sorcery card from your graveyard this turn.
//! If that spell would be put into a graveyard, exile it instead." The first
//! sentence permits casting a targeted card; the second replaces where the
//! spell it becomes would go. Each is recognized on its own sentence, and the
//! replacement binds to the spell the permission tagged.

use super::dispatch_entry::SentenceInput;
use super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::{
    graveyard_cast_with_exile_replacement, graveyard_cast_with_exile_replacement_and_surcharge,
};
use crate::cards::builders::{CardTextError, ConditionalEffectAst, EffectAst, IfResultPredicate};
use crate::grammar::effects::{self as effect_grammar, GraveyardCastReplacementShape};
use crate::lexer::OwnedLexToken;

pub(super) struct GraveyardCastGroup {
    permission: Vec<OwnedLexToken>,
    shape: GraveyardCastReplacementShape,
    replaced: bool,
    /// The permission sat under \"When you do,\": the pair is that result's effect.
    when_result: bool,
    /// The rider read "If an instant or sorcery spell cast this way ...".
    cast_this_way: bool,
    /// No exile rider follows the permission.
    permission_only: bool,
    /// "If that spell doesn't target <filter>, it costs {N} more to cast this
    /// way." between the permission and the exile rider.
    surcharge: Option<(crate::mana::ManaCost, crate::cards::builders::ObjectFilter)>,
    /// The surcharge sentence is still to be consumed.
    surcharge_pending: bool,
    /// "Target opponent mills nine cards, then you may cast ..." (Sorcerous
    /// Squall): the instruction before ", then", read as its own clause.
    leading_effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

/// Open at a graveyard-cast permission when the exile replacement follows it.
pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<GraveyardCastGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let next = sentences.get(sentence_idx + 1);
    // "When you do, you may cast target instant or sorcery card ...": the
    // permission under a reflexive result prefix, the whole pair its effect.
    let (permission, when_result) =
        match crate::grammar::structure::split_leading_result_prefix_lexed(sentence.lexed()) {
            Some(prefix)
                if prefix.kind == crate::grammar::structure::LeadingResultPrefixKind::When
                    && prefix.predicate == IfResultPredicate::Did
                    && crate::word_primitives::parse_sequence_prefix(
                        &crate::lexer::token_word_refs(prefix.trailing_tokens),
                        &["you", "may", "cast", "target"],
                    ) =>
            {
                (
                    crate::util::trim_commas(
                        SentenceInput::from_lexed(prefix.trailing_tokens).lowered(),
                    ),
                    true,
                )
            }
            _ => (crate::util::trim_commas(sentence.lowered()), false),
        };
    let mut leading_effects = Vec::new();
    let mut permission = permission;
    if !when_result
        && effect_grammar::parse_graveyard_cast_permission_shape(&permission).is_none()
        && let Some(then) = (1..permission.len()).find(|&index| {
            permission[index].is_word("then")
                && crate::word_primitives::parse_sequence_prefix(
                    &crate::lexer::token_word_refs(&permission[index + 1..]),
                    &["you", "may", "cast"],
                )
        })
    {
        let rest = crate::util::trim_commas(&permission[then + 1..]);
        if effect_grammar::parse_graveyard_cast_permission_shape(&rest).is_some()
            && is_replacement_follow_up(next)
        {
            let leading = crate::util::trim_commas(&permission[..then]);
            leading_effects = super::parse_effect_sentence_lexed(&leading)?;
            permission = rest;
        }
    }
    let Some(shape) = effect_grammar::parse_graveyard_cast_permission_shape(&permission) else {
        return Ok(None);
    };
    // "If that spell doesn't target a creature you control, it costs {8}
    // more to cast this way." (Mavinda, Students' Advocate) sits between a
    // turn-long permission and its exile rider.
    let surcharge = if shape.until_end_of_turn && !when_result {
        next.and_then(|next| {
            parse_unless_targets_surcharge(&crate::util::trim_commas(next.lowered()))
        })
    } else {
        None
    };
    let replacement_sentence = if surcharge.is_some() {
        sentences.get(sentence_idx + 2)
    } else {
        next
    };
    let replacement_tokens = replacement_sentence
        .map(|next| crate::util::trim_commas(next.lowered()))
        .unwrap_or_default();
    let has_replacement = effect_grammar::is_graveyard_cast_replacement_sentence(&replacement_tokens);
    // "When this creature enters, you may cast target instant or sorcery card
    // from an opponent's graveyard without paying its mana cost." (Chancellor
    // of the Spires): another player's graveyard carries no exile rider.
    let permission_only = !has_replacement
        && !when_result
        && !shape.until_end_of_turn
        && effect_grammar::graveyard_cast_permission_names_other_player_graveyard(&permission);
    if !has_replacement && !permission_only {
        return Ok(None);
    }
    let surcharge = if has_replacement { surcharge } else { None };
    let cast_this_way = crate::word_primitives::sequence_occurs(
        &crate::lexer::token_word_refs(&replacement_tokens),
        &["cast", "this", "way"],
    );
    // The permission must read as a targeted graveyard spell card for the
    // pair to be this procedure; the same check the program made.
    if graveyard_cast_with_exile_replacement(&permission, &shape)?.is_none() {
        return Ok(None);
    }
    Ok(Some(GraveyardCastGroup {
        permission,
        shape,
        when_result,
        cast_this_way,
        replaced: permission_only,
        permission_only,
        surcharge_pending: surcharge.is_some(),
        surcharge,
        leading_effects,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

/// The replacement statement, once.
pub(super) fn continue_with(
    group: &mut GraveyardCastGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.surcharge_pending {
        if parse_unless_targets_surcharge(&crate::util::trim_commas(sentence.lowered())).is_none() {
            return Ok(false);
        }
        group.surcharge_pending = false;
        group.consumed += 1;
        return Ok(true);
    }
    if group.replaced {
        return Ok(false);
    }
    if !effect_grammar::is_graveyard_cast_replacement_sentence(&crate::util::trim_commas(
        sentence.lowered(),
    )) {
        return Ok(false);
    }
    group.replaced = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: GraveyardCastGroup) -> Vec<EffectAst> {
    let effects = graveyard_cast_with_exile_replacement_and_surcharge(
        &group.permission,
        &group.shape,
        group.cast_this_way,
        group.surcharge.clone(),
    )
    .ok()
    .flatten()
    .unwrap_or_default();
    let effects = if group.permission_only {
        effects
            .into_iter()
            .filter(|effect| {
                !matches!(
                    effect,
                    EffectAst::Conditionals(ConditionalEffectAst::IfResult { .. })
                )
            })
            .collect()
    } else {
        effects
    };
    let effects = if group.when_result {
        vec![EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
            predicate: IfResultPredicate::Did,
            effects,
        })]
    } else {
        effects
    };
    let mut all = group.leading_effects;
    all.extend(effects);
    all
}

fn is_replacement_follow_up(next: Option<&SentenceInput>) -> bool {
    next.is_some_and(|next| {
        effect_grammar::is_graveyard_cast_replacement_sentence(&crate::util::trim_commas(
            next.lowered(),
        ))
    })
}

/// "If that spell doesn't target <filter>, it costs {N} more to cast this
/// way": the permission's surcharge and the target filter that waives it.
fn parse_unless_targets_surcharge(
    tokens: &[OwnedLexToken],
) -> Option<(crate::mana::ManaCost, crate::cards::builders::ObjectFilter)> {
    let words = crate::lexer::token_word_refs(tokens);
    let negated_target = [
        &["if", "that", "spell", "doesnt", "target"][..],
        &["if", "that", "spell", "doesn't", "target"][..],
        &["if", "that", "spell", "does", "not", "target"][..],
    ]
    .into_iter()
    .any(|head| crate::word_primitives::parse_sequence_prefix(&words, head));
    if !negated_target {
        return None;
    }
    let target = tokens.iter().position(|token| token.is_word("target"))?;
    let costs = target
        + 1
        + tokens[target + 1..]
            .iter()
            .position(|token| token.is_word("costs"))?;
    let it = costs.checked_sub(1).filter(|&it| tokens[it].is_word("it"))?;
    let more = costs
        + 1
        + tokens[costs + 1..]
            .iter()
            .position(|token| token.is_word("more"))?;
    if crate::lexer::token_word_refs(&tokens[more + 1..]) != ["to", "cast", "this", "way"] {
        return None;
    }
    let cost = crate::grammar::leaf::parse_leaf_mana_cost_tokens(&tokens[costs + 1..more]).ok()?;
    let filter_tokens = crate::util::trim_commas(&tokens[target + 1..it]);
    if filter_tokens.is_empty() {
        return None;
    }
    let filter = crate::object_filters::parse_object_filter_lexed(&filter_tokens, false).ok()?;
    Some((cost, filter))
}
