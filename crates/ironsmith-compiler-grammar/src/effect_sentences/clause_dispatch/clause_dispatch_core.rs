use super::*;

pub(super) fn parse_effect_clause_unstacked(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    let stripped_instead = super::super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);
    let tokens = if tokens.first().is_some_and(|token| token.is_word("then")) {
        &tokens[1..]
    } else {
        tokens
    };

    if let Some(player) = super::super::chain_carry::parse_leading_player_may_lexed(tokens)
        && matches!(player, PlayerAst::Any | PlayerAst::Opponent)
    {
        let stripped = super::super::chain_carry::remove_through_first_word(tokens);
        let stripped = crate::util::trim_edge_punctuation_tokens(&stripped);
        if stripped.first().is_some_and(|token| token.is_word("pay")) {
            let payment = super::super::zone_handlers::parse_pay(
                crate::util::trim_edge_punctuation_tokens(&stripped[1..]),
                Some(SubjectAst::Player(PlayerAst::That)),
            )?;
            return Ok(EffectAst::AnyPlayerMay {
                players: if player == PlayerAst::Opponent {
                    PlayerFilter::Opponent
                } else {
                    PlayerFilter::Any
                },
                effects: vec![payment],
            });
        }
    }

    // A standalone effect sentence reaches clause dispatch directly, without
    // passing through the coordinated-chain parser. Preserve the dedicated
    // sequential-offer model for "any player/opponent may sacrifice ..." here
    // too: a broad player filter is not itself an actor and must not become
    // the chooser or sacrificing player for a single MayEffect.
    if let Some(shape) = effect_grammar::parse_any_player_may_sacrifice_shape(tokens) {
        let sacrifice = parse_sacrifice(
            shape.action_tokens,
            Some(SubjectAst::Player(PlayerAst::That)),
            None,
        )?;
        return Ok(EffectAst::AnyPlayerMay {
            players: shape.players,
            effects: vec![sacrifice],
        });
    }

    // `assigns no combat damage` is a complete effect even when Oracle
    // coordinates another effect after it. The direct shape intentionally
    // requires a sentence boundary, so split this prefix before dispatching
    // the rest of the coordinated clause.
    for (and_idx, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let prefix = trim_edge_punctuation(&tokens[..and_idx]);
        let suffix = trim_edge_punctuation(&tokens[and_idx + 1..]);
        if suffix.is_empty()
            || !matches!(
                clause_grammar::parse_assigns_no_combat_damage_shape(&prefix),
                Some(clause_grammar::AssignsNoCombatDamageShape::Supported { .. })
            )
        {
            continue;
        }
        let first = parse_effect_clause(&prefix)?;
        let mut effects = vec![first];
        effects.extend(crate::effect_sentences::parse_effect_chain_lexed(&suffix)?);
        if effects.len() > 1 {
            return Ok(EffectAst::Sequence { effects });
        }
    }

    if let Some(effect) = parse_conditional_become_pair(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = followup_grammar::parse_counter_linked_land_subtype_followup(tokens) {
        return Ok(EffectAst::subject_verb_add_subtypes(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                span_from_tokens(tokens),
            ),
            vec![shape.subtype],
            Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                    shape.counter_type,
                ),
            ),
        ));
    }

    if let Some(effect) = effect_grammar::parse_prevent_damage_sentence_lexed(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_heal_damage_clause(tokens)? {
        return Ok(effect);
    }

    // A return may itself be conditional and then immediately turn the
    // returned object face up:
    //
    // "return it ... face down if it's a permanent card, then turn it face
    // up."
    //
    // The ordinary trailing-if splitter intentionally accepts a broad
    // predicate tail, so without this structural split the final turn action
    // is swallowed into the predicate (and even makes "face up" look like a
    // characteristic of "permanent card").  Recognize the typed return/turn
    // pair before the general trailing-if route.
    for split in 0..tokens.len().saturating_sub(1) {
        if !tokens[split].is_comma() || !tokens[split + 1].is_word("then") {
            continue;
        }
        let prefix = trim_edge_punctuation(&tokens[..split]);
        let suffix = trim_edge_punctuation(&tokens[split + 2..]);
        let Some(trailing_if) = split_trailing_if_clause_lexed(&prefix) else {
            continue;
        };
        let Some(return_tokens) = trailing_if
            .leading_tokens
            .first()
            .is_some_and(|token| token.is_word("return"))
            .then_some(&trailing_if.leading_tokens[1..])
        else {
            continue;
        };
        let Ok(return_effect) = parse_return(return_tokens) else {
            continue;
        };
        let Some(turn_shape) = clause_grammar::parse_direct_clause_shape(&suffix) else {
            continue;
        };
        let turn_effect = lower_direct_clause_shape(turn_shape, &suffix);
        let returns_face_down = matches!(
            &return_effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Battlefield,
                    battlefield_face_down: true,
                    ..
                },
                ..
            })
        );
        let turns_face_up = matches!(
            &turn_effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::TurnFaceUp { .. },
                ..
            })
        );
        if returns_face_down && turns_face_up {
            return Ok(EffectAst::TrailingIf {
                predicate: trailing_if.predicate,
                effects: vec![return_effect, turn_effect],
            });
        }
    }

    if effect_grammar::control_flow::is_anaphoric_destroy_battlefield_guard(tokens)
        && tokens.first().is_some_and(|token| token.is_word("destroy"))
    {
        return crate::effect_sentences::parse_destroy(&tokens[1..]);
    }

    if let Some(trailing_if) = split_trailing_if_clause_lexed(tokens)
        && let Ok(base_effect) = parse_effect_clause(trailing_if.leading_tokens)
    {
        return Ok(EffectAst::TrailingIf {
            predicate: trailing_if.predicate,
            effects: vec![base_effect],
        });
    }

    if let Some(spec) = parse_may_cast_it_sentence(tokens) {
        return Ok(build_may_cast_tagged_effect(&spec));
    }

    if let Some(effect) = parse_play_exiled_cards_for_as_long_as_exiled_clause(tokens) {
        return Ok(effect);
    }

    if let Some(shape) =
        clause_grammar::parse_cast_target_from_your_graveyard_this_turn_shape(tokens)
    {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
                    crate::tag::CompilerReferenceTag::It.key(),
                    PlayerAst::You,
                    false,
                    false,
                    false,
                ),
            ],
        });
    }

    if let Some(effect) = parse_cast_or_play_tagged_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_any_number_from_among_tagged_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_cast_single_spell_from_among_hand_cards_clause(tokens) {
        return Ok(effect);
    }

    if let Some(effect) = parse_mana_any_type_cast_tagged_this_way_clause(tokens) {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_leading_may_shape(tokens) {
        // In permission text such as "You may play an additional land this
        // turn", "may" describes the granted game-rule permission. It is not
        // an optional resolution action and therefore must not become a
        // MayEffect decision at resolution time.
        if let Some(mut permission) = parse_additional_land_plays_clause(shape.effect_tokens)? {
            if let clause_grammar::LeadingMayActorShape::Player(player) = shape.actor {
                bind_implicit_player_context(&mut permission, player);
            }
            return Ok(permission);
        }
        let mut effects = parse_effect_chain_with_subject_verb_primitives(shape.effect_tokens)?;
        return Ok(match shape.actor {
            clause_grammar::LeadingMayActorShape::Player(player) => {
                for effect in &mut effects {
                    bind_implicit_player_context(effect, player);
                }
                EffectAst::MayByPlayer { player, effects }
            }
            clause_grammar::LeadingMayActorShape::Implicit => EffectAst::May { effects },
        });
    }

    if let Some(shape) = clause_grammar::parse_tagged_plural_pump_shape(tokens)
        && let Some(effect) =
            parse_get_pump_clause(shape.subject_tokens, shape.modifier_tokens, tokens)?
    {
        return Ok(effect);
    }

    let clause_word_view = ClauseDispatchCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();

    if let Some(effect) = parse_for_each_prevent_damage_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_for_each_counter_group_removed_this_way_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_turn_target_face_up_shape(tokens) {
        return Ok(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::TurnFaceUp {
                target: parse_target_phrase(shape.target_tokens)?,
            },
        ));
    }

    if let Some(shape) = clause_grammar::parse_direct_clause_shape(tokens) {
        return Ok(lower_direct_clause_shape(shape, tokens));
    }

    if let Some(shape) = clause_grammar::parse_shared_ability_gain_shape(tokens) {
        return Ok(EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                Some(crate::cards::builders::TextSpan::synthetic()),
            ),
            shape
                .abilities
                .into_iter()
                .map(GrantedAbilityAst::from)
                .collect(),
            Until::Forever,
        ));
    }
    if let Some(effect) = parse_take_extra_turn_sentence(tokens)? {
        return Ok(effect);
    }
    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(effect);
    }
    if let Some(spec) = parse_mana_replacement_clause_spec_lexed(tokens) {
        return Ok(EffectAst::subject_verb_register_mana_replacement(
            ObjectFilter::land().you_control(),
            vec![spec.replacement_mana],
            crate::effects::ReplacementApplyMode::UntilEndOfTurn,
        ));
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
            clause_words.join(" ")
        )));
    }

    if let Some(shape) = clause_grammar::parse_for_each_card_payment_shape(tokens) {
        let mut filter = ObjectFilter::default();
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.key(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        return Ok(EffectAst::ForEachObject {
            filter,
            effects: vec![EffectAst::UnlessAction {
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                        span_from_tokens(tokens),
                    ),
                    crate::zone::Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                alternative: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::LoseLife {
                        amount: Value::Fixed(shape.life_amount as i32),
                    },
                )],
                player: PlayerAst::You,
            }],
        });
    }

    if let Some(shape) = clause_grammar::parse_opponent_return_choice_shape(tokens) {
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    )],
                    player: PlayerAst::ItsController,
                },
            ],
        });
    }

    if let Some(effects) =
        parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
    {
        return Ok(match effects.as_slice() {
            [effect] => effect.clone(),
            _ => EffectAst::Sequence { effects },
        });
    }

    if let Some(effect) =
        parse_each_opponent_exiles_card_from_their_hand_or_permanent_they_control(tokens)
    {
        return Ok(effect);
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    let clause = SubjectVerbPrimitiveClause::new(tokens);
    if let Some(unless_idx) = find_unquoted_token_word(clause, "unless") {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        if !main_tokens.is_empty()
            && let Ok(main_effect) = parse_effect_clause(&main_tokens)
            && let Some(unless_effect) = try_build_unless(vec![main_effect], clause, unless_idx)?
        {
            return Ok(unless_effect);
        }
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_passive_sacrifice_by_controller_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_copular_base_pt_animation_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_participant_choice_then_return_chosen_set(tokens)? {
        return Ok(effect);
    }

    let choice_head = parse_choice_clause_head_tokens(tokens);
    let choice_actor = choice_head
        .as_ref()
        .map_or(ChoiceClauseActor::Implicit, |head| head.actor);
    let choice_tokens = choice_head.as_ref().map_or_else(
        || clause_grammar::strip_optional_you_choice_tokens(tokens),
        |head| head.choice_tokens,
    );
    let choice_player = match choice_actor {
        ChoiceClauseActor::Implicit => PlayerAst::Implicit,
        ChoiceClauseActor::You => PlayerAst::You,
        ChoiceClauseActor::Opponent => PlayerAst::Opponent,
    };
    let choice_word_view = ClauseDispatchCompatWords::new(choice_tokens);
    let choice_words = choice_word_view.to_word_refs();

    if let Some((consumed, excluded_color)) = parse_choose_color_phrase_words(&choice_words)?
        && consumed == choice_words.len()
        && excluded_color.is_none()
    {
        return Ok(EffectAst::subject_verb_choose_color(choice_player));
    }

    if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_creature_type(
            choice_player,
            excluded_subtypes,
        ));
    }

    if let Some(parsed) = parse_choice_land_type_phrase_words(&choice_words)
        && parsed.consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_land_type(
            choice_player,
            parsed.exclude_basic,
        ));
    }

    if let Some(parsed) = parse_choice_subtype_family_phrase_words(&choice_words)
        && parsed.consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_subtype_type(
            choice_player,
            parsed.family,
        ));
    }

    if let Some((consumed, options)) = parse_choose_card_type_phrase_words(&choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_card_type(
            choice_player,
            options,
        ));
    }

    if let Some(consumed) = parse_choose_player_phrase_words(&choice_words)
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::subject_verb_choose_player(
            choice_player,
            PlayerFilter::Any,
            crate::tag::CompilerReferenceTag::It.key(),
            false,
            0,
        ));
    }

    if let Some(shape) = clause_grammar::parse_ordered_choose_all_shape(tokens) {
        let filter = parse_object_filter(shape.filter_tokens, false)?;
        let repeated_filter = parse_object_filter(shape.repeated_filter_tokens, false)?;
        if filter != repeated_filter {
            return Err(CardTextError::ParseError(format!(
                "ordered choice stopping filter differs from chosen filter (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(EffectAst::ChooseObjects {
            filter: filter.clone(),
            count: ChoiceCount::dynamic_x(),
            count_value: Some(
                Value::Count(filter).with_surface_hint(ValueSurfaceHint::ChooseAllInOrder),
            ),
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        });
    }

    if let Some(shape) = clause_grammar::parse_choose_target_shape(tokens)
        && let Ok(mut target) = parse_target_phrase(shape.target_tokens)
    {
        if shape.excludes_chooser_controller {
            preserve_target_choice_controller_exclusion(&mut target, shape.chooser);
        }
        let player_target = match &target {
            TargetAst::Player(_, _) => true,
            TargetAst::WithCount(inner, _) => matches!(inner.as_ref(), TargetAst::Player(_, _)),
            _ => false,
        };
        if player_target
            || clause_grammar::parse_clause_subject_verb_shape(shape.target_tokens).is_none()
        {
            return Ok(explicit_target_choice(shape, target));
        }
    }

    if let Some((chooser, choose_filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(tokens)?
    {
        return Ok(EffectAst::subject_verb_choose_player(
            chooser,
            choose_filter,
            crate::tag::CompilerReferenceTag::It.key(),
            random,
            exclude_previous_choices,
        ));
    }

    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_target_player_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        });
    }

    if let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        });
    }

    if let Some(shape) = clause_grammar::parse_assigns_no_combat_damage_shape(tokens) {
        match shape {
            clause_grammar::AssignsNoCombatDamageShape::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
                    clause_words.join(" ")
                )));
            }
            clause_grammar::AssignsNoCombatDamageShape::Supported { source, duration } => {
                let source = match source {
                    clause_grammar::AssignDamageSourceShape::Source => TargetAst::Source(None),
                    clause_grammar::AssignDamageSourceShape::Tagged => TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                        span_from_tokens(tokens),
                    ),
                    clause_grammar::AssignDamageSourceShape::Target(target_tokens) => {
                        parse_target_phrase(target_tokens)?
                    }
                };
                return Ok(EffectAst::subject_verb_assign_no_combat_damage(
                    source, duration,
                ));
            }
        }
    }

    let restriction_duration_shape = if find_negation_span(tokens).is_some() {
        effect_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
    } else {
        None
    };
    let (
        restriction_duration,
        restriction_clause_tokens,
        restriction_duration_surface,
        has_restriction_duration,
    ) = match restriction_duration_shape {
        Some(shape) => {
            let surface = if shape.duration == Until::EndOfTurn
                && shape.placement == effect_grammar::SearchRestrictionDurationPlacement::Prefix
            {
                crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
            } else {
                crate::effect::RestrictionDurationSurface::Default
            };
            (shape.duration, shape.remainder, surface, true)
        }
        None => (
            Until::Forever,
            tokens.to_vec(),
            crate::effect::RestrictionDurationSurface::Default,
            false,
        ),
    };

    if starts_with_target_indicator(&restriction_clause_tokens)
        && find_negation_span(&restriction_clause_tokens).is_some_and(|(neg_start, _)| {
            find_verb(&restriction_clause_tokens[..neg_start]).is_none()
        })
        && let Some(restrictions) = parse_cant_restrictions(&restriction_clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && let Some(target) = parsed.target.clone()
    {
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::subject_verb_cant_starting_with_duration_surface(
                    parsed.restriction.clone(),
                    restriction_duration,
                    crate::effect::RestrictionStart::Immediate,
                    restriction_duration_surface,
                    None,
                ),
            ],
        });
    }

    if let Some(shape) = clause_grammar::parse_target_only_shape(tokens) {
        if find_negation_span(tokens).is_some() || shape.restriction_like {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::subject_verb_target_only(target));
    }

    if let Some(shape) = clause_grammar::parse_embedded_choose_target_shape(tokens) {
        let mut target = parse_target_phrase(shape.target_tokens)?;
        if shape.excludes_chooser_controller {
            preserve_target_choice_controller_exclusion(&mut target, shape.chooser);
        }
        return Ok(explicit_target_choice(shape, target));
    }

    if let Some(effect) = parse_next_turn_cant_clause(tokens)? {
        return Ok(effect);
    }

    if has_restriction_duration
        && find_negation_span(&restriction_clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&restriction_clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(EffectAst::subject_verb_cant_starting_with_duration_surface(
            parsed.restriction.clone(),
            restriction_duration,
            crate::effect::RestrictionStart::Immediate,
            restriction_duration_surface,
            None,
        ));
    }

    if let Some(effect) = parse_hexproof_targeting_override_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(shape) = clause_grammar::parse_cast_target_without_paying_shape(tokens) {
        let _ = parse_target_phrase(shape.target_tokens)?;
        return Ok(EffectAst::SubjectVerb(
            crate::model::ast::SubjectVerbEffectAst {
                subject: crate::model::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::Implicit,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: crate::tag::CompilerReferenceTag::It.key(),
                    player: PlayerAst::Implicit,
                    allow_land: false,
                    as_copy: false,
                    copy_cast_reminder_surface: false,
                    copy_instruction_surface: None,
                    without_paying_mana_cost: true,
                    additional_mana_cost: None,
                    cost_reduction: None,
                    mana_spend_mode: ironsmith_core::value_model::ManaSpendMode::Normal,
                },
            },
        ));
    }

    if let Some(effect) = parse_passive_goad_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_control_player_clause(tokens)? {
        return Ok(effect);
    }

    // Generic "X if <predicate>" fallback: clauses like "play the exiled card
    // without paying its mana cost if you attacked with three or more
    // creatures this turn" have no known leading verb, but the head parses on
    // its own and the tail is a recognizable predicate. Only attempted where
    // the clause would otherwise be a hard no-verb error.
    if clause_grammar::parse_clause_subject_verb_shape(tokens).is_none()
        && let Some(shape) = clause_grammar::parse_trailing_if_fallback_shape(tokens)
        && let Ok(head_effects) = super::super::parse_effect_sentence_lexed(shape.head_tokens)
        && !head_effects.is_empty()
    {
        parser_trace("parse_effect_clause:trailing-if-fallback", tokens);
        return Ok(EffectAst::Conditional {
            predicate: shape.predicate,
            if_true: head_effects,
            if_false: Vec::new(),
        });
    }

    let (verb, _) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "unattach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
            "suspect",
            "end",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    let verb_shape = clause_grammar::parse_clause_subject_verb_shape(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "could not split subject and verb in effect clause (clause: '{}')",
            render_lower_words(tokens)
        ))
    })?;
    let subject_tokens_storage = trim_commas(verb_shape.subject_tokens);
    let subject_tokens = subject_tokens_storage.as_slice();
    let rest = verb_shape.action_tokens;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);
    crate::parse_trace::event(format!(
        "effect-route: subject-verb verb={verb:?} subject={}",
        if subject_tokens.is_empty() {
            "implicit"
        } else {
            "explicit"
        }
    ));

    if matches!(verb, Verb::Counter)
        && !subject_tokens.is_empty()
        && contains_token_word(tokens, "on")
        && let Ok(effect) = parse_put_counters(tokens)
    {
        parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
        return Ok(effect);
    }

    if matches!(verb, Verb::Get)
        && let Some(effect) = parse_get_pump_clause(subject_tokens, rest, tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Sacrifice)
        && let Some((subject, target)) = parse_controller_or_owner_of_target_subject(subject_tokens)
    {
        return parse_sacrifice(rest, Some(subject), Some(target));
    }
    if matches!(verb, Verb::Put)
        && let Some((SubjectAst::Player(PlayerAst::ItsOwner), target)) =
            parse_controller_or_owner_of_target_subject(subject_tokens)
        && is_pronoun_top_or_bottom_library_choice_put_tail(rest)
    {
        return Ok(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::ItsOwner,
            SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target },
        ));
    }
    let subject_word_view = ClauseDispatchCompatWords::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain)
        && !subject_tokens.is_empty()
        && let Some(shape) = clause_grammar::parse_protection_choice_shape(rest)
    {
        let target = parse_target_phrase(subject_tokens)?;
        return Ok(EffectAst::subject_verb_grant_protection_choice(
            target,
            match shape.chooser {
                clause_grammar::ProtectionChoiceChooserShape::You => PlayerAst::You,
                clause_grammar::ProtectionChoiceChooserShape::TargetController => {
                    PlayerAst::ItsController
                }
            },
            shape.includes_colorless,
            shape.includes_artifacts,
            shape.chooses_card_type,
        ));
    }
    if matches!(verb, Verb::Gain)
        && let Some(effects) =
            super::super::fanout_family::parse_shared_color_target_fanout_sentence(tokens)?
    {
        return Ok(EffectAst::Sequence { effects });
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let parsed_actions = parse_ability_line(tail.ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(tail.ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !tail.ability_tokens.is_empty()
            && tail.trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_grant_abilities_to_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Lose) && clause_grammar::parse_shared_ability_gain_shape(rest).is_some()
    {
        let target = match clause_grammar::parse_reference_subject_shape(subject_tokens) {
            clause_grammar::ReferenceSubjectShape::Source => {
                TargetAst::Source(span_from_tokens(subject_tokens))
            }
            clause_grammar::ReferenceSubjectShape::Tagged => TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                span_from_tokens(subject_tokens),
            ),
            clause_grammar::ReferenceSubjectShape::Other => parse_target_phrase(subject_tokens)?,
        };
        return Ok(EffectAst::subject_verb_remove_abilities_from_target(
            target,
            Vec::new(),
            Until::EndOfTurn,
        ));
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Lose) {
        let tail = clause_grammar::parse_ability_tail_shape(rest);
        let ability_tokens = trim_edge_punctuation(tail.ability_tokens);
        let trailing_tokens = trim_edge_punctuation(tail.trailing_tokens);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = ClauseDispatchCompatWords::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_tokens
                .first()
                .is_some_and(|token| token.is_word(TARGET_WORD))
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                tail.duration,
            ));
        }
    }
    if matches!(verb, Verb::Deal)
        && let Some(effect) = parse_explicit_target_object_damage_source(subject_tokens, rest)?
    {
        return Ok(effect);
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let subject_words = crate::lexer::parser_token_word_refs(subject_tokens);
    let each_other_player = crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[&["each"], &["other"], &["player", "players"]],
    );
    let another_target_player = crate::word_primitives::parse_sequence_complete(
        &subject_words,
        &["another", "target", "player"],
    );
    let optional_target_player = if crate::word_primitives::parse_choice_sequence_complete(
        &subject_words,
        &[
            &["up"],
            &["to"],
            &["one"],
            &["target"],
            &["player", "players"],
        ],
    ) {
        Some(TargetAst::WithCount(
            Box::new(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(subject_tokens),
            )),
            ChoiceCount::up_to(1),
        ))
    } else if crate::word_primitives::parse_sequence_prefix(&subject_words, &["up", "to"]) {
        let target = parse_target_phrase(subject_tokens)?;
        let is_optional_player = matches!(
            &target,
            TargetAst::WithCount(inner, count)
                if matches!(inner.as_ref(), TargetAst::Player(_, _))
                    && count.min == 0
                    && count.max == Some(1)
        );
        is_optional_player.then_some(target)
    } else {
        None
    };
    if matches!(verb, Verb::Return)
        && clause_grammar::is_return_tagged_reference_shape(subject_tokens)
    {
        let mut return_tokens = subject_tokens.to_vec();
        return_tokens.extend(rest.iter().cloned());
        return parse_effect_with_verb(verb, Some(SubjectAst::This), &return_tokens);
    }
    if matches!(verb, Verb::Put)
        && clause_grammar::is_exiled_cards_to_hand_shape(subject_tokens, rest)
    {
        let filter = parse_object_filter(subject_tokens, false)?;
        return Ok(EffectAst::subject_verb_return_all_to_hand(filter));
    }
    let relative_player_subject = if matches!(verb, Verb::Gain)
        && rest.first().is_some_and(|token| token.is_word("control"))
        && subject_tokens
            .first()
            .is_some_and(|token| token.is_word(TARGET_WORD))
    {
        match parse_target_phrase(subject_tokens) {
            Ok(target) => match &target {
                TargetAst::Player(filter, _)
                    if !matches!(filter, PlayerFilter::Any | PlayerFilter::Opponent) =>
                {
                    Some(target)
                }
                _ => None,
            },
            Err(_) => None,
        }
    } else {
        None
    };
    let mut effect = if let Some(target) = optional_target_player {
        let action = parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if another_target_player {
        let target = TargetAst::Player(
            PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::target_player()),
            span_from_tokens(subject_tokens),
        );
        let action = parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), action],
        }
    } else if let Some(target) = relative_player_subject {
        let source_relative_target = target_player_mentions_source_object(&target);
        let mut gain_control =
            parse_effect_with_verb(verb, Some(SubjectAst::Player(PlayerAst::That)), rest)?;
        if source_relative_target {
            bind_gain_control_pronoun_to_source(&mut gain_control);
        }
        EffectAst::Sequence {
            effects: vec![EffectAst::subject_verb_target_only(target), gain_control],
        }
    } else if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = if each_other_player {
            SubjectAst::Player(PlayerAst::That)
        } else {
            parse_subject(subject_tokens)
        };
        if let Some(clause) = CommonPlayerActionClause::recognize(subject, verb, rest) {
            clause.lower()?
        } else {
            parse_effect_with_verb(verb, Some(subject), rest)?
        }
    };
    let authored_control_pronoun = {
        let rest_words = ClauseDispatchCompatWords::new(rest).to_word_refs();
        crate::word_primitives::sequence_occurs(&rest_words, &["they", "control"])
    };
    if matches!(verb, Verb::Return)
        && (crate::word_primitives::parse_sequence_complete(&subject_words, &["they"])
            || authored_control_pronoun)
        && let EffectAst::SubjectVerb(subject_verb) = &mut effect
        && let SubjectVerbActionAst::ReturnToHand { target, .. } = &mut subject_verb.action
    {
        fn mark_iterated_actor_pronoun(target: &mut TargetAst) {
            match target {
                TargetAst::Object(filter, ..) => {
                    filter.set_iterated_actor_pronoun_surface(true);
                }
                TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
                    mark_iterated_actor_pronoun(inner);
                }
                _ => {}
            }
        }
        mark_iterated_actor_pronoun(target);
    }
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    if each_other_player {
        effect = EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::NotYou,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

pub(super) fn parse_passive_goad_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = clause_grammar::parse_passive_goad_shape(tokens) else {
        return Ok(None);
    };
    let target = match shape.target {
        clause_grammar::GoadTargetShape::TaggedToken => TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span_from_tokens(tokens),
        ),
        clause_grammar::GoadTargetShape::Target(target_tokens) => {
            parse_target_phrase(target_tokens)?
        }
    };
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    }

    let duration = if shape.for_rest_of_game {
        Until::Forever
    } else {
        Until::YourNextTurn
    };
    Ok(Some(EffectAst::subject_verb_goad_for(target, duration)))
}

pub fn parse_effect_clause_lexed(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_effect_clause(tokens)
}
