fn parse_player_villainous_choice_mode_program(
    program: crate::grammar::semantic_lowering::VillainousChoiceModeProgram<'_>,
) -> Result<Vec<EffectAst>, CardTextError> {
    match program {
        crate::grammar::semantic_lowering::VillainousChoiceModeProgram::Direct(tokens) => {
            if tokens.len() >= 2 && tokens[0].is_word("you") && tokens[1].is_word("create") {
                return crate::effect_sentences::parse_create(
                    &tokens[1..],
                    Some(
                        crate::grammar::shared_util::reference_shapes::SubjectAst::Player(
                            crate::cards::builders::PlayerAst::You,
                        ),
                    ),
                )
                .map(|effect| vec![effect]);
            }
            parse_effect_sentence_lexed(tokens)
        }
        crate::grammar::semantic_lowering::VillainousChoiceModeProgram::SharedSubjectPair(pair) => {
            let parse_action = |action_tokens: &[OwnedLexToken]| {
                let mut clause = Vec::with_capacity(
                    pair.subject_tokens
                        .len()
                        .saturating_add(action_tokens.len()),
                );
                clause.extend_from_slice(pair.subject_tokens);
                clause.extend_from_slice(action_tokens);
                parse_effect_sentence_lexed(&clause)
            };
            let mut effects = parse_action(pair.first_action_tokens)?;
            effects.extend(parse_action(pair.second_action_tokens)?);
            Ok(effects)
        }
    }
}

fn parse_player_villainous_choice_statement(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let shape =
        crate::grammar::semantic_lowering::parse_villainous_choice_player_statement_tokens(tokens);
    let Some(shape) = shape else {
        return Ok(None);
    };
    let first_mode_effects = parse_player_villainous_choice_mode_program(shape.first_mode_program)?;
    let second_mode_effects =
        parse_player_villainous_choice_mode_program(shape.second_mode_program)?;
    let choice = EffectAst::VillainousChoice {
        player: PlayerFilter::IteratedPlayer,
        player_surface: Some("that player".to_string()),
        modes: vec![
            crate::cards::builders::ChooseOneModeAst {
                description: render_token_slice(shape.first_mode_tokens),
                effects: first_mode_effects,
            },
            crate::cards::builders::ChooseOneModeAst {
                description: render_token_slice(shape.second_mode_tokens),
                effects: second_mode_effects,
            },
        ],
    };
    Ok(Some(match shape.iteration {
        crate::grammar::semantic_lowering::VillainousChoicePlayerIteration::EachOpponent => {
            vec![EffectAst::ForEachOpponent {
                effects: vec![choice],
            }]
        }
    }))
}

pub fn parse_effect_sentence_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_sentence_inner_lexed_unstacked(tokens)
}

fn parse_effect_sentence_inner_lexed_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = super::dispatch_entry::parse_if_you_dont_sentence(tokens)? {
        return Ok(vec![EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::ExplicitDidNot,
            effects,
        }]);
    }
    let dispatch_shape = effect_grammar::labeled_dispatch::parse_labeled_dispatch_shape(tokens);

    if let Some(effect_grammar::SentencePreludeShape::RollDiceChooseOneResult {
        count,
        sides,
        die_text,
    }) = effect_grammar::parse_sentence_prelude_shape_tokens(tokens)
    {
        return Ok(vec![
            EffectAst::subject_verb_roll_dice_choose_result_with_die_text(
                PlayerAst::Implicit,
                count,
                sides,
                Some(die_text),
            ),
        ]);
    }

    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let trailing_effects =
            if let Some(copy_effect) = parse_copy_spell_clause(prefix.trailing_tokens)? {
                // The copy specialist can own a complete `copy ..., then copy
                // ...` program. Keep its typed coordination boundary inside
                // the result wrapper instead of flattening its actions
                // through the generic chain splitter.
                vec![copy_effect]
            } else {
                super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?
            };
        let mut result = vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
        }];
        super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(result);
    }

    let villainous_tokens = tokens
        .first()
        .filter(|token| token.is_word("then"))
        .map_or(tokens, |_| &tokens[1..]);
    if let Some(effects) = parse_player_villainous_choice_statement(villainous_tokens)? {
        return Ok(effects);
    }
    if is_activate_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    // Choice-complement clauses begin with the same `each player chooses`
    // surface as several generic mechanic markers. Give the typed choice
    // grammar first refusal so the broad subject/verb extension cannot turn
    // the `then sacrifices the rest` tail into an unsupported marker.
    if dispatch_shape.each_player_choose
        && let Some(effect) = parse_choice_complement_subject_verb(tokens)?
    {
        return Ok(vec![effect]);
    }
    if let Some(effects) =
        super::subject_verb_special_recognizers::parse_scaled_target_power_sentence(tokens)?
    {
        return Ok(effects);
    }
    if dispatch_shape.round_up_each_time {
        return Ok(Vec::new());
    }

    if let Some(effects) = parse_vote_affinity_subject_verb(tokens)? {
        return Ok(effects);
    }

    if let Some(stripped) = split_labeled_effect_prefix_lexed(tokens) {
        return parse_effect_sentence_lexed(stripped);
    }
    if dispatch_shape.starts_if
        && effect_grammar::control_copy_attach_shapes::contains_source_exiled_owner_library_bottom_shape(tokens)
        && let Some(effects) = parse_conditional_sentence_family_lexed(
            tokens,
            parse_effect_chain_preserving_source_exiled_owner_library_bottom,
        )?
    {
        return Ok(effects);
    }
    if dispatch_shape.starts_if
        && let Some(mut effects) = parse_exile_replacement_subject_verb_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.starts_if
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effect) = lower_matching_spell_cost_reduction_sentence(tokens) {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_source_exiled_owner_library_bottom_subject_verb(tokens) {
        return Ok(vec![effect]);
    }
    // A complete trailing no-combat-damage action must be separated before
    // the prefix-tolerant subject/verb extension can absorb it into a broad
    // destroy target. The helper independently grammar-proves and lowers both
    // arms, so ordinary `and` lists remain on the normal path.
    if let Some(effects) = parse_explicit_assign_no_combat_damage_followup(tokens)? {
        return Ok(effects);
    }
    if dispatch_shape.pre_extension_head
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(mut effects) = parse_conditional_sentence_family_lexed(
        tokens,
        parse_effect_chain_preserving_source_exiled_owner_library_bottom,
    )? {
        super::preserve_leading_result_coordination_lexed(tokens, &mut effects);
        return Ok(effects);
    }
    if dispatch_shape.exile_then {
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
        if let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if let Some(then_tail) = dispatch_shape.then_tail {
        return parse_effect_sentence_lexed(then_tail);
    }
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        let trailing_effects = super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?;
        let mut result = vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: trailing_effects,
            },
        }];
        // This late result-prefix route intentionally parses the trailing
        // actions through the inner chain parser to avoid re-entering result
        // dispatch. Restore the authored conjunction after that parse, using
        // the same grammar-confirmed boundary as the ordinary conditional
        // route rather than inferring it from adjacent effects.
        super::preserve_leading_result_coordination_lexed(tokens, &mut result);
        return Ok(result);
    }
    if dispatch_shape.each_player_choose
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effect) = parse_for_each_opponent_clause(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_for_each_player_clause(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(cast_from_among) = dispatch_shape.cast_from_among_free {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.card_types.push(CardType::Artifact);
        filter.card_types.push(CardType::Creature);
        filter.card_types.push(CardType::Enchantment);
        filter.card_types.push(CardType::Planeswalker);
        filter.card_types.push(CardType::Battle);
        filter.type_or_subtype_union = true;
        if let Some(bound) = cast_from_among.mana_value_or_less {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(bound as i32));
        }
        let chosen = crate::tag::CompilerReferenceTag::ChosenCastFromAmong.key();
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                },
            }),
        ]);
    }
    if dispatch_shape.cast_hand_free {
        let chosen = crate::tag::CompilerReferenceTag::ChosenHandSpellToCast.key();
        let filter = ObjectFilter::nonland()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You);
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                },
            }),
        ]);
    }
    if dispatch_shape.has_unquoted_search
        && let Some(mut effects) = parse_search_library_sentence_lexed(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.exile_all_cards_from_hand_graveyard
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.starts_enchant
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.starts_earthbend
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.has_unless
        && super::lex_chain_helpers::has_explicit_comma_then_boundary_lexed(tokens)
    {
        // The unless clause belongs only to the ordered tail. Split the
        // grammar-proven `, then` boundary before the whole-sentence unless
        // primitive can wrap the earlier action and drop the tail action.
        return parse_effect_chain_lexed(tokens);
    }
    if dispatch_shape.has_unless
        && let Some(mut effects) =
            super::parse_sentence_unless_pays(super::SubjectVerbPrimitiveClause::new(tokens))?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.has_unless
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    // Voter-relative player sets contain an ordinary action verb (for
    // example, "loses"), so route the typed voting subject before the generic
    // gain/lose primitive can erase the vote-affinity predicate.
    if dispatch_shape.has_gain_or_lose {
        // An independent action followed by an explicit gain/lose action is
        // an action choice, not one unusually long gain-ability subject. The
        // broad grant parser accepts object-filter prefixes and can otherwise
        // consume the leading action while retaining only the grant branch.
        if let Some(unless_action) = super::parse_or_action_clause_lexed(tokens)? {
            return Ok(vec![unless_action]);
        }
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
        if let Ok(mut effects) = parse_effect_chain_lexed(tokens) {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
        if let Ok(mut effect) = super::parse_effect_clause_with_trailing_if(tokens) {
            apply_where_x_to_damage_amounts(tokens, std::slice::from_mut(&mut effect))?;
            return Ok(vec![effect]);
        }
        if let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if dispatch_shape.has_vote
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.return_rounded_up
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.choose_do_same_for
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.cast_any_number_graveyard_free {
        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.type_or_subtype_union = true;
        filter.colors = Some(crate::color::ColorSet::from(crate::color::Color::Red));
        let tag = crate::tag::CompilerReferenceTag::ChosenCastFromGraveyard.key();
        return Ok(vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
                zones: vec![Zone::Graveyard],
                search_mode: Some(crate::effect::SearchSelectionMode::Optional),
            },
            EffectAst::subject_verb_cast_tagged(tag, PlayerAst::You, false, false, true, None),
        ]);
    }
    if let Some(diag) = super::sentence_unsupported::diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }
    if super::parse_leading_player_may_lexed(tokens).is_some() {
        return parse_effect_chain_lexed(tokens);
    }
    if super::looks_like_multi_create_chain_lexed(tokens) {
        if let Some(unless_action) = super::parse_or_action_clause_lexed(tokens)? {
            return Ok(vec![unless_action]);
        }
        let mut effects = super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.starts_sacrifice && !dispatch_shape.sacrifice_counted {
        let mut effects = parse_effect_chain_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if dispatch_shape.tap_all_or_each_then_untap_all_or_each {
        let mut effects = super::parse_effect_chain_with_subject_verb_primitives_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }

    let (_, effects) = super::sentence_registry::run_sentence_parse_rules_lexed(tokens)?;
    Ok(effects)
}

fn lower_matching_spell_cost_reduction_sentence(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let shape =
        effect_grammar::labeled_dispatch::parse_matching_spell_cost_reduction_shape(tokens)?;
    let mut reduction = shape.reduction;
    if let Some(where_tokens) = shape.where_value_tokens
        && let Some(where_value) = parse_value_binding_clause(where_tokens)
    {
        reduction = where_value;
    }

    if let Some(mana_reduction) = shape.next_spell_mana_reduction {
        Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
            shape.player,
            shape.filter,
            mana_reduction,
        ))
    } else if shape.next_spell {
        (!matches!(reduction, Value::X)).then(|| {
            EffectAst::subject_verb_reduce_next_spell_generic_cost_this_turn(
                shape.player,
                shape.filter,
                reduction,
            )
        })
    } else if shape.duration == Until::EndOfTurn {
        Some(
            EffectAst::subject_verb_reduce_matching_spell_cost_this_turn(
                shape.player,
                shape.filter,
                reduction,
            ),
        )
    } else {
        Some(EffectAst::subject_verb_reduce_matching_spell_cost(
            shape.player,
            shape.filter,
            reduction,
            shape.duration,
        ))
    }
}

fn parse_exile_replacement_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(effect) = parse_zone_replacement_subject_verb(tokens)? else {
        return Ok(None);
    };
    crate::parse_trace::event(
        "effect-route: subject-verb verb=Exile subject=implicit recognizer=instead-replacement",
    );
    Ok(Some(vec![effect]))
}

#[path = "labeled_prefixes/followup_predicates.rs"]
mod followup_predicates;
pub use followup_predicates::*;

#[path = "labeled_prefixes/reference_programs.rs"]
mod reference_programs;
use reference_programs::{parse_earthbend_subject_verb_sentence, parse_for_each_opponent_doesnt_subject_verb_sentence, parse_gain_ability_subject_verb_sentence, parse_gain_ability_to_source_subject_verb_sentence};
pub use reference_programs::{parse_subject_verb_extension_sentence};
#[path = "labeled_prefixes/core_programs.rs"]
mod labeled_prefixes_core_programs;
use labeled_prefixes_core_programs::parse_passive_color_type_addition_sentence;
