use super::*;

pub(super) fn parse_destroy_all_shape(tokens: &[OwnedLexToken]) -> DestroyAllShape<'_> {
    if let Some((filter_tokens, player_tokens)) = parse_dealt_damage_to_player_filter(tokens) {
        return DestroyAllShape::DealtDamageToPlayerThisTurn {
            filter_tokens,
            player_tokens,
        };
    }
    if let Some(filter_tokens) = parse_dealt_damage_filter(tokens) {
        return DestroyAllShape::DealtDamageThisTurn { filter_tokens };
    }
    if let Some(shape) = parse_attached_destroy_all_shape(tokens) {
        return shape;
    }
    if let Some((except_idx, (), exception_tokens)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["except", "for"]))
        && except_idx > 0
    {
        let filter_tokens = trim_lexed_commas(&tokens[..except_idx]);
        let exception_tokens = trim_lexed_commas(exception_tokens);
        let exception_words = crate::lexer::parser_token_word_refs(exception_tokens);
        let attack_eligibility_exception = crate::word_primitives::parse_choice_sequence_complete(
            &exception_words,
            &[
                &["creature", "creatures"],
                &["that"],
                &["couldnt", "couldn't"],
                &["attack"],
            ],
        );
        if !attack_eligibility_exception
            && !filter_tokens.is_empty()
            && !exception_tokens.is_empty()
        {
            return DestroyAllShape::ExceptFor {
                filter_tokens,
                exception_tokens,
            };
        }
    }
    if let Some((filter_tokens, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || color_choice_suffix)
    {
        let filter_tokens = trim_lexed_commas(filter_tokens);
        if !filter_tokens.is_empty() {
            return DestroyAllShape::ChosenColor { filter_tokens };
        }
    }
    if let Some((base_tokens, ())) =
        primitives::split_lexed_once_before_suffix(tokens, 0, || chosen_this_way_suffix)
    {
        let mut base_tokens = trim_lexed_commas(base_tokens);
        let base_words = crate::lexer::parser_token_word_refs(base_tokens);
        // In "creatures that aren't of a type chosen this way", `chosen this
        // way` modifies the creature type, not the creatures themselves. Keep
        // the complete phrase in the ordinary object filter so its typed
        // chosen-type exclusion survives. The result-tag route below is for
        // objects that were themselves chosen this way.
        if crate::word_primitives::parse_any_sequence_suffix(
            &base_words,
            &[&["of", "a", "type"], &["of", "the", "type"]],
        ) {
            return DestroyAllShape::Plain {
                filter_tokens: tokens,
            };
        }
        // "not chosen this way" is the complement of the accumulated chosen
        // set. Keep the negation out of the object filter and preserve it as
        // the typed tagged-set relation.
        if let Some(positive_base) = strip_negated_chosen_copula(base_tokens) {
            base_tokens = trim_lexed_commas(positive_base);
            return DestroyAllShape::ChosenThisWay {
                filter_tokens: base_tokens,
                relation: TaggedDestroyRelation::ExceptMatching,
            };
        }
        if let Some((except_idx, (), _)) =
            primitives::find_prefix(base_tokens, || primitives::kw("except").void())
            && except_idx > 0
        {
            let filter_tokens = trim_lexed_commas(&base_tokens[..except_idx]);
            if !filter_tokens.is_empty() {
                return DestroyAllShape::ChosenThisWay {
                    filter_tokens,
                    relation: TaggedDestroyRelation::ExceptMatching,
                };
            }
        }
        return DestroyAllShape::ChosenThisWay {
            filter_tokens: base_tokens,
            relation: TaggedDestroyRelation::Matching,
        };
    }
    DestroyAllShape::Plain {
        filter_tokens: tokens,
    }
}

pub fn parse_destroy_clause_shape(tokens: &[OwnedLexToken]) -> DestroyClauseShape<'_> {
    let tokens = trim_shape_edges(tokens);
    let (core_tokens, timing) = split_destroy_timing(tokens);
    let kind = if core_tokens.is_empty() {
        DestroyClauseKind::Empty
    } else if timing.is_none() && has_unsupported_delayed_timing(tokens) {
        DestroyClauseKind::UnsupportedDelayedTiming
    } else if let Some(((), all_tokens)) = primitives::parse_prefix(core_tokens, all_or_each_word) {
        let all_shape = parse_destroy_all_shape(trim_lexed_commas(all_tokens));
        if matches!(
            all_shape,
            DestroyAllShape::DealtDamageThisTurn { .. }
                | DestroyAllShape::DealtDamageToPlayerThisTurn { .. }
        ) || !has_combat_history_surface(core_tokens)
        {
            DestroyClauseKind::All(all_shape)
        } else {
            DestroyClauseKind::UnsupportedCombatHistory
        }
    } else if let Some(combat) = parse_target_combat_history_shape(core_tokens) {
        DestroyClauseKind::CombatHistory(combat)
    } else if has_combat_history_surface(core_tokens) {
        DestroyClauseKind::UnsupportedCombatHistory
    } else if let Some((target_tokens, unless_tokens)) =
        primitives::split_lexed_once_on_separator(core_tokens, || primitives::kw("unless").void())
    {
        let target_tokens = trim_lexed_commas(target_tokens);
        if let Some(predicate) = conditions::parse_target_set_predicate(unless_tokens)
            && !target_tokens.is_empty()
        {
            DestroyClauseKind::UnlessTargetSetPredicate {
                target_tokens,
                predicate,
            }
        } else {
            match unless_clause::parse_unless_pays_shape_tokens(unless_tokens) {
                Some(payment) if !target_tokens.is_empty() => DestroyClauseKind::UnlessPays {
                    target_tokens,
                    payment,
                },
                _ => DestroyClauseKind::UnsupportedUnless,
            }
        }
    } else if has_trailing_attack_or_block_restriction(core_tokens) {
        DestroyClauseKind::TrailingAttackOrBlockRestriction
    } else if let Some((target_tokens, predicate_tokens)) =
        parse_conditional_destroy_shape(core_tokens)
    {
        if target_tokens.is_empty() || predicate_tokens.is_empty() {
            DestroyClauseKind::UnsupportedConditional
        } else {
            DestroyClauseKind::Conditional {
                target_tokens,
                predicate_tokens,
            }
        }
    } else if let Some(target_tokens) = parse_inline_no_regeneration_target(core_tokens) {
        DestroyClauseKind::InlineNoRegeneration { target_tokens }
    } else if let Some(shape) = parse_destroy_target_and_attached_shape(core_tokens) {
        DestroyClauseKind::TargetAndAttached(shape)
    } else if has_multi_target_tail(core_tokens) {
        DestroyClauseKind::MultiTarget
    } else if primitives::parse_prefix(core_tokens, primitives::phrase(&["target", "blocked"]))
        .is_some()
    {
        DestroyClauseKind::Blocked {
            target_tokens: core_tokens.to_vec(),
        }
    } else {
        DestroyClauseKind::Plain {
            target_tokens: core_tokens,
        }
    };
    DestroyClauseShape { timing, kind }
}
