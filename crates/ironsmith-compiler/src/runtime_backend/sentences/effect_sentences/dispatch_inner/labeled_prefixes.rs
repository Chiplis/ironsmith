fn parse_player_villainous_choice_mode_program(
    program: crate::runtime_backend::grammar::semantic_lowering::VillainousChoiceModeProgram<'_>,
) -> Result<Vec<EffectAst>, CardTextError> {
    match program {
        crate::runtime_backend::grammar::semantic_lowering::VillainousChoiceModeProgram::Direct(
            tokens,
        ) => parse_effect_sentence_lexed(tokens),
        crate::runtime_backend::grammar::semantic_lowering::VillainousChoiceModeProgram::SharedSubjectPair(pair) => {
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
    let Some(shape) = crate::runtime_backend::grammar::semantic_lowering::parse_villainous_choice_player_statement_tokens(tokens)
    else {
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
        crate::runtime_backend::grammar::semantic_lowering::VillainousChoicePlayerIteration::EachOpponent => {
            vec![EffectAst::ForEachOpponent {
                effects: vec![choice],
            }]
        }
    }))
}

pub(crate) fn parse_effect_sentence_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let dispatch_shape = effect_grammar::labeled_dispatch::parse_labeled_dispatch_shape(tokens);
    if let Some(effects) = parse_player_villainous_choice_statement(tokens)? {
        return Ok(effects);
    }
    if is_activate_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if dispatch_shape.round_up_each_time {
        return Ok(Vec::new());
    }

    if let Some(stripped) = split_labeled_effect_prefix_lexed(tokens) {
        return parse_effect_sentence_lexed(stripped);
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
    if dispatch_shape.pre_extension_head
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effects) =
        parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
    {
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
        return Ok(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
            },
        }]);
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
        let chosen = TagKey::from("__chosen_cast_from_among");
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                },
            }),
        ]);
    }
    if dispatch_shape.cast_hand_free {
        let chosen = TagKey::from("__chosen_hand_spell_to_cast");
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
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
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
    if dispatch_shape.has_gain_or_lose {
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
        let tag = TagKey::from("__chosen_cast_from_graveyard");
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
    } else {
        if shape.duration == Until::EndOfTurn {
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

fn parse_passive_color_type_addition_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        effect_grammar::labeled_dispatch::parse_passive_color_type_addition_shape(tokens)
    else {
        return Ok(None);
    };

    let target = if shape.tagged_subject {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(shape.subject_tokens)?
    };

    let mut effects = Vec::new();
    if !shape.colors.is_empty() {
        let color_effect = if shape.adds_colors {
            EffectAst::subject_verb_add_colors(target.clone(), shape.colors, Until::Forever)
        } else {
            EffectAst::subject_verb_set_colors(target.clone(), shape.colors, Until::Forever)
        };
        effects.push(color_effect);
    }
    if !shape.card_types.is_empty() {
        effects.push(EffectAst::subject_verb_add_card_types(
            target.clone(),
            shape.card_types,
            Until::Forever,
        ));
    }
    if !shape.subtypes.is_empty() {
        effects.push(EffectAst::subject_verb_add_subtypes(
            target,
            shape.subtypes,
            Until::Forever,
        ));
    }

    Ok((!effects.is_empty()).then_some(effects))
}

pub(crate) fn parse_subject_verb_extension_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    macro_rules! one {
        ($route:literal, $parser:expr) => {{
            if let Some(effect) = $parser? {
                crate::parse_trace::event(concat!("effect-route: subject-verb ", $route));
                return Ok(Some(vec![effect]));
            }
        }};
    }
    macro_rules! many {
        ($route:literal, $parser:expr) => {{
            if let Some(effects) = $parser? {
                crate::parse_trace::event(concat!("effect-route: subject-verb ", $route));
                return Ok(Some(effects));
            }
        }};
    }

    one!(
        "verb=Take subject=implicit recognizer=extra-turn-after-anchor",
        parse_take_extra_turn_sentence(tokens)
    );
    one!(
        "verb=Prevent subject=implicit recognizer=damage-prevention",
        parse_prevent_damage_sentence(tokens)
    );
    one!(
        "verb=Monstrosity subject=implicit recognizer=keyword-action",
        parse_monstrosity_sentence(tokens)
    );
    many!(
        "verb=Earthbend subject=implicit recognizer=keyword-action",
        parse_earthbend_subject_verb_sentence(tokens)
    );
    one!(
        "verb=Enchant subject=implicit recognizer=aura-attachment",
        super::search_library::parse_enchant_sentence(tokens)
    );
    one!(
        "verb=Play subject=explicit recognizer=zone-permission",
        parse_play_permission_subject_verb(tokens)
    );
    one!(
        "verb=Exile subject=implicit recognizer=instead-replacement",
        parse_zone_replacement_subject_verb(tokens)
    );
    many!(
        "verb=Is subject=explicit recognizer=passive-color-type-addition",
        parse_passive_color_type_addition_sentence(tokens)
    );
    many!(
        "verb=When subject=implicit recognizer=delayed-trigger-this-turn",
        parse_sentence_delayed_trigger_this_turn(tokens)
    );
    one!(
        "verb=Deal subject=triggering-spell recognizer=spell-count-opponent-damage",
        parse_triggered_spell_opponent_damage_subject_verb(tokens)
    );
    one!(
        "verb=Choose subject=explicit recognizer=choice-complement-sacrifice",
        parse_choice_complement_subject_verb(tokens)
    );
    many!(
        "verb=Gain subject=explicit recognizer=life-equal-stat",
        parse_gain_life_equal_to_power_sentence(tokens)
    );
    one!(
        "verb=Get subject=explicit recognizer=last-effect-counter-loop",
        parse_for_each_counter_removed_sentence(tokens)
    );
    many!(
        "verb=Exile subject=explicit recognizer=exile-return-same-object",
        parse_exile_then_return_same_object_sentence(tokens)
    );
    if let Some(effects) =
        super::chain_carry::parse_return_it_then_loses_all_abilities_lexed(tokens)?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Return subject=explicit recognizer=return-then-lose-abilities",
        );
        return Ok(Some(effects));
    }
    let ability_candidates =
        effect_grammar::labeled_dispatch::parse_ability_candidate_shape(tokens);
    if ability_candidates.simple_source_gain {
        many!(
            "verb=Gain subject=implicit recognizer=source-ability-grant",
            parse_gain_ability_to_source_subject_verb_sentence(tokens)
        );
    }
    if ability_candidates.simple_gain {
        many!(
            "verb=Gain subject=explicit recognizer=ability-grant",
            parse_gain_ability_subject_verb_sentence(tokens)
        );
    }
    many!(
        "verb=Choose subject=explicit recognizer=opponent-decline-loop",
        parse_for_each_opponent_doesnt_subject_verb_sentence(tokens)
    );
    many!(
        "verb=Vote subject=explicit recognizer=vote-affinity",
        parse_vote_affinity_subject_verb(tokens)
    );
    one!(
        "verb=Vote subject=explicit recognizer=vote-procedure",
        parse_vote_subject_verb(tokens)
    );

    Ok(None)
}

fn parse_earthbend_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = super::search_library::parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while token_slice_first_is(&tail, "then") {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    if token_slice_first_is(&tail, "earthbend") {
        if let Some(mut tail_effects) = parse_earthbend_subject_verb_sentence(&tail)? {
            effects.append(&mut tail_effects);
            return Ok(Some(effects));
        }
    }
    effects.extend(parse_effect_chain_lexed(&tail)?);
    Ok(Some(effects))
}

fn parse_gain_ability_to_source_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        super::gain_ability::parse_gain_ability_to_source_sentence(tokens)?
            .map(|effect| vec![effect]),
    )
}

fn parse_gain_ability_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn parse_for_each_opponent_doesnt_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(super::conditionals::parse_for_each_opponent_doesnt(tokens)?.map(|effect| vec![effect]))
}

#[path = "labeled_prefixes/followup_predicates.rs"]
mod followup_predicates;
pub(crate) use followup_predicates::*;
