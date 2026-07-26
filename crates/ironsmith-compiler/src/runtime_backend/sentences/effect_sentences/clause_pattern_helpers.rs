use crate::cards::builders::{
    CHOSEN_OBJECTS_TAG, CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, IfResultPredicate,
    OwnedLexToken, PlayerAst, PreventNextTimeDamageSourceAst, PreventNextTimeDamageTargetAst,
    RedirectNextTimeDamageDestinationAst, SubjectAst, SubjectVerbActionAst, TagKey, TargetAst,
    TextSpan, Verb,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
use crate::{ChoiceCount, Supertype};

use super::super::activation_and_restrictions::activation_restriction_clauses::starts_with_target_indicator;
use super::super::activation_and_restrictions::trigger_subject_filters::title_case_token_word;
use super::super::grammar::effects::clause_pattern_shapes as clause_shapes;
use super::super::grammar::structure::split_trailing_if_clause_lexed;
use super::super::keyword_static::parse_value_binding_clause;
use super::super::lexer::{
    LexedClause, token_slice_first_is, token_slice_last_is, trim_lexed_commas,
};
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    parse_subject, parse_target_phrase, parse_value, span_from_tokens, trim_commas,
    wrap_target_count,
};
use super::chain_carry::find_verb;
use super::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, parse_distribute_counters_sentence,
};
use super::verb_dispatch::parse_effect_with_verb;

const ODD_RESULT_VALUES_D6: &[i32] = &[1, 3, 5];
const EVEN_RESULT_VALUES_D6: &[i32] = &[2, 4, 6];

pub(crate) fn extract_subject_player(subject: Option<SubjectAst>) -> Option<PlayerAst> {
    match subject {
        Some(SubjectAst::Player(player)) => Some(player),
        _ => None,
    }
}

pub(crate) fn parse_prevent_next_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_prevent_next_damage_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let Some((amount, amount_used)) = parse_value(shape.amount_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing prevent damage amount (clause: '{}')",
            clause_text
        )));
    };
    if amount_used != shape.amount_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent damage amount (clause: '{}')",
            clause_text
        )));
    }
    let target = if shape.protects_you_and_permanents_you_control {
        TargetAst::Player(PlayerFilter::You, span_from_tokens(shape.target_tokens))
    } else {
        parse_target_phrase(shape.target_tokens)?
    };

    Ok(Some(EffectAst::subject_verb_prevent_damage_with_options(
        amount,
        target,
        Until::EndOfTurn,
        shape.source_of_your_choice,
        shape.protects_you_and_permanents_you_control,
        Vec::new(),
    )))
}

pub(crate) fn parse_double_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_double_counters_tokens(tokens) else {
        return Ok(None);
    };
    let effect = match shape.holder {
        clause_shapes::DoubleCounterHolderShape::You => {
            EffectAst::subject_verb_double_counters_on_target(
                shape.counter_type,
                TargetAst::Player(PlayerFilter::You, span_from_tokens(tokens)),
            )
        }
        clause_shapes::DoubleCounterHolderShape::Source {
            tokens: holder_tokens,
            surface,
        } => {
            let span = span_from_tokens(holder_tokens);
            crate::runtime_backend::util::record_source_reference_surface(span, surface);
            EffectAst::subject_verb_double_counters_on_target(
                shape.counter_type,
                TargetAst::Source(span),
            )
        }
        clause_shapes::DoubleCounterHolderShape::Target(holder_tokens) => {
            EffectAst::subject_verb_double_counters_on_target(
                shape.counter_type,
                parse_target_phrase(holder_tokens)?,
            )
        }
        clause_shapes::DoubleCounterHolderShape::Filter(holder_tokens) => {
            EffectAst::subject_verb_double_counters_on_each(
                shape.counter_type,
                parse_object_filter(holder_tokens, false)?,
            )
        }
    };
    Ok(Some(effect))
}

pub(crate) fn parse_distribute_counters_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_distribute_counters_sentence(SubjectVerbPrimitiveClause::new(tokens))
}

pub(crate) fn parse_verb_first_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(word) = tokens.first().and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };

    let verb = match word {
        "add" => Verb::Add,
        "move" => Verb::Move,
        "counter" => Verb::Counter,
        "destroy" => Verb::Destroy,
        "exile" => Verb::Exile,
        "draw" => Verb::Draw,
        "deal" => Verb::Deal,
        "sacrifice" => Verb::Sacrifice,
        "create" => Verb::Create,
        "investigate" => Verb::Investigate,
        "proliferate" => Verb::Proliferate,
        "tap" => Verb::Tap,
        "attach" => Verb::Attach,
        "untap" => Verb::Untap,
        "scry" => Verb::Scry,
        "discard" => Verb::Discard,
        "transform" => Verb::Transform,
        "convert" => Verb::Convert,
        "regenerate" => Verb::Regenerate,
        "heal" | "heals" | "healed" => Verb::Heal,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        "incubate" => Verb::Incubate,
        "shuffle" => Verb::Shuffle,
        "pay" => Verb::Pay,
        "detain" => Verb::Detain,
        "goad" => Verb::Goad,
        "suspect" => Verb::Suspect,
        "note" => Verb::Note,
        "look" => Verb::Look,
        "end" => Verb::End,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

pub(crate) fn parse_choose_target_and_verb_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_choose_target_verb_shape_tokens(tokens) else {
        return Ok(None);
    };
    let Some((verb, verb_idx)) = find_verb(shape.action_tokens) else {
        return Ok(None);
    };
    if verb_idx != 0 {
        return Ok(None);
    }
    let effect = parse_effect_with_verb(verb, None, shape.target_tokens)?;
    Ok(Some(effect))
}

pub(crate) fn parse_copy_spell_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // Preserve the optionality and actor of clauses such as "that permanent's
    // controller may copy this spell". This helper is reached before the
    // generic leading-may dispatcher on the subject/verb route, so parse the
    // action after `may`, bind its implicit actor, and retain the wrapper here.
    if let Some(player) = super::chain_carry::parse_leading_player_may_lexed(tokens) {
        let stripped = super::chain_carry::remove_through_first_word(tokens);
        let Some(mut effect) = parse_copy_spell_clause(&stripped)? else {
            return Ok(None);
        };
        super::chain_carry::bind_implicit_player_context(&mut effect, player);
        return Ok(Some(EffectAst::MayByPlayer {
            player,
            effects: vec![effect],
        }));
    }
    if super::super::grammar::effects::clause_dispatch_shapes::parse_leading_may_clause_shape(
        tokens,
    )
    .is_some()
    {
        return Ok(None);
    }
    // Duration-scoped trigger grants such as "Until end of turn, whenever a
    // player casts an instant or sorcery spell, that player copies it ..."
    // register a temporary delayed trigger. They are not resolution-time copy
    // effects, so leave them for `parse_until_duration_triggered_clause`.
    if super::super::grammar::effects::clause_primitive_shapes::parse_duration_trigger_prefix_shape(
        tokens,
    )
    .is_some()
        && LexedClause::new(tokens)
            .word_refs()
            .iter()
            .any(|word| matches!(*word, "when" | "whenever"))
    {
        return Ok(None);
    }

    fn target_from_shape(shape: clause_shapes::CopyTargetShape<'_>) -> Option<TargetAst> {
        match shape {
            clause_shapes::CopyTargetShape::Source => Some(TargetAst::Source(None)),
            clause_shapes::CopyTargetShape::Triggering => {
                Some(TargetAst::Tagged(TagKey::from("triggering"), None))
            }
            clause_shapes::CopyTargetShape::TriggeringSource => {
                Some(TargetAst::Tagged(TagKey::from("triggering_source"), None))
            }
            clause_shapes::CopyTargetShape::TaggedIt => {
                Some(TargetAst::Tagged(TagKey::from(IT_TAG), None))
            }
            clause_shapes::CopyTargetShape::Explicit(_) => None,
        }
    }

    fn removed_supertypes(shape: &clause_shapes::CopyClauseShape) -> Vec<Supertype> {
        if shape.removed_legendary {
            vec![Supertype::Legendary]
        } else {
            Vec::new()
        }
    }

    let clause = LexedClause::new(tokens);
    let clause_text = clause.text();
    let Some(copy_shape) = clause_shapes::parse_copy_clause_shape_tokens(tokens) else {
        return Ok(None);
    };
    if copy_shape.emblem_with {
        return Ok(None);
    }
    let copy_idx = copy_shape.copy_word;
    let tail = &tokens[copy_idx + 1..];
    let split_idx = copy_shape.tail.retarget_split;
    if let Some(then_idx) = copy_shape.tail.then_split {
        let then_token_idx = copy_idx + 1 + then_idx;
        let first_clause = trim_lexed_commas(&tokens[..then_token_idx]);
        let second_clause = trim_lexed_commas(&tokens[then_token_idx + 1..]);
        let Some(first) = parse_copy_spell_clause(first_clause)? else {
            return Ok(None);
        };
        let Some(second) = parse_copy_spell_clause(second_clause)? else {
            return Ok(None);
        };
        return Ok(Some(EffectAst::Coordinated {
            effects: vec![first, second],
            leading_duration: false,
            result_conjunction: false,
        }));
    }
    if copy_shape.simple_reference {
        let trailing_if = split_trailing_if_clause_lexed(tokens);
        let copy_clause_tokens = trailing_if
            .as_ref()
            .map_or(tokens, |spec| spec.leading_tokens);
        let Some(copy_clause_shape) =
            clause_shapes::parse_copy_clause_shape_tokens(copy_clause_tokens)
        else {
            return Ok(None);
        };
        let copy_clause_copy_idx = copy_clause_shape.copy_word;
        let copy_clause_tail = &copy_clause_tokens[copy_clause_copy_idx + 1..];
        let copy_clause_tail_shape = copy_clause_shape.tail;
        let copy_clause_split_idx = copy_clause_tail_shape.retarget_split;

        if let Some(then_idx) = copy_clause_tail_shape
            .then_split
            .map(|index| copy_clause_copy_idx + 1 + index)
        {
            let tail_clause = LexedClause::new(&copy_clause_tokens[then_idx + 1..]).trimmed();
            if let Some(spec) =
                super::super::activation_and_restrictions::parse_may_cast_it_sentence(
                    tail_clause.tokens(),
                )
                && spec.as_copy
            {
                return Ok(Some(
                    super::super::activation_and_restrictions::build_may_cast_tagged_effect(&spec),
                ));
            }
        }
        let mut count = Value::Fixed(1);
        let copy_clause_exception_idx = copy_clause_tail_shape.exception_split;
        let copy_target_tail = if let Some(idx) = copy_clause_split_idx {
            &copy_clause_tail[..idx]
        } else if let Some(idx) = copy_clause_exception_idx {
            &copy_clause_tail[..idx]
        } else {
            copy_clause_tail
        };
        let (copy_target_tail, explicit_count) = strip_copy_count_suffix(copy_target_tail);
        if let Some(count_value) = explicit_count {
            count = count_value;
        }
        if let Some(for_each_idx) =
            clause_shapes::parse_copy_tail_shape_tokens(copy_target_tail).for_each_split
        {
            let copy_target_clause = LexedClause::new(copy_target_tail);
            let count_filter_clause = copy_target_clause
                .after_words(for_each_idx + 2)
                .unwrap_or_else(|| copy_target_clause.from(copy_target_clause.len()))
                .trimmed();
            if count_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing count filter after 'for each' in copy clause (clause: '{}')",
                    clause_text
                )));
            }
            count = if let Some(history_count) =
                crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(
                    count_filter_clause.tokens(),
                )
            {
                history_count
            } else {
                Value::Count(parse_object_filter(count_filter_clause.tokens(), false)?)
            };
        }
        let target = target_from_shape(clause_shapes::parse_copy_target_shape_tokens(
            copy_target_tail,
        ))
        .unwrap_or(TargetAst::Source(None));
        let base = EffectAst::subject_verb_copy_spell(
            target,
            count,
            PlayerAst::Implicit,
            copy_clause_split_idx.is_some(),
            copy_clause_tail_shape.retarget_single_target,
            removed_supertypes(&copy_clause_shape),
        );
        if let Some(trailing_if) = trailing_if {
            return Ok(Some(EffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![base],
            }));
        }
        return Ok(Some(base));
    }
    let subject = parse_subject(&tokens[..copy_idx]);
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };

    if !copy_shape.mentions_spell_or_ability {
        // "that player copies it and may choose new targets for the copy"
        // names the copied spell only through a pronoun. A player subject
        // plus a retarget tail still pins the reference to a stack object.
        let pronoun_stack_reference = matches!(subject, SubjectAst::Player(_))
            && split_idx.is_some_and(|idx| {
                matches!(
                    clause_shapes::parse_copy_target_shape_tokens(trim_lexed_commas(
                        &tail[..idx]
                    )),
                    clause_shapes::CopyTargetShape::TaggedIt
                        | clause_shapes::CopyTargetShape::Triggering
                )
            });
        if !pronoun_stack_reference {
            return Ok(None);
        }
    }

    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_text
        )));
    }

    let mut count = Value::Fixed(1);
    let exception_split_idx = copy_shape.tail.exception_split;
    let mut copy_target_tail = if let Some(idx) = split_idx {
        &tail[..idx]
    } else if let Some(idx) = exception_split_idx {
        &tail[..idx]
    } else {
        tail
    };
    let (stripped_copy_target_tail, explicit_count) = strip_copy_count_suffix(copy_target_tail);
    copy_target_tail = stripped_copy_target_tail;
    if let Some(count_value) = explicit_count {
        count = count_value;
    }
    if let Some(for_each_idx) =
        clause_shapes::parse_copy_tail_shape_tokens(copy_target_tail).for_each_split
    {
        let copy_target_clause = LexedClause::new(copy_target_tail);
        let count_filter_clause = copy_target_clause
            .after_words(for_each_idx + 2)
            .unwrap_or_else(|| copy_target_clause.from(copy_target_clause.len()))
            .trimmed();
        if count_filter_clause.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing count filter after 'for each' in copy clause (clause: '{}')",
                clause_text
            )));
        }
        count = if let Some(history_count) =
            crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(
                count_filter_clause.tokens(),
            )
        {
            history_count
        } else {
            Value::Count(parse_object_filter(count_filter_clause.tokens(), false)?)
        };
        copy_target_tail = &copy_target_tail[..for_each_idx];
    }

    let copy_target_clause = LexedClause::new(copy_target_tail).trimmed();
    if copy_target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_text
        )));
    }

    let target_shape = clause_shapes::parse_copy_target_shape_tokens(copy_target_clause.tokens());
    let target = if let Some(target) = target_from_shape(target_shape) {
        target
    } else if let clause_shapes::CopyTargetShape::Explicit(target_tokens) = target_shape {
        parse_counter_target_phrase(target_tokens)?
    } else {
        unreachable!("typed copy target reference is explicit or directly lowerable")
    };

    let mut may_choose_new_targets = false;
    let mut choose_new_target_singular = false;
    if let Some(idx) = split_idx {
        let Some(retarget) = clause_shapes::parse_copy_retarget_shape_tokens(&tail[idx + 1..])
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing copy clause (clause: '{}')",
                clause_text
            )));
        };
        if !retarget.has_new {
            return Err(CardTextError::ParseError(format!(
                "missing 'new' in copy retarget clause (clause: '{}')",
                clause_text
            )));
        }
        may_choose_new_targets = retarget.may_choose;
        choose_new_target_singular = retarget.single_target;
    }

    let copy_all_matches = token_slice_first_is(copy_target_clause.tokens(), "all");
    Ok(Some(
        EffectAst::subject_verb_copy_spell(
            target,
            count,
            player,
            may_choose_new_targets,
            choose_new_target_singular,
            removed_supertypes(&copy_shape),
        )
        .with_copy_all_matches(copy_all_matches),
    ))
}

fn strip_copy_count_suffix(tokens: &[OwnedLexToken]) -> (&[OwnedLexToken], Option<Value>) {
    if token_slice_last_is(tokens, "twice") {
        return (
            &tokens[..tokens.len().saturating_sub(1)],
            Some(Value::Fixed(2)),
        );
    }
    (tokens, None)
}

#[cfg(test)]
mod copy_all_tests {
    use super::*;
    use crate::runtime_backend::ast::SubjectVerbEffectAst;

    #[test]
    fn parses_coordinated_copy_all_stack_sets_without_collapsing_them() {
        let tokens = crate::runtime_backend::lex_line(
            "Copy all spells you control, then copy all other activated and triggered abilities you control.",
            0,
        )
        .expect("copy-all sentence should lex");
        let parsed = parse_copy_spell_clause(&tokens)
            .expect("copy-all sentence should parse")
            .expect("copy-all parser should match");
        let EffectAst::Coordinated { effects, .. } = parsed else {
            panic!("expected a coordinated copy pair, got {parsed:#?}");
        };
        assert_eq!(effects.len(), 2, "{effects:#?}");
        for effect in effects {
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CopySpell {
                        all_matches,
                        target: TargetAst::Object(filter, ..),
                        ..
                    },
                ..
            }) = effect
            else {
                panic!("expected a typed copy-all action, got {effect:#?}");
            };
            assert!(all_matches, "set quantifier must survive parsing");
            assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
            assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
        }
    }
}

pub(crate) fn parse_counter_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    if let Some(target) = parse_counter_ability_target_phrase(tokens)? {
        return Ok(target);
    }

    if clause_shapes::has_counter_ability_markers_tokens(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            LexedClause::new(tokens).text()
        )));
    }

    parse_target_phrase(tokens)
}

fn parse_counter_ability_target_phrase(
    tokens: &[OwnedLexToken],
) -> Result<Option<TargetAst>, CardTextError> {
    let shape_result = clause_shapes::parse_counter_ability_target_tokens(tokens);
    if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
        eprintln!(
            "counter-ability shape: tokens={:?} shape={:?}",
            crate::runtime_backend::token_word_refs(tokens),
            shape_result.is_some()
        );
    }
    let Some(shape) = shape_result else {
        return Ok(None);
    };
    let target = TargetAst::Object(
        shape.target_filter,
        if shape.explicit_target {
            span_from_tokens(tokens)
        } else {
            None
        },
        None,
    );
    Ok(Some(wrap_target_count(target, shape.target_count)))
}

fn parse_prevention_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    if let Some(filter) = clause_shapes::parse_you_and_permanents_filter_tokens(tokens) {
        return Ok(TargetAst::ObjectOrPlayer(filter, PlayerFilter::You, None));
    }
    parse_target_phrase(tokens)
}

pub(crate) fn parse_prevent_all_damage_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_prevent_all_damage_shape_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    match shape {
        clause_shapes::PreventAllDamageShape::FromSource { source_tokens } => {
            let source_filter_target = parse_target_phrase(source_tokens)?;
            let TargetAst::Object(source_filter, _, _) = source_filter_target else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported prevent-all damage source filter target (clause: '{}')",
                    clause_text
                )));
            };
            Ok(Some(
                EffectAst::subject_verb_prevent_all_damage_from_source_filter(
                    source_filter,
                    Until::EndOfTurn,
                ),
            ))
        }
        clause_shapes::PreventAllDamageShape::ToTarget { target_tokens } => {
            let target = parse_prevention_target_phrase(target_tokens)?;
            Ok(Some(EffectAst::subject_verb_prevent_all_damage_to_target(
                target,
                Until::EndOfTurn,
            )))
        }
        clause_shapes::PreventAllDamageShape::ToTargetFromSource {
            target_tokens,
            source,
        } => {
            let target = parse_prevention_target_phrase(target_tokens)?;
            match source {
                clause_shapes::PreventAllDamageSourceShape::Choice => {
                    if !matches!(target, TargetAst::Player(PlayerFilter::You, _)) {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported prevent-all damage source choice target (clause: '{}')",
                            clause_text
                        )));
                    }
                    Ok(Some(
                        EffectAst::subject_verb_prevent_all_damage_to_target_with_source_choice(
                            target,
                            Until::EndOfTurn,
                            true,
                        ),
                    ))
                }
                clause_shapes::PreventAllDamageSourceShape::ChoiceSharingActivationManaColor => {
                    if !matches!(target, TargetAst::Player(PlayerFilter::You, _)) {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported prevent-all damage source choice target (clause: '{}')",
                            clause_text
                        )));
                    }
                    Ok(Some(
                        EffectAst::subject_verb_prevent_all_damage_to_target_with_mana_color_source_choice(
                            target,
                            Until::EndOfTurn,
                        ),
                    ))
                }
                clause_shapes::PreventAllDamageSourceShape::Filter(source_tokens) => {
                    if starts_with_target_indicator(source_tokens) {
                        let source_target = parse_target_phrase(source_tokens)?;
                        return Ok(Some(
                            EffectAst::subject_verb_prevent_all_damage_to_target_from_target_source(
                                target,
                                source_target,
                                Until::EndOfTurn,
                            ),
                        ));
                    }
                    let source_filter_target = parse_target_phrase(source_tokens)?;
                    let TargetAst::Object(source_filter, _, _) = source_filter_target else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported prevent-all damage source filter target (clause: '{}')",
                            clause_text
                        )));
                    };
                    Ok(Some(
                        EffectAst::subject_verb_prevent_all_damage_to_target_from_source_filter(
                            target,
                            source_filter,
                            Until::EndOfTurn,
                        ),
                    ))
                }
            }
        }
    }
}

pub(crate) fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(subject_tokens) = clause_shapes::parse_can_attack_no_defender_subject_tokens(tokens)
    else {
        return Ok(None);
    };
    let target = if subject_tokens.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(subject_tokens)?
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::CanAttackAsThoughNoDefender],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_prevent_next_time_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = clause_shapes::parse_prevent_next_time_damage_tokens(tokens) else {
        return Ok(None);
    };
    let source = match shape.source {
        clause_shapes::DamageSourceShape::Choice => PreventNextTimeDamageSourceAst::Choice,
        clause_shapes::DamageSourceShape::ChoiceMatching(filter) => {
            // A non-targeted object AST with no reference span is reserved here
            // for a filtered source choice made as this effect resolves.
            PreventNextTimeDamageSourceAst::Target(TargetAst::Object(filter, None, None))
        }
        clause_shapes::DamageSourceShape::Target(source_tokens) => {
            PreventNextTimeDamageSourceAst::Target(parse_target_phrase(source_tokens)?)
        }
        clause_shapes::DamageSourceShape::Tagged {
            card_type,
            source_tokens,
        } => {
            let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
            if let Some(card_type) = card_type {
                filter.card_types.push(card_type);
            }
            PreventNextTimeDamageSourceAst::Target(TargetAst::Object(
                filter,
                None,
                span_from_tokens(source_tokens),
            ))
        }
        clause_shapes::DamageSourceShape::Filter(filter) => {
            PreventNextTimeDamageSourceAst::Filter(filter)
        }
    };
    let target = match shape.target {
        clause_shapes::DamageTargetShape::AnyTarget => PreventNextTimeDamageTargetAst::AnyTarget,
        clause_shapes::DamageTargetShape::You => PreventNextTimeDamageTargetAst::You,
        clause_shapes::DamageTargetShape::Target(target_tokens) => {
            PreventNextTimeDamageTargetAst::Target(parse_target_phrase(target_tokens)?)
        }
    };

    let effect = if shape.reflect_damage_to_source_controller {
        EffectAst::subject_verb_prevent_next_time_damage_with_reflection(source, target, true)
    } else {
        EffectAst::subject_verb_prevent_next_time_damage(source, target)
    };
    Ok(Some(vec![effect]))
}

pub(crate) fn parse_redirect_next_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = clause_shapes::parse_redirect_next_damage_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let effect = match shape {
        clause_shapes::RedirectNextDamageShape::AllToYouAndPermanents {
            other,
            destination_tokens,
        } => {
            let object_filter = if other {
                ObjectFilter::permanent().you_control().other()
            } else {
                ObjectFilter::permanent().you_control()
            };
            let target = parse_target_phrase(destination_tokens)?;
            EffectAst::subject_verb_redirect_all_damage_this_turn_to_target(
                PlayerFilter::You,
                object_filter,
                target,
            )
        }
        clause_shapes::RedirectNextDamageShape::AllBySourceToSourceController { source_tokens } => {
            let source = parse_target_phrase(source_tokens)?;
            EffectAst::subject_verb_redirect_all_damage_this_turn_by_source_to_source_controller(
                source,
            )
        }
        clause_shapes::RedirectNextDamageShape::AllToTargetByChosenSource {
            target_tokens,
            destination,
        } => {
            let target = parse_target_phrase(target_tokens)?;
            let destination = match destination {
                clause_shapes::RedirectDamageDestinationShape::SourceObject => {
                    RedirectNextTimeDamageDestinationAst::SourceObject
                }
                clause_shapes::RedirectDamageDestinationShape::Controller => {
                    RedirectNextTimeDamageDestinationAst::Controller
                }
                clause_shapes::RedirectDamageDestinationShape::Target(_) => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported redirected-all-damage destination (clause: '{}')",
                        clause_text
                    )));
                }
                clause_shapes::RedirectDamageDestinationShape::TargetOfChoice(_) => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported redirected-all-damage destination (clause: '{}')",
                        clause_text
                    )));
                }
            };
            EffectAst::subject_verb_redirect_all_damage_this_turn_to_source(
                PreventNextTimeDamageSourceAst::Choice,
                target,
                destination,
            )
        }
        clause_shapes::RedirectNextDamageShape::NextTime {
            source,
            target_tokens,
            destination,
        } => {
            let source = match source {
                clause_shapes::DamageSourceShape::Choice => PreventNextTimeDamageSourceAst::Choice,
                // Redirect effects do not yet expose a filtered-choice runtime
                // shape. Keep their prior choice semantics while prevention
                // effects preserve the filter structurally.
                clause_shapes::DamageSourceShape::ChoiceMatching(_) => {
                    PreventNextTimeDamageSourceAst::Choice
                }
                clause_shapes::DamageSourceShape::Target(source_tokens) => {
                    let source_target = parse_target_phrase(source_tokens)?;
                    let TargetAst::Object(filter, _, _) = source_target else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported redirected damage source target (clause: '{}')",
                            clause_text
                        )));
                    };
                    PreventNextTimeDamageSourceAst::Filter(filter)
                }
                clause_shapes::DamageSourceShape::Tagged {
                    card_type,
                    source_tokens,
                } => {
                    let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
                    if let Some(card_type) = card_type {
                        filter.card_types.push(card_type);
                    }
                    PreventNextTimeDamageSourceAst::Target(TargetAst::Object(
                        filter,
                        None,
                        span_from_tokens(source_tokens),
                    ))
                }
                clause_shapes::DamageSourceShape::Filter(filter) => {
                    PreventNextTimeDamageSourceAst::Filter(filter)
                }
            };
            let target = parse_target_phrase(target_tokens)?;
            match destination {
                clause_shapes::RedirectDamageDestinationShape::SourceObject => {
                    EffectAst::subject_verb_redirect_next_time_damage_to_source(
                        source,
                        target,
                        RedirectNextTimeDamageDestinationAst::SourceObject,
                    )
                }
                clause_shapes::RedirectDamageDestinationShape::Controller => {
                    EffectAst::subject_verb_redirect_next_time_damage_to_source(
                        source,
                        target,
                        RedirectNextTimeDamageDestinationAst::Controller,
                    )
                }
                clause_shapes::RedirectDamageDestinationShape::Target(destination_tokens) => {
                    EffectAst::subject_verb_redirect_next_time_damage_to_target(
                        source,
                        target,
                        parse_target_phrase(destination_tokens)?,
                    )
                }
                clause_shapes::RedirectDamageDestinationShape::TargetOfChoice(_) => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported redirected-next-time damage destination (clause: '{}')",
                        clause_text
                    )));
                }
            }
        }
        clause_shapes::RedirectNextDamageShape::NextAmount {
            amount_tokens,
            protected_tokens,
            destination,
        } => {
            let Some((amount, amount_used)) = parse_value(amount_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported redirected-next-damage amount (clause: '{}')",
                    clause_text
                )));
            };
            if amount_used != amount_tokens.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported redirected-next-damage amount (clause: '{}')",
                    clause_text
                )));
            }
            let protected_target = protected_tokens.map(parse_target_phrase).transpose()?;
            match destination {
                clause_shapes::RedirectDamageDestinationShape::Controller => {
                    let protected_target = protected_target.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "missing redirected-next-damage protected target (clause: '{}')",
                            clause_text
                        ))
                    })?;
                    EffectAst::subject_verb_redirect_next_damage_to_controller(
                        amount,
                        protected_target,
                    )
                }
                clause_shapes::RedirectDamageDestinationShape::Target(destination_tokens) => {
                    let target = parse_target_phrase(destination_tokens)?;
                    let mut effect =
                        EffectAst::subject_verb_redirect_next_damage_from_source_to_target(
                            amount, target,
                        );
                    if let EffectAst::SubjectVerb(subject_verb) = &mut effect {
                        if let SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                            protected_target: effect_protected_target,
                            ..
                        } = &mut subject_verb.action
                        {
                            *effect_protected_target = protected_target;
                        }
                    }
                    effect
                }
                clause_shapes::RedirectDamageDestinationShape::SourceObject => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported redirected-next-damage destination (clause: '{}')",
                        clause_text
                    )));
                }
                clause_shapes::RedirectDamageDestinationShape::TargetOfChoice(_) => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported redirected-next-damage destination (clause: '{}')",
                        clause_text
                    )));
                }
            }
        }
    };
    Ok(Some(vec![effect]))
}
pub(crate) fn parse_can_block_additional_creature_this_turn_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_can_block_additional_tokens(tokens) else {
        return Ok(None);
    };
    let target = if shape.subject_tokens.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(shape.subject_tokens)?
    };

    Ok(Some(EffectAst::subject_verb_grant_abilities_to_target(
        target,
        vec![GrantedAbilityAst::CanBlockAdditionalCreatureEachCombat {
            additional: shape.additional as usize,
        }],
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_win_the_game_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_win_game_shape_tokens(tokens) else {
        return Ok(None);
    };
    match shape {
        clause_shapes::WinGameShape::Simple => {
            Ok(Some(EffectAst::subject_verb_win_game(PlayerAst::You)))
        }
        clause_shapes::WinGameShape::ConditionalTail => {
            if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) {
                return Ok(Some(EffectAst::TrailingIf {
                    predicate: trailing_if.predicate,
                    effects: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
                }));
            }
            Ok(None)
        }
        clause_shapes::WinGameShape::NamedZones { name_tokens } => {
            let name = crate::runtime_backend::token_word_refs(name_tokens)
                .iter()
                .map(|word| title_case_token_word(word))
                .collect::<Vec<_>>()
                .join(" ");
            if name.is_empty() {
                return Ok(None);
            }
            return Ok(Some(EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::PlayerOwnsCardNamedInZones {
                    player: PlayerAst::You,
                    name,
                    zones: vec![Zone::Exile, Zone::Hand, Zone::Graveyard, Zone::Battlefield],
                },
                if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
                if_false: Vec::new(),
            }));
        }
    }
}

fn parse_choose_target_prelude_targets(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<TargetAst>>, CardTextError> {
    let mut ranges = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < target_tokens.len() {
        let mut next = if target_tokens[index].is_comma() {
            index + 1
        } else if target_tokens[index].is_word("and") || target_tokens[index].is_word("and/or") {
            index + 1
        } else {
            index += 1;
            continue;
        };
        if target_tokens[index].is_comma()
            && target_tokens
                .get(next)
                .is_some_and(|token| token.is_word("and") || token.is_word("and/or"))
        {
            next += 1;
        }
        if next < target_tokens.len()
            && starts_with_target_indicator(&target_tokens[next..])
            && !trim_commas(&target_tokens[start..index]).is_empty()
        {
            ranges.push(start..index);
            start = next;
            index = next;
            continue;
        }
        index += 1;
    }
    if ranges.is_empty() {
        return Ok(None);
    }
    ranges.push(start..target_tokens.len());

    let mut targets = Vec::with_capacity(ranges.len());
    for range in ranges {
        let part = trim_commas(&target_tokens[range]);
        if part.is_empty() || !starts_with_target_indicator(&part) {
            return Ok(None);
        }
        targets.push(parse_target_phrase(&part)?);
    }
    if targets.len() < 2 {
        return Ok(None);
    }
    Ok(Some(targets))
}

fn parse_kicked_additional_targets_prelude(
    target_tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = clause_shapes::parse_kicked_additional_targets_tokens(target_tokens) else {
        return Ok(None);
    };
    let first_target_tokens = trim_commas(shape.first_target_tokens);
    let first_target = parse_target_phrase(&first_target_tokens)?;
    let count = Value::Add(Box::new(Value::Fixed(1)), Box::new(Value::KickCount));
    Ok(Some(vec![EffectAst::subject_verb_explicit_target_only(
        TargetAst::WithCountValue(Box::new(first_target), ChoiceCount::dynamic_x(), count),
    )]))
}

pub(crate) fn parse_choose_target_prelude_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = clause_shapes::parse_choose_target_prelude_shape_tokens(tokens) else {
        return Ok(None);
    };
    let target_tokens = shape.target_tokens;

    if let Some(effects) = parse_kicked_additional_targets_prelude(target_tokens)? {
        return Ok(Some(effects));
    }

    if let Some(targets) = parse_choose_target_prelude_targets(target_tokens)? {
        return Ok(Some(
            targets
                .into_iter()
                .map(|target| EffectAst::TagAffected {
                    effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
                    tag: TagKey::from(CHOSEN_OBJECTS_TAG),
                })
                .collect(),
        ));
    }

    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(vec![EffectAst::subject_verb_explicit_target_only(
        target,
    )]))
}

#[cfg(test)]
mod choose_target_prelude_tests {
    use super::*;

    #[test]
    fn preserves_three_repeated_optional_target_slots_under_one_chosen_tag() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose up to one target artifact, up to one target creature, and up to one target land.",
            0,
        )
        .expect("repeated target prelude should lex");
        let effects = parse_choose_target_prelude_sentence(&tokens)
            .expect("repeated target prelude should parse")
            .expect("choose-target prelude parser should match");
        assert_eq!(effects.len(), 3, "{effects:#?}");

        for (effect, expected_type) in effects.iter().zip([
            crate::CardType::Artifact,
            crate::CardType::Creature,
            crate::CardType::Land,
        ]) {
            let EffectAst::TagAffected { effect, tag } = effect else {
                panic!("expected a shared chosen-set wrapper, got {effect:#?}");
            };
            assert_eq!(tag.as_str(), CHOSEN_OBJECTS_TAG);
            let EffectAst::SubjectVerb(subject_verb) = effect.as_ref() else {
                panic!("expected an explicit target-only action, got {effect:#?}");
            };
            let SubjectVerbActionAst::TargetOnly {
                target,
                explicit_declaration: true,
            } = &subject_verb.action
            else {
                panic!("expected an explicit target-only action, got {subject_verb:#?}");
            };
            let TargetAst::WithCount(target, count) = target else {
                panic!("expected an up-to-one target, got {target:#?}");
            };
            assert_eq!((count.min, count.max), (0, Some(1)), "{target:#?}");
            let TargetAst::Object(filter, ..) = target.as_ref() else {
                panic!("expected an object target, got {target:#?}");
            };
            assert_eq!(filter.card_types.as_slice(), [expected_type], "{filter:#?}");
        }
    }
}

fn parse_keyword_value_tokens(
    tokens: &[OwnedLexToken],
    mechanic: &str,
    clause_text: &str,
) -> Result<Value, CardTextError> {
    let Some((mut value, used)) = parse_value(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing {mechanic} amount (clause: '{clause_text}')"
        )));
    };
    let trailing = trim_commas(&tokens[used..]);
    if trailing.is_empty() {
        return Ok(value);
    }
    let Some(where_value) = parse_value_binding_clause(&trailing) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported {mechanic} amount tail (clause: '{clause_text}')"
        )));
    };
    value = super::super::util::replace_unbound_x_with_value(value, &where_value, clause_text)?;
    Ok(value)
}

fn keyword_repeat_value(
    repeat: clause_shapes::KeywordRepeatShape<'_>,
    mechanic: &str,
    clause_text: &str,
) -> Result<Value, CardTextError> {
    match repeat {
        clause_shapes::KeywordRepeatShape::Once => Ok(Value::Fixed(1)),
        clause_shapes::KeywordRepeatShape::Twice => Ok(Value::Fixed(2)),
        clause_shapes::KeywordRepeatShape::Count(tokens) => {
            parse_keyword_value_tokens(tokens, mechanic, clause_text)
        }
    }
}

pub(crate) fn parse_keyword_mechanic_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_keyword_mechanic_tokens(tokens) else {
        return Ok(None);
    };
    let clause_text = LexedClause::new(tokens).text();
    let effect = match shape {
        clause_shapes::KeywordMechanicShape::Amass {
            subtype,
            amount_and_binding_tokens,
        } => {
            let Some((mut amount, used)) = parse_value(amount_and_binding_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "missing numeric amount for amass clause (clause: '{clause_text}')"
                )));
            };
            let trailing_tokens = trim_commas(&amount_and_binding_tokens[used..]);
            if !trailing_tokens.is_empty() {
                let Some(where_value) = parse_value_binding_clause(&trailing_tokens) else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing amass clause (clause: '{clause_text}')"
                    )));
                };
                amount = super::super::util::replace_unbound_x_with_value(
                    amount,
                    &where_value,
                    &clause_text,
                )?;
            }
            EffectAst::subject_verb_amass(subtype, amount)
        }
        clause_shapes::KeywordMechanicShape::Forage => {
            EffectAst::subject_verb_emit_keyword_action(crate::events::KeywordActionKind::Forage, 1)
        }
        clause_shapes::KeywordMechanicShape::Harness => {
            EffectAst::subject_verb_emit_keyword_action(
                crate::events::KeywordActionKind::Harness,
                1,
            )
        }
        clause_shapes::KeywordMechanicShape::RollD6 { count_tokens } => EffectAst::RepeatEffects {
            count: parse_keyword_value_tokens(count_tokens, "roll-dice", &clause_text)?,
            effects: vec![EffectAst::subject_verb_roll_die(PlayerAst::Implicit, 6)],
        },
        clause_shapes::KeywordMechanicShape::OddEvenResult { odd, action_tokens } => {
            let predicate = if odd {
                crate::effect::Comparison::OneOf(ODD_RESULT_VALUES_D6)
            } else {
                crate::effect::Comparison::OneOf(EVEN_RESULT_VALUES_D6)
            };
            let Some((verb, verb_idx)) = find_verb(action_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "missing action after odd/even-result clause (clause: '{clause_text}')"
                )));
            };
            if verb_idx != 0 {
                return Err(CardTextError::ParseError(format!(
                    "unsupported odd/even-result action prefix (clause: '{clause_text}')"
                )));
            }
            let action = parse_effect_with_verb(verb, None, &action_tokens[1..])?;
            EffectAst::IfResult {
                predicate: IfResultPredicate::Value(predicate),
                effects: vec![action],
            }
        }
        clause_shapes::KeywordMechanicShape::Phase { direction, subject } => match subject {
            clause_shapes::PhaseSubjectShape::All(filter_tokens) => {
                let mut filter = parse_object_filter(filter_tokens, false)?;
                filter.zone.get_or_insert(Zone::Battlefield);
                match direction {
                    clause_shapes::PhaseDirectionShape::In => {
                        EffectAst::subject_verb_phase_in_all(filter)
                    }
                    clause_shapes::PhaseDirectionShape::Out => {
                        EffectAst::subject_verb_phase_out_all(filter)
                    }
                }
            }
            clause_shapes::PhaseSubjectShape::Target(target_tokens) => {
                let target = parse_target_phrase(target_tokens)?;
                match direction {
                    clause_shapes::PhaseDirectionShape::In => {
                        EffectAst::subject_verb_phase_in(target)
                    }
                    clause_shapes::PhaseDirectionShape::Out => {
                        EffectAst::subject_verb_phase_out(target)
                    }
                }
            }
        },
        clause_shapes::KeywordMechanicShape::OpenAttraction => {
            EffectAst::subject_verb_open_attraction(PlayerAst::Implicit)
        }
        clause_shapes::KeywordMechanicShape::Behold { subtype, count } => {
            EffectAst::subject_verb_behold(subtype, count)
        }
        clause_shapes::KeywordMechanicShape::Blight { amount } => {
            EffectAst::subject_verb_put_counters(
                crate::object::CounterType::MinusOneMinusOne,
                Value::Fixed(amount as i32)
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::BlightKeywordAction),
                TargetAst::Object(ObjectFilter::creature().you_control(), None, None),
                None,
                false,
            )
        }
        clause_shapes::KeywordMechanicShape::ManifestDread { repeat } => {
            let manifest = EffectAst::subject_verb_manifest_dread(PlayerAst::Implicit);
            match repeat {
                clause_shapes::KeywordRepeatShape::Once => manifest,
                _ => EffectAst::RepeatEffects {
                    count: keyword_repeat_value(repeat, "manifest dread", &clause_text)?,
                    effects: vec![manifest],
                },
            }
        }
        clause_shapes::KeywordMechanicShape::ManifestTop { player } => {
            let player = match player {
                clause_shapes::ManifestPlayerShape::You => PlayerAst::You,
                clause_shapes::ManifestPlayerShape::ThatPlayerOrTargetController => {
                    PlayerAst::ThatPlayerOrTargetController
                }
            };
            EffectAst::subject_verb_manifest_top_card(player)
        }
        clause_shapes::KeywordMechanicShape::CloakTop { player } => {
            let player = match player {
                clause_shapes::ManifestPlayerShape::You => PlayerAst::You,
                clause_shapes::ManifestPlayerShape::ThatPlayerOrTargetController => {
                    PlayerAst::ThatPlayerOrTargetController
                }
            };
            EffectAst::subject_verb_cloak_top_card(player)
        }
        clause_shapes::KeywordMechanicShape::ManifestFromHand => {
            EffectAst::subject_verb_manifest_from_hand(PlayerAst::You)
        }
        clause_shapes::KeywordMechanicShape::Populate { repeat } => {
            EffectAst::subject_verb_populate(keyword_repeat_value(
                repeat,
                "populate",
                &clause_text,
            )?)
        }
        clause_shapes::KeywordMechanicShape::Meld { result_name_tokens } => {
            let result_name = crate::runtime_backend::token_word_refs(result_name_tokens).join(" ");
            EffectAst::subject_verb_meld(result_name, false, false)
        }
        clause_shapes::KeywordMechanicShape::Numeric { keyword, amount } => match keyword {
            clause_shapes::NumericKeywordShape::Bolster => EffectAst::subject_verb_bolster(amount),
            clause_shapes::NumericKeywordShape::Support => EffectAst::subject_verb_support(amount),
            clause_shapes::NumericKeywordShape::Adapt => EffectAst::subject_verb_adapt(amount),
        },
        clause_shapes::KeywordMechanicShape::Fateseal { count_tokens } => {
            EffectAst::subject_verb_fateseal(
                PlayerAst::You,
                parse_keyword_value_tokens(count_tokens, "fateseal", &clause_text)?,
            )
        }
        clause_shapes::KeywordMechanicShape::DiscoverSameValue => EffectAst::subject_verb_discover(
            PlayerAst::You,
            Value::EventValue(EventValueSpec::Amount),
        ),
        clause_shapes::KeywordMechanicShape::Discover { count_tokens } => {
            EffectAst::subject_verb_discover(
                PlayerAst::You,
                parse_keyword_value_tokens(count_tokens, "discover", &clause_text)?,
            )
        }
        clause_shapes::KeywordMechanicShape::Explore { subject, repeat } => {
            let target = match subject {
                clause_shapes::KeywordSubjectShape::Source(subject_tokens) => {
                    TargetAst::Source(span_from_tokens(subject_tokens))
                }
                clause_shapes::KeywordSubjectShape::Target(subject_tokens) => {
                    parse_target_phrase(subject_tokens)?
                }
            };
            let explore = EffectAst::subject_verb_explore(target);
            match repeat {
                clause_shapes::KeywordRepeatShape::Once => explore,
                _ => EffectAst::RepeatEffects {
                    count: keyword_repeat_value(repeat, "explore", &clause_text)?,
                    effects: vec![explore],
                },
            }
        }
        clause_shapes::KeywordMechanicShape::Endure {
            subject,
            amount_tokens,
        } => {
            let target = match subject {
                clause_shapes::KeywordSubjectShape::Source(subject_tokens) => {
                    TargetAst::Source(span_from_tokens(subject_tokens))
                }
                clause_shapes::KeywordSubjectShape::Target(subject_tokens) => {
                    parse_target_phrase(subject_tokens)?
                }
            };
            EffectAst::subject_verb_endure(
                target,
                parse_keyword_value_tokens(amount_tokens, "endure", &clause_text)?,
            )
        }
    };
    Ok(Some(effect))
}
pub(crate) fn parse_connive_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_shapes::parse_connive_clause_shape_tokens(tokens) else {
        return Ok(None);
    };

    let mut count = Value::Fixed(1);
    let mut trailing_tokens = trim_commas(shape.count_tokens);
    if !trailing_tokens.is_empty() {
        let Some((parsed_count, used)) = parse_value(&trailing_tokens) else {
            return Ok(None);
        };
        count = parsed_count;
        trailing_tokens = trim_commas(&trailing_tokens[used..]);
        if !trailing_tokens.is_empty() {
            let Some(where_value) = parse_value_binding_clause(&trailing_tokens) else {
                return Ok(None);
            };
            count = super::super::util::replace_unbound_x_with_value(
                count,
                &where_value,
                &crate::runtime_backend::token_word_refs(&trailing_tokens).join(" "),
            )?;
        }
    }

    if trailing_tokens
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let target_tokens = match shape.subject {
        clause_shapes::ConniveSubjectShape::ConvokedThisSpell => {
            return Ok(Some(EffectAst::ForEachTagged {
                tag: TagKey::from("convoked_this_spell"),
                effects: vec![EffectAst::subject_verb_connive_iterated()],
            }));
        }
        clause_shapes::ConniveSubjectShape::Target(target_tokens) => target_tokens,
    };
    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(EffectAst::subject_verb_connive(target, count)))
}
