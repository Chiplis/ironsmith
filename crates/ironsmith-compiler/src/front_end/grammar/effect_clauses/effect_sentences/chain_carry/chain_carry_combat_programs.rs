use super::*;

pub(super) fn source_damage_then_gain_ability_actions(effects: &[EffectAst]) -> bool {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { source, .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. },
            ..
        }),
    ] = effects
    else {
        return false;
    };
    target_ast_is_source(source) && target_ast_is_source(target)
}

pub(super) fn append_shared_damage_player_operand(
    effects: &mut Vec<EffectAst>,
    segment: &[OwnedLexToken],
) -> bool {
    let words = token_word_refs(trim_lexed_commas(segment));
    let start = usize::from(crate::word_primitives::parse_sequence_prefix(
        &words,
        &["and"],
    ));
    if !words.get(start..).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_complete(tail, &["each", "player"])
    }) {
        return false;
    }
    let amount = match effects.last() {
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DealDamage { amount, .. }
                | SubjectVerbActionAst::DealDamageEach { amount, .. }
                | SubjectVerbActionAst::DealDamageEqualToPower { amount, .. },
            ..
        })) => amount.clone(),
        _ => return false,
    };
    effects.push(EffectAst::subject_verb_damage(
        amount,
        TargetAst::Player(PlayerFilter::Any, span_from_tokens(segment)),
    ));
    true
}

pub fn collapse_token_copy_end_of_combat_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let facts = chain_grammar::parse_delayed_copy_facts_tokens(tokens);
    if !facts.has_exile
        || !facts.has_token
        || facts.timing != Some(chain_grammar::DelayedCopyTiming::EndOfCombat)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_end_of_combat_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenCopy { .. }
                        | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
                        | SubjectVerbActionAst::CreateTokenWithMods { .. },
                    ..
                }),
                EffectAst::SubjectVerb(subject_verb),
            ) => match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                }
                | SubjectVerbActionAst::Exile { target, .. } => {
                    target_is_generic_token_filter(target)
                }
                _ => false,
            },
            _ => false,
        };

        if !mark_end_of_combat_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy {
                        exile_at_end_of_combat,
                        exile_at_end_of_combat_reference_surface,
                        ..
                    }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource {
                        exile_at_end_of_combat,
                        exile_at_end_of_combat_reference_surface,
                        ..
                    },
                ..
            }) => {
                *exile_at_end_of_combat = true;
                *exile_at_end_of_combat_reference_surface =
                    token_copy_action_reference_surface(tokens, "exile");
            }
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenWithMods {
                        exile_at_end_of_combat,
                        ..
                    },
                ..
            }) => {
                *exile_at_end_of_combat = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub fn collapse_token_copy_end_of_combat_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_end_of_combat_exile_followup_lexed(effects, tokens);
}
