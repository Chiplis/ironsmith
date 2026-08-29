use super::*;

pub(super) fn first_for_each_object_filter(effects: &[EffectAst]) -> Option<ObjectFilter> {
    for effect in effects {
        if let EffectAst::ForEachObject { filter, .. } = effect {
            return Some(filter.clone());
        }
        let mut found = None;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = first_for_each_object_filter(nested);
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

pub(super) fn mark_matching_for_each_object_leading_then(
    effects: &mut [EffectAst],
    expected: &ObjectFilter,
) -> bool {
    for effect in effects {
        if let EffectAst::ForEachObject { filter, .. } = effect
            && filter == expected
            && !filter.has_for_each_leading_then_surface()
        {
            filter.set_for_each_leading_then_surface(true);
            return true;
        }
        let mut marked = false;
        crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
            if !marked {
                marked = mark_matching_for_each_object_leading_then(nested, expected);
            }
        });
        if marked {
            return true;
        }
    }
    false
}

/// Parse one complete effect body while retaining every source sentence whose
/// boundary is stable under the same joint parse.
///
/// Parsing each sentence in isolation loses the discourse context needed by
/// followups such as "it", "those cards", and "this way". Instead, parse
/// successively longer prefixes and compare them with the corresponding
/// prefix of the whole-body AST. This keeps one shared semantic parse while
/// proving exactly where a later sentence did not rewrite or absorb an
/// earlier effect. Any cross-sentence structural rewrite falls back to the
/// ordinary flat program.
pub(crate) fn parse_effect_sentences_preserving_source_boundaries(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    if matches!(
        crate::grammar::lowering_surfaces::parse_statement_replacement_surface_tokens(tokens),
        Some(crate::model::facts::StatementReplacementSurfaceKind::ClashWinTopOfLibrary)
    ) && let [clash_and_return, win_replacement] = sentences.as_slice()
    {
        return parse_clash_return_replacement_source_groups(clash_and_return, win_replacement);
    }
    parse_effect_sentences_preserving_source_boundaries_general(tokens)
}

fn parse_clash_return_replacement_source_groups(
    clash_and_return: &[OwnedLexToken],
    win_replacement: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    // The two sentences have distinct result scopes: the first produces the
    // clash result and return event, while the second consumes that result to
    // replace the return destination.  Their typed statement classification
    // therefore proves that no joint grammar candidate may absorb either
    // source boundary.
    let first_effects = vec![build_clash_then_return_coordination(clash_and_return)?];
    let second_effects = vec![build_clash_win_replacement_followup(win_replacement)?];
    Ok(vec![
        EffectAst::SourceSentence {
            effects: first_effects,
            leading_then: false,
            starting_with_controller: false,
        },
        EffectAst::SourceSentence {
            effects: second_effects,
            leading_then: false,
            starting_with_controller: false,
        },
    ])
}

fn build_clash_then_return_coordination(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    use crate::cards::builders::{SubjectVerbEffectAst, SubjectVerbSubjectAst};
    use crate::model::coordination::{
        CoordinationAst, CoordinationBoundaryAst, CoordinationKindAst, CoordinationMemberAst,
        CoordinationOperatorAst, EffectDependencyAst, EffectOrderingAst,
    };

    let words = token_word_refs(tokens);
    if words.as_slice()
        != [
            "clash", "with", "an", "opponent", "then", "return", "target", "creature", "to", "its",
            "owner's", "hand",
        ]
    {
        return Err(CardTextError::InvariantViolation(
            "typed clash-return replacement had an unexpected first-sentence shape".to_string(),
        ));
    }
    let return_idx =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("return"))
            .ok_or_else(|| {
                CardTextError::InvariantViolation(
                    "typed clash-return replacement is missing its return verb".to_string(),
                )
            })?;
    let destination_idx =
        crate::slice_primitives::select_position(&tokens[return_idx + 1..], |token| {
            token.is_word("to")
        })
        .map(|offset| return_idx + 1 + offset)
        .ok_or_else(|| {
            CardTextError::InvariantViolation(
                "typed clash-return replacement is missing its hand destination".to_string(),
            )
        })?;
    let target_tokens = trim_lexed_commas(&tokens[return_idx + 1..destination_idx]);
    let mut filter = ObjectFilter::creature();
    filter.set_explicit_card_type_noun(Some(CardType::Creature));
    let returned = EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: PlayerAst::Implicit,
        },
        action: SubjectVerbActionAst::ReturnToHand {
            target: TargetAst::Object(filter, crate::util::span_from_tokens(target_tokens), None),
            random: false,
            destination_player_surface: None,
            exiled_with_source_surface: None,
            set_quantifier_surface: None,
            set_reference_surface: None,
        },
    });
    let clash = EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: PlayerAst::Implicit,
        },
        action: SubjectVerbActionAst::Clash {
            opponent: crate::ClashOpponentAst::Opponent,
        },
    });
    let coordination = CoordinationAst::new(
        CoordinationKindAst::Sequence,
        vec![
            CoordinationMemberAst::new(vec![clash]),
            CoordinationMemberAst::new(vec![returned]),
        ],
        vec![CoordinationBoundaryAst {
            operator: CoordinationOperatorAst::CommaThen,
            ordering: EffectOrderingAst::Ordered,
            dependency: EffectDependencyAst::DependsOnMembers(vec![0]),
            carries: Vec::new(),
            provenance: None,
        }],
        None,
    )
    .map_err(|error| {
        CardTextError::InvariantViolation(format!(
            "invalid typed clash-return coordination: {error:?}"
        ))
    })?;
    Ok(EffectAst::Coordination(coordination))
}

fn build_clash_win_replacement_followup(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let shape = crate::grammar::effects::followup_shapes::parse_conditional_followup(tokens)
        .filter(|shape| {
            matches!(
                shape.kind,
                crate::grammar::effects::followup_shapes::ConditionalFollowupKind::IfYouWin
                    | crate::grammar::effects::followup_shapes::ConditionalFollowupKind::IfYouWinClash
            )
        })
        .ok_or_else(|| {
            CardTextError::InvariantViolation(
                "typed clash-return replacement is missing its win condition".to_string(),
            )
        })?;
    let continuation = crate::grammar::primitives::strip_lexed_prefix_phrase(
        trim_lexed_commas(shape.continuation_tokens),
        &["you", "may"],
    )
    .ok_or_else(|| {
        CardTextError::InvariantViolation(
            "typed clash-return replacement is missing its optional continuation".to_string(),
        )
    })?;
    let continuation = crate::util::trim_edge_punctuation_tokens(continuation);
    let continuation =
        crate::grammar::primitives::strip_lexed_suffix_phrase(continuation, &["instead"])
            .unwrap_or(continuation);
    let move_effect =
        crate::effect_sentences::parse_simple_that_creature_owner_library_placement(continuation)
            .ok_or_else(|| {
            CardTextError::InvariantViolation(
                "typed clash-return replacement has an invalid library destination".to_string(),
            )
        })?;
    Ok(EffectAst::IfResult {
        predicate: crate::IfResultPredicate::Did,
        effects: vec![EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![move_effect],
        }],
    })
}

fn parse_effect_sentences_preserving_source_boundaries_general(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn contains_local_rewrite_dependency(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SelfReplacement { .. }
                | EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::RegisterZoneReplacement {
                        duration: crate::cards::builders::ZoneReplacementDurationAst::OneShot,
                        ..
                    },
                    ..
                })
        ) {
            return true;
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_local_rewrite_dependency);
        });
        found
    }

    fn depends_on_prior_resolution_result(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::IfResult { .. }
                | EffectAst::WhenResult { .. }
                | EffectAst::ResolvedIfResult { .. }
        ) || matches!(
            effect,
            EffectAst::ControlFlow(control)
                if matches!(
                    &control.node,
                        crate::model::ControlFlowNodeAst::Condition { condition, .. }
                        if matches!(
                            &condition.predicate,
                            crate::model::ControlPredicateAst::Result(_)
                        )
                )
        ) {
            return true;
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(depends_on_prior_resolution_result);
        });
        found
    }

    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    if sentences.len() >= 2
        && crate::grammar::semantic_lowering::parse_returned_object_move_head_tokens(&sentences[0])
            .is_some()
        && sentences.iter().skip(1).all(|sentence| {
            crate::grammar::semantic_lowering::parse_returned_object_followup_tokens(sentence)
                .is_some_and(|facts| facts.has_characteristic_changes())
        })
    {
        // The later sentences describe the exact permanent produced by the
        // return instruction. They are one typed object-result pipeline even
        // though their authored sentence boundaries remain visible through
        // the characteristic surface facts. Keeping the AST flat lets
        // reference resolution and mechanical lowering assign one durable
        // result identity to the return and every modification.
        let parsed = parse_effect_sentences_lexed(tokens)?;
        let mut flattened = Vec::new();
        for effect in parsed {
            match effect {
                EffectAst::SourceSentence { effects, .. } => flattened.extend(effects),
                effect => flattened.push(effect),
            }
        }
        return Ok(flattened);
    }
    if sentences.len() >= 2 {
        let program_sentences = sentences
            .iter()
            .map(|sentence| crate::effect_sentences::SentenceInput::from_lexed(sentence))
            .collect::<Vec<_>>();
        if let Some(matched) =
            crate::effect_sentences::try_parse_document_program(&program_sentences, 0)?
            && matched.consumed_sentences == program_sentences.len()
        {
            // A registered program can own the cross-sentence recognition
            // without changing the meaning of either authored sentence. In
            // that case, prove the boundary structurally: independently
            // parse every sentence and require their exact concatenation to
            // equal the registry candidate. This retains provenance for
            // ordinary reference consumers (for example, a permission that
            // reuses the triggering-object tag) while still keeping truly
            // correlated programs atomic below.
            let mut independent_groups = Vec::with_capacity(sentences.len());
            let mut independent_effects = Vec::new();
            for sentence in &sentences {
                let Ok(effects) = parse_effect_sentences_lexed(sentence) else {
                    independent_groups.clear();
                    break;
                };
                independent_effects.extend(effects.iter().cloned());
                independent_groups.push(EffectAst::SourceSentence {
                    effects: crate::effect_sentences::preserve_coordinated_effect_chain_surface(
                        sentence, effects,
                    ),
                    leading_then: token_word_refs(sentence)
                        .first()
                        .is_some_and(|word| word.eq_ignore_ascii_case("then")),
                    starting_with_controller: token_word_refs(sentence).get(..3).is_some_and(
                        |words| {
                            words[0].eq_ignore_ascii_case("starting")
                                && words[1].eq_ignore_ascii_case("with")
                                && words[2].eq_ignore_ascii_case("you")
                        },
                    ),
                });
            }
            if independent_groups.len() == sentences.len() && independent_effects == matched.effects
            {
                return Ok(independent_groups);
            }
            // A complete registered program proves a semantic dependency
            // across these authored sentences.  Do not let the fast path
            // below accept every sentence independently: that would erase
            // participant loops and reference bindings before lowering sees
            // the typed program.
            return Ok(matched.effects);
        }
    }
    if sentences.len() >= 2
        && sentences.iter().any(|sentence| {
            token_word_refs(sentence).get(..2).is_some_and(|words| {
                words[0].eq_ignore_ascii_case("then") && words[1].eq_ignore_ascii_case("if")
            })
        })
    {
        let mut groups = Vec::with_capacity(sentences.len());
        for sentence in &sentences {
            let words = token_word_refs(sentence);
            let leading_then_if = words.get(..2).is_some_and(|words| {
                words[0].eq_ignore_ascii_case("then") && words[1].eq_ignore_ascii_case("if")
            });
            let parse_tokens = if leading_then_if {
                &sentence[1..]
            } else {
                sentence.as_slice()
            };
            let Ok(effects) = parse_effect_sentences_lexed(parse_tokens) else {
                groups.clear();
                break;
            };
            groups.push(EffectAst::SourceSentence {
                effects: crate::effect_sentences::preserve_coordinated_effect_chain_surface(
                    parse_tokens,
                    effects,
                ),
                leading_then: leading_then_if,
                starting_with_controller: false,
            });
        }
        if groups.len() == sentences.len() {
            return Ok(groups);
        }
    }
    if sentences.len() >= 2 {
        let mut direct_groups = Vec::with_capacity(sentences.len());
        for sentence in &sentences {
            let Some(effect) =
                crate::effect_sentences::parse_complete_simple_subject_verb_sentence(sentence)?
            else {
                direct_groups.clear();
                break;
            };
            let words = token_word_refs(sentence);
            direct_groups.push(EffectAst::SourceSentence {
                effects: vec![effect],
                leading_then: words
                    .first()
                    .is_some_and(|word| word.eq_ignore_ascii_case("then")),
                starting_with_controller: words.get(..3).is_some_and(|words| {
                    words[0].eq_ignore_ascii_case("starting")
                        && words[1].eq_ignore_ascii_case("with")
                        && words[2].eq_ignore_ascii_case("you")
                }),
            });
        }
        if direct_groups.len() == sentences.len() {
            return Ok(direct_groups);
        }
    }
    if sentences.len() >= 2
        && effect_grammar::generic_sequence_shapes::parse_starting_each_player_optional_repeat_shape(
            &sentences[0],
            &sentences[1],
        )
        .is_some()
    {
        // The boundary-preserving fallback below strips the participant-order
        // prefix so ordinary per-sentence parsing can proceed. A repeat
        // sequence needs that prefix while its two authored sentences are
        // still adjacent: the second sentence is the first action's loop
        // terminator, not an independent each-player action.
        return parse_effect_sentences_lexed(tokens);
    }
    let mut parse_sentences = sentences.clone();
    let mut stripped_participant_ordering = false;
    if let Some(first) = parse_sentences.first_mut()
        && let Some((_, remainder)) = crate::grammar::primitives::strip_lexed_prefix_phrases(
            first,
            &[&["starting", "with", "you"]],
        )
    {
        *first = trim_lexed_commas(remainder).to_vec();
        stripped_participant_ordering = true;
    }
    let mut parsed_together = if stripped_participant_ordering {
        parse_effect_sentences_lexed(&join_sentences_with_period(&parse_sentences))?
    } else {
        parse_effect_sentences_lexed(tokens)?
    };
    // A sentence-local comma-then sequence followed by `If you can't` is
    // sometimes normalized into one outer `CommaThen` whose final member is
    // an `IfResult`. Restore the two authored source groups after proving the
    // first sentence is exactly the prefix and the second sentence is exactly
    // that typed result branch. This lets reference annotation assign one ID
    // to the complete first-sentence sequence and lets the second segment
    // consume that same ID.
    if let [first_sentence, fallback_sentence] = sentences.as_slice()
        && let [EffectAst::CommaThen { effects }] = parsed_together.as_slice()
        && effects.len() >= 2
        && matches!(
            effects.last(),
            Some(EffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::DidNot,
                ..
            })
        )
    {
        let prefix = EffectAst::CommaThen {
            effects: effects[..effects.len() - 1].to_vec(),
        };
        let fallback = effects.last().cloned().expect("result branch was proved");
        if parse_effect_sentences_lexed(first_sentence).ok().as_deref()
            == Some(std::slice::from_ref(&prefix))
            && parse_effect_sentences_lexed(fallback_sentence)
                .ok()
                .as_deref()
                == Some(std::slice::from_ref(&fallback))
        {
            return Ok(vec![
                EffectAst::SourceSentence {
                    effects: vec![prefix],
                    leading_then: false,
                    starting_with_controller: false,
                },
                EffectAst::SourceSentence {
                    effects: vec![fallback],
                    leading_then: false,
                    starting_with_controller: false,
                },
            ]);
        }
    }
    // Cross-sentence self-replacement construction can rebuild a token-copy
    // action after the sentence-local parser has already discarded its
    // quoted exception. The complete authored token stream is still present
    // at this boundary, so reattach the typed inline rule before comparing
    // prefix parses. A changed joint AST intentionally falls back to the flat
    // program below, preserving the enriched replacement as one unit.
    crate::effect_sentences::attach_inline_token_granted_abilities_to_last_create(
        &mut parsed_together,
        tokens,
    );
    crate::effect_sentences::reconcile_inline_copy_self_replacement_grants(
        &mut parsed_together,
        tokens,
    );
    if parsed_together.len() == sentences.len()
        && parsed_together
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. }))
    {
        let has_explicit_ordered_boundary = parsed_together.iter().skip(1).any(|effect| {
            matches!(
                effect,
                EffectAst::SourceSentence {
                    leading_then: true,
                    ..
                }
            )
        });
        let later_group_depends_on_prior_result = parsed_together.iter().skip(1).any(|effect| {
            let EffectAst::SourceSentence { effects, .. } = effect else {
                return false;
            };
            effects.iter().any(depends_on_prior_resolution_result)
        });
        if !later_group_depends_on_prior_result || has_explicit_ordered_boundary {
            // The compositional statement parser has already supplied the
            // exact source groups. An explicit ordered boundary remains safe
            // when the next sentence consumes a prior result: reference
            // annotation runs across the complete program before individual
            // source segments are materialized, so the result binding stays
            // shared without discarding the authored `Then` surface.
            return Ok(parsed_together);
        }

        // A later `if/when ... this way` consumes the immediately preceding
        // runtime result. Resolution segments allocate result identifiers
        // independently, so those two authored sentences must remain one
        // semantic lowering slice even though their source provenance is
        // known. Keep every typed child and remove only the top-level source
        // grouping markers.
        let mut flattened = Vec::new();
        for effect in parsed_together {
            let EffectAst::SourceSentence { effects, .. } = effect else {
                unreachable!("source-sentence shape proved above")
            };
            flattened.extend(effects);
        }
        return Ok(flattened);
    }
    let has_delegated_partition_prefix = (2..sentences.len()).rev().any(|prefix_len| {
        let prefix = sentences[..prefix_len]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        effect_grammar::delegated_partition_shapes::is_delegated_partition_program(
            &join_sentences_with_period(&prefix),
        )
    });
    if effect_grammar::delegated_partition_shapes::is_delegated_partition_program(tokens)
        || has_delegated_partition_prefix
    {
        // Every later sentence consumes the collection or subset introduced
        // by an earlier sentence, and the conditional variant also scopes
        // its remainder to the alternative branch. A trailing instruction
        // can also consume the exact complement (for example, by granting it
        // haste and scheduling its exile). The typed program parser already
        // owns those dependencies; prefix reparsing would discard that shared
        // reference frame and cannot prove an independent source boundary.
        return Ok(parsed_together);
    }
    if let [choice_sentence, copy_sentence] = sentences.as_slice()
        && crate::effect_sentences::is_controlled_type_choice_then_each_other_copy_shape(
            choice_sentence,
            copy_sentence,
        )
    {
        // The second sentence consumes the durable object selection made by
        // the first, so this is intentionally one cross-sentence semantic
        // program. Source-boundary proof would clone the nested copy AST only
        // to discover that the reference context must remain shared.
        return Ok(parsed_together);
    }
    if sentences.len() < 2 {
        let Some(sentence) = sentences.first() else {
            return Ok(parsed_together);
        };
        let effects = crate::effect_sentences::preserve_coordinated_effect_chain_surface(
            sentence,
            parsed_together,
        );
        if stripped_participant_ordering {
            return Ok(vec![EffectAst::SourceSentence {
                effects,
                leading_then: false,
                starting_with_controller: true,
            }]);
        }
        return Ok(effects);
    }

    // Some authored follow-up sentences modify the preceding action instead
    // of adding a new top-level effect. Treat that exact typed attachment as
    // part of the preceding boundary group while proving source provenance:
    // parsing the preceding sentence alone must differ from the joint AST by
    // construction (the move has not acquired its entry state or grant yet).
    // Keeping the two sentences in one proof group still preserves an earlier
    // leading `Then` boundary and lets the optional procedure own the whole
    // follow-up at runtime.
    let mut boundary_parse_sentences = Vec::<Vec<OwnedLexToken>>::new();
    let mut boundary_surface_sentences = Vec::<Vec<OwnedLexToken>>::new();
    for (parse_sentence, surface_sentence) in parse_sentences
        .iter()
        .cloned()
        .zip(sentences.iter().cloned())
    {
        if effect_grammar::followup_shapes::parse_moved_object_entry_followup_shape(
            &surface_sentence,
        )
        .is_some()
            && let Some(previous) = boundary_parse_sentences.pop()
        {
            boundary_parse_sentences.push(join_sentences_with_period(&[previous, parse_sentence]));
            continue;
        }
        boundary_parse_sentences.push(parse_sentence);
        boundary_surface_sentences.push(surface_sentence);
    }

    // Reference resolution may canonicalize one sentence more deeply when a
    // later sentence consumes its demonstrative (for example, upgrading a
    // trailing-if fact into canonical control flow). Exact prefix equality
    // is then too strict even though the joint parse still has the same
    // ordered top-level ownership as the independently parsed sentences.
    // Prove that ownership by effect cardinality, retain the already-resolved
    // joint effects, and keep semantic-rewrite programs atomic.
    let mut independent_effect_counts = Vec::with_capacity(boundary_parse_sentences.len());
    let mut independent_effect_count = 0usize;
    for sentence in &boundary_parse_sentences {
        let Ok(effects) = parse_effect_sentences_lexed(sentence) else {
            independent_effect_counts.clear();
            break;
        };
        independent_effect_count += effects.len();
        independent_effect_counts.push(effects.len());
    }
    if independent_effect_counts.len() == boundary_parse_sentences.len()
        && independent_effect_count == parsed_together.len()
        && !parsed_together
            .iter()
            .any(contains_local_rewrite_dependency)
    {
        let mut groups = Vec::with_capacity(boundary_parse_sentences.len());
        let mut effect_idx = 0usize;
        for (effect_count, surface_sentence) in independent_effect_counts
            .into_iter()
            .zip(&boundary_surface_sentences)
        {
            let sentence_effects = parsed_together[effect_idx..effect_idx + effect_count].to_vec();
            effect_idx += effect_count;
            let sentence_effects =
                crate::effect_sentences::preserve_coordinated_effect_chain_surface(
                    surface_sentence,
                    sentence_effects,
                );
            let words = token_word_refs(surface_sentence);
            groups.push(EffectAst::SourceSentence {
                effects: sentence_effects,
                leading_then: words
                    .first()
                    .is_some_and(|word| word.eq_ignore_ascii_case("then")),
                starting_with_controller: words.get(..3).is_some_and(|words| {
                    words[0].eq_ignore_ascii_case("starting")
                        && words[1].eq_ignore_ascii_case("with")
                        && words[2].eq_ignore_ascii_case("you")
                }),
            });
        }
        return Ok(groups);
    }

    let mut groups = Vec::with_capacity(boundary_parse_sentences.len());
    let mut previous_effect_count = 0usize;
    for prefix_len in 1..=boundary_parse_sentences.len() {
        let prefix_tokens = join_sentences_with_period(&boundary_parse_sentences[..prefix_len]);
        let Ok(parsed_prefix) = parse_effect_sentences_lexed(&prefix_tokens) else {
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        };
        let prefix_effect_count = parsed_prefix.len();
        if prefix_effect_count <= previous_effect_count
            || prefix_effect_count > parsed_together.len()
            || parsed_prefix.as_slice() != &parsed_together[..prefix_effect_count]
        {
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        }

        let sentence_effects = parsed_together[previous_effect_count..prefix_effect_count].to_vec();
        if previous_effect_count > 0
            && parsed_together[..previous_effect_count]
                .iter()
                .rev()
                .any(|effect| crate::effect_sentences::primary_target_from_effect(effect).is_some())
            && sentence_effects
                .iter()
                .any(crate::compile_support::effect_references_it_tag)
            && sentence_effects
                .iter()
                .any(contains_local_rewrite_dependency)
        {
            // The later sentence's demonstrative consumes an explicit target
            // introduced by an earlier sentence and installs a replacement
            // on that producer. That replacement must share one lowering
            // slice even though the later effects append cleanly to the joint
            // AST. Ordinary tagged consumers retain their source boundary;
            // global reference annotation carries those bindings safely.
            return Ok(preserve_flat_leading_then_for_each_surface(
                &sentences,
                parsed_together,
            ));
        }
        let sentence_effects = crate::effect_sentences::preserve_coordinated_effect_chain_surface(
            &boundary_surface_sentences[prefix_len - 1],
            sentence_effects,
        );
        let leading_then = token_word_refs(&boundary_surface_sentences[prefix_len - 1])
            .first()
            .is_some_and(|word| word.eq_ignore_ascii_case("then"));
        let sentence_words = token_word_refs(&boundary_surface_sentences[prefix_len - 1]);
        let starting_with_controller = sentence_words.get(..3).is_some_and(|words| {
            words[0].eq_ignore_ascii_case("starting")
                && words[1].eq_ignore_ascii_case("with")
                && words[2].eq_ignore_ascii_case("you")
        });
        groups.push(EffectAst::SourceSentence {
            effects: sentence_effects,
            leading_then,
            starting_with_controller,
        });
        previous_effect_count = prefix_effect_count;
    }

    if previous_effect_count == parsed_together.len() {
        Ok(groups)
    } else {
        Ok(preserve_flat_leading_then_for_each_surface(
            &sentences,
            parsed_together,
        ))
    }
}
