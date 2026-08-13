{
    fn unwrap_modal_common_suffix_effect(effect: &Effect) -> &Effect {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return unwrap_modal_common_suffix_effect(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return unwrap_modal_common_suffix_effect(&with_id.effect);
        }
        effect
    }

    fn modal_common_return_effect(
        mode: &crate::effect::EffectMode,
    ) -> Option<&crate::effects::ReturnFromGraveyardToHandEffect> {
        unwrap_modal_common_suffix_effect(mode.effects.last()?)
            .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    }

    fn describe_modal_common_suffix(
        choose_mode: &crate::effects::ChooseModeEffect,
    ) -> Option<String> {
        if choose_mode.common_suffix_effect_count != 1 || choose_mode.modes.is_empty() {
            return None;
        }

        let first = modal_common_return_effect(choose_mode.modes.first()?)?;
        if first.random || first.actor_surface.is_some() || !first.target.is_target() {
            return None;
        }
        let ChooseSpec::Object(first_filter) = first.target.base() else {
            return None;
        };
        if first_filter.zone != Some(crate::zone::Zone::Graveyard)
            || !first_filter.has_explicit_card_noun()
        {
            return None;
        }
        let graveyard_player = first.graveyard_player_surface.as_ref()?;
        let destination_player = first.destination_player_surface.as_ref()?;

        for mode in &choose_mode.modes {
            let returned = modal_common_return_effect(mode)?;
            let ChooseSpec::Object(filter) = returned.target.base() else {
                return None;
            };
            if returned.random
                || returned.actor_surface.is_some()
                || !returned.target.is_target()
                || filter.zone != Some(crate::zone::Zone::Graveyard)
                || !filter.has_explicit_card_noun()
                || returned.graveyard_player_surface.as_ref() != Some(graveyard_player)
                || returned.destination_player_surface.as_ref() != Some(destination_player)
            {
                return None;
            }
        }

        Some(format!(
            "Return those cards from {} graveyard to {} hand.",
            describe_possessive_player_filter(graveyard_player),
            describe_possessive_player_filter(destination_player),
        ))
    }

    if let Some(target_only) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
        let target = describe_choose_spec(&target_only.target);
        return target_only.chooser.as_ref().map_or_else(
            || format!("Choose {target}"),
            |chooser| {
                format!(
                    "{} chooses {target}",
                    capitalize_first(&describe_player_filter(chooser))
                )
            },
        );
    }
    if let Some(villainous) = effect.downcast_ref::<crate::effects::VillainousChoiceEffect>() {
        return describe_villainous_choice(villainous);
    }
    if let Some(compact) = describe_compact_protection_choice(effect) {
        return compact;
    }
    if let Some(compact) = describe_compact_destroy_color_choice(effect) {
        return compact;
    }
    if let Some(compact) = describe_compact_return_to_hand_color_choice(effect) {
        return compact;
    }
    if let Some(compact) = describe_compact_keyword_choice(effect) {
        return compact;
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        if let Some(compact) = describe_choose_card_type_reveal_partition(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_inline_token_creation_choice(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_inline_destroy_all_choice(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_inline_pt_modifier_choice(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_endure_mode(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_tap_or_untap_mode(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_put_counter_choice_mode(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_put_or_remove_counter_mode(choose_mode) {
            return compact;
        }
        let mut header = describe_mode_choice_header(
            &choose_mode.choose_count,
            Some(&choose_mode.min_choose_count),
            Some(choose_mode.modes.len()),
        );
        if choose_mode.spree {
            header = "Spree (Choose one or more additional costs.)".to_string();
        }
        if choose_mode.tiered {
            header = "Tiered (Choose one additional cost.)".to_string();
        }
        if let Some(range) = &choose_mode.conditional_mode_range
            && !choose_mode.spree
            && !choose_mode.tiered
        {
            let condition = match &range.required_optional_cost.kind {
                crate::cost::OptionalCostKind::Kicker => "this spell was kicked".to_string(),
                crate::cost::OptionalCostKind::Additional => {
                    "this spell's additional cost was paid".to_string()
                }
                _ => describe_condition(&crate::effect::Condition::ThisSpellPaidLabel(
                    range.required_optional_cost.clone(),
                )),
            };
            let selection = match (&range.min_modes, &range.max_modes) {
                (Value::Fixed(0), Value::Fixed(max))
                    if usize::try_from(*max).ok() == Some(choose_mode.modes.len()) =>
                {
                    "any number".to_string()
                }
                (Value::Fixed(1), Value::Fixed(max))
                    if usize::try_from(*max).ok() == Some(choose_mode.modes.len()) =>
                {
                    "one or more".to_string()
                }
                (Value::Fixed(1), Value::Fixed(2)) if choose_mode.modes.len() == 2 => {
                    "both".to_string()
                }
                (_, max) => describe_value(max),
            };
            let base = header
                .trim_end_matches('-')
                .trim_end_matches('—')
                .trim()
                .trim_end_matches('.')
                .to_string();
            header = format!("{base}. If {condition}, choose {selection} instead.");
        }
        if choose_mode.random {
            if let Some(prefix) = header.strip_suffix(" —") {
                header = format!("{prefix} at random —");
            }
        }
        let has_weighted_modes = choose_mode.mode_point_costs.iter().any(|cost| *cost != 1);
        if has_weighted_modes {
            if let crate::effect::Value::Fixed(max) = &choose_mode.choose_count {
                if choose_mode.min_choose_count == crate::effect::Value::Fixed(0) {
                    let max_word = number_word(*max).unwrap_or_else(|| max.to_string());
                    header = format!("Choose up to {max_word} {{P}} worth of modes —");
                }
            }
        }
        if choose_mode.disallow_previously_chosen_modes {
            header = if choose_mode.disallow_previously_chosen_modes_this_turn {
                "Choose one that hasn't been chosen this turn —".to_string()
            } else {
                "Choose one that hasn't been chosen —".to_string()
            };
        }
        if let Some(basis) = describe_shared_spells_cast_modal_x_basis(choose_mode)
            && let Some(prefix) = header.strip_suffix(" —")
        {
            header = format!("{prefix}. X is {basis} —");
        }
        if !choose_mode.common_prefix_effects.is_empty() {
            let normalized_header = header
                .trim_end_matches('-')
                .trim_end_matches('—')
                .trim()
                .trim_end_matches('.')
                .to_string();
            let mut common = describe_effect_list(&choose_mode.common_prefix_effects)
                .trim()
                .trim_end_matches('.')
                .to_string();
            if ["This ", "That ", "Those ", "Target ", "You "]
                .iter()
                .any(|prefix| common.starts_with(prefix))
            {
                common = lowercase_first(&common);
            }
            header = format!("{normalized_header} and {common}.");
        }
        if choose_mode.distinct_player_targets_per_mode
            && let Some(prefix) = header.strip_suffix(" —")
        {
            header = format!("{prefix}. Each mode must target a different player.");
        }
        if let Some(common_suffix) = describe_modal_common_suffix(choose_mode) {
            let normalized_header = header
                .trim_end_matches('-')
                .trim_end_matches('—')
                .trim()
                .trim_end_matches('.')
                .to_string();
            header = format!("{normalized_header}. {common_suffix}");
        }
        if let Some(label) = choose_mode
            .presentation_label
            .as_ref()
            .and_then(crate::ability::PresentationLabel::display_prefix)
        {
            header = format!("{} — {header}", label.trim());
        }
        let use_mode_source_text = choose_mode.modes.iter().all(|mode| {
            let source = mode.source_text.trim();
            !source.is_empty()
                && !source.eq_ignore_ascii_case("POISON")
                && !source.to_ascii_lowercase().contains("enters with")
        });
        let modes = choose_mode
            .modes
            .iter()
            .enumerate()
            .map(|(mode_idx, mode)| {
                let mode_effects = describe_effect_list(&mode.effects);
                let description_raw = if use_mode_source_text {
                    mode.source_text.trim()
                } else {
                    mode_effects.trim()
                };
                let mode_label = if choose_mode.spree && !choose_mode.tiered {
                    choose_mode
                        .mode_additional_mana_costs
                        .get(mode_idx)
                        .map(|cost| format!("+ {} — ", cost.to_oracle()))
                } else if has_weighted_modes {
                    let cost = choose_mode
                        .mode_point_costs
                        .get(mode_idx)
                        .copied()
                        .unwrap_or(1)
                        .max(1);
                    Some(format!("{} — ", "{P}".repeat(cost as usize)))
                } else {
                    None
                };
                let description = capitalize_first(&ensure_trailing_period(description_raw));
                if !description.trim().is_empty() {
                    let mode_effects_trimmed = description_raw;
                    if mode_effects_trimmed.is_empty() {
                        return format!("{}{}", mode_label.as_deref().unwrap_or(""), description);
                    }

                    let effects_lower = mode_effects_trimmed.to_ascii_lowercase();
                    let description_lower = description_raw.to_ascii_lowercase();
                    let has_followup = (effects_lower.contains("if you do")
                        || effects_lower.contains("if they do")
                        || effects_lower.contains("choose one")
                        || effects_lower.contains("choose one or"))
                        && !description_lower.contains("if you do")
                        && !description_lower.contains("if they do")
                        && !description_lower.contains("choose one");

                    if !has_followup {
                        return format!("{}{}", mode_label.as_deref().unwrap_or(""), description);
                    }

                    let mut followup = mode_effects_trimmed.to_string();
                    if let Some((_, tail)) = followup.split_once(". ") {
                        followup = tail.trim().to_string();
                    } else if let Some((_, tail)) = followup.split_once('.') {
                        followup = tail.trim().to_string();
                    }
                    let description_head = description.trim_end_matches('.');
                    if followup.is_empty() {
                        if let Some(stripped) = mode_effects_trimmed.strip_prefix(description_head)
                        {
                            followup = stripped.trim_start_matches('.').trim().to_string();
                        } else {
                            followup = mode_effects_trimmed.to_string();
                        }
                    }
                    if followup.is_empty() {
                        format!("{}{}", mode_label.as_deref().unwrap_or(""), description)
                    } else {
                        format!(
                            "{}{} {}",
                            mode_label.as_deref().unwrap_or(""),
                            description_head,
                            ensure_trailing_period(followup.trim())
                        )
                    }
                } else {
                    format!(
                        "{}{}",
                        mode_label.as_deref().unwrap_or(""),
                        ensure_trailing_period(mode_effects.trim())
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(" • ");
        let bullet_modes = if choose_mode.spree && !choose_mode.tiered {
            format!("{header}\n{}", modes.replace(" • ", "\n"))
        } else {
            format!("{header}\n• {}", modes.replace(" • ", "\n• "))
        };
        if choose_mode.disallow_previously_chosen_modes {
            return bullet_modes;
        }
        if choose_mode.allow_repeated_modes {
            let normalized_header = header
                .trim_end_matches('-')
                .trim_end_matches('—')
                .trim()
                .trim_end_matches('.')
                .to_string();
            let repeated_header =
                format!("{normalized_header}. You may choose the same mode more than once.");
            return if choose_mode.spree && !choose_mode.tiered {
                format!("{repeated_header}\n{}", modes.replace(" • ", "\n"))
            } else {
                format!("{repeated_header}\n• {}", modes.replace(" • ", "\n• "))
            };
        }
        return bullet_modes;
    }
    if let Some(create_token) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        if let Some(compact) = describe_token_definition_with_end_step_sacrifice(create_token) {
            return compact;
        }
        if let Some(compact) = describe_compact_create_token(create_token) {
            return compact;
        }
        let append_token_cleanup_sentences = |mut text: String, singular: bool| {
            let token_pronoun = if singular { "it" } else { "them" };
            if create_token.exile_at_end_of_combat {
                text.push_str(&format!(". Exile {token_pronoun} at end of combat"));
            }
            if create_token.sacrifice_at_end_of_combat {
                text.push_str(&format!(". Sacrifice {token_pronoun} at end of combat"));
            }
            if create_token.sacrifice_at_next_end_step {
                let timing =
                    describe_next_end_step_cleanup_timing(&create_token.next_end_step_player);
                text.push_str(&format!(
                    ". Sacrifice {token_pronoun} at the beginning of {timing}"
                ));
            }
            if create_token.exile_at_next_end_step {
                let timing =
                    describe_next_end_step_cleanup_timing(&create_token.next_end_step_player);
                text.push_str(&format!(
                    ". Exile {token_pronoun} at the beginning of {timing}"
                ));
            }
            text
        };
        let attack_target_suffix = || match &create_token.attack_target_mode {
            Some(crate::effects::CopyAttackTargetMode::Player(player)) => {
                format!(" {}", describe_player_filter(player))
            }
            Some(
                crate::effects::CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player),
            ) => format!(
                " {} or a planeswalker they control",
                describe_player_filter(player)
            ),
            None => String::new(),
        };
        let append_token_entry_flags = |mut text: String, singular: bool| {
            if create_token.enters_tapped && create_token.enters_attacking {
                if singular {
                    text.push_str(" that's tapped and attacking");
                } else {
                    text.push_str(" that are tapped and attacking");
                }
                text.push_str(&attack_target_suffix());
                return text;
            }
            if create_token.enters_attacking {
                text.push_str(", attacking");
                text.push_str(&attack_target_suffix());
            }
            text
        };
        let describe_created_token_blueprint = || {
            let blueprint = describe_create_token_blueprint_with_presentation(
                create_token,
                create_token.ability_presentation,
            );
            if create_token.enters_tapped && !create_token.enters_attacking {
                format!("tapped {blueprint}")
            } else {
                blueprint
            }
        };
        if value_is_iterated_object_count(&create_token.count) {
            let token_blueprint = describe_created_token_blueprint();
            let (token_main, token_ability) = split_token_ability_sentence(&token_blueprint);
            let mut text = describe_create_token_action(
                &format!("a {token_main}"),
                &create_token.controller,
                create_token.actor_surface_explicit,
            );
            text = append_token_entry_flags(text, true);
            text = append_token_ability_sentence(text, token_ability);
            return append_token_cleanup_sentences(text, true);
        }
        if let Some(for_each_count) = describe_create_for_each_count(&create_token.count) {
            let token_blueprint = describe_created_token_blueprint();
            let (token_main, token_ability) = split_token_ability_sentence(&token_blueprint);
            let token_ability = token_ability.map(|sentence| {
                sentence
                    .replacen(". It has ", ". Those tokens have ", 1)
                    .replacen(". They have ", ". Those tokens have ", 1)
                    .replacen(". It gains ", ". Those tokens gain ", 1)
                    .replacen(". They gain ", ". Those tokens gain ", 1)
            });
            let mut text = if create_token.enters_tapped && create_token.enters_attacking {
                let action = describe_create_token_action(
                    &format!("a {token_main}"),
                    &create_token.controller,
                    create_token.actor_surface_explicit,
                );
                format!(
                    "{} for each {for_each_count}",
                    append_token_entry_flags(action, true)
                )
            } else {
                let action = describe_create_token_action(
                    &format!("a {token_main} for each {for_each_count}"),
                    &create_token.controller,
                    create_token.actor_surface_explicit,
                );
                append_token_entry_flags(action, true)
            };
            text = append_token_ability_sentence(text, token_ability);
            text = append_token_cleanup_sentences(text, false);
            if matches!(
                create_token.count.unhinted(),
                Value::SourceRegeneratedThisTurnCount
            ) {
                return format!(
                    "if this creature regenerated this turn, {}",
                    lowercase_first(&text)
                );
            }
            return text;
        }
        let use_where_x = should_render_token_count_with_where_x(&create_token.count);
        let singular_count = matches!(create_token.count, Value::Fixed(1)) && !use_where_x;
        let token_blueprint = describe_created_token_blueprint();
        let token_phrase = if singular_count {
            token_blueprint
        } else {
            pluralize_token_phrase(&token_blueprint)
        };
        let count_text = if use_where_x {
            "X".to_string()
        } else if singular_count {
            "a".to_string()
        } else if matches!(
            create_token.count.unhinted(),
            Value::EffectMetric {
                metric: crate::effect::EffectMetric::OtherNumber,
                ..
            } | Value::PendingEffectMetric {
                metric: crate::effect::EffectMetric::OtherNumber,
                ..
            }
        ) {
            "a number of".to_string()
        } else {
            describe_effect_count_backref(&create_token.count)
                .unwrap_or_else(|| describe_object_count(&create_token.count))
        };
        let (token_main_raw, token_ability) = split_token_ability_sentence(&token_phrase);
        let mut token_main = token_main_raw.to_string();
        let dynamic_pt_where = if singular_count {
            normalize_dynamic_equal_pt_token_phrase(&token_main).map(|(normalized, where_x)| {
                token_main = normalized;
                where_x
            })
        } else {
            None
        };
        let object_text = if value_has_surface_hint(
            &create_token.count,
            ValueSurfaceHint::EqualTo,
        ) {
            let equal_to_value = create_token
                .count
                .clone()
                .without_surface_hint(ValueSurfaceHint::EqualTo);
            format!(
                "a number of {token_main} equal to {}",
                describe_value(&equal_to_value)
            )
        } else if singular_count && token_main.contains(", a ") {
            token_main
        } else if singular_count {
            singular_token_phrase_with_article(&token_main)
        } else if matches!(
            create_token.count.unhinted(),
            Value::EffectMetric {
                metric: crate::effect::EffectMetric::OtherNumber,
                ..
            } | Value::PendingEffectMetric {
                metric: crate::effect::EffectMetric::OtherNumber,
                ..
            }
        ) {
            format!("{} {} equal to the other result", count_text, token_main)
        } else {
            format!("{} {}", count_text, token_main)
        };
        let mut text = describe_create_token_action(
            &object_text,
            &create_token.controller,
            create_token.actor_surface_explicit,
        );
        text = append_token_entry_flags(text, singular_count);
        if use_where_x {
            text = append_token_where_x_continuation(
                text,
                &describe_value(&create_token.count),
            );
        } else if let Some(where_x) = dynamic_pt_where {
            text = append_token_where_x_continuation(text, &where_x);
        }
        text = append_token_ability_sentence(text, token_ability);
        return append_token_cleanup_sentences(text, singular_count);
    }
    if let Some(create_copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        if let Some(text) = describe_source_exiled_retyped_keyword_token_copy(create_copy) {
            return text;
        }
        if let Some(text) =
            describe_retyped_token_copy_with_granted_activated_ability(create_copy)
        {
            return text;
        }
        let target = match &create_copy.target {
            ChooseSpec::Tagged(tag) if tag.as_str().starts_with("exile_cost_") => {
                "the exiled card".to_string()
            }
            ChooseSpec::Object(filter) if is_source_exiled_cards_filter(filter) => {
                "the exiled card".to_string()
            }
            ChooseSpec::Object(filter) => describe_exiled_card_copy_target_filter(filter)
                .map(str::to_string)
                .unwrap_or_else(|| describe_choose_spec(&create_copy.target)),
            ChooseSpec::Tagged(tag)
                if tag.as_str().starts_with("__sentence_helper_exiled")
                    && create_copy.set_base_power_toughness.is_some()
                    && create_copy.added_card_types.is_empty()
                    && create_copy.added_subtypes.is_empty() =>
            {
                "that card".to_string()
            }
            _ => describe_choose_spec(&create_copy.target),
        };
        let player_only_attack = matches!(
            create_copy.attack_target_mode,
            Some(crate::effects::CopyAttackTargetMode::Player(_))
        );
        let inline_tapped = create_copy.enters_tapped && !player_only_attack;
        let inline_attacking =
            create_copy.enters_attacking && create_copy.attack_target_mode.is_none();
        let token_state = match (inline_tapped, inline_attacking) {
            (true, true) => "tapped and attacking ",
            (true, false) => "tapped ",
            (false, true) => "attacking ",
            (false, false) => "",
        };
        let use_where_x = value_prefers_where_x(&create_copy.count);
        let token_object = match create_copy.count {
            _ if use_where_x => format!("X {token_state}tokens that are copies of {target}"),
            Value::Fixed(1) => format!("a {token_state}token that's a copy of {target}"),
            Value::Fixed(n) => {
                format!("{n} {token_state}tokens that are copies of {target}")
            }
            _ => format!(
                "{} {token_state}tokens that are copies of {target}",
                describe_value(&create_copy.count)
            ),
        };
        let copied_object_controller_is_actor = match (
            &create_copy.controller,
            create_copy.target.unhinted(),
        ) {
            (
                PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(controller_tag)),
                ChooseSpec::Tagged(target_tag),
            ) => controller_tag == target_tag,
            (
                PlayerFilter::ControllerOf(crate::target::ObjectRef::Target),
                ChooseSpec::Target(_),
            ) => true,
            (
                PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(controller_tag)),
                ChooseSpec::Iterated,
            ) => controller_tag.as_str() == "__it__",
            _ => false,
        };
        let mut text = if copied_object_controller_is_actor {
            let actor = if target == "it" {
                "Its controller".to_string()
            } else {
                format!("{}'s controller", capitalize_first(&target))
            };
            format!("{actor} creates {token_object}")
        } else {
            format!("Create {token_object}")
        };
        if !copied_object_controller_is_actor
            && !matches!(create_copy.controller, PlayerFilter::You)
        {
            text.push_str(&format!(
                " under {} control",
                describe_possessive_player_filter(&create_copy.controller)
            ));
        }
        if create_copy.enters_tapped && !inline_tapped && !player_only_attack {
            text.push_str(", tapped");
        }
        if create_copy.enters_attacking && !inline_attacking {
            match &create_copy.attack_target_mode {
                Some(crate::effects::CopyAttackTargetMode::Player(_)) => {}
                Some(
                    crate::effects::CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                        player_filter,
                    ),
                ) => {
                    let player = describe_player_filter(player_filter);
                    text.push_str(&format!(
                        ", attacking {player} or a planeswalker they control"
                    ));
                }
                None => text.push_str(", attacking"),
            }
        }
        let singular_copy = matches!(create_copy.count, Value::Fixed(1));
        let append_copy_cleanup = |mut text: String| {
            let default_created_object = || {
                if singular_copy && player_only_attack {
                    "it"
                } else if singular_copy {
                    "that token"
                } else {
                    "those tokens"
                }
                .to_string()
            };
            if create_copy.exile_at_end_of_combat {
                let created_object = create_copy
                    .exile_at_end_of_combat_reference_surface
                    .map(|surface| token_copy_reference_text(surface, false).to_string())
                    .unwrap_or_else(&default_created_object);
                text.push_str(&format!(". Exile {created_object} at end of combat"));
            }
            if create_copy.sacrifice_at_next_end_step
                && create_copy
                    .sacrifice_at_next_end_step_ability_text
                    .is_none()
            {
                let created_object = create_copy
                    .sacrifice_at_next_end_step_reference_surface
                    .map(|surface| token_copy_reference_text(surface, false).to_string())
                    .unwrap_or_else(&default_created_object);
                let timing =
                    describe_next_end_step_cleanup_timing(&create_copy.next_end_step_player);
                text.push_str(&format!(
                    ". Sacrifice {created_object} at the beginning of {timing}"
                ));
            }
            if create_copy.exile_at_next_end_step {
                let created_object = create_copy
                    .exile_at_next_end_step_reference_surface
                    .map(|surface| token_copy_reference_text(surface, false).to_string())
                    .unwrap_or_else(default_created_object);
                let timing =
                    describe_next_end_step_cleanup_timing(&create_copy.next_end_step_player);
                text.push_str(&format!(
                    ". Exile {created_object} at the beginning of {timing}"
                ));
            }
            text
        };
        let mut exception_clauses = Vec::new();
        if let Some((power, toughness)) = &create_copy.set_base_power_toughness_value {
            let possessive = if singular_copy { "its" } else { "their" };
            exception_clauses.push(format!(
                "{possessive} power is equal to {}, {possessive} toughness is equal to {}",
                describe_value(power),
                describe_value(toughness)
            ));
        }
        if create_copy.pt_adjustment.is_some() {
            if singular_copy {
                exception_clauses.push(
                    "its power and toughness are each half that permanent's power and toughness, rounded up"
                        .to_string(),
                );
            } else {
                exception_clauses.push(
                    "their power and toughness are each half that permanent's power and toughness, rounded up"
                        .to_string(),
                );
            }
        }
        let describe_copy_colors = |colors: crate::color::ColorSet| {
            if colors.is_empty() {
                return vec!["colorless".to_string()];
            }
            let mut words = Vec::new();
            if colors.contains(crate::color::Color::White) {
                words.push("white".to_string());
            }
            if colors.contains(crate::color::Color::Blue) {
                words.push("blue".to_string());
            }
            if colors.contains(crate::color::Color::Black) {
                words.push("black".to_string());
            }
            if colors.contains(crate::color::Color::Red) {
                words.push("red".to_string());
            }
            if colors.contains(crate::color::Color::Green) {
                words.push("green".to_string());
            }
            words
        };
        let mut base_power_toughness = create_copy.set_base_power_toughness;
        let has_added_types =
            !create_copy.added_card_types.is_empty() || !create_copy.added_subtypes.is_empty();
        if has_added_types
            && create_copy.set_colors.is_none()
            && let Some((power, toughness)) = base_power_toughness.take()
        {
            let subject = if singular_copy { "it's" } else { "they're" };
            exception_clauses.push(format!("{subject} {power}/{toughness}"));
        }
        if !create_copy.added_card_types.is_empty() || !create_copy.added_subtypes.is_empty() {
            let mut type_words = create_copy
                .set_colors
                .map(&describe_copy_colors)
                .unwrap_or_default();
            type_words.extend(
                create_copy
                    .added_subtypes
                    .iter()
                    .map(|subtype| subtype.to_string()),
            );
            type_words.extend(create_copy.added_card_types.iter().map(|card_type| {
                card_type.name().to_string()
            }));
            if !type_words.is_empty() {
                let descriptor = type_words.join(" ");
                let identity = if let Some((power, toughness)) = base_power_toughness
                    && create_copy.set_colors.is_some()
                {
                    base_power_toughness = None;
                    format!("{power}/{toughness} {descriptor}")
                } else {
                    with_indefinite_article(&descriptor)
                };
                let subject = if singular_copy { "it's" } else { "they're" };
                let possessive = if singular_copy { "its" } else { "their" };
                let retained_types = if create_copy.added_card_types.is_empty()
                    && !create_copy.added_subtypes.is_empty()
                {
                    "creature types"
                } else {
                    "types"
                };
                exception_clauses.push(format!(
                    "{subject} {identity} in addition to {possessive} other {retained_types}"
                ));
            }
        }
        if create_copy.set_colors.is_some()
            || create_copy.set_card_types.is_some()
            || create_copy.set_subtypes.is_some()
        {
            let mut words = if has_added_types {
                Vec::new()
            } else {
                create_copy
                    .set_colors
                    .map(&describe_copy_colors)
                    .unwrap_or_default()
            };
            if let Some(subtypes) = &create_copy.set_subtypes {
                words.extend(subtypes.iter().map(|subtype| subtype.to_string()));
            }
            if let Some(card_types) = &create_copy.set_card_types {
                words.extend(
                    card_types
                        .iter()
                        .map(|card_type| card_type.name().to_string()),
                );
            }
            if !words.is_empty() {
                let has_noun = create_copy
                    .set_subtypes
                    .as_ref()
                    .is_some_and(|subtypes| !subtypes.is_empty())
                    || create_copy
                        .set_card_types
                        .as_ref()
                        .is_some_and(|card_types| !card_types.is_empty());
                let descriptor = words.join(" ");
                let identity = if let Some((power, toughness)) = base_power_toughness {
                    base_power_toughness = None;
                    format!("{power}/{toughness} {descriptor}")
                } else if has_noun {
                    with_indefinite_article(&descriptor)
                } else {
                    descriptor
                };
                let subject = if singular_copy { "it's" } else { "they're" };
                let mut clause = format!("{subject} {identity}");
                if let Some(card_types) = &create_copy.set_card_types
                    && card_types.len() == 1
                    && create_copy.set_colors.is_none()
                    && create_copy.set_subtypes.is_none()
                    && create_copy.added_card_types.is_empty()
                    && create_copy.added_subtypes.is_empty()
                {
                    clause.push_str(" and it loses all other card types");
                }
                exception_clauses.push(clause);
            }
        }
        if let Some((power, toughness)) = base_power_toughness {
            let subject = if singular_copy { "it's" } else { "they're" };
            exception_clauses.push(format!("{subject} {power}/{toughness}"));
        }
        if create_copy
            .removed_supertypes
            .iter()
            .any(|supertype| *supertype == Supertype::Legendary)
        {
            exception_clauses.push(if singular_copy {
                "it isn't legendary".to_string()
            } else {
                "they aren't legendary".to_string()
            });
        }
        if let Some(loyalty) = create_copy.starting_loyalty {
            exception_clauses.push(if singular_copy {
                format!("its starting loyalty is {loyalty}")
            } else {
                format!("their starting loyalty is {loyalty}")
            });
        }
        if create_copy.has_haste && create_copy.haste_followup_reference_surface.is_none() {
            let mut clause = if singular_copy {
                "it has haste".to_string()
            } else {
                "they have haste".to_string()
            };
            if create_copy.loses_soulbond {
                clause.push_str(if singular_copy {
                    " and loses soulbond"
                } else {
                    " and lose soulbond"
                });
            }
            exception_clauses.push(clause);
        } else if create_copy.loses_soulbond {
            exception_clauses.push(if singular_copy {
                "it loses soulbond".to_string()
            } else {
                "they lose soulbond".to_string()
            });
        }
        if let Some(ability_text) = &create_copy.sacrifice_at_next_end_step_ability_text {
            exception_clauses.push(quote_token_granted_ability_text(ability_text));
        }
        if !create_copy.granted_static_abilities.is_empty() {
            let mut granted = Vec::new();
            for ability in &create_copy.granted_static_abilities {
                let normalized = normalize_token_granted_static_ability_text(&ability.display());
                let ability_text = if is_keyword_style_line(&normalized) {
                    normalized.to_ascii_lowercase()
                } else {
                    quote_token_granted_ability_text(&normalized)
                };
                if !granted.contains(&ability_text) {
                    granted.push(ability_text);
                }
            }
            let subject = if singular_copy { "it has" } else { "they have" };
            exception_clauses.push(format!("{subject} {}", join_with_and(&granted)));
        }
        if !exception_clauses.is_empty() {
            text.push_str(", except ");
            text.push_str(&join_with_and(&exception_clauses));
        }
        if use_where_x {
            text.push_str(&format!(", where X is {}", describe_value(&create_copy.count)));
        }
        if player_only_attack {
            let subject = if singular_copy {
                "The token enters"
            } else {
                "Those tokens enter"
            };
            let state = if create_copy.enters_tapped {
                "tapped and attacking that player"
            } else {
                "attacking that player"
            };
            text.push_str(&format!(". {subject} {state}"));
        }
        if create_copy.has_haste
            && let Some(surface) = create_copy.haste_followup_reference_surface
        {
            let subject = capitalize_first(token_copy_reference_text(surface, true));
            let verb = if surface.is_plural() { "gain" } else { "gains" };
            text.push_str(&format!(". {subject} {verb} haste"));
        }
        return append_copy_cleanup(text);
    }
    if let Some(earthbend) = effect.downcast_ref::<crate::effects::EarthbendEffect>() {
        return format!("Earthbend {}", earthbend.counters);
    }
    if let Some(explore) = effect.downcast_ref::<crate::effects::ExploreEffect>() {
        if let Some(surface) = explore.target.source_reference_surface() {
            return format!(
                "{} explores",
                capitalize_first(&describe_source_reference_surface_text(surface))
            );
        }
        if matches!(explore.target.base(), ChooseSpec::Source) {
            return "it explores".to_string();
        }
        let subject = capitalize_first(&describe_choose_spec(&explore.target));
        return format!("{subject} explores");
    }
    if let Some(behold) = effect.downcast_ref::<crate::effects::BeholdEffect>() {
        let subtype_name = behold.subtype.to_string();
        if behold.count == 1 {
            return format!("Behold {}", with_indefinite_article(&subtype_name));
        }
        let count_text =
            small_number_word(behold.count).unwrap_or_else(|| behold.count.to_string());
        return format!("Behold {count_text} {subtype_name}s");
    }
    if let Some(open) = effect.downcast_ref::<crate::effects::OpenAttractionEffect>() {
        if open.reminder {
            return format!(
                "Open an Attraction. {STANDARD_REMINDER_OPEN_SENTINEL}Put the top card of your Attraction deck onto the battlefield.{STANDARD_REMINDER_CLOSE_SENTINEL}"
            );
        }
        return "Open an Attraction".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::ManifestDreadEffect>()
        .is_some()
    {
        return "Manifest dread".to_string();
    }
    if let Some(manifest) = effect.downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
    {
        let owner = match manifest.player {
            crate::filter::PlayerFilter::TargetPlayerOrControllerOfTarget => {
                "that player's".to_string()
            }
            _ => describe_possessive_player_filter(&manifest.player),
        };
        let action = if manifest.cloak { "Cloak" } else { "Manifest" };
        return format!("{action} the top card of {owner} library");
    }
    if effect
        .downcast_ref::<crate::effects::ManifestCardFromHandEffect>()
        .is_some()
    {
        return "Manifest a card from your hand".to_string();
    }
    if let Some(populate) = effect.downcast_ref::<crate::effects::PopulateEffect>() {
        let mut text = match &populate.count {
            Value::Fixed(1) => "Populate".to_string(),
            Value::Fixed(2) => "Populate twice".to_string(),
            count => format!("Populate {} times", describe_value(count)),
        };
        if populate.enters_tapped && populate.enters_attacking {
            text.push_str(". The token created this way enters tapped and attacking");
        }
        if populate.has_haste {
            text.push_str(". The token created this way gains haste");
        }
        if populate.sacrifice_at_next_end_step {
            let timing = describe_next_end_step_cleanup_timing(&populate.next_end_step_player);
            text.push_str(&format!(". Sacrifice it at the beginning of {timing}"));
        }
        if populate.exile_at_next_end_step {
            let timing = describe_next_end_step_cleanup_timing(&populate.next_end_step_player);
            text.push_str(&format!(". Exile it at the beginning of {timing}"));
        }
        if populate.exile_at_end_of_combat {
            text.push_str(". Exile it at end of combat");
        }
        if populate.sacrifice_at_end_of_combat {
            text.push_str(". Sacrifice it at end of combat");
        };
        return text;
    }
    if effect
        .downcast_ref::<crate::effects::CipherEffect>()
        .is_some()
    {
        return "Cipher".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::CastEncodedCardCopyEffect>()
        .is_some()
    {
        return "its controller may cast a copy of the encoded card without paying its mana cost"
            .to_string();
    }
    if let Some(backup) = effect.downcast_ref::<crate::effects::BackupEffect>() {
        return format!("Backup {}", backup.amount);
    }
    if let Some(bolster) = effect.downcast_ref::<crate::effects::BolsterEffect>() {
        return format!("Bolster {}", bolster.amount);
    }
    if let Some(support) = effect.downcast_ref::<crate::effects::SupportEffect>() {
        return format!("Support {}", support.amount);
    }
    if let Some(adapt) = effect.downcast_ref::<crate::effects::AdaptEffect>() {
        return format!("Adapt {}", adapt.amount);
    }
    if effect
        .downcast_ref::<crate::effects::AuraSwapEffect>()
        .is_some()
    {
        return "Aura swap".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::CounterAbilityEffect>()
        .is_some()
    {
        return "Counter target activated or triggered ability".to_string();
    }
    if let Some(regenerate) = effect.downcast_ref::<crate::effects::RegenerateEffect>() {
        let mut target = describe_choose_spec(&regenerate.target);
        if let Some(rest) = target.strip_prefix("all ") {
            target = format!("each {rest}");
        }
        let base = if regenerate.duration == Until::EndOfTurn {
            format!("Regenerate {target}")
        } else {
            format!(
                "Regenerate {target} {}",
                describe_until(&regenerate.duration)
            )
        };
        fn follow_up_inner(effect: &Effect) -> &Effect {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map(|tagged| tagged.effect.as_ref())
                .unwrap_or(effect)
        }
        if let [follow_up] = regenerate.follow_up_effects.as_slice()
            && let Some(gain_control) =
                follow_up_inner(follow_up).downcast_ref::<crate::effects::GainControlEffect>()
            && gain_control.duration == Until::Forever
            && (matches!(&gain_control.target, ChooseSpec::Tagged(tag) if tag.as_str() == "__it__")
                || describe_choose_spec(&gain_control.target).eq_ignore_ascii_case("it"))
        {
            return format!("{base}. You gain control of that creature if it regenerates this way");
        }
        if let [follow_up] = regenerate.follow_up_effects.as_slice()
            && let Some(apply) = follow_up_inner(follow_up)
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            && apply.until == Until::Forever
            && apply.modification.is_none()
            && apply.additional_modifications.is_empty()
            && matches!(
                apply.runtime_modifications.as_slice(),
                [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
            )
            && matches!(&apply.target_spec, Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "__it__")
        {
            return format!("{base}. You gain control of that creature if it regenerates this way");
        }
        if !regenerate.follow_up_effects.is_empty() {
            return format!(
                "{base}. {} if it regenerates this way",
                describe_effect_list(&regenerate.follow_up_effects)
            );
        }
        return base;
    }
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
        if let crate::effect::RestrictionStart::NextTurn(player) = &cant.start {
            return format!(
                "{} during {} next turn",
                describe_restriction(&cant.restriction),
                describe_possessive_player_filter(player)
            );
        }
        if cant.duration == Until::EndOfTurn
            && cant.duration_surface
                == crate::effect::RestrictionDurationSurface::LeadingUntilEndOfTurn
        {
            return format!(
                "Until end of turn, {}",
                lowercase_first(&describe_restriction(&cant.restriction))
            );
        }
        if cant.duration == Until::YourNextTurn
            && cant.duration_surface
                == crate::effect::RestrictionDurationSurface::LeadingUntilYourNextTurn
        {
            return format!(
                "Until your next turn, {}",
                lowercase_first(&describe_restriction(&cant.restriction))
            );
        }
        if cant.duration == Until::Forever
            && matches!(
                cant.restriction,
                crate::effect::Restriction::GainLife(_)
                    | crate::effect::Restriction::NoMaximumHandSize(_)
            )
        {
            return format!(
                "{} for the rest of the game",
                describe_restriction(&cant.restriction)
            );
        }
        if cant.duration == Until::EndOfTurn
            && matches!(
                &cant.restriction,
                crate::effect::Restriction::GainLife(PlayerFilter::DamagedPlayer)
            )
        {
            return "If that player would gain life this turn, that player gains no life instead"
                .to_string();
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
            && filter_is_exactly_one_tagged_object(blockers)
            && attacker.source
        {
            return "Target creature blocks this creature this turn if able".to_string();
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
            && blockers.description().eq_ignore_ascii_case("creature")
            && filter_is_exactly_one_tagged_object(attacker)
        {
            return "All creatures able to block target creature this turn do so".to_string();
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
            && blockers.description().eq_ignore_ascii_case("creature")
            && attacker.source
        {
            return "All creatures able to block this creature this turn do so".to_string();
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
            && blockers.card_types == vec![crate::types::CardType::Creature]
            && blockers.controller == Some(PlayerFilter::NotYou)
            && attacker.card_types == vec![crate::types::CardType::Creature]
            && attacker.controller == Some(PlayerFilter::You)
            && attacker.other
            && attacker
                .ability_markers
                .iter()
                .any(|marker| marker.eq_ignore_ascii_case("a power and toughness sticker on it"))
        {
            return "Target creature you don't control blocks target creature you control with a power and toughness sticker on it other than this creature this turn if able".to_string();
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBlockSpecificAttacker { blockers, attacker } =
                &cant.restriction
            && blockers.description().eq_ignore_ascii_case("creature")
            && attacker.description().eq_ignore_ascii_case("creature")
        {
            return "All creatures able to block target creature this turn do so".to_string();
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::MustBeBlocked(filter) = &cant.restriction
        {
            let description = filter.description();
            let subject = match description.as_str() {
                "permanent" | "creature" | "target permanent" | "target creature" => {
                    "It".to_string()
                }
                "a creature you control" | "creature you control" => {
                    "Each creature you control".to_string()
                }
                _ => capitalize_first(&description),
            };
            return format!("{subject} must be blocked this turn if able");
        }
        if let crate::effect::Restriction::BeTargetedPlayer(player) = &cant.restriction {
            return describe_player_gain_keyword(player, "shroud", &cant.duration);
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::Block(filter) = &cant.restriction
            && filter.power_toughness_relation.is_some()
            && filter.card_types.as_slice() == [CardType::Creature]
        {
            let subject = strip_indefinite_article(&filter.description()).to_string();
            return format!("Each {subject} can't block this turn");
        }
        if cant.duration == Until::EndOfTurn
            && let crate::effect::Restriction::Block(filter) = &cant.restriction
            && let Some(subject) = describe_plural_block_restriction_subject(filter)
        {
            return format!("{subject} can't block this turn");
        }
        if cant.duration == Until::EndOfTurn {
            let restriction_text = describe_restriction(&cant.restriction);
            if restriction_text.to_ascii_lowercase().contains(" each turn") {
                return restriction_text;
            }
            return format!("{restriction_text} this turn");
        }
        // An untap restriction scoped to the controller's next untap step is
        // the skip-step effect; oracle words it "{plural subject} don't untap
        // during your next untap step" (Bontu's Last Reckoning family).
        if cant.duration == Until::ControllersNextUntapStep
            && let crate::effect::Restriction::Untap(filter) = &cant.restriction
            && !filter.source
            && filter.tagged_constraints.is_empty()
            && filter.controller == Some(PlayerFilter::You)
        {
            let base = filter.description();
            let base = base
                .strip_prefix("a ")
                .or_else(|| base.strip_prefix("an "))
                .unwrap_or(&base);
            let subject = capitalize_first(&pluralize_relative_object_phrase(base));
            return format!("{subject} don't untap during your next untap step");
        }
        return format!(
            "{} {}",
            describe_restriction(&cant.restriction),
            describe_until(&cant.duration)
        );
    }
    if let Some(surveil) = effect.downcast_ref::<crate::effects::SurveilEffect>() {
        if let Some(where_x) = describe_where_x_basis(&surveil.count) {
            if surveil.player == PlayerFilter::You {
                return format!("Surveil X, where X is {where_x}");
            }
            let player = describe_player_filter(&surveil.player);
            return format!(
                "{player} {} X, where X is {where_x}",
                player_verb(&player, "surveil", "surveils")
            );
        }
        if surveil.player == PlayerFilter::You {
            return format!("Surveil {}", describe_value(&surveil.count));
        }
        let player = describe_player_filter(&surveil.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "surveil", "surveils"),
            describe_value(&surveil.count)
        );
    }
    if let Some(fateseal) = effect.downcast_ref::<crate::effects::FatesealEffect>() {
        if fateseal.player == PlayerFilter::You {
            return format!("Fateseal {}", describe_value(&fateseal.count));
        }
        let player = describe_player_filter(&fateseal.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "fateseal", "fateseals"),
            describe_value(&fateseal.count)
        );
    }
    if let Some(scry) = effect.downcast_ref::<crate::effects::ScryEffect>() {
        if scry.count == Value::X {
            if scry.player == PlayerFilter::You {
                return "Scry X".to_string();
            }
            let player = describe_player_filter(&scry.player);
            return format!("{player} {} X", player_verb(&player, "scry", "scries"));
        }
        if let Some(where_x) = describe_where_x_basis(&scry.count) {
            if scry.player == PlayerFilter::You {
                return format!("Scry X, where X is {where_x}");
            }
            let player = describe_player_filter(&scry.player);
            return format!(
                "{player} {} X, where X is {where_x}",
                player_verb(&player, "scry", "scries")
            );
        }
        if scry.player == PlayerFilter::You {
            return format!("Scry {}", describe_value(&scry.count));
        }
        let player = describe_player_filter(&scry.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "scry", "scries"),
            describe_value(&scry.count)
        );
    }
    if let Some(scry) = effect.downcast_ref::<crate::effects::EachPlayerScryEffect>() {
        if scry.player_filter == PlayerFilter::Any {
            return format!("Each player scries {}", describe_value(&scry.count));
        }
        if scry.player_filter == PlayerFilter::Opponent {
            return format!("Each opponent scries {}", describe_value(&scry.count));
        }
        let player = describe_player_filter(&scry.player_filter);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "scry", "scries"),
            describe_value(&scry.count)
        );
    }
    if let Some(discover) = effect.downcast_ref::<crate::effects::DiscoverEffect>() {
        if value_prefers_where_x(&discover.count)
            && let Some(where_x) = describe_where_x_basis(&discover.count)
        {
            if discover.player == PlayerFilter::You {
                return format!("Discover X, where X is {where_x}");
            }
            let player = describe_player_filter(&discover.player);
            return format!(
                "{player} {} X, where X is {where_x}",
                player_verb(&player, "discover", "discovers")
            );
        }
        if discover.player == PlayerFilter::You {
            return format!("Discover {}", describe_value(&discover.count));
        }
        let player = describe_player_filter(&discover.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "discover", "discovers"),
            describe_value(&discover.count)
        );
    }
    if let Some(consult) = effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>() {
        let player = describe_player_filter(&consult.player);
        let library_owner = if player == "you" { "your" } else { "their" };
        let (subject_verb, followup_verb) = match consult.mode {
            crate::effects::consult_helpers::LibraryConsultMode::Reveal => {
                (player_verb(&player, "reveal", "reveals"), "reveal")
            }
            crate::effects::consult_helpers::LibraryConsultMode::Exile => {
                (player_verb(&player, "exile", "exiles"), "exile")
            }
        };
        let pronoun = if player == "you" { "you" } else { "they" };
        let mut selection = describe_single_search_filter_in_zone(&consult.filter, Zone::Library);
        // Oracle elides the comparandum inside a consult stop condition
        // ("until you reveal a nonlegendary creature card with lesser mana
        // value, ..." — Kethek); the explicit "than it" only appears outside
        // consults.
        if let Some(head) = selection.strip_suffix(" with lesser mana value than it") {
            selection = format!("{head} with lesser mana value");
        }
        let stop_text = describe_consult_stop_text(
            &selection,
            &consult.stop_rule,
            consult.max_exposed.as_ref(),
        );
        if player == "you" {
            return format!(
                "{} cards from the top of your library until you {followup_verb} {stop_text}",
                capitalize_first(&subject_verb)
            );
        }
        return format!(
            "{player} {subject_verb} cards from the top of {library_owner} library until {pronoun} {followup_verb} {stop_text}"
        );
    }
    if let Some(remainder) =
        effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
    {
        let library_owner = describe_possessive_player_filter(&remainder.player);
        let remainder_text = match remainder.surface {
            ironsmith_core::LibraryRemainderSurface::RestOfCardsRevealedThisWay => {
                "Put the rest of the cards revealed this way"
            }
            ironsmith_core::LibraryRemainderSurface::CardsYouRevealedThisWay => {
                "Put the cards you revealed this way"
            }
            ironsmith_core::LibraryRemainderSurface::RevealedCardsNotPutOntoBattlefield => {
                "Put all cards revealed this way that weren't put onto the battlefield"
            }
            ironsmith_core::LibraryRemainderSurface::RestBare => "Put the rest",
            ironsmith_core::LibraryRemainderSurface::SentenceLeadingThenRest => {
                "Then put the rest"
            }
            ironsmith_core::LibraryRemainderSurface::Rest => {
                if remainder.keep_tagged.is_some()
                    && crate::cards::is_sentence_helper_tag(remainder.tag.as_str(), "revealed")
                {
                    "Put the rest of the revealed cards"
                } else if remainder.keep_tagged.is_some()
                    && crate::cards::is_sentence_helper_tag(remainder.tag.as_str(), "exiled")
                {
                    "Put the rest of the exiled cards"
                } else if remainder.keep_tagged.is_some() {
                    "Put the remaining tagged cards"
                } else if crate::cards::is_sentence_helper_tag(remainder.tag.as_str(), "revealed") {
                    "Put the revealed cards"
                } else if crate::cards::is_sentence_helper_tag(remainder.tag.as_str(), "exiled") {
                    "Put the exiled cards"
                } else {
                    "Put the tagged remainder"
                }
            }
        };
        let order_text = match remainder.order {
            crate::effects::consult_helpers::LibraryBottomOrder::Random => {
                " in a random order".to_string()
            }
            crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => {
                " in any order".to_string()
            }
        };
        return format!("{remainder_text} on the bottom of {library_owner} library{order_text}");
    }
    if let Some(exile_until_match) = effect.downcast_ref::<crate::effects::ExileUntilMatchEffect>()
    {
        let player = describe_player_filter(&exile_until_match.player);
        let player_verb = player_verb(&player, "exile", "exiles");
        let player_pronoun = if player == "you" { "you" } else { "they" };
        let library_owner = describe_possessive_player_filter(&exile_until_match.player);
        let selection =
            describe_search_selection_with_cards(&exile_until_match.filter.description());
        return format!(
            "{player} {player_verb} cards from the top of {library_owner} library until {player_pronoun} exile {selection}"
        );
    }
    if let Some(exile_until_match) =
        effect.downcast_ref::<crate::effects::ExileUntilMatchCastEffect>()
    {
        let player = describe_player_filter(&exile_until_match.player);
        let player_verb = player_verb(&player, "exile", "exiles");
        let player_pronoun = if player == "you" { "you" } else { "they" };
        let library_owner = describe_possessive_player_filter(&exile_until_match.player);
        let selection =
            describe_search_selection_with_cards(&exile_until_match.filter.description());
        let caster = describe_player_filter(&exile_until_match.caster);
        let free_cast_suffix = if exile_until_match.without_paying_mana_cost {
            " without paying its mana cost"
        } else {
            ""
        };
        return format!(
            "{player} {player_verb} cards from the top of {library_owner} library until {player_pronoun} exile {selection}. {caster} may cast that card{free_cast_suffix}. Then {player} puts the exiled cards that weren't cast this way on the bottom of {library_owner} library in a random order"
        );
    }
    if let Some(exile_until_match) =
        effect.downcast_ref::<crate::effects::ExileUntilMatchGrantPlayEffect>()
    {
        let player = describe_player_filter(&exile_until_match.player);
        let player_verb = player_verb(&player, "exile", "exiles");
        let player_pronoun = if player == "you" { "you" } else { "they" };
        let library_owner = describe_possessive_player_filter(&exile_until_match.player);
        let selection =
            describe_search_selection_with_cards(&exile_until_match.filter.description());
        let caster = describe_player_filter(&exile_until_match.caster);
        return format!(
            "{player} {player_verb} cards from the top of {library_owner} library until {player_pronoun} exile {selection}. {caster} may play that card until end of turn"
        );
    }
    if let Some(become_basic) =
        effect.downcast_ref::<crate::effects::BecomeBasicLandTypeChoiceEffect>()
    {
        let target = match &become_basic.target {
            ChooseSpec::Object(filter) => describe_choose_spec(&ChooseSpec::All(filter.clone())),
            other => describe_choose_spec(other),
        };
        let plural_subject = target.starts_with("all ") || target.starts_with("those ");
        let target = capitalize_first(&target);
        if let Some(subtype) = become_basic.fixed_subtype {
            let subtype_text = if plural_subject {
                pluralize_noun_phrase(&subtype.to_string())
            } else {
                let article = match subtype {
                    crate::types::Subtype::Island => "an",
                    _ => "a",
                };
                format!("{article} {subtype}")
            };
            // Static type-setting text for a whole class of lands is stated
            // as a characteristic ("All Mountains are Plains"), not as a
            // resolving transformation with an artificial "forever" suffix.
            if become_basic.duration == Until::Forever && plural_subject {
                return format!("{target} are {subtype_text}");
            }
            let verb = if plural_subject { "become" } else { "becomes" };
            if become_basic.duration == Until::EndOfTurn {
                return format!("{target} {verb} {subtype_text} until end of turn");
            }
            return format!(
                "{target} {verb} {subtype_text} {}",
                describe_until(&become_basic.duration),
            );
        }
        let subject_text = describe_each_object_subject(&become_basic.target)
            .unwrap_or_else(|| describe_choose_spec(&become_basic.target));
        let subject_text = subject_text.replacen("Each a ", "Each ", 1);
        let verb = if subject_text.starts_with("Each ") {
            "becomes"
        } else if plural_subject {
            "become"
        } else {
            "becomes"
        };
        if become_basic.duration == Until::EndOfTurn {
            return format!(
                "Choose a basic land type. {subject_text} {verb} that type until end of turn"
            );
        }
        return format!(
            "Choose a basic land type. {subject_text} {verb} that type {}",
            describe_until(&become_basic.duration)
        );
    }
    if let Some(investigate) = effect.downcast_ref::<crate::effects::InvestigateEffect>() {
        let player = describe_player_filter(&investigate.player);
        if let Some(count) = describe_effect_count_backref(&investigate.count) {
            return if player == "you" {
                format!("Investigate {count} times")
            } else {
                format!("{player} investigates {count} times")
            };
        }
        if let Some(where_x) = describe_where_x_basis(&investigate.count) {
            return if player == "you" {
                format!("Investigate X times, where X is {where_x}")
            } else {
                format!("{player} investigates X times, where X is {where_x}")
            };
        }
        return match (&investigate.count, player.as_str()) {
            (Value::Fixed(1), "you") => "Investigate".to_string(),
            (Value::Fixed(2), "you") => "Investigate twice".to_string(),
            (Value::Fixed(amount), "you") => format!("Investigate {amount} times"),
            (Value::Count(filter), "you") => {
                format!(
                    "Investigate for each {}",
                    describe_for_each_count_filter(filter)
                )
            }
            (Value::CountScaled(filter, multiplier), "you") if *multiplier == 1 => {
                format!(
                    "Investigate once for each {}",
                    describe_for_each_count_filter(filter)
                )
            }
            (Value::Fixed(1), _) => format!("{player} investigates"),
            (Value::Fixed(2), _) => format!("{player} investigates twice"),
            (Value::Fixed(amount), _) => format!("{player} investigates {amount} times"),
            (Value::Count(filter), _) => format!(
                "{player} investigates for each {}",
                describe_for_each_count_filter(filter)
            ),
            (Value::CountScaled(filter, multiplier), _) if *multiplier == 1 => format!(
                "{player} investigates once for each {}",
                describe_for_each_count_filter(filter)
            ),
            _ if player == "you" => format!("Investigate {}", describe_value(&investigate.count)),
            _ => format!(
                "{player} investigates {}",
                describe_value(&investigate.count)
            ),
        };
    }
    if let Some(incubate) = effect.downcast_ref::<crate::effects::IncubateEffect>() {
        let player = describe_player_filter(&incubate.controller);
        let amount_uses_where = value_prefers_where_x(&incubate.amount);
        let count_uses_where = value_prefers_where_x(&incubate.count);
        let amount_text = if amount_uses_where {
            "X".to_string()
        } else {
            describe_value(&incubate.amount)
        };
        let mut tail = amount_text;
        match &incubate.count {
            Value::Fixed(1) => {}
            Value::Fixed(2) => tail.push_str(" twice"),
            Value::Fixed(count) => {
                tail.push_str(&format!(" {count} times"));
            }
            _ if count_uses_where => tail.push_str(" X times"),
            other => {
                tail.push_str(&format!(" {} times", describe_value(other)));
            }
        }
        let where_clause = if amount_uses_where {
            describe_where_x_basis(&incubate.amount)
        } else if count_uses_where {
            describe_where_x_basis(&incubate.count)
        } else {
            None
        };
        if let Some(where_x) = where_clause {
            tail.push_str(&format!(", where X is {where_x}"));
        }
        if player == "you" {
            return format!("Incubate {tail}");
        }
        return format!(
            "{player} {} {tail}",
            player_verb(&player, "incubate", "incubates")
        );
    }
    if let Some(amass) = effect.downcast_ref::<crate::effects::AmassEffect>() {
        let (amount, where_x) = if value_prefers_where_x(&amass.amount) {
            (
                "X".to_string(),
                describe_where_x_basis(&amass.amount)
                    .map(|basis| format!(", where X is {basis}"))
                    .unwrap_or_default(),
            )
        } else {
            (describe_value(&amass.amount), String::new())
        };
        if let Some(subtype) = amass.subtype {
            let subtype_name = subtype.to_string();
            return format!("Amass {} {amount}{where_x}", pluralize_word(&subtype_name));
        }
        return format!("Amass {amount}{where_x}");
    }
    if let Some(poison) = effect.downcast_ref::<crate::effects::PoisonCountersEffect>() {
        let amount = match poison.count {
            Value::Fixed(1) => "a poison counter".to_string(),
            _ => format!("{} poison counters", describe_value(&poison.count)),
        };
        return format!("{} gets {}", describe_player_filter(&poison.player), amount);
    }
    if let Some(pay_any_energy) = effect.downcast_ref::<crate::effects::PayAnyEnergyEffect>() {
        let payer = describe_choose_spec(&pay_any_energy.player);
        let payment = describe_pay_any_energy_amount(pay_any_energy)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{} or more {{E}}", pay_any_energy.min_amount));
        if payer == "you" {
            return format!("Pay {payment}");
        }
        return format!("{payer} {} {payment}", player_verb(&payer, "pay", "pays"));
    }
    if let Some(pay_energy) = effect.downcast_ref::<crate::effects::PayEnergyEffect>() {
        let payer = describe_choose_spec(&pay_energy.player);
        let amount = describe_energy_payment_amount(&pay_energy.amount);
        if payer == "you" {
            return format!("Pay {amount}");
        }
        return format!("{payer} {} {amount}", player_verb(&payer, "pay", "pays"));
    }
    if let Some(pay_any_life) = effect.downcast_ref::<crate::effects::PayAnyLifeEffect>() {
        let payer = describe_choose_spec(&pay_any_life.player);
        let payment = match pay_any_life.min_amount {
            0 => "any amount of life".to_string(),
            1 => "one or more life".to_string(),
            amount => format!("{amount} or more life"),
        };
        if payer == "you" {
            return format!("Pay {payment}");
        }
        return format!("{payer} {} {payment}", player_verb(&payer, "pay", "pays"));
    }
    if let Some(energy) = effect.downcast_ref::<crate::effects::EnergyCountersEffect>() {
        let player = describe_player_filter(&energy.player);
        let verb = player_verb(&player, "get", "gets");
        if let Some(count) = describe_effect_count_backref(&energy.count) {
            return format!("{player} {verb} {count} {{E}}");
        }
        return match &energy.count {
            Value::Fixed(amount) if *amount > 0 => format!(
                "{player} {verb} {}",
                repeated_energy_symbols(*amount as usize)
            ),
            Value::X => format!("{player} {verb} X {{E}}"),
            Value::Count(filter)
                if value_has_surface_hint(&energy.count, ValueSurfaceHint::ForEach) =>
            {
                format!(
                    "{player} {verb} {{E}} for each {}",
                    describe_for_each_count_filter(filter)
                )
            }
            Value::Count(filter) => format!(
                "{player} {verb} an amount of {{E}} equal to the number of {}",
                pluralize_noun_phrase(&describe_for_each_count_filter(filter))
            ),
            Value::CountScaled(filter, multiplier) if *multiplier > 0 => format!(
                "{player} {verb} {} for each {}",
                repeated_energy_symbols(*multiplier as usize),
                describe_for_each_count_filter(filter)
            ),
            _ => format!(
                "{player} {verb} an amount of {{E}} equal to {}",
                describe_value(&energy.count)
            ),
        };
    }
    if let Some(ticket) = effect.downcast_ref::<crate::effects::TicketCountersEffect>() {
        let player = describe_player_filter(&ticket.player);
        let verb = player_verb(&player, "get", "gets");
        return match &ticket.count {
            Value::Fixed(amount) if *amount > 0 => {
                format!("{player} {verb} {}", "{TK}".repeat(*amount as usize))
            }
            _ => format!(
                "{player} {verb} an amount of {{TK}} equal to {}",
                describe_value(&ticket.count)
            ),
        };
    }
    if let Some(connive) = effect.downcast_ref::<crate::effects::ConniveEffect>() {
        let target = describe_choose_spec(&connive.target);
        let subject = if connive.target.count().is_single() {
            target
        } else {
            format!("Each of {target}")
        };
        let verb = if connive.target.count().is_single() {
            "connives"
        } else {
            "connive"
        };
        return match &connive.count {
            Value::Fixed(1) => format!("{subject} {verb}"),
            count => format!(
                "{subject} {verb} X, where X is {}",
                describe_value(count)
            ),
        };
    }
    if let Some(detain) = effect.downcast_ref::<crate::effects::DetainEffect>() {
        return format!("Detain {}", describe_choose_spec(&detain.target));
    }
    if let Some(goad) = effect.downcast_ref::<crate::effects::GoadEffect>() {
        let target = describe_goad_target(&goad.target);
        return match goad.duration {
            crate::effect::Until::Forever => format!(
                "{} is goaded for the rest of the game",
                capitalize_first(&target)
            ),
            _ => format!("Goad {target}"),
        };
    }
    if let Some(suspect) = effect.downcast_ref::<crate::effects::SuspectEffect>() {
        return format!("Suspect {}", describe_choose_spec(&suspect.target));
    }
    if let Some(clear_suspected) = effect.downcast_ref::<crate::effects::ClearSuspectedEffect>() {
        return match &clear_suspected.target {
            Some(target) => format!("{} is no longer suspected", describe_choose_spec(target)),
            None => "All suspected creatures are no longer suspected".to_string(),
        };
    }
    if let Some(extra_turn) = effect.downcast_ref::<crate::effects::ExtraTurnAfterNextTurnEffect>()
    {
        let player = describe_player_filter(&extra_turn.player);
        return format!(
            "After that turn, {} {} an extra turn",
            player,
            player_verb(&player, "take", "takes")
        );
    }
    if let Some(extra_turn) = effect.downcast_ref::<crate::effects::ExtraTurnEffect>() {
        let player = describe_player_filter(&extra_turn.player);
        return format!(
            "{} {} an extra turn after this one",
            player,
            player_verb(&player, "take", "takes")
        );
    }
    if let Some(win_game) = effect.downcast_ref::<crate::effects::WinTheGameEffect>() {
        let player = describe_player_filter(&win_game.player);
        return format!(
            "{} {} the game",
            player,
            player_verb(&player, "win", "wins")
        );
    }
    if effect
        .downcast_ref::<crate::effects::DrawTheGameEffect>()
        .is_some()
    {
        return "The game is a draw".to_string();
    }
    if let Some(lose_game) = effect.downcast_ref::<crate::effects::LoseTheGameEffect>() {
        let player = describe_player_filter(&lose_game.player);
        return format!(
            "{} {} the game",
            player,
            player_verb(&player, "lose", "loses")
        );
    }
    if let Some(restart_game) = effect.downcast_ref::<crate::effects::RestartGameEffect>() {
        return describe_restart_game(restart_game);
    }
    if effect
        .downcast_ref::<crate::effects::ReverseTurnOrderEffect>()
        .is_some()
    {
        return "Reverse the game's turn order. (For example, if play had proceeded clockwise around the table, it now goes counterclockwise.)".to_string();
    }
    if let Some(subgame) = effect.downcast_ref::<crate::effects::PlaySubgameEffect>() {
        return describe_play_subgame(subgame);
    }
    if let Some(skip_draw) = effect.downcast_ref::<crate::effects::SkipDrawStepEffect>() {
        let player = describe_player_filter(&skip_draw.player);
        return format!(
            "{} {} {} next draw step",
            player,
            player_verb(&player, "skip", "skips"),
            describe_possessive_player_filter(&skip_draw.player),
        );
    }
    if let Some(end_turn) = effect.downcast_ref::<crate::effects::EndTurnEffect>() {
        if matches!(
            end_turn.player,
            PlayerFilter::You | PlayerFilter::EffectController
        ) {
            return "end the turn".to_string();
        }
        return format!("{} ends the turn", describe_player_filter(&end_turn.player));
    }
    if effect
        .downcast_ref::<crate::effects::EndCombatPhaseEffect>()
        .is_some()
    {
        return "end the combat phase".to_string();
    }
    if let Some(skip_turn) = effect.downcast_ref::<crate::effects::SkipTurnEffect>() {
        if skip_turn.player == PlayerFilter::IteratedPlayer {
            return "that player skips that turn instead".to_string();
        }
        return format!(
            "{} skips their next turn",
            describe_player_filter(&skip_turn.player)
        );
    }
    if let Some(skip_combat) = effect.downcast_ref::<crate::effects::SkipCombatPhasesEffect>() {
        return format!(
            "{} skips all combat phases of their next turn",
            describe_player_filter(&skip_combat.player)
        );
    }
    if let Some(skip_combat) =
        effect.downcast_ref::<crate::effects::SkipNextCombatPhaseThisTurnEffect>()
    {
        return format!(
            "{} skips their next combat phase this turn",
            describe_player_filter(&skip_combat.player)
        );
    }
    if let Some(skip_main) = effect.downcast_ref::<crate::effects::SkipMainPhasesThisTurnEffect>() {
        return format!(
            "{} skips each remaining main phase this turn",
            describe_player_filter(&skip_main.player)
        );
    }
    if let Some(skip_combat) =
        effect.downcast_ref::<crate::effects::SkipCombatPhasesThisTurnEffect>()
    {
        return format!(
            "{} skips each remaining combat phase this turn",
            describe_player_filter(&skip_combat.player)
        );
    }
    if let Some(additional_phases) = effect.downcast_ref::<crate::effects::AdditionalPhasesEffect>()
    {
        if additional_phases.phases == [crate::effects::AdditionalPhase::Combat] {
            return "After this phase, there is an additional combat phase".to_string();
        }
        if additional_phases.phases
            == [
                crate::effects::AdditionalPhase::Combat,
                crate::effects::AdditionalPhase::Combat,
            ]
        {
            return "After this main phase, there are two additional combat phases".to_string();
        }
        if additional_phases.phases
            == [
                crate::effects::AdditionalPhase::Combat,
                crate::effects::AdditionalPhase::Main,
            ]
        {
            return "After this main phase, there is an additional combat phase followed by an additional main phase".to_string();
        }
    }
    if let Some(monstrosity) = effect.downcast_ref::<crate::effects::MonstrosityEffect>() {
        if value_prefers_where_x(&monstrosity.n)
            && let Some(basis) = describe_where_x_basis(&monstrosity.n)
        {
            return format!("Monstrosity X, where X is {basis}");
        }
        return format!("Monstrosity {}", describe_value(&monstrosity.n));
    }

    fn finish_copy_spell_text(
        mut text: String,
        copy: &crate::effects::CopySpellEffect,
    ) -> String {
        let mut exceptions = Vec::new();
        if copy
            .removed_supertypes
            .contains(&crate::types::Supertype::Legendary)
        {
            exceptions.push("the copy isn't legendary".to_string());
        }
        if let Some(colors) = copy.set_colors {
            let color_names = crate::color::Color::ALL
                .iter()
                .copied()
                .filter(|color| colors.contains(*color))
                .map(|color| color.name().to_string())
                .collect::<Vec<_>>();
            let colors = if color_names.is_empty() {
                "colorless".to_string()
            } else {
                join_with_and(&color_names)
            };
            exceptions.push(format!("the copy is {colors}"));
        }
        if copy.set_base_power_toughness.is_some()
            || !copy.added_card_types.is_empty()
            || !copy.added_subtypes.is_empty()
        {
            let mut words = Vec::new();
            if let Some((power, toughness)) = copy.set_base_power_toughness {
                words.push(format!("{power}/{toughness}"));
            }
            words.extend(
                copy.added_card_types
                    .iter()
                    .map(|card_type| card_type.name().to_string()),
            );
            words.extend(copy.added_subtypes.iter().map(ToString::to_string));
            let description = words.join(" ");
            let article = if description
                .chars()
                .next()
                .is_some_and(|first| matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
            {
                "an"
            } else {
                "a"
            };
            let addition = if copy.added_card_types.is_empty() && !copy.added_subtypes.is_empty() {
                " in addition to its other creature types"
            } else if !copy.added_card_types.is_empty() {
                " in addition to its other types"
            } else {
                ""
            };
            exceptions.push(format!("the copy is {article} {description}{addition}"));
        }
        if !exceptions.is_empty() {
            text.push_str(", except ");
            text.push_str(&join_with_and(&exceptions));
        }
        text
    }

    if let Some(copy_for_each) =
        effect.downcast_ref::<crate::effects::CopySpellForEachTargetEffect>()
    {
        return describe_copy_spell_for_each_target(copy_for_each);
    }
    if let Some(copy_spell) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        if matches!(
            copy_spell.count.unhinted(),
            Value::CommanderCastCount(PlayerFilter::You)
        ) && (matches!(&copy_spell.target, ChooseSpec::Source)
            || matches!(&copy_spell.target, ChooseSpec::Tagged(tag)
                if tag.as_str() == "triggering" || tag.as_str() == "__it__"))
            && copy_spell.removed_supertypes.is_empty()
            && !copy_spell.has_characteristic_modifiers()
        {
            return "Copy it for each time you've cast your commander from the command zone this game"
                .to_string();
        }
        if let Value::DistinctCounterTypesAmong(filter) = copy_spell.count.unhinted()
            && matches!(
                &copy_spell.target,
                ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"
            )
            && copy_spell.target_reference_pronoun
            && matches!(
                copy_spell.target_reference_kind,
                None | Some(StackObjectKind::Spell)
            )
            && filter
                == &ObjectFilter::permanent_card()
                    .in_zone(Zone::Battlefield)
                    .you_control()
            && copy_spell.count_surface.is_none()
            && copy_spell.copier == PlayerFilter::You
            && copy_spell.removed_supertypes.is_empty()
            && !copy_spell.has_characteristic_modifiers()
        {
            return "Copy it for each kind of counter among permanents you control".to_string();
        }
        if copy_spell.count == Value::Fixed(1)
            && matches!(
                &copy_spell.target,
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::PRIOR_EXILED_CARD_TAG
            )
        {
            return finish_copy_spell_text("Copy the exiled card".to_string(), copy_spell);
        }
        if matches!(copy_spell.target, ChooseSpec::Source) {
            if matches!(
                copy_spell.count,
                Value::SpellsCastBeforeThisTurn(PlayerFilter::You)
            ) {
                return "Copy this spell for each spell cast before it this turn".to_string();
            }
            if let Value::TurnHistoryCount(
                ironsmith_core::TurnHistoryCount::SpellsCast {
                    before_triggering_spell: true,
                    ..
                },
            ) = copy_spell.count.unhinted()
            {
                let for_each_count = copy_spell
                    .count
                    .clone()
                    .with_surface_hint(ValueSurfaceHint::ForEach);
                if let Some(basis) = describe_turn_history_for_each_basis(&for_each_count) {
                    return format!("Copy it for each {basis}");
                }
                let count = describe_value(&copy_spell.count);
                let basis = count.strip_prefix("the number of ").unwrap_or(&count);
                return format!("Copy it for each {basis}");
            }
            if matches!(copy_spell.count, Value::Fixed(1)) {
                return finish_copy_spell_text("Copy this spell".to_string(), copy_spell);
            }
            if let Value::Count(filter) = &copy_spell.count {
                let mut each_filter = filter.description();
                if each_filter.ends_with('s') {
                    each_filter = each_filter.trim_end_matches('s').to_string();
                }
                return format!("Copy this spell for each {each_filter}");
            }
        }
        let mut target_text = describe_stack_object_copy_target(&copy_spell.target);
        target_text = target_text.replace(
            "target instant and sorcery",
            "target instant or sorcery spell",
        );
        if target_text.contains("instant or sorcery") && !target_text.contains(" spell") {
            target_text = target_text.replacen("instant or sorcery", "instant or sorcery spell", 1);
        }
        if matches!(copy_spell.count, Value::Fixed(1)) {
            if matches!(copy_spell.target, ChooseSpec::Iterated) {
                return finish_copy_spell_text("Copy that spell".to_string(), copy_spell);
            }
            if let ChooseSpec::Tagged(tag) = &copy_spell.target
                && matches!(
                    tag.as_str(),
                    "triggering" | "triggering_source" | "__it__"
                )
            {
                let reference = match (
                    copy_spell.target_reference_pronoun,
                    tag.as_str(),
                    copy_spell.target_reference_kind,
                ) {
                    (true, _, _) => "it",
                    (false, "__it__", _) => "it",
                    (false, _, Some(StackObjectKind::Spell)) => "that spell",
                    (
                        false,
                        _,
                        Some(
                            StackObjectKind::Ability
                                | StackObjectKind::ActivatedAbility
                                | StackObjectKind::TriggeredAbility,
                        ),
                    ) => "that ability",
                    (false, _, Some(StackObjectKind::SpellOrAbility)) => {
                        "that spell or ability"
                    }
                    (false, "triggering_source", None) => "that ability",
                    (false, _, None) => "that spell or ability",
                };
                return finish_copy_spell_text(format!("Copy {reference}"), copy_spell);
            }
            return finish_copy_spell_text(format!("Copy {target_text}"), copy_spell);
        }
        if matches!(
            copy_spell.count_surface,
            Some(
                ironsmith_core::effect::CopyCountSurface::OncePlusAdditionalPerOpponentWhoCopiedThisWay
            )
        ) {
            return format!(
                "Copy {target_text} once plus an additional time for each opponent who copied the spell this way"
            );
        }
        if matches!(copy_spell.count.unhinted(), Value::Fixed(2))
            && copy_spell.removed_supertypes.is_empty()
        {
            return finish_copy_spell_text(format!("Copy {target_text} twice"), copy_spell);
        }
        let count = describe_value(&copy_spell.count);
        return finish_copy_spell_text(format!(
            "Copy {target_text} {count} {}",
            if count == "1" { "time" } else { "times" }
        ), copy_spell);
    }
    if let Some(choose_new) = effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>() {
        let chooser_text = choose_new
            .chooser
            .as_ref()
            .map(describe_player_filter)
            .unwrap_or_else(|| "you".to_string());
        let target_text = if choose_new.single_target_surface {
            "a new target"
        } else {
            "new targets"
        };
        return format!(
            "{} {} {target_text} for the copy",
            chooser_text,
            if choose_new.may {
                "may choose"
            } else {
                "chooses"
            },
        );
    }
    if let Some(scale_x) = effect.downcast_ref::<crate::effects::ScaleXValueEffect>() {
        if scale_x.multiplier == 2 {
            return "Double the value of X".to_string();
        }
        return format!("Multiply the value of X by {}", scale_x.multiplier);
    }
    if let Some(retarget) = effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>() {
        let target_text = if retarget.copy_reference_plural {
            "the copies".to_string()
        } else {
            match retarget.target.base() {
            // The triggering stack object reads as "that spell" in a
            // retargeting clause ("You may choose new targets for that
            // spell." — Speedball).
                ChooseSpec::Tagged(tag) if tag.as_str() == "triggering" => {
                    "that spell".to_string()
                }
                _ => describe_choose_spec(&retarget.target),
            }
        };
        let mut base = match &retarget.mode {
            crate::effects::RetargetMode::All => {
                if retarget.require_change {
                    format!("Change the target of {target_text}")
                } else {
                    format!("Choose new targets for {target_text}")
                }
            }
            crate::effects::RetargetMode::OneToFixed(spec) => {
                let fixed_text = describe_choose_spec(spec);
                format!("Change a target of {target_text} to {fixed_text}")
            }
        };

        if let Some(restriction) = &retarget.new_target_restriction {
            let restriction_text = match restriction {
                crate::effects::NewTargetRestriction::Player(filter) => {
                    let mut text = describe_player_filter(filter);
                    if let Some(rest) = text.strip_prefix("target ") {
                        text = rest.to_string();
                    }
                    if text == "you" {
                        text
                    } else {
                        ensure_indefinite_article(&text)
                    }
                }
                crate::effects::NewTargetRestriction::Object(filter) => {
                    ensure_indefinite_article(&filter.description())
                }
            };
            base.push_str(". The new target must be ");
            base.push_str(&restriction_text);
        }
        return base;
    }
    if let Some(double_mana) = effect.downcast_ref::<crate::effects::DoubleManaPoolEffect>() {
        let subject = match &double_mana.player {
            PlayerFilter::You => "you have".to_string(),
            PlayerFilter::Target(base) if matches!(base.as_ref(), PlayerFilter::Any) => {
                "target player has".to_string()
            }
            PlayerFilter::Target(base) if matches!(base.as_ref(), PlayerFilter::Opponent) => {
                "target opponent has".to_string()
            }
            other => format!("{} has", describe_player_filter(other)),
        };
        return format!("Double the amount of each type of unspent mana {subject}");
    }
    if let Some(empty_mana) = effect.downcast_ref::<crate::effects::EmptyManaPoolEffect>() {
        let subject = match &empty_mana.player {
            PlayerFilter::You => "you lose".to_string(),
            PlayerFilter::Target(base) if matches!(base.as_ref(), PlayerFilter::Any) => {
                "target player loses".to_string()
            }
            PlayerFilter::Target(base) if matches!(base.as_ref(), PlayerFilter::Opponent) => {
                "target opponent loses".to_string()
            }
            other => format!("{} loses", describe_player_filter(other)),
        };
        return format!("{subject} all unspent mana");
    }
    if let Some(set_life) = effect.downcast_ref::<crate::effects::SetLifeTotalEffect>() {
        if let Value::Scaled(base, 2) = &set_life.amount
            && let Value::LifeTotal(player) = base.as_ref()
            && player == &set_life.player
        {
            return format!(
                "Double {} life total",
                describe_possessive_player_filter(&set_life.player)
            );
        }
        if let Value::Count(filter) = &set_life.amount
            && matches!(set_life.player, PlayerFilter::You)
        {
            return format!(
                "Count the number of {}. Your life total becomes that number",
                describe_count_filter_value_subject(filter)
            );
        }
        return format!(
            "{} life total becomes {}",
            describe_possessive_player_filter(&set_life.player),
            describe_value(&set_life.amount)
        );
    }
    if effect
        .downcast_ref::<crate::effects::NoteLifeTotalEffect>()
        .is_some()
    {
        return "Note your life total".to_string();
    }
    if let Some(pay_mana) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
        let player = describe_choose_spec(&pay_mana.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "pay", "pays"),
            describe_pay_mana_cost(pay_mana)
        );
    }
    if let Some(add_any) = effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>() {
        if !add_any.distinct_colors
            && add_any
                .available_colors
                .as_ref()
                .is_none_or(|colors| crate::color::Color::ALL.iter().all(|c| colors.contains(c)))
            && let Some(multiplier) = counters_removed_this_way_multiplier(&add_any.amount)
            && multiplier > 0
        {
            let amount = small_number_word(multiplier as u32)
                .unwrap_or_else(|| multiplier.to_string());
            return format!(
                "Add {amount} mana of any color for each counter removed this way{}",
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        if !add_any.distinct_colors
            && let Some(colors) = add_any.available_colors.as_ref()
            && !colors.is_empty()
            && !crate::color::Color::ALL
                .iter()
                .all(|color| colors.contains(color))
            && !add_any.amount.has_surface_hint(ValueSurfaceHint::WhereXIs)
            && let Value::PriorEffectMetric { query, .. }
                | Value::PendingPriorEffectMetric(query) = add_any.amount.unhinted()
            && query.metric == crate::effect::EffectMetric::Count
            && query.action.is_some()
        {
            let options = colors
                .iter()
                .copied()
                .map(crate::mana::ManaSymbol::from_color)
                .collect::<Vec<_>>();
            return format!(
                "Add {} for each {}{}",
                describe_mana_alternatives(&options),
                describe_prior_effect_metric_basis(query, false),
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        if !add_any.distinct_colors
            && let Some(colors) = add_any.available_colors.as_ref()
            && !colors.is_empty()
            && !crate::color::Color::ALL
                .iter()
                .all(|color| colors.contains(color))
            && let Some(for_each) = describe_turn_history_for_each_basis(&add_any.amount)
        {
            let options = colors
                .iter()
                .copied()
                .map(crate::mana::ManaSymbol::from_color)
                .collect::<Vec<_>>();
            return format!(
                "Add {} for each {for_each}{}",
                describe_mana_alternatives(&options),
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        if add_any.amount.has_surface_hint(ValueSurfaceHint::WhereXIs)
            && !add_any.distinct_colors
            && add_any
                .available_colors
                .as_ref()
                .is_none_or(|colors| crate::color::Color::ALL.iter().all(|c| colors.contains(c)))
            && let Some(where_x) = describe_where_x_basis(&add_any.amount)
        {
            return format!(
                "Add X mana in any combination of colors{}, where X is {where_x}",
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        if add_any.amount.has_surface_hint(ValueSurfaceHint::WhereXIs)
            && !add_any.distinct_colors
            && let Some(colors) = &add_any.available_colors
            && let Some(where_x) = describe_where_x_basis(&add_any.amount)
        {
            let options = colors
                .iter()
                .copied()
                .map(crate::mana::ManaSymbol::from_color)
                .map(describe_mana_symbol)
                .collect::<Vec<_>>()
                .join(" and/or ");
            return format!(
                "Add X mana in any combination of {options}{}, where X is {where_x}",
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        if !add_any.distinct_colors
            && add_any
                .available_colors
                .as_ref()
                .is_none_or(|colors| crate::color::Color::ALL.iter().all(|c| colors.contains(c)))
            && let Some(subject_form) =
                describe_other_player_adds_any_color(&add_any.player, &add_any.amount, false)
        {
            return subject_form;
        }
        if add_any.distinct_colors {
            return format!(
                "Add {} mana of different colors{}",
                describe_mana_amount_for_add_effect(&add_any.amount),
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        if let Some(colors) = &add_any.available_colors {
            let has_all_colors = crate::color::Color::ALL
                .iter()
                .all(|color| colors.contains(color));
            if has_all_colors && matches!(add_any.amount, Value::Fixed(1)) {
                return format!(
                    "Add one mana of any color{}",
                    describe_add_mana_destination_suffix(&add_any.player)
                );
            }
            if matches!(add_any.amount, Value::Fixed(1)) {
                let options = colors
                    .iter()
                    .copied()
                    .map(crate::mana::ManaSymbol::from_color)
                    .collect::<Vec<_>>();
                return format!(
                    "Add {}{}",
                    describe_mana_alternatives(&options),
                    describe_add_mana_destination_suffix(&add_any.player)
                );
            }
            if has_all_colors {
                return format!(
                    "Add {} mana in any combination of colors{}",
                    describe_mana_amount_for_add_effect(&add_any.amount),
                    describe_add_mana_destination_suffix(&add_any.player)
                );
            }
            let options = colors
                .iter()
                .copied()
                .map(crate::mana::ManaSymbol::from_color)
                .map(describe_mana_symbol)
                .collect::<Vec<_>>()
                .join(" and/or ");
            return format!(
                "Add {} mana in any combination of {}{}",
                describe_mana_amount_for_add_effect(&add_any.amount),
                options,
                describe_add_mana_destination_suffix(&add_any.player)
            );
        }
        return format!(
            "Add {} mana of any color{}",
            describe_mana_amount_for_add_effect(&add_any.amount),
            describe_add_mana_destination_suffix(&add_any.player)
        );
    }
    if let Some(add_one) = effect.downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>() {
        if let Value::Count(filter) = &add_one.amount {
            let mut count_subject = pluralize_noun_phrase(&filter.description());
            let lower_subject = count_subject.to_ascii_lowercase();
            if filter.zone == Some(Zone::Battlefield) && !lower_subject.contains("battlefield") {
                count_subject.push_str(" on the battlefield");
            }
            return format!(
                "Add X mana of any one color{}, where X is the number of {}",
                describe_add_mana_destination_suffix(&add_one.player),
                count_subject
            );
        }
        if !matches!(&add_one.amount, Value::Fixed(_) | Value::X) {
            return format!(
                "Add X mana of any one color{}, where X is {}",
                describe_add_mana_destination_suffix(&add_one.player),
                describe_value(&add_one.amount)
            );
        }
        if let Some(subject_form) =
            describe_other_player_adds_any_color(&add_one.player, &add_one.amount, true)
        {
            return subject_form;
        }
        return format!(
            "Add {} mana of any one color{}",
            describe_mana_amount_for_add_effect(&add_one.amount),
            describe_add_mana_destination_suffix(&add_one.player)
        );
    }
    if let Some(add_chosen) =
        effect.downcast_ref::<crate::effects::mana::AddManaOfChosenColorEffect>()
    {
        let destination = describe_add_mana_destination_suffix(&add_chosen.player);
        let amount = describe_mana_amount_for_add_effect(&add_chosen.amount);
        if let Some(fixed) = add_chosen.fixed_option {
            let fixed_symbol = describe_mana_symbol(crate::mana::ManaSymbol::from_color(fixed));
            if matches!(add_chosen.amount, Value::Fixed(1)) {
                return format!(
                    "Add {} or one mana of the chosen color{}",
                    fixed_symbol, destination
                );
            }
            return format!(
                "Add {} or {} mana of the chosen color{}",
                fixed_symbol, amount, destination
            );
        }
        if matches!(add_chosen.amount, Value::Fixed(1)) {
            return format!("Add one mana of the chosen color{}", destination);
        }
        if let Value::DistinctPowers(filter) = &add_chosen.amount {
            return format!(
                "Add one mana of the chosen color for each different power among {}{}",
                pluralize_noun_phrase(&describe_for_each_count_filter(filter)),
                destination
            );
        }
        if !matches!(&add_chosen.amount, Value::Fixed(_) | Value::X) {
            return format!(
                "Add an amount of mana of the chosen color equal to {}{}",
                describe_value(&add_chosen.amount),
                destination
            );
        }
        return format!("Add {} mana of the chosen color{}", amount, destination);
    }
    if let Some(add_land_produced) =
        effect.downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()
    {
        let any_word = if add_land_produced.allow_colorless {
            "type"
        } else {
            "color"
        };
        let one_word = if add_land_produced.same_type {
            " one"
        } else {
            ""
        };
        if add_land_produced.mana_type_source
            == crate::effects::ManaTypeSource::TriggeringEventProduced
        {
            let subject = capitalize_first(&describe_player_filter(&add_land_produced.player));
            let verb = if matches!(add_land_produced.player, PlayerFilter::You) {
                "add"
            } else {
                "adds"
            };
            let producer_description = add_land_produced.land_filter.description();
            let producer = strip_leading_article(&producer_description);
            return format!(
                "{subject} {verb} {} mana of any{} {any_word} that {producer} produced",
                describe_mana_amount_for_add_effect(&add_land_produced.amount),
                one_word,
            );
        }
        return format!(
            "Add {} mana of any{} {}{} that {} could produce",
            describe_mana_amount_for_add_effect(&add_land_produced.amount),
            one_word,
            any_word,
            describe_add_mana_destination_suffix(&add_land_produced.player),
            add_land_produced.land_filter.description()
        );
    }
    if let Some(add_colors_among) =
        effect.downcast_ref::<crate::effects::AddManaOfColorsAmongEffect>()
    {
        return format!(
            "For each color among {}, add one mana of that color{}",
            pluralize_noun_phrase(&describe_for_each_filter(&add_colors_among.filter)),
            describe_add_mana_destination_suffix(&add_colors_among.player)
        );
    }
    if let Some(add_any_color_among) =
        effect.downcast_ref::<crate::effects::AddOneManaOfAnyColorAmongEffect>()
    {
        if add_any_color_among.choose_color_of_object_surface {
            return format!(
                "Choose a color of {}. Add one mana of that color{}",
                add_any_color_among.filter.description(),
                describe_add_mana_destination_suffix(&add_any_color_among.player)
            );
        }
        return format!(
            "Add one mana of any color among {}{}",
            pluralize_noun_phrase(&describe_for_each_filter(&add_any_color_among.filter)),
            describe_add_mana_destination_suffix(&add_any_color_among.player)
        );
    }
    if let Some(add_commander) =
        effect.downcast_ref::<crate::effects::AddManaFromCommanderColorIdentityEffect>()
    {
        let destination = describe_add_mana_destination_suffix(&add_commander.player);
        if matches!(add_commander.amount, Value::Fixed(1)) {
            return format!(
                "Add one mana of any color in your commander's color identity{}",
                destination
            );
        }
        return format!(
            "Add {} mana of any color in your commander's color identity{}",
            describe_mana_amount_for_add_effect(&add_commander.amount),
            destination
        );
    }
    if effect
        .downcast_ref::<crate::effects::mana::AddManaOfImprintedColorsEffect>()
        .is_some()
    {
        return "Add one mana of any of the exiled card's colors".to_string();
    }
    if let Some(prevent_damage) = effect.downcast_ref::<crate::effects::PreventDamageEffect>() {
        let filter = &prevent_damage.damage_filter;
        let is_default_filter = !filter.combat_only
            && !filter.noncombat_only
            && filter.from_source.is_none()
            && filter.from_specific_source.is_none()
            && filter
                .from_colors
                .as_ref()
                .is_none_or(|colors| colors.is_empty())
            && filter
                .from_card_types
                .as_ref()
                .is_none_or(|types| types.is_empty());
        let damage_text = if is_default_filter {
            "damage".to_string()
        } else {
            describe_damage_filter(filter)
        };
        let timing = if matches!(prevent_damage.duration, Until::EndOfTurn) {
            "this turn".to_string()
        } else {
            describe_until(&prevent_damage.duration)
        };
        let source_text = if prevent_damage.source_of_your_choice {
            " by a source of your choice"
        } else {
            ""
        };
        let (amount_text, where_x_suffix) = if value_prefers_where_x(&prevent_damage.amount) {
            (
                "X".to_string(),
                describe_where_x_basis(&prevent_damage.amount)
                    .map(|basis| format!(", where X is {basis}"))
                    .unwrap_or_default(),
            )
        } else {
            (describe_value(&prevent_damage.amount), String::new())
        };
        if let Some(put) = prevention_put_counters_follow_up(&prevent_damage.follow_up_effects) {
            let protected = if prevent_damage.protect_you_and_permanents_you_control {
                "you and/or permanents you control".to_string()
            } else {
                describe_choose_spec(&prevent_damage.target)
            };
            return format!(
                "Prevent the next {} {} that would be dealt to {} {}{}{}. For each 1 damage prevented this way, put a {} counter on {}",
                amount_text,
                damage_text,
                protected,
                timing,
                source_text,
                where_x_suffix,
                describe_counter_type(put.counter_type),
                describe_prevention_follow_up_target(&prevent_damage.target)
            );
        }
        let protected = if prevent_damage.protect_you_and_permanents_you_control {
            "you and/or permanents you control".to_string()
        } else {
            describe_choose_spec(&prevent_damage.target)
        };
        if prevention_damage_any_target_follow_up(&prevent_damage.follow_up_effects).is_some() {
            return format!(
                "Prevent the next {} {} that would be dealt to {} {}{}{}. If damage is prevented this way, this spell deals that much damage to any target",
                amount_text,
                damage_text,
                protected,
                timing,
                source_text,
                where_x_suffix
            );
        }
        let mut rendered = format!(
            "Prevent the next {} {} that would be dealt to {} {}{}{}",
            amount_text,
            damage_text,
            protected,
            timing,
            source_text,
            where_x_suffix
        );
        if prevention_gain_life_follow_up(&prevent_damage.follow_up_effects).is_some() {
            rendered.push_str(". You gain life equal to the damage prevented this way");
        }
        return rendered;
    }
    if let Some(prevent_all_target) =
        effect.downcast_ref::<crate::effects::PreventAllDamageToTargetEffect>()
    {
        let filter = &prevent_all_target.damage_filter;
        let is_default_filter = !filter.combat_only
            && !filter.noncombat_only
            && filter.from_source.is_none()
            && filter.from_specific_source.is_none()
            && filter
                .from_colors
                .as_ref()
                .is_none_or(|colors| colors.is_empty())
            && filter
                .from_card_types
                .as_ref()
                .is_none_or(|types| types.is_empty());
        let damage_text = if is_default_filter {
            "damage".to_string()
        } else {
            describe_damage_filter(filter)
        };
        let timing = if matches!(prevent_all_target.duration, Until::EndOfTurn) {
            "this turn".to_string()
        } else {
            describe_until(&prevent_all_target.duration)
        };
        if let Some(put) = prevention_put_counters_follow_up(&prevent_all_target.follow_up_effects)
        {
            if matches!(prevent_all_target.target.base(), ChooseSpec::Tagged(_)) {
                return format!(
                    "If {} would be dealt to {} {}, prevent that damage and put that many {} counters on {}",
                    damage_text,
                    describe_choose_spec(&prevent_all_target.target),
                    timing,
                    describe_counter_type(put.counter_type),
                    describe_prevention_follow_up_target(&prevent_all_target.target)
                );
            }
            return format!(
                "Prevent all {} that would be dealt to {} {}. For each 1 damage prevented this way, put a {} counter on {}",
                damage_text,
                describe_choose_spec(&prevent_all_target.target),
                timing,
                describe_counter_type(put.counter_type),
                describe_prevention_follow_up_target(&prevent_all_target.target)
            );
        }
        return format!(
            "Prevent all {} that would be dealt to {} {}",
            damage_text,
            describe_choose_spec(&prevent_all_target.target),
            timing
        );
    }
    if let Some(replace_next) =
        effect.downcast_ref::<crate::effects::ReplaceNextDamageToTargetEffect>()
    {
        let ChooseSpec::Object(target_filter) = replace_next.target.base() else {
            return "Unsupported effect".to_string();
        };
        let [tag_effect, replacement_effect] = replace_next.replacement_effects.as_slice() else {
            return "Unsupported effect".to_string();
        };
        let tag_damage_target = tag_effect
            .downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>();
        let destroy = unwrap_basic_tag_wrappers(replacement_effect)
            .downcast_ref::<crate::effects::DestroyEffect>();
        if let (Some(tag_damage_target), Some(destroy)) = (tag_damage_target, destroy)
            && matches!(destroy.spec.base(), ChooseSpec::Tagged(tag) if tag == &tag_damage_target.tag)
        {
            let target_text = describe_choose_spec(&replace_next.target);
            let reference = if target_filter.card_types.as_slice() == [CardType::Creature] {
                "that creature"
            } else {
                "that permanent"
            };
            return format!(
                "The next time damage would be dealt to {target_text} this turn, destroy {reference} instead"
            );
        }
        return "Unsupported effect".to_string();
    }
    if let Some(prevent_next_time) =
        effect.downcast_ref::<crate::effects::PreventNextTimeDamageEffect>()
    {
        let source_text = match &prevent_next_time.source {
            crate::effects::PreventNextTimeDamageSource::Choice => {
                "a source of your choice".to_string()
            }
            crate::effects::PreventNextTimeDamageSource::ChoiceMatching(filter) => {
                describe_prevention_damage_source(filter, true)
            }
            crate::effects::PreventNextTimeDamageSource::Target(spec) => {
                prevent_next_time_target_source_text(spec)
            }
            crate::effects::PreventNextTimeDamageSource::Filter(filter)
                if prevent_next_time_tagged_source_text(filter).is_some() =>
            {
                prevent_next_time_tagged_source_text(filter).unwrap()
            }
            crate::effects::PreventNextTimeDamageSource::Filter(filter) => {
                describe_prevention_damage_source(filter, false)
            }
        };
        let target_text = match &prevent_next_time.target {
            crate::effects::PreventNextTimeDamageTarget::AnyTarget => "any target".to_string(),
            crate::effects::PreventNextTimeDamageTarget::Omitted => String::new(),
            crate::effects::PreventNextTimeDamageTarget::You => "you".to_string(),
            crate::effects::PreventNextTimeDamageTarget::Target(spec) => describe_choose_spec(spec),
        };
        let omits_any_target =
            matches!(
                prevent_next_time.target,
                crate::effects::PreventNextTimeDamageTarget::Omitted
            );
        let target_clause = if omits_any_target {
            String::new()
        } else {
            format!(" to {target_text}")
        };
        let mut rendered = format!(
            "The next time {source_text} would deal damage{target_clause} this turn, prevent that damage"
        );
        if prevent_next_time.reflect_damage_to_source_controller {
            rendered.push_str(
                ". If damage is prevented this way, this spell deals that much damage to that source's controller",
            );
        }
        if prevention_gain_life_follow_up(&prevent_next_time.follow_up_effects).is_some() {
            rendered.push_str(". You gain life equal to the damage prevented this way");
        }
        if let Some(exile_top) =
            prevention_exile_prevented_top_follow_up(&prevent_next_time.follow_up_effects)
        {
            let owner = describe_possessive_player_filter(&exile_top.player);
            rendered.push_str(&format!(
                ". Exile cards from the top of {owner} library equal to the damage prevented this way"
            ));
        }
        return rendered;
    }
    if let Some(redirect_next) =
        effect.downcast_ref::<crate::effects::RedirectNextDamageToTargetEffect>()
    {
        let protected_text = redirect_next
            .protected_target
            .as_ref()
            .map(describe_choose_spec)
            .unwrap_or_else(|| "this creature".to_string());
        let destination_text = match redirect_next.destination {
            crate::effects::RedirectNextDamageDestination::Controller => "you".to_string(),
            crate::effects::RedirectNextDamageDestination::TargetObject => describe_choose_spec(
                redirect_next
                    .destination_target
                    .as_ref()
                    .expect("redirect-next damage destination target"),
            ),
        };
        return format!(
            "The next {} damage that would be dealt to {} this turn is dealt to {} instead",
            describe_value(&redirect_next.amount),
            protected_text,
            destination_text
        );
    }
    if let Some(redirect_next_time) =
        effect.downcast_ref::<crate::effects::RedirectNextTimeDamageToSourceEffect>()
    {
        let source_text = match &redirect_next_time.source {
            crate::effects::RedirectNextTimeDamageSource::Choice => {
                "a source of your choice".to_string()
            }
            crate::effects::RedirectNextTimeDamageSource::Filter(filter) => {
                let desc = filter.description();
                if desc.is_empty() {
                    "a source".to_string()
                } else {
                    format!("{desc} source")
                }
            }
            crate::effects::RedirectNextTimeDamageSource::Target(spec) => {
                describe_choose_spec(spec)
            }
        };
        if redirect_next_time.all_this_turn {
            let destination_text = match redirect_next_time.destination {
                crate::effects::RedirectNextTimeDamageDestination::SourceObject => {
                    "this creature".to_string()
                }
                crate::effects::RedirectNextTimeDamageDestination::Controller => "you".to_string(),
                crate::effects::RedirectNextTimeDamageDestination::SourceController => {
                    if source_text.ends_with("spell") {
                        "that spell's controller".to_string()
                    } else {
                        "that source's controller".to_string()
                    }
                }
                crate::effects::RedirectNextTimeDamageDestination::TargetObject => {
                    describe_choose_spec(
                        redirect_next_time
                            .destination_target
                            .as_ref()
                            .expect("redirect-next damage destination target"),
                    )
                }
            };
            return if let Some(target) = &redirect_next_time.target {
                format!(
                    "All damage that would be dealt to {} this turn by {source_text} is dealt to {destination_text} instead",
                    describe_choose_spec(target)
                )
            } else {
                format!(
                    "All damage that would be dealt this turn by {source_text} is dealt to {destination_text} instead"
                )
            };
        }
        return match redirect_next_time.destination {
            crate::effects::RedirectNextTimeDamageDestination::SourceObject => format!(
                "The next time {source_text} would deal damage to {} this turn, that damage is dealt to this creature instead",
                describe_choose_spec(
                    redirect_next_time
                        .target
                        .as_ref()
                        .expect("redirect-next damage target")
                )
            ),
            crate::effects::RedirectNextTimeDamageDestination::Controller => format!(
                "The next time {source_text} would deal damage to {} this turn, that source deals that damage to you instead",
                describe_choose_spec(
                    redirect_next_time
                        .target
                        .as_ref()
                        .expect("redirect-next damage target")
                )
            ),
            crate::effects::RedirectNextTimeDamageDestination::SourceController => format!(
                "The next time {source_text} would deal damage this turn, that damage is dealt to {} instead",
                if source_text.ends_with("spell") {
                    "that spell's controller"
                } else {
                    "that source's controller"
                }
            ),
            crate::effects::RedirectNextTimeDamageDestination::TargetObject => format!(
                "The next time {source_text} would deal damage to {} this turn, that damage is dealt to {} instead",
                describe_choose_spec(
                    redirect_next_time
                        .target
                        .as_ref()
                        .expect("redirect-next damage target")
                ),
                describe_choose_spec(
                    redirect_next_time
                        .destination_target
                        .as_ref()
                        .expect("redirect-next damage destination target")
                )
            ),
        };
    }
    if let Some(redirect_all) =
        effect.downcast_ref::<crate::effects::RedirectAllDamageThisTurnToTargetEffect>()
    {
        let target_set = if redirect_all.player_filter == crate::target::PlayerFilter::You
            && redirect_all.object_filter == crate::target::ObjectFilter::permanent().you_control()
        {
            "you and permanents you control".to_string()
        } else {
            let player = describe_player_filter(&redirect_all.player_filter);
            let object = redirect_all.object_filter.description();
            format!("{player} and {object}")
        };
        let destination = describe_choose_spec(&redirect_all.target);
        // Attached references ("enchanted creature", "equipped creature") take no article.
        let destination = destination
            .strip_prefix("an ")
            .or_else(|| destination.strip_prefix("a "))
            .filter(|rest| rest.starts_with("enchanted ") || rest.starts_with("equipped "))
            .map(str::to_string)
            .unwrap_or(destination);
        return format!(
            "All damage that would be dealt this turn to {target_set} is dealt to {destination} instead"
        );
    }
    if let Some(assign_none) = effect.downcast_ref::<crate::effects::AssignNoCombatDamageEffect>() {
        let source = match assign_none.source.base() {
            ChooseSpec::Source => "this creature".to_string(),
            _ => describe_choose_spec(&assign_none.source),
        };
        let timing = match assign_none.until {
            Until::EndOfTurn => "this turn".to_string(),
            Until::EndOfCombat => "this combat".to_string(),
            _ => describe_until(&assign_none.until),
        };
        return format!("{source} assigns no combat damage {timing}");
    }
    if let Some(prevent_from) =
        effect.downcast_ref::<crate::effects::PreventAllCombatDamageFromEffect>()
    {
        if let Some(rendered) = describe_target_creature_and_blockers_combat_prevention(
            &prevent_from.source,
            &prevent_from.until,
        ) {
            return rendered.to_string();
        }
        if let Some(rendered) = describe_implicit_source_combat_damage_prevention(
            &prevent_from.source,
            &prevent_from.until,
        ) {
            return rendered;
        }
        let timing = match prevent_from.until {
            Until::EndOfTurn => "this turn".to_string(),
            _ => describe_until(&prevent_from.until),
        };
        return format!(
            "Prevent all combat damage that would be dealt by {} {}",
            describe_choose_spec(&prevent_from.source),
            timing
        );
    }
    if let Some(prevent_combat) =
        effect.downcast_ref::<crate::effects::PreventAllCombatDamageEffect>()
    {
        let timing = match prevent_combat.until {
            Until::EndOfTurn => "this turn".to_string(),
            _ => describe_until(&prevent_combat.until),
        };
        return match &prevent_combat.target {
            crate::effects::CombatDamagePreventionTarget::All => {
                format!("Prevent all combat damage that would be dealt {timing}")
            }
            crate::effects::CombatDamagePreventionTarget::Players => {
                format!("Prevent all combat damage that would be dealt to players {timing}")
            }
            crate::effects::CombatDamagePreventionTarget::You => {
                format!("Prevent all combat damage that would be dealt to you {timing}")
            }
            crate::effects::CombatDamagePreventionTarget::From(source)
                if prevent_combat.source_would_deal_surface =>
            {
                format!(
                    "Prevent all combat damage {} would deal {timing}",
                    describe_choose_spec(source)
                )
            }
            crate::effects::CombatDamagePreventionTarget::From(source) => {
                describe_target_creature_and_blockers_combat_prevention(
                    source,
                    &prevent_combat.until,
                )
                .map(str::to_string)
                .or_else(|| {
                    describe_implicit_source_combat_damage_prevention(
                        source,
                        &prevent_combat.until,
                    )
                })
                .unwrap_or_else(|| {
                    format!(
                        "Prevent all combat damage that would be dealt by {} {}",
                        describe_choose_spec(source),
                        timing
                    )
                })
            }
        };
    }
    if let Some(prevent_all) = effect.downcast_ref::<crate::effects::PreventAllDamageEffect>() {
        if let Some(source_target) = &prevent_all.source_target
            && matches!(prevent_all.until, Until::EndOfTurn)
        {
            let protected = if prevent_all.protect_source {
                "this creature".to_string()
            } else {
                describe_prevention_target(&prevent_all.target)
            };
            return format!(
                "Prevent all damage that would be dealt to {protected} by {} this turn",
                describe_choose_spec(source_target)
            );
        }
        if let Some(excluded_source_target) = &prevent_all.excluded_source_target
            && prevent_all.damage_filter.combat_only
            && matches!(prevent_all.until, Until::EndOfTurn)
            && matches!(prevent_all.target, crate::prevention::PreventionTarget::All)
            && let Some(source_filter) = &prevent_all.damage_filter.from_source
        {
            let sources = pluralize_noun_phrase(
                strip_leading_article(&source_filter.description()).trim(),
            );
            return format!(
                "Prevent all combat damage that would be dealt by {sources} other than {} this turn",
                describe_choose_spec(excluded_source_target)
            );
        }
        if prevent_all.source_of_your_choice && matches!(prevent_all.until, Until::EndOfTurn) {
            let protected = describe_prevention_target(&prevent_all.target);
            if prevent_all.damage_filter == crate::prevention::DamageFilter::all() {
                let source_choice = if prevent_all.source_choice_shares_activation_mana_color {
                    "a source of your choice that shares a color with the mana spent on this activation cost"
                } else {
                    "a source of your choice"
                };
                if matches!(prevent_all.target, crate::prevention::PreventionTarget::All) {
                    return format!(
                        "Prevent all damage that would be dealt this turn by {source_choice}"
                    );
                }
                return format!(
                    "Prevent all damage that would be dealt to {protected} this turn by {source_choice}"
                );
            }
        }
        let simple_damage_filter = prevent_all.damage_filter.from_source.is_none()
            && prevent_all.damage_filter.from_colors.is_none()
            && prevent_all.damage_filter.from_card_types.is_none()
            && prevent_all.damage_filter.from_specific_source.is_none()
            && prevent_all.damage_filter.excluded_specific_source.is_none()
            && !prevent_all.damage_filter.noncombat_only;
        let damage_type = describe_damage_filter(&prevent_all.damage_filter);
        let protected = describe_prevention_target(&prevent_all.target);
        if simple_damage_filter && matches!(prevent_all.until, Until::EndOfTurn) {
            if prevent_all.damage_filter.combat_only {
                if matches!(prevent_all.target, crate::prevention::PreventionTarget::All) {
                    return "Prevent all combat damage that would be dealt this turn".to_string();
                }
                return format!(
                    "Prevent all combat damage that would be dealt to {} this turn",
                    protected
                );
            }
            if matches!(prevent_all.target, crate::prevention::PreventionTarget::All) {
                return "Prevent all damage this turn".to_string();
            }
            return format!(
                "Prevent all damage that would be dealt to {} this turn",
                protected
            );
        }
        if matches!(prevent_all.until, Until::EndOfTurn) {
            if matches!(prevent_all.target, crate::prevention::PreventionTarget::All) {
                if prevent_all.damage_filter.combat_only
                    && let Some(source_filter) = &prevent_all.damage_filter.from_source
                    && let Some(source_text) =
                        describe_colored_card_type_damage_sources(source_filter)
                {
                    return format!("Prevent all combat damage {source_text} would deal this turn");
                }
                if prevent_all.damage_filter.combat_only
                    && let Some(source_filter) = &prevent_all.damage_filter.from_source
                    && let Some(source_text) = describe_tagged_it_damage_source(source_filter)
                {
                    return format!(
                        "Prevent all combat damage that would be dealt by {source_text} this turn"
                    );
                }
                if let Some(source_phrase) = damage_type
                    .strip_prefix("all damage from ")
                    .and_then(|rest| rest.strip_suffix(" sources"))
                {
                    if source_phrase.starts_with("non-") {
                        return format!(
                            "Prevent all damage that would be dealt this turn by {source_phrase} sources"
                        );
                    }
                    return format!(
                        "Prevent all damage that would be dealt this turn by {source_phrase}"
                    );
                }
                if let Some(source_phrase) = damage_type
                    .strip_prefix("combat damage from ")
                    .and_then(|rest| rest.strip_suffix(" sources"))
                {
                    if source_phrase.starts_with("non-") {
                        return format!(
                            "Prevent all combat damage that would be dealt this turn by {source_phrase} sources"
                        );
                    }
                    return format!(
                        "Prevent all combat damage that would be dealt this turn by {source_phrase}"
                    );
                }
                return format!("Prevent {damage_type} that would be dealt this turn");
            }
            if let Some(source_phrase) = damage_type
                .strip_prefix("all damage from ")
                .and_then(|rest| rest.strip_suffix(" sources"))
            {
                let source_phrase = pluralize_noun_phrase(source_phrase);
                return format!(
                    "Prevent all damage that would be dealt to {protected} this turn by {source_phrase}"
                );
            }
            if let Some(source_phrase) = damage_type
                .strip_prefix("combat damage from ")
                .and_then(|rest| rest.strip_suffix(" sources"))
            {
                let source_phrase = pluralize_noun_phrase(source_phrase);
                return format!(
                    "Prevent all combat damage that would be dealt to {protected} this turn by {source_phrase}"
                );
            }
            return format!(
                "Prevent {damage_type} that would be dealt to {} this turn",
                protected
            );
        }
        if matches!(prevent_all.target, crate::prevention::PreventionTarget::All) {
            return format!(
                "Prevent {} {}",
                damage_type,
                describe_until(&prevent_all.until)
            );
        }
        return format!(
            "Prevent {} to {} {}",
            damage_type,
            protected,
            describe_until(&prevent_all.until)
        );
    }
    if let Some(ring_tempts) = effect.downcast_ref::<crate::effects::RingTemptsYouEffect>() {
        if ring_tempts.player == crate::target::PlayerFilter::You {
            return "The Ring tempts you".to_string();
        }
        return format!(
            "The Ring tempts {}",
            describe_player_filter(&ring_tempts.player)
        );
    }
    if let Some(venture) = effect.downcast_ref::<crate::effects::VentureIntoDungeonEffect>() {
        if venture.player == crate::target::PlayerFilter::You {
            return "Venture into the dungeon".to_string();
        }
        return format!(
            "{} ventures into the dungeon",
            describe_player_filter(&venture.player)
        );
    }
    if let Some(become_monarch) = effect.downcast_ref::<crate::effects::BecomeMonarchEffect>() {
        if become_monarch.player == crate::target::PlayerFilter::You {
            return "You become the monarch".to_string();
        }
        return format!(
            "{} becomes the monarch",
            describe_player_filter(&become_monarch.player)
        );
    }
    if let Some(initiative) = effect.downcast_ref::<crate::effects::TakeInitiativeEffect>() {
        if initiative.player == crate::target::PlayerFilter::You {
            return "You take the initiative".to_string();
        }
        return format!(
            "{} takes the initiative",
            describe_player_filter(&initiative.player)
        );
    }
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        if let Some(text) = describe_delayed_target_land_damages_tagged_creature(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_exile_referenced_controller_graveyard(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_trailing_if_next_end_step(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_draw_step_life_loss_unless_payment(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_upkeep_additional_poison_unless_payment(schedule) {
            return text;
        }
        if let Some(text) =
            describe_delayed_source_return_under_target_player_control_at_next_upkeep(schedule)
        {
            return text;
        }
        if let Some(text) = describe_delayed_owned_object_return_at_next_upkeep(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_dynamic_token_creation_at_next_upkeep(schedule) {
            return text;
        }
        if let Some(text) = describe_collection_scoped_each_upkeep_return(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_life_loss_and_source_return(schedule) {
            return text;
        }
        if let Some(text) = describe_delayed_coin_flip_result(schedule) {
            return text;
        }
        if let Some(text) = describe_play_card_this_way_delayed_trigger(schedule) {
            return text;
        }
        if let Some(text) = describe_next_spell_each_other_opponent_copy_loop(schedule) {
            return text;
        }
        if let Some(text) = describe_next_spell_delayed_trigger(schedule, false) {
            return text;
        }
        if let Some(text) = describe_delayed_each_player_discard_hand_return_exiled(schedule) {
            return text;
        }
        let trigger_display = schedule.trigger.display();
        let mut trigger_text = trigger_display.trim().trim_end_matches('.').to_string();
        if schedule.one_shot
            && schedule.until_end_of_turn
            && schedule
                .trigger
                .downcast_ref::<crate::triggers::BeginningOfCombatTrigger>()
                .is_some_and(|combat| combat.player == crate::target::PlayerFilter::Any)
        {
            // A one-shot beginning-of-combat trigger that expires at end of
            // turn is exactly "the next combat phase this turn".
            trigger_text = "At the beginning of the next combat phase this turn".to_string();
        }
        if schedule.until_end_of_turn
            && !schedule.until_end_of_combat
            && !schedule.leading_duration_surface
        {
            let trigger_lower = trigger_text.to_ascii_lowercase();
            if !trigger_lower.contains(" this turn") {
                trigger_text.push_str(" this turn");
            }
        }
        trigger_text = cleanup_decompiled_text(&trigger_text);
        let trigger_lower = trigger_text.to_ascii_lowercase();
        if schedule.one_shot
            && schedule.start_next_turn
            && trigger_lower.contains("your upkeep")
            && let Some(payment) = describe_unless_pays_lose_game_payment(&schedule.effects)
        {
            return format!(
                "At the beginning of your next upkeep, pay {payment}. If you don't, you lose the game"
            );
        }
        let mut delayed_text = lowercase_first(&describe_effect_list(&schedule.effects));
        delayed_text = delayed_text
            .trim_start_matches(|character: char| character.is_whitespace() || character == '.')
            .to_string();
        if let Some(prepayment) = &schedule.prepayment {
            let payment = describe_total_cost_payment(&prepayment.cost);
            let payment = payment.strip_prefix("Pay ").unwrap_or(&payment);
            let payer = match &prepayment.player {
                crate::target::PlayerFilter::You => "you pay".to_string(),
                crate::target::PlayerFilter::DamagedPlayer
                | crate::target::PlayerFilter::IteratedPlayer => "they pay".to_string(),
                _ => format!(
                    "{} pays",
                    describe_player_filter(&prepayment.player)
                ),
            };
            delayed_text = format!(
                "{delayed_text} unless {payer} {payment} before that step"
            );
        }
        {
            // When the delayed effects are target declarations followed by a
            // tagged-target subject/verb pair ("choose target creature you
            // control" + "that creature deals damage equal to its power to
            // ..."), the oracle reads them as one sentence with the subject
            // inlined. The declarations are re-stated by the joint sentence,
            // so they carry no extra text.
            let flat = schedule.effects.flattened_default_effects();
            if flat.len() >= 2
                && flat[..flat.len() - 2].iter().all(|declaration| {
                    declaration
                        .downcast_ref::<crate::effects::TargetOnlyEffect>()
                        .is_some()
                })
                && let Some(joint) =
                    describe_joint_subject_pair(&flat[flat.len() - 2], &flat[flat.len() - 1])
            {
                delayed_text = lowercase_first(&joint);
            }
        }
        if delayed_text.contains("if it matches card in exile, put it into its owner's graveyard") {
            delayed_text = delayed_text.replace(
                "if it matches card in exile, put it into its owner's graveyard",
                "if any of those cards remain exiled, return them to their owners' graveyards",
            );
        }
        if delayed_text.starts_with("draw ") {
            delayed_text = format!("you {delayed_text}");
        }
        if schedule.target_tag.is_some()
            && schedule.target_filter.is_none()
            && !schedule.one_shot
            && schedule.duration == ironsmith_core::DelayedTriggerDuration::EndOfTurn
            && let Some(dies) = schedule
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
            && matches!(
                dies.from,
                crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
            )
            && matches!(
                dies.to,
                crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
            )
            && !dies.this_object
            && dies.object_filter.dealt_damage_by_source_this_turn
                == Some(ironsmith_core::DamagedBySource::ThisCreature)
        {
            let mut victim = dies.object_filter.clone();
            victim.dealt_damage_by_source_this_turn = None;
            victim.zone = None;
            let victim = with_indefinite_article(strip_leading_article(
                &describe_for_each_filter(&victim),
            ));
            return format!(
                "Whenever {victim} dealt damage by that creature dies this turn, {delayed_text}"
            );
        }
        if schedule.one_shot
            && let Some(untap) = schedule
                .trigger
                .downcast_ref::<crate::triggers::AsPermanentsUntapTrigger>()
        {
            let timing = if untap.player == crate::target::PlayerFilter::You {
                "During your next untap step, as you untap your permanents".to_string()
            } else {
                format!(
                    "During {} next untap step, as they untap their permanents",
                    crate::triggers::describe_player_filter_possessive(&untap.player)
                )
            };
            return format!("{timing}, {delayed_text}");
        }
        if schedule.leading_duration_surface
            && schedule.duration == ironsmith_core::DelayedTriggerDuration::EndOfTurn
            && !schedule.start_next_turn
        {
            return format!(
                "Until end of turn, {}, {delayed_text}",
                lowercase_first(&trigger_text)
            );
        }
        if schedule.duration
            == ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurn
        {
            let scoped_trigger = if schedule.either_of_watched_objects
                && (schedule.target_tag.is_some() || schedule.watch_all_object_targets)
                && schedule
                    .trigger
                    .downcast_ref::<crate::triggers::DealsDamageTrigger>()
                    .is_some_and(|damage| damage.combat_only)
            {
                let noun = schedule
                    .target_filter
                    .as_ref()
                    .map(|filter| {
                        if filter.card_types.contains(&CardType::Creature) {
                            "creatures"
                        } else if filter.card_types.contains(&CardType::Artifact) {
                            "artifacts"
                        } else if filter.card_types.contains(&CardType::Enchantment) {
                            "enchantments"
                        } else if filter.card_types.contains(&CardType::Land) {
                            "lands"
                        } else {
                            "permanents"
                        }
                    })
                    .unwrap_or("permanents");
                format!("Whenever either of those {noun} deals combat damage")
            } else {
                trigger_text.clone()
            };
            return format!(
                "Until your next turn, {}, {delayed_text}",
                lowercase_first(&scoped_trigger)
            );
        }
        if schedule.one_shot
            && !schedule.start_next_turn
            && !schedule.until_end_of_turn
            && schedule
                .trigger
                .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()
                .is_some_and(|trigger| {
                    trigger.phase_type == crate::triggers::phase_step::MainPhaseType::Either
                })
        {
            let next_trigger = trigger_text.replacen("main phase", "next main phase", 1);
            return format!("{next_trigger}, {delayed_text}");
        }
        if schedule.one_shot
            && !schedule.start_next_turn
            && !schedule.until_end_of_turn
            && schedule
                .trigger
                .downcast_ref::<crate::triggers::BeginningOfMainPhaseTrigger>()
                .is_some_and(|trigger| {
                    trigger.phase_type
                        == crate::triggers::phase_step::MainPhaseType::Precombat
                })
        {
            let next_trigger =
                trigger_text.replacen("first main phase", "next first main phase", 1);
            return format!("{next_trigger}, {delayed_text}");
        }
        if schedule.one_shot
            && !schedule.start_next_turn
            && !schedule.until_end_of_turn
            && !schedule.until_end_of_combat
            && schedule
                .trigger
                .downcast_ref::<crate::triggers::BeginningOfCleanupStepTrigger>()
                .is_some_and(|cleanup| cleanup.next)
        {
            let single_clause = !delayed_text.contains(". ")
                && !delayed_text.contains(" and ")
                && !delayed_text.contains(", then");
            if single_clause {
                return format!(
                    "{} {}",
                    capitalize_first(&delayed_text),
                    lowercase_first(&trigger_text)
                );
            }
            return format!("{trigger_text}, {delayed_text}");
        }
        if schedule.target_tag.is_some()
            && (trigger_lower.contains("when this creature is dealt damage")
                || trigger_lower.contains("whenever this creature is dealt damage"))
        {
            return format!("Whenever that creature is dealt damage this turn, {delayed_text}");
        }
        if schedule.target_tag.is_some()
            && (trigger_lower.contains("when this permanent is dealt damage")
                || trigger_lower.contains("whenever this permanent is dealt damage"))
        {
            return format!("Whenever that permanent is dealt damage this turn, {delayed_text}");
        }
        if schedule.target_tag.is_some()
            && let Some((_, recipient)) = trigger_text.split_once(" deals combat damage to ")
        {
            let subject = if schedule.target_tag.as_ref().is_some_and(|tag| {
                tag.as_str() == ironsmith_core::ATTACKING_GROUP_TAG
            }) {
                "either of those creatures".to_string()
            } else if schedule
                .target_tag
                .as_ref()
                .is_some_and(|tag| tag.as_str().contains("targeted"))
                && !trigger_lower.contains(" deals combat damage to a player")
                && let Some(filter) = &schedule.target_filter
            {
                // A synthetic inline target declaration keeps its authored
                // subject ("Whenever target creature deals combat damage to
                // a non-Wall creature..."). A player-recipient watch instead
                // back-references a target declared by an earlier effect
                // ("Whenever that creature deals combat damage to a player").
                let mut base = filter.clone();
                base.tagged_constraints.retain(|constraint| {
                    !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && schedule.target_tag.as_ref().is_some_and(|tag| {
                            constraint.tag.as_str() == tag.as_str()
                                || constraint.tag.as_str() == "__it__"
                        }))
                });
                format!("target {}", strip_leading_article(&base.description()))
            } else {
                schedule
                    .target_filter
                    .as_ref()
                    .map(|filter| {
                        if filter.card_types.contains(&CardType::Creature) {
                            "that creature"
                        } else {
                            "that permanent"
                        }
                    })
                    .unwrap_or("that creature")
                    .to_string()
            };
            let duration = if schedule.until_end_of_combat {
                " this combat"
            } else {
                ""
            };
            return format!(
                "Whenever {subject} deals combat damage to {recipient}{duration}, {delayed_text}"
            );
        }
        if schedule.target_tag.is_some()
            && (trigger_lower.contains("when this creature attacks and isn't blocked")
                || trigger_lower.contains("whenever this creature attacks and isn't blocked"))
        {
            let subject = schedule
                .target_filter
                .as_ref()
                .map(|filter| {
                    let mut base = filter.clone();
                    base.tagged_constraints.retain(|constraint| {
                        !(constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                            && schedule.target_tag.as_ref().is_some_and(|tag| {
                                constraint.tag.as_str() == tag.as_str()
                                    || constraint.tag.as_str() == "__it__"
                            }))
                    });
                    let desc = base.description();
                    format!("target {}", strip_leading_article(&desc))
                })
                .unwrap_or_else(|| "target creature".to_string());
            return format!("This turn, when {subject} attacks and isn't blocked, {delayed_text}");
        }
        if schedule.one_shot && schedule.start_next_turn {
            if trigger_lower.contains("that player's end step")
                || trigger_lower.contains("target player's end step")
            {
                return format!(
                    "At the beginning of the end step of that player's next turn, {delayed_text}"
                );
            }
            if trigger_lower.contains("that player's upkeep")
                || trigger_lower.contains("target player's upkeep")
            {
                return format!("At the beginning of that player's next upkeep, {delayed_text}");
            }
            if trigger_lower.contains("that player's draw step")
                || trigger_lower.contains("target player's draw step")
            {
                return format!("At the beginning of that player's next draw step, {delayed_text}");
            }
            if trigger_lower.contains("your end step") {
                return format!("At the beginning of your next end step, {delayed_text}");
            }
            if trigger_lower.contains("your upkeep") {
                return format!("At the beginning of your next upkeep, {delayed_text}");
            }
            if trigger_lower.contains("your draw step") {
                return format!("At the beginning of your next draw step, {delayed_text}");
            }
            if trigger_lower.contains("upkeep") {
                return describe_next_turn_upkeep_delayed_instruction(schedule, &delayed_text);
            }
            if trigger_lower.contains("draw step") {
                return format!("At the beginning of the next turn's draw step, {delayed_text}");
            }
            return format!("At the beginning of the next end step, {delayed_text}");
        }
        if schedule.one_shot
            && (trigger_lower.contains("beginning of each player's end step")
                || trigger_lower.contains("beginning of end step"))
        {
            // A short delayed cleanup that refers back to an object, or a
            // single typed destroy instruction, reads action-first in Oracle:
            // "Sacrifice it at ..." / "Destroy all permanents at ...".
            // Independent compound payloads keep timing-first wording.
            let delayed_lower = delayed_text.to_ascii_lowercase();
            let single_clause = !delayed_lower.contains(". ")
                && !delayed_lower.contains(" and ")
                && !delayed_lower.contains(", then");
            let back_references = delayed_lower
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
                .any(|word| word == "it" || word == "them")
                || delayed_lower.contains("that creature")
                || delayed_lower.contains("that permanent")
                || delayed_lower.contains("those creatures")
                || delayed_lower.contains("those permanents");
            let delayed_effects = schedule.effects.flattened_default_effects();
            let is_single_destroy_instruction = matches!(
                delayed_effects,
                [effect]
                    if structural_unwrap_render_wrappers(effect)
                        .downcast_ref::<crate::effects::DestroyEffect>()
                        .is_some()
            );
            let is_single_phase_out_instruction = matches!(
                delayed_effects,
                [effect]
                    if structural_unwrap_render_wrappers(effect)
                        .downcast_ref::<crate::effects::PhaseOutEffect>()
                        .is_some()
            );
            if (single_clause && back_references)
                || is_single_destroy_instruction
                || is_single_phase_out_instruction
            {
                return format!(
                    "{} at the beginning of the next end step",
                    capitalize_first(&delayed_text)
                );
            }
            return format!("At the beginning of the next end step, {delayed_text}");
        }
        if schedule.one_shot && trigger_lower.contains("end of combat") {
            // When a delayed sacrifice is followed by a result branch, the
            // permanent's controller is the actor of both the sacrifice and
            // the "If the player does" antecedent. Rendering the schedule as
            // an imperative otherwise changes that actor to the ability's
            // controller ("sacrifice ... If you do").
            for noun in ["creature", "permanent"] {
                for result_surface in ["If it happened", "If you do"] {
                    let prefix = format!(
                        "sacrifice that {noun}. {result_surface}, that player "
                    );
                    if let Some(follow_up) = delayed_text.strip_prefix(&prefix) {
                        let follow_up = normalize_you_verb_phrase(follow_up);
                        return format!(
                            "that {noun}'s controller sacrifices it at end of combat. If the player does, they {follow_up}"
                        );
                    }
                }
            }
            // Short back-referencing cleanups read as a suffix in oracle:
            // "destroy that creature at end of combat".  Longer payloads keep
            // the explicit "At this turn's next end of combat, ..." prefix.
            let delayed_lower = delayed_text.to_ascii_lowercase();
            let single_clause = !delayed_lower.contains(". ")
                && !delayed_lower.contains(" and ")
                && !delayed_lower.contains(", then");
            let back_references = delayed_lower
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
                .any(|word| word == "it")
                || delayed_lower.contains("that creature")
                || delayed_lower.contains("that permanent")
                || delayed_lower.contains("this creature")
                || delayed_lower.contains("this permanent");
            if single_clause && back_references {
                return format!("{delayed_text} at end of combat");
            }
            return format!("At this turn's next end of combat, {delayed_text}");
        }
        if schedule.one_shot && trigger_lower.contains("beginning of your end step") {
            return format!("At the beginning of your next end step, {delayed_text}");
        }
        if schedule.one_shot
            && schedule.target_tag.is_some()
            && (trigger_lower.contains("creature dies")
                || trigger_lower.contains("creature is put into a graveyard"))
        {
            if let Some(filter) = &schedule.target_filter
                && filter.demonstrative_antecedent_surface().is_some()
            {
                let mut base = filter.clone();
                base.zone = None;
                base.set_demonstrative_antecedent_surface(None);
                base.tagged_constraints.retain(|constraint| {
                    !(constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && schedule.target_tag.as_ref().is_some_and(|tag| {
                            constraint.tag.as_str() == tag.as_str()
                                || constraint.tag.as_str() == "__it__"
                        }))
                });
                let subject = if base.controller.as_ref() == Some(&PlayerFilter::NotYou) {
                    let mut uncontrolled = base.clone();
                    uncontrolled.controller = None;
                    let described = describe_for_each_filter(&uncontrolled);
                    format!("{} you don't control", strip_leading_article(&described))
                } else {
                    let described = describe_for_each_filter(&base);
                    strip_leading_article(&described).to_string()
                };
                return format!("When the {subject} dies this turn, {delayed_text}");
            }
            if schedule
                .target_tag
                .as_ref()
                .is_some_and(|tag| tag.as_str().contains("targeted"))
                && let Some(filter) = &schedule.target_filter
            {
                let mut base = filter.clone();
                base.zone = None;
                // The destination graveyard is a trigger condition, not a
                // targeting restriction on the permanent chosen now.
                base.owner = None;
                base.tagged_constraints.retain(|constraint| {
                    !(constraint.relation
                        == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        && schedule.target_tag.as_ref().is_some_and(|tag| {
                            constraint.tag.as_str() == tag.as_str()
                                || constraint.tag.as_str() == "__it__"
                        }))
                });
                let subject = format!(
                    "target {}",
                    strip_leading_article(&describe_for_each_filter(&base))
                );
                let delayed_text = delayed_text
                    .replacen("return it from graveyard to ", "return that card to ", 1)
                    .replacen("return it to ", "return that card to ", 1);
                let event = if trigger_lower.contains("put into your graveyard")
                    || filter.owner == Some(crate::target::PlayerFilter::You)
                {
                    "is put into your graveyard"
                } else if trigger_lower.contains("put into a graveyard") {
                    "is put into a graveyard"
                } else {
                    "dies"
                };
                return format!("When {subject} {event} this turn, {delayed_text}");
            }
            if let Some(filter) = &schedule.target_filter {
                let subject = with_indefinite_article(&describe_for_each_filter(filter));
                return format!(
                    "When {subject} dealt damage this way dies this turn, {delayed_text}"
                );
            }
            return format!("When that creature dies this turn, {delayed_text}");
        }
        if schedule.target_tag.is_some() && trigger_lower.contains("leaves the battlefield") {
            let subject = schedule
                .target_filter
                .as_ref()
                .map(|filter| {
                    let mut base = filter.clone();
                    base.tagged_constraints.retain(|constraint| {
                        !(constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                            && schedule.target_tag.as_ref().is_some_and(|tag| {
                                constraint.tag.as_str() == tag.as_str()
                                    || constraint.tag.as_str() == "__it__"
                            }))
                    });
                    let desc = base.description();
                    format!("that {}", strip_leading_article(&desc))
                })
                .unwrap_or_else(|| "that permanent".to_string());
            let duration = if schedule.until_end_of_turn {
                " this turn"
            } else {
                ""
            };
            return format!("When {subject} leaves the battlefield{duration}, {delayed_text}");
        }
        if trigger_lower.starts_with("when ")
            || trigger_lower.starts_with("whenever ")
            || trigger_lower.starts_with("if ")
        {
            // A this-turn delayed watch prints its window on the trigger
            // ("Whenever a nontoken creature ... enters this turn, ...").
            let duration = if schedule.until_end_of_turn
                && !schedule.start_next_turn
                && !trigger_lower.contains("this turn")
            {
                " this turn"
            } else {
                ""
            };
            return format!("{trigger_text}{duration}, {delayed_text}");
        }
        if trigger_lower.starts_with("at ") {
            return format!("{trigger_text}, {delayed_text}");
        }
        return format!("At {}, {delayed_text}", lowercase_first(&trigger_text));
    }
    if let Some(exile_instead) =
        effect.downcast_ref::<crate::effects::ExileInsteadOfGraveyardEffect>()
    {
        let graveyard_owner = describe_possessive_player_filter(&exile_instead.player);
        return format!(
            "If a card would be put into {graveyard_owner} graveyard from anywhere this turn, exile that card instead"
        );
    }
    if let Some(local) = effect.downcast_ref::<crate::effects::LocalRewriteEffect>() {
        let base = describe_effect(&local.effect);
        let followups = local
            .zone_replacements
            .iter()
            .map(|register| {
                if let Some(followup) = describe_countered_spell_exile_replacement_followup(
                    &local.effect,
                    register,
                ) {
                    return followup;
                }
                if let Some(followup) =
                    describe_tagged_die_exile_replacement_followup(&local.effect, register)
                {
                    return followup;
                }
                let target = describe_choose_spec(&register.target);
                let referent = if target.contains("spell") {
                    "that spell".to_string()
                } else if target.contains("creature") {
                    "that creature".to_string()
                } else if target.contains("permanent") {
                    "that permanent".to_string()
                } else {
                    "it".to_string()
                };
                if register.from_zone == Some(Zone::Stack)
                    && register.to_zone == Some(Zone::Graveyard)
                {
                    return format!(
                        "If {referent} is countered this way, it goes to {:?} instead of graveyard",
                        register.replacement_zone
                    )
                    .to_ascii_lowercase();
                }

                let from = register
                    .from_zone
                    .map(|zone| format!(" from {zone:?}"))
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let to = register
                    .to_zone
                    .map(|zone| format!(" into {zone:?}"))
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let replacement = format!("{:?}", register.replacement_zone).to_ascii_lowercase();
                if register.optional {
                    format!(
                        "If {referent} would go{from}{to}, you may put it into {replacement} instead"
                    )
                } else {
                    format!("If {referent} would go{from}{to}, it goes to {replacement} instead")
                }
            })
            .collect::<Vec<_>>();
        if followups.is_empty() {
            return base;
        }
        return format!("{base}. {}", followups.join(". "));
    }
    if let Some(register) = effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>() {
        if register.from_zone == Some(Zone::Stack)
            && register.to_zone == Some(Zone::Graveyard)
            && register.replacement_zone == Zone::Exile
            && matches!(register.mode, crate::effects::ReplacementApplyMode::OneShot)
            && !register.optional
            && register.counters.is_empty()
            && matches!(register.target.base(), ChooseSpec::Tagged(_))
            && register.linked_exile_follow_up
                == Some(ironsmith_core::LinkedExileFollowUp::ReturnToHandAtNextEndStep)
        {
            return "Exile that card instead of putting it into your graveyard as it resolves. If you do, return it to your hand at the beginning of the next end step"
                .to_string();
        }
        if register.from_zone == Some(Zone::Battlefield)
            && register.to_zone.is_none()
            && register.replacement_zone == Zone::Exile
            && matches!(
                register.mode,
                crate::effects::ReplacementApplyMode::Resolution
            )
            && !register.optional
            && register.counters.is_empty()
        {
            let target = describe_choose_spec(&register.target);
            return format!(
                "If {target} would leave the battlefield, exile it instead of putting it anywhere else"
            );
        }
        if register.from_zone == Some(Zone::Stack)
            && register.to_zone == Some(Zone::Graveyard)
            && register.replacement_zone == Zone::Exile
            && matches!(
                register.mode,
                crate::effects::ReplacementApplyMode::OneShot
                    | crate::effects::ReplacementApplyMode::UntilEndOfTurn
            )
            && !register.optional
            && register.counters.is_empty()
            && matches!(register.target.base(), ChooseSpec::Tagged(_))
        {
            return "If that spell would be put into a graveyard, exile it instead".to_string();
        }
        let target = describe_choose_spec(&register.target);
        let from = register
            .from_zone
            .map(|zone| format!(" from {zone:?}"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let to = register
            .to_zone
            .map(|zone| format!(" into {zone:?}"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let duration = match register.mode {
            crate::effects::ReplacementApplyMode::OneShot
            | crate::effects::ReplacementApplyMode::UntilEndOfTurn => " this turn",
            crate::effects::ReplacementApplyMode::Resolution => "",
        };
        let replacement = format!("{:?}", register.replacement_zone).to_ascii_lowercase();
        if register.optional {
            return format!(
                "If {target} would go{from}{to}{duration}, you may put it into {replacement} instead"
            );
        }
        return format!(
            "If {target} would go{from}{to}{duration}, it goes to {replacement} instead"
        );
    }
    if let Some(register) = effect.downcast_ref::<crate::effects::RegisterDrawReplacementEffect>() {
        let player = if register.player == PlayerFilter::IteratedPlayer {
            "they".to_string()
        } else {
            describe_player_filter(&register.player)
        };
        let duration = match register.mode {
            crate::effects::ReplacementApplyMode::OneShot
            | crate::effects::ReplacementApplyMode::UntilEndOfTurn => " this turn",
            crate::effects::ReplacementApplyMode::Resolution => "",
        };
        if let Some(replacement) = describe_draw_replacement_exile_top_play(
            &register.player,
            &register.replacement_effects,
        ) {
            return format!(
                "The next time {player} would draw a card{duration}, instead {replacement}"
            );
        }
        let replacement = lowercase_first(&describe_effect_list(&register.replacement_effects));
        return format!(
            "The next time {player} would draw a card{duration}, instead {replacement}"
        );
    }
    if let Some(register) = effect.downcast_ref::<crate::effects::RegisterManaReplacementEffect>() {
        let source = register.source_filter.description();
        let mana = register
            .replacement_mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        let prefix = match register.mode {
            crate::effects::ReplacementApplyMode::UntilEndOfTurn => "Until end of turn, if ",
            crate::effects::ReplacementApplyMode::OneShot => "The next time ",
            crate::effects::ReplacementApplyMode::Resolution => "If ",
        };
        if matches!(register.source_filter.controller, Some(PlayerFilter::You)) {
            return format!(
                "{prefix}you tap {source} for mana, it produces {mana} instead of any other type"
            );
        }
        return format!(
            "{prefix}{source} is tapped for mana, it produces {mana} instead of any other type"
        );
    }
    if let Some(register) =
        effect.downcast_ref::<crate::effects::RegisterEnterTappedReplacementEffect>()
    {
        let mut subject_filter = register.filter.clone();
        subject_filter.zone = None;
        let subject = if subject_filter.has_all_permanent_card_types() {
            "Permanents".to_string()
        } else {
            capitalize_first(&pluralize_noun_phrase(&subject_filter.description()))
        };
        return match register.mode {
            crate::effects::ReplacementApplyMode::UntilEndOfTurn => {
                format!("{subject} enter tapped this turn")
            }
            crate::effects::ReplacementApplyMode::OneShot => {
                let singular = with_indefinite_article(strip_leading_article(
                    &subject_filter.description(),
                ));
                format!(
                    "The next time {singular} would enter the battlefield, it enters tapped instead"
                )
            }
            crate::effects::ReplacementApplyMode::Resolution => {
                format!("{subject} enter tapped")
            }
        };
    }
    if let Some(register) =
        effect.downcast_ref::<crate::effects::RegisterNextBatchEnterWithCountersEffect>()
    {
        let mut subject_filter = register.filter.clone();
        subject_filter.zone = None;
        let subject = pluralize_noun_phrase(strip_leading_article(
            &subject_filter.description(),
        ));
        let count = match register.count.unhinted() {
            Value::Fixed(count) => number_word(*count).unwrap_or_else(|| count.to_string()),
            count => describe_value(count),
        };
        let counter = describe_counter_type(register.counter_type);
        let counter_noun = if register.count.unhinted() == &Value::Fixed(1) {
            "counter"
        } else {
            "counters"
        };
        return format!(
            "The next time one or more {subject} enter this turn, each enters with {count} additional {counter} {counter_noun} on it"
        );
    }
    if let Some(register) =
        effect.downcast_ref::<crate::effects::RegisterFutureZoneReplacementEffect>()
    {
        if register.filter == ObjectFilter::creature()
            && register.from_zone == Some(Zone::Battlefield)
            && register.to_zone == Some(Zone::Graveyard)
            && register.replacement_zone == Zone::Exile
            && register.mode == crate::effects::ReplacementApplyMode::UntilEndOfTurn
            && register.cause_filter.is_none()
            && !register.require_cause_source_match
        {
            return "If a creature would die this turn, exile it instead".to_string();
        }
        if register.filter == ObjectFilter::permanent().controlled_by(PlayerFilter::You)
            && register.from_zone == Some(Zone::Battlefield)
            && register.to_zone == Some(Zone::Graveyard)
            && register.replacement_zone == Zone::Exile
            && register.mode == crate::effects::ReplacementApplyMode::UntilEndOfTurn
            && register.cause_filter.is_none()
            && !register.require_cause_source_match
        {
            return "If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead".to_string();
        }
        if register.filter == ObjectFilter::instant_or_sorcery().cast_by_you()
            && register.from_zone == Some(Zone::Stack)
            && register.to_zone == Some(Zone::Graveyard)
            && register.replacement_zone == Zone::Hand
            && matches!(
                register.mode,
                crate::effects::ReplacementApplyMode::OneShot
                    | crate::effects::ReplacementApplyMode::UntilEndOfTurn
            )
        {
            return "The next time you cast an instant or sorcery spell from your hand this turn, put that card into your hand instead of into your graveyard".to_string();
        }

        let target = register.filter.description();
        let from = register
            .from_zone
            .map(|zone| format!(" from {zone:?}"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let to = register
            .to_zone
            .map(|zone| format!(" into {zone:?}"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let duration = match register.mode {
            crate::effects::ReplacementApplyMode::OneShot
            | crate::effects::ReplacementApplyMode::UntilEndOfTurn => " this turn",
            crate::effects::ReplacementApplyMode::Resolution => "",
        };
        let replacement = format!("{:?}", register.replacement_zone).to_ascii_lowercase();
        let prefix = if register.mode == crate::effects::ReplacementApplyMode::OneShot {
            "The next time"
        } else {
            "If"
        };
        return format!(
            "{prefix} {target} would go{from}{to}{duration}, it goes to {replacement} instead"
        );
    }
    if let Some(register) =
        effect.downcast_ref::<crate::effects::RegisterDamagedBySourceZoneReplacementEffect>()
    {
        if matches!(register.filter.zone, None | Some(Zone::Battlefield))
            && register.from_zone == Some(Zone::Battlefield)
            && register.to_zone == Some(Zone::Graveyard)
            && register.replacement_zone == Zone::Exile
        {
            let subject = with_indefinite_article(strip_leading_article(
                &register.filter.description(),
            ));
            let duration = match register.mode {
                crate::effects::ReplacementApplyMode::OneShot
                | crate::effects::ReplacementApplyMode::UntilEndOfTurn => " this turn",
                crate::effects::ReplacementApplyMode::Resolution => "",
            };
            return format!(
                "If {subject} dealt damage this way would die{duration}, exile it instead"
            );
        }

        let target = register.filter.description();
        let from = register
            .from_zone
            .map(|zone| format!(" from {zone:?}"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let to = register
            .to_zone
            .map(|zone| format!(" into {zone:?}"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let replacement = format!("{:?}", register.replacement_zone).to_ascii_lowercase();
        return format!(
            "If {target} dealt damage this way would go{from}{to} this turn, it goes to {replacement} instead"
        );
    }
    if let Some(additional_land_plays) =
        effect.downcast_ref::<crate::effects::AdditionalLandPlaysEffect>()
    {
        let player = describe_player_filter(&additional_land_plays.player);
        let count = describe_value(&additional_land_plays.count);
        let land_text = if matches!(additional_land_plays.count, Value::Fixed(1)) {
            "an additional land".to_string()
        } else {
            format!("{count} additional lands")
        };
        return match additional_land_plays.duration {
            Until::EndOfTurn => format!("{player} may play {land_text} this turn"),
            _ => format!(
                "{player} may play {land_text} {}",
                describe_until(&additional_land_plays.duration)
            ),
        };
    }
    if let Some(control_player) = effect.downcast_ref::<crate::effects::ControlPlayerEffect>() {
        return format!(
            "Control {} during their next turn",
            describe_player_filter(&control_player.player)
        );
    }
    if let Some(control_combat) =
        effect.downcast_ref::<crate::effects::ControlCombatChoicesThisTurnEffect>()
    {
        let scope = if control_combat.this_combat {
            "this combat"
        } else {
            "this turn"
        };
        return match (control_combat.attackers, control_combat.blockers) {
            (true, false) => format!("You choose which creatures attack {scope}"),
            (false, true) => {
                format!("You choose which creatures block {scope} and how those creatures block")
            }
            (true, true) => format!("You choose which creatures attack and block {scope}"),
            (false, false) => format!("You choose combat {scope}"),
        };
    }
    if let Some((count, color_filter)) = effect.0.exile_from_hand_cost_info() {
        return capitalize_first(&describe_exile_from_hand_as_cost_phrase(
            count,
            color_filter,
        ));
    }
    if let Some(imprint) = effect.downcast_ref::<crate::effects::cards::ImprintFromHandEffect>() {
        return describe_imprint_from_hand_phrase(imprint);
    }
    if let Some(for_each_ctrl) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return format!(
            "For each controller of tagged '{}' objects, {}",
            for_each_ctrl.tag.as_str(),
            describe_effect_list(&for_each_ctrl.effects)
        );
    }
    if let Some(for_each_tagged_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        if for_each_tagged_player.tag.as_str() == "voted_against_you" {
            let body = describe_effect_list(&for_each_tagged_player.effects);
            let body = body.trim().trim_end_matches('.');
            let predicate = body
                .strip_prefix("That player ")
                .or_else(|| body.strip_prefix("that player "))
                .or_else(|| body.strip_prefix("They "))
                .or_else(|| body.strip_prefix("they "))
                .unwrap_or(body);
            return format!(
                "Each opponent who voted for a choice you didn't vote for {}",
                lowercase_first(predicate)
            );
        }
        return format!(
            "For each tagged '{}' player, {}",
            for_each_tagged_player.tag.as_str(),
            describe_effect_list(&for_each_tagged_player.effects)
        );
    }
    if let Some(_apply_replacement) =
        effect.downcast_ref::<crate::effects::ApplyReplacementEffect>()
    {
        return "Apply a replacement effect".to_string();
    }
    if let Some(become_color) = effect.downcast_ref::<crate::effects::BecomeColorChoiceEffect>() {
        let choice = if become_color.allow_multiple {
            "color or colors"
        } else {
            "color"
        };
        let duration = match &become_color.duration {
            Until::Forever => String::new(),
            _ => format!(" {}", describe_until(&become_color.duration)),
        };
        return format!(
            "{} becomes the {choice} of {} choice{duration}",
            describe_choose_spec(&become_color.target),
            describe_possessive_player_filter(&become_color.chooser),
        );
    }
    if let Some(become_type) =
        effect.downcast_ref::<crate::effects::BecomeCreatureTypeChoiceEffect>()
    {
        let choice_text = if become_type.excluded_subtypes.is_empty() {
            "Choose a creature type".to_string()
        } else {
            let excluded = become_type
                .excluded_subtypes
                .iter()
                .map(|subtype| subtype.to_string().to_ascii_lowercase())
                .collect::<Vec<_>>();
            format!(
                "Choose a creature type other than {}",
                join_with_or(&excluded)
            )
        };
        let subject_text = describe_each_object_subject(&become_type.target)
            .unwrap_or_else(|| describe_choose_spec(&become_type.target));
        if become_type.excluded_subtypes.is_empty() {
            return format!(
                "{subject_text} becomes the creature type of {} choice {}",
                describe_possessive_player_filter(&become_type.chooser),
                describe_until(&become_type.duration)
            );
        }
        return format!(
            "{choice_text}. {subject_text} becomes that type {}",
            describe_until(&become_type.duration)
        );
    }
    if effect
        .downcast_ref::<crate::effects::BecomeSaddledUntilEotEffect>()
        .is_some()
    {
        return "This permanent becomes saddled until end of turn".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::CascadeEffect>()
        .is_some()
    {
        return "Cascade".to_string();
    }
    if let Some(cast_source) = effect.downcast_ref::<crate::effects::CastSourceEffect>() {
        let mut parts = Vec::new();
        if cast_source.require_exile {
            parts.push("Cast this card from exile".to_string());
        } else {
            parts.push("Cast this card".to_string());
        }
        if cast_source.without_paying_mana_cost {
            parts.push("without paying its mana cost".to_string());
        }
        return parts.join(" ");
    }
    if let Some(clash) = effect.downcast_ref::<crate::effects::ClashEffect>() {
        return match &clash.opponent_mode {
            crate::effects::ClashOpponentMode::AnyOpponent => "Clash with an opponent".to_string(),
            crate::effects::ClashOpponentMode::TargetOpponent => {
                "Clash with target opponent".to_string()
            }
            crate::effects::ClashOpponentMode::DefendingPlayer => {
                "Clash with defending player".to_string()
            }
        };
    }
    if let Some(clear_damage) = effect.downcast_ref::<crate::effects::ClearDamageEffect>() {
        return format!(
            "Remove all damage from {}",
            describe_choose_spec(&clear_damage.target)
        );
    }
    if let Some(heal) = effect.downcast_ref::<crate::effects::HealDamageEffect>() {
        let target = describe_choose_spec(&heal.target);
        return match &heal.amount {
            Some(amount) => format!(
                "Heal {} damage already dealt to {target}",
                describe_value(amount)
            ),
            None => format!("All damage already dealt to {target} is healed"),
        };
    }
    if let Some(create_emblem) = effect.downcast_ref::<crate::effects::CreateEmblemEffect>() {
        let emblem_text = create_emblem.emblem.text.trim();
        if !emblem_text.is_empty() {
            let emblem_text = capitalize_first(&ensure_trailing_period(emblem_text));
            return format!("You get an emblem with \"{emblem_text}\"");
        }
        return format!("Create an emblem named {}", create_emblem.emblem.name);
    }
    if effect
        .downcast_ref::<crate::effects::ConspireCostEffect>()
        .is_some()
    {
        return "Tap two untapped creatures you control that each share a color with this spell"
            .to_string();
    }
    if let Some(crew) = effect.downcast_ref::<crate::effects::CrewCostEffect>() {
        return format!(
            "Tap any number of untapped creatures you control with total power {} or more",
            crew.required_power
        );
    }
    if let Some(enter_attacking) = effect.downcast_ref::<crate::effects::EnterAttackingEffect>() {
        return format!(
            "Put {} onto the battlefield tapped and attacking",
            describe_choose_spec(&enter_attacking.target)
        );
    }
    if effect
        .downcast_ref::<crate::effects::EvolveEffect>()
        .is_some()
    {
        return "Evolve".to_string();
    }
    if let Some(amplify) = effect.downcast_ref::<crate::effects::AmplifyEffect>() {
        return format!("Amplify {}", amplify.amount);
    }
    if let Some(devour) = effect.downcast_ref::<crate::effects::DevourEffect>() {
        return format!("Devour {}", devour.multiplier);
    }
    if let Some(exchange_life) = effect.downcast_ref::<crate::effects::ExchangeLifeTotalsEffect>() {
        if let (PlayerFilter::Target(first), PlayerFilter::Target(second)) =
            (&exchange_life.player1, &exchange_life.player2)
            && matches!(first.as_ref(), PlayerFilter::Any)
            && matches!(second.as_ref(), PlayerFilter::Any)
        {
            return "Two target players exchange life totals".to_string();
        }
        return format!(
            "Exchange life totals of {} and {}",
            describe_player_filter(&exchange_life.player1),
            describe_player_filter(&exchange_life.player2)
        );
    }
    if let Some(exchange_values) = effect.downcast_ref::<crate::effects::ExchangeValuesEffect>() {
        let describe_operand = |operand: &crate::effects::ExchangeValueOperand| match operand {
            crate::effects::ExchangeValueOperand::LifeTotal(player) => {
                format!("{} life total", describe_possessive_player_filter(player))
            }
            crate::effects::ExchangeValueOperand::Power(target)
                if *target == crate::target::ChooseSpec::Source =>
            {
                "this creature's power".to_string()
            }
            crate::effects::ExchangeValueOperand::Toughness(target)
                if *target == crate::target::ChooseSpec::Source =>
            {
                "this creature's toughness".to_string()
            }
            crate::effects::ExchangeValueOperand::Power(target) => {
                format!("the power of {}", describe_choose_spec(target))
            }
            crate::effects::ExchangeValueOperand::Toughness(target) => {
                format!("the toughness of {}", describe_choose_spec(target))
            }
        };
        let connective = if matches!(
            (&exchange_values.left, &exchange_values.right),
            (
                crate::effects::ExchangeValueOperand::LifeTotal(_),
                crate::effects::ExchangeValueOperand::Power(_)
                    | crate::effects::ExchangeValueOperand::Toughness(_)
            ) | (
                crate::effects::ExchangeValueOperand::Power(_)
                    | crate::effects::ExchangeValueOperand::Toughness(_),
                crate::effects::ExchangeValueOperand::LifeTotal(_)
            )
        ) {
            " with "
        } else {
            " and "
        };
        let duration = if exchange_values.duration == Until::Forever {
            String::new()
        } else {
            format!(" {}", describe_until(&exchange_values.duration))
        };
        return format!(
            "Exchange {}{}{}{}",
            describe_operand(&exchange_values.left),
            connective,
            describe_operand(&exchange_values.right),
            duration
        );
    }
    if let Some(exile_top) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() {
        return describe_exile_top_clause(exile_top, false)
            .map(|(clause, _)| clause)
            .unwrap_or_else(|| {
                describe_exile_top_of_library(&exile_top.player, &exile_top.count, false)
            });
    }
    if let Some(experience) = effect.downcast_ref::<crate::effects::ExperienceCountersEffect>() {
        let player = describe_player_filter(&experience.player);
        let amount = if let Some((multiplier, basis)) =
            describe_for_each_multiplier_and_basis(&experience.count)
        {
            let counters = if multiplier == 1 {
                "an experience counter".to_string()
            } else {
                let multiplier =
                    number_word(multiplier).unwrap_or_else(|| multiplier.to_string());
                format!("{multiplier} experience counters")
            };
            format!("{counters} for each {basis}")
        } else {
            match experience.count {
                Value::Fixed(1) => "an experience counter".to_string(),
                _ => format!("{} experience counters", describe_value(&experience.count)),
            }
        };
        return format!("{player} {} {amount}", player_verb(&player, "get", "gets"));
    }
    if let Some(for_each_counter_kind) =
        effect.downcast_ref::<crate::effects::ForEachCounterKindPutOrRemoveEffect>()
    {
        if for_each_counter_kind.fixed_counter_type == Some(crate::object::CounterType::Time)
            && for_each_counter_kind.optional_action
            && is_time_travel_object_set(&for_each_counter_kind.target)
        {
            return "Time travel".to_string();
        }
        let target = describe_choose_spec(&for_each_counter_kind.target);
        if for_each_counter_kind.all_kinds {
            return format!(
                "For each kind of counter on {target}, choose to put or remove one of that kind"
            );
        }
        if is_target_permanent_or_suspended_card(&for_each_counter_kind.target) {
            return "Choose a counter on target permanent or suspended card. Remove that counter from that permanent or card or put another of those counters on it".to_string();
        }
        return format!(
            "Choose a counter on {target}. Remove that counter from it or put another of those counters on it"
        );
    }
    if let Some(chosen_kind) = effect.downcast_ref::<crate::effects::PutCounterOfChosenKindEffect>()
    {
        if matches!(&chosen_kind.target, ChooseSpec::Target(_)) {
            return "Choose a counter on target permanent. Put an additional counter of that kind on that permanent".to_string();
        }
        let target = describe_choose_spec(&chosen_kind.target);
        return format!(
            "Choose a counter on {target}. Put an additional counter of that kind on it"
        );
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantEffect>() {
        if grant.duration == crate::grant::GrantDuration::Forever
            && matches!(&grant.target, ChooseSpec::Tagged(_))
            && let crate::grant::Grantable::Ability(ability) = &grant.grantable
            && let Some(cost_increase) = ability.cost_increase_mana_cost()
            && cost_increase.filter.stack_kind == Some(crate::filter::StackObjectKind::Spell)
        {
            let subject = match cost_increase.filter.cast_by {
                None => Some("A spell cast this way"),
                Some(crate::filter::PlayerFilter::Opponent) => {
                    Some("A spell cast by an opponent this way")
                }
                Some(crate::filter::PlayerFilter::You) => {
                    Some("A spell cast by you this way")
                }
                Some(crate::filter::PlayerFilter::IteratedPlayer) => {
                    Some("A spell cast by that player this way")
                }
                _ => None,
            };
            if let Some(subject) = subject {
                return format!(
                    "{subject} costs {} more to cast",
                    cost_increase.increase.to_oracle()
                );
            }
        }
        let duration = match grant.duration {
            crate::grant::GrantDuration::UntilEndOfTurn => " until end of turn",
            crate::grant::GrantDuration::UntilYourNextTurnEnd => " until the end of your next turn",
            crate::grant::GrantDuration::Forever => "",
        };
        if grant.duration == crate::grant::GrantDuration::UntilEndOfTurn
            && matches!(grant.target.base(), ChooseSpec::Source)
            && let crate::grant::Grantable::Ability(ability) = &grant.grantable
            && let Some(model) = ability.compiled_model()
            && let ironsmith_core::StaticAbilityPayload::TargetingAsThoughNoAbility(spec) =
                &model.payload
        {
            return format!("Until end of turn, {}", lowercase_first(&spec.display));
        }
        if let crate::grant::Grantable::DerivedAlternativeCast(
            crate::grant::DerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs },
        ) = &grant.grantable
        {
            let cost_text = if additional_costs.is_empty() {
                "its mana cost".to_string()
            } else {
                format!(
                    "its mana cost plus {}",
                    describe_additional_costs(additional_costs)
                )
            };
            return format!(
                "{} gains flashback{}. The flashback cost is equal to {cost_text}",
                describe_choose_spec(&grant.target),
                duration
            );
        }
        let granted_text = match &grant.grantable {
            crate::grant::Grantable::Ability(ability) => ability
                .granted_inline_ability()
                .map(describe_inline_ability)
                .unwrap_or_else(|| lowercase_first(&ability.display())),
            _ => grant.grantable.display(),
        };
        return format!(
            "{} gains {}{}",
            describe_choose_spec(&grant.target),
            granted_text,
            duration
        );
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantBySpecEffect>() {
        if matches!(grant.spec.grantable, crate::grant::Grantable::PlayFrom)
            && grant.spec.zone == Zone::Exile
            && !grant.spec.filter.tagged_constraints.is_empty()
            && matches!(
                grant.player,
                crate::filter::PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(_))
            )
        {
            return "That card's owner may play it for as long as it remains exiled".to_string();
        }
        if grant.duration == crate::grant::GrantDuration::UntilEndOfTurn
            && grant.spec.zone == Zone::Hand
            && matches!(
                &grant.spec.grantable,
                crate::grant::Grantable::Ability(ability) if ability.has_flash()
            )
        {
            let permission = grant
                .spec
                .clone()
                .with_beneficiary(grant.player.clone())
                .display();
            if let Some((subject, timing)) = permission.split_once(" as though") {
                return format!("{subject} this turn as though{timing}");
            }
        }
        let duration = match grant.duration {
            crate::grant::GrantDuration::UntilEndOfTurn => " until end of turn",
            crate::grant::GrantDuration::UntilYourNextTurnEnd => " until the end of your next turn",
            crate::grant::GrantDuration::Forever => "",
        };
        return format!(
            "{}{}",
            grant
                .spec
                .clone()
                .with_beneficiary(grant.player.clone())
                .display(),
            duration
        );
    }
    if let Some(grant_play_tagged) = effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
    {
        if let Some(rendered) =
            describe_temporary_tagged_permission_surface(grant_play_tagged, false)
        {
            return rendered;
        }
        if let Some(counter_type) = grant_play_tagged.during_turns_counter_put_on_source {
            let verb = if grant_play_tagged.allow_land { "play" } else { "cast" };
            return format!(
                "During any turn you put {} on this permanent, {} may {verb} that card",
                with_indefinite_article(&format!("{} counter", counter_type.description())),
                describe_player_filter(&grant_play_tagged.player),
            );
        }
        let timing = match grant_play_tagged.duration {
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "this turn".to_string(),
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                "until the end of your next turn".to_string()
            }
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => {
                "until your next end step".to_string()
            }
            crate::effects::GrantPlayTaggedDuration::UntilSourceExilesAnother => {
                "until this source exiles another card".to_string()
            }
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled => {
                if grant_play_tagged.cast_pool_is_plural {
                    "for as long as they remain exiled".to_string()
                } else {
                    "for as long as it remains exiled".to_string()
                }
            }
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource => {
                let source = grant_play_tagged
                    .surface
                    .as_ref()
                    .and_then(|surface| surface.control_source.as_ref())
                    .map(ironsmith_core::SourceReferenceSurface::display_text)
                    .unwrap_or_else(|| "this source".to_string());
                format!("for as long as you control {source}")
            }
        };
        let verb = if grant_play_tagged.allow_land {
            "play"
        } else {
            "cast"
        };
        let helper_tag = grant_play_tagged.tag.as_str().starts_with("targeted_")
            || grant_play_tagged.tag.as_str().starts_with("__source_")
            || grant_play_tagged.tag.as_str() == "__it__"
            || matches!(
                grant_play_tagged.tag.as_str(),
                "exiled" | "revealed" | "looked" | "chosen" | "searched"
            )
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "exiled")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "revealed")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "looked")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "chosen")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "searched");
        let helper_exiled =
            crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "exiled");
        let helper_consult_match = crate::cards::is_sentence_helper_tag(
            grant_play_tagged.tag.as_str(),
            "consult_match",
        );
        let helper_tag = helper_tag || helper_consult_match;
        let object_text = if grant_play_tagged.cast_pool_is_plural && helper_tag {
            if grant_play_tagged.allow_land {
                "those cards".to_string()
            } else {
                match grant_play_tagged
                    .surface
                    .as_ref()
                    .and_then(|surface| surface.object.as_ref())
                {
                    Some(
                        ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseExiledCards,
                    ) => "spells from among those exiled cards".to_string(),
                    _ => "spells from among those cards".to_string(),
                }
            }
        } else if grant_play_tagged.allow_land && helper_exiled {
            if grant_play_tagged.cast_pool_is_plural {
                "those cards".to_string()
            } else {
                "that card".to_string()
            }
        } else if grant_play_tagged.tag.as_str().starts_with("targeted_")
            || grant_play_tagged.tag.as_str().starts_with("__source_")
            || grant_play_tagged.tag.as_str() == "__it__"
            || matches!(
                grant_play_tagged.tag.as_str(),
                "exiled" | "revealed" | "looked" | "chosen" | "searched"
            )
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "exiled")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "revealed")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "looked")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "chosen")
            || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "searched")
            || helper_consult_match
        {
            "that card".to_string()
        } else {
            format!("tagged '{}' cards", grant_play_tagged.tag.as_str())
        };
        if grant_play_tagged.allow_any_color_for_cast {
            if helper_tag
                && !grant_play_tagged.allow_land
                && grant_play_tagged.duration
                    == crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
            {
                if !grant_play_tagged.cast_pool_is_plural {
                    let mana_clause = grant_play_tagged
                        .mana_spend_cast_clause("that spell")
                        .expect("flexible mana grant has a cast clause");
                    return format!(
                        "{} may cast that card this turn, and {mana_clause}",
                        describe_player_filter(&grant_play_tagged.player),
                    );
                }
                let mana_clause = grant_play_tagged
                    .mana_spend_cast_clause("them")
                    .expect("flexible mana grant has a cast clause");
                return format!(
                    "{} may cast spells from among those cards this turn, and {mana_clause}",
                    describe_player_filter(&grant_play_tagged.player),
                );
            }
            if helper_tag
                && !grant_play_tagged.allow_land
                && grant_play_tagged.duration
                    == crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd
            {
                let mana_clause = grant_play_tagged
                    .mana_spend_cast_clause("that spell")
                    .expect("flexible mana grant has a cast clause");
                return format!(
                    "Until the end of your next turn, {} may cast that card and {mana_clause}",
                    describe_player_filter(&grant_play_tagged.player),
                );
            }
            let pronoun = if object_text == "that card" {
                "that spell"
            } else {
                "them"
            };
            let mana_clause = grant_play_tagged
                .mana_spend_cast_clause(pronoun)
                .expect("flexible mana grant has a cast clause");
            return format!(
                "{} may {verb} {object_text} {timing}, and {mana_clause}",
                describe_player_filter(&grant_play_tagged.player),
            );
        }
        if helper_tag
            && !grant_play_tagged.allow_land
            && grant_play_tagged.duration == crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
            && matches!(grant_play_tagged.player, PlayerFilter::You)
        {
            if (helper_exiled || helper_consult_match)
                && !grant_play_tagged.cast_pool_is_plural
            {
                return "you may cast that card this turn".to_string();
            }
            let cards_text = if grant_play_tagged.tag.as_str() == "exiled"
                || crate::cards::is_sentence_helper_tag(grant_play_tagged.tag.as_str(), "exiled")
            {
                "those exiled cards"
            } else {
                "those cards"
            };
            return format!("Until end of turn, you may cast spells from among {cards_text}");
        }
        let mut rendered = format!(
            "{} may {verb} {object_text} {timing}",
            describe_player_filter(&grant_play_tagged.player),
        );
        if let Some(cost) = &grant_play_tagged.spell_cost_increase {
            rendered.push_str(&format!(
                ". Each spell cast this way costs {} more to cast",
                cost.to_oracle()
            ));
        }
        if grant_play_tagged.lands_enter_tapped {
            rendered.push_str(". Each land played this way enters tapped");
        }
        return rendered;
    }
    if let Some(grant_tagged_spell_life) =
        effect.downcast_ref::<crate::effects::GrantTaggedSpellLifeCostByManaValueEffect>()
    {
        return format!(
            "{} may cast {} from exile this turn by paying life equal to their mana value",
            describe_player_filter(&grant_tagged_spell_life.player),
            "those spells"
        );
    }
    if let Some(grant_tagged_spell_free_cast) =
        effect.downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
    {
        let helper_tag = grant_tagged_spell_free_cast
            .tag
            .as_str()
            .starts_with("targeted_")
            || grant_tagged_spell_free_cast
                .tag
                .as_str()
                .starts_with("__source_")
            || grant_tagged_spell_free_cast.tag.as_str() == "__it__"
            || matches!(
                grant_tagged_spell_free_cast.tag.as_str(),
                "exiled" | "revealed" | "looked" | "chosen" | "searched"
            )
            || crate::cards::is_sentence_helper_tag(
                grant_tagged_spell_free_cast.tag.as_str(),
                "exiled",
            )
            || crate::cards::is_sentence_helper_tag(
                grant_tagged_spell_free_cast.tag.as_str(),
                "revealed",
            )
            || crate::cards::is_sentence_helper_tag(
                grant_tagged_spell_free_cast.tag.as_str(),
                "looked",
            )
            || crate::cards::is_sentence_helper_tag(
                grant_tagged_spell_free_cast.tag.as_str(),
                "chosen",
            )
            || crate::cards::is_sentence_helper_tag(
                grant_tagged_spell_free_cast.tag.as_str(),
                "searched",
            )
            || crate::cards::is_sentence_helper_tag(
                grant_tagged_spell_free_cast.tag.as_str(),
                "consult_match",
            );
        let helper_exiled = crate::cards::is_sentence_helper_tag(
            grant_tagged_spell_free_cast.tag.as_str(),
            "exiled",
        ) || grant_tagged_spell_free_cast.tag.as_str() == "exiled";
        let object_text = if helper_exiled {
            "those exiled cards"
        } else if helper_tag {
            "that card"
        } else {
            "those spells"
        };
        let cost_text = if helper_exiled {
            "their mana costs"
        } else if helper_tag {
            "its mana cost"
        } else {
            "their mana costs"
        };
        let zone_text = match grant_tagged_spell_free_cast.zone {
            Some(crate::zone::Zone::Exile) => " from exile",
            Some(crate::zone::Zone::Library) => " from a library",
            Some(crate::zone::Zone::Graveyard) => " from a graveyard",
            Some(crate::zone::Zone::Hand) => " from hand",
            Some(crate::zone::Zone::Battlefield) => " from the battlefield",
            Some(crate::zone::Zone::Stack) => " from the stack",
            Some(crate::zone::Zone::Command) => " from the command zone",
            Some(crate::zone::Zone::Ante) => " from ante",
            Some(crate::zone::Zone::OutsideGame) => " from outside the game",
            None => "",
        };
        let timing_text = match grant_tagged_spell_free_cast.duration {
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "this turn",
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                "until the end of your next turn"
            }
            crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep => {
                "until your next end step"
            }
            crate::effects::GrantPlayTaggedDuration::UntilSourceExilesAnother => {
                "until this source exiles another card"
            }
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled => {
                "for as long as it remains exiled"
            }
            crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource => {
                "for as long as you control this source"
            }
        };
        return format!(
            "{} may cast {object_text}{zone_text} {timing_text} without paying {cost_text}",
            describe_player_filter(&grant_tagged_spell_free_cast.player),
        );
    }
    if let Some(may_cast_matching) =
        effect.downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()
    {
        let player = describe_player_filter(&may_cast_matching.player);
        fn alternative_cast_kind_text(kind: crate::filter::AlternativeCastKind) -> &'static str {
            match kind {
                crate::filter::AlternativeCastKind::Blitz => "blitz",
                crate::filter::AlternativeCastKind::Dash => "dash",
                crate::filter::AlternativeCastKind::Flashback => "flashback",
                crate::filter::AlternativeCastKind::JumpStart => "jump-start",
                crate::filter::AlternativeCastKind::Escape => "escape",
                crate::filter::AlternativeCastKind::Madness => "madness",
                crate::filter::AlternativeCastKind::Miracle => "miracle",
                crate::filter::AlternativeCastKind::Suspend => "suspend",
            }
        }

        if may_cast_matching.zone == Zone::Hand
            && may_cast_matching.zone_owner != may_cast_matching.player
            && may_cast_matching.filter
                == crate::target::ObjectFilter::nonland().in_zone(Zone::Hand)
            && matches!(
                may_cast_matching.payment,
                ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost
            )
        {
            return format!(
                "{player} may cast a spell from among those cards without paying its mana cost"
            );
        }

        let mut spell_text = if may_cast_matching.filter
            == crate::target::ObjectFilter::default()
                .commander()
                .owned_by(crate::target::PlayerFilter::You)
        {
            "your commander".to_string()
        } else {
            describe_cast_spell_filter(
                &may_cast_matching.filter,
                CastSpellFilterContext::EnclosingPermission,
            )
        };
        if spell_text == "spell" {
            spell_text = "a spell".to_string();
        } else if !spell_text.starts_with("a ")
            && !spell_text.starts_with("an ")
            && !spell_text.starts_with("the ")
            && !spell_text.starts_with("your ")
        {
            spell_text = with_indefinite_article(&spell_text);
        }
        let zone_text = match may_cast_matching.zone {
            Zone::Hand => {
                let owner = if may_cast_matching.zone_owner == may_cast_matching.player {
                    &may_cast_matching.player
                } else {
                    &may_cast_matching.zone_owner
                };
                format!("from {} hand", describe_possessive_player_filter(owner))
            }
            Zone::Graveyard => {
                if may_cast_matching.filter.owner == Some(crate::filter::PlayerFilter::You) {
                    "from your graveyard".to_string()
                } else {
                    "from a graveyard".to_string()
                }
            }
            Zone::Library => "from a library".to_string(),
            Zone::Exile => "from exile".to_string(),
            Zone::Battlefield => "from the battlefield".to_string(),
            Zone::Stack => "from the stack".to_string(),
            Zone::Command => "from the command zone".to_string(),
            Zone::Ante => "from ante".to_string(),
            Zone::OutsideGame => "from outside the game".to_string(),
        };
        match may_cast_matching.payment {
            ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost => {
                return format!(
                    "{player} may cast {spell_text} {zone_text} without paying its mana cost"
                );
            }
            ironsmith_core::MayCastMatchingSpellPayment::AlternativeCost(kind) => {
                return format!(
                    "{player} may cast {spell_text} {zone_text}. If you do, pay its {} cost rather than its mana cost",
                    alternative_cast_kind_text(kind)
                );
            }
        }
    }
    if let Some(grant_next_spell_cost_reduction) =
        effect.downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()
    {
        let player_text = describe_player_filter(&grant_next_spell_cost_reduction.player);
        let spell_text = describe_cast_limit_spell_filter(&grant_next_spell_cost_reduction.filter);
        let player_suffix = format!(" cast by {player_text}");
        let cast_by_text = grant_next_spell_cost_reduction
            .filter
            .cast_by
            .as_ref()
            .map(describe_player_filter);
        let cast_by_suffix = cast_by_text.as_ref().map(|text| format!(" cast by {text}"));
        let spell_text = spell_text
            .strip_suffix(player_suffix.as_str())
            .or_else(|| {
                cast_by_suffix
                    .as_ref()
                    .and_then(|suffix| spell_text.strip_suffix(suffix.as_str()))
            })
            .unwrap_or(spell_text.as_str());
        if grant_next_spell_cost_reduction.applies_to_all_matching_this_turn {
            let duration_text = match grant_next_spell_cost_reduction.duration {
                Until::EndOfTurn => "this turn".to_string(),
                _ => describe_until(&grant_next_spell_cost_reduction.duration),
            };
            let (reduction, where_suffix) = grant_next_spell_cost_reduction
                .generic_reduction
                .as_ref()
                .map(|value| match value {
                    Value::Fixed(amount) => (format!("{{{amount}}}"), String::new()),
                    Value::X => ("{X}".to_string(), String::new()),
                    _ => (
                        "{X}".to_string(),
                        format!(", where X is {}", describe_value(value)),
                    ),
                })
                .unwrap_or_else(|| {
                    (
                        grant_next_spell_cost_reduction.reduction.to_oracle(),
                        String::new(),
                    )
                });
            let plural_spell_text = if grant_next_spell_cost_reduction.filter.zone
                == Some(Zone::Exile)
                && grant_next_spell_cost_reduction
                    .filter
                    .cast_by
                    .as_ref()
                    .is_some_and(|cast_by| cast_by == &grant_next_spell_cost_reduction.player)
                && grant_next_spell_cost_reduction.filter.card_types.is_empty()
            {
                format!("spells {player_text} cast from exile")
            } else {
                pluralize_cast_spell_description(spell_text)
            };
            // The exile-cast-by-self surface ("spells you cast from exile") already names
            // the caster, so don't re-append it (which produced "... you casts cost ...").
            let cast_by_already_in_plural = grant_next_spell_cost_reduction.filter.zone
                == Some(Zone::Exile)
                && grant_next_spell_cost_reduction
                    .filter
                    .cast_by
                    .as_ref()
                    .is_some_and(|cast_by| cast_by == &grant_next_spell_cost_reduction.player)
                && grant_next_spell_cost_reduction.filter.card_types.is_empty();
            if let Some(cast_by) = grant_next_spell_cost_reduction.filter.cast_by.as_ref()
                && !cast_by_already_in_plural
            {
                let caster_text = if matches!(cast_by, PlayerFilter::Target(_)) {
                    "that player".to_string()
                } else {
                    describe_player_filter(cast_by)
                };
                let plural_spell_text =
                    if !grant_next_spell_cost_reduction.filter.card_types.is_empty() {
                        let card_type_words = grant_next_spell_cost_reduction
                            .filter
                            .card_types
                            .iter()
                            .map(|card_type| describe_card_type_word_local(*card_type).to_string())
                            .collect::<Vec<_>>();
                        format!("{} spells", join_with_and(&card_type_words))
                    } else {
                        plural_spell_text
                    };
                return format!(
                    "{} {} casts cost {} less to cast {}{}",
                    plural_spell_text, caster_text, reduction, duration_text, where_suffix,
                );
            }
            if grant_next_spell_cost_reduction.filter.cast_by.is_none()
                && grant_next_spell_cost_reduction.filter.zone.is_none()
            {
                return format!(
                    "{} cost {} less to cast {}{}",
                    plural_spell_text, reduction, duration_text, where_suffix,
                );
            }
            return format!(
                "{} {} cost {} less to cast{}",
                plural_spell_text, duration_text, reduction, where_suffix,
            );
        }
        let duration_text = match grant_next_spell_cost_reduction.duration {
            Until::EndOfTurn => "this turn".to_string(),
            _ => describe_until(&grant_next_spell_cost_reduction.duration),
        };
        let (reduction, where_suffix) = grant_next_spell_cost_reduction
            .generic_reduction
            .as_ref()
            .map(|value| {
                let resolution_suffix = if value.has_surface_hint(
                    ironsmith_core::ValueSurfaceHint::AsThisAbilityResolves,
                ) {
                    " as this ability resolves"
                } else {
                    ""
                };
                (
                    "{X}".to_string(),
                    format!(
                        ", where X is {}{resolution_suffix}",
                        describe_value(value)
                    ),
                )
            })
            .unwrap_or_else(|| {
                (
                    grant_next_spell_cost_reduction.reduction.to_oracle(),
                    String::new(),
                )
            });
        return format!(
            "The next {} {} cast {} costs {} less to cast{}",
            spell_text,
            player_text,
            duration_text,
            reduction,
            where_suffix,
        );
    }
    if let Some(grant_next_spell_ability) =
        effect.downcast_ref::<crate::effects::GrantNextSpellAbilityEffect>()
    {
        let player_text = describe_player_filter(&grant_next_spell_ability.player);
        let mut spell_text = describe_cast_limit_spell_filter(&grant_next_spell_ability.filter);
        if grant_next_spell_ability.filter.zone == Some(Zone::Hand)
            && grant_next_spell_ability
                .filter
                .cast_by
                .as_ref()
                .is_some_and(|cast_by| cast_by == &grant_next_spell_ability.player)
            && grant_next_spell_ability.filter.card_types.len() == 2
            && grant_next_spell_ability
                .filter
                .card_types
                .contains(&CardType::Instant)
            && grant_next_spell_ability
                .filter
                .card_types
                .contains(&CardType::Sorcery)
        {
            spell_text = "an instant or sorcery spell from your hand".to_string();
        }
        let player_suffix = format!(" cast by {player_text}");
        let spell_text = spell_text
            .strip_suffix(player_suffix.as_str())
            .unwrap_or(spell_text.as_str());
        let granted_text = describe_inline_ability(&grant_next_spell_ability.ability);
        if spell_text.contains("from your hand") && player_text == "you" {
            return format!(
                "When you next cast {} this turn, it gains {}",
                spell_text,
                lowercase_first(&granted_text),
            );
        }
        return format!(
            "The next {} {} cast this turn has {}",
            spell_text,
            player_text,
            lowercase_first(&granted_text),
        );
    }
    if effect
        .downcast_ref::<crate::effects::MeleeEffect>()
        .is_some()
    {
        return "This creature gets +1/+1 until end of turn for each opponent you attacked this combat".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::player::MayCastForMiracleCostEffect>()
        .is_some()
    {
        return "You may cast it for its miracle cost".to_string();
    }
    if let Some(move_counters) = effect.downcast_ref::<crate::effects::MoveCountersEffect>() {
        return format!(
            "Move {} from {} to {}",
            describe_put_counter_phrase(&move_counters.count, move_counters.counter_type),
            describe_choose_spec(&move_counters.from),
            describe_choose_spec(&move_counters.to)
        );
    }
    if let Some(move_counter) = effect.downcast_ref::<crate::effects::MoveOneCounterEffect>() {
        let mut to_text = describe_choose_spec(&move_counter.to);
        if let Some(rest) = to_text.strip_prefix("another target ") {
            to_text = format!("a second target {rest}");
        }
        return format!(
            "Move a counter from {} onto {}",
            describe_choose_spec(&move_counter.from),
            to_text
        );
    }
    if effect
        .downcast_ref::<crate::effects::NinjutsuCostEffect>()
        .is_some()
    {
        return "Return an unblocked attacker you control to its owner's hand".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::SneakCostEffect>()
        .is_some()
    {
        return "Return an unblocked attacker you control to its owner's hand".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::NinjutsuEffect>()
        .is_some()
    {
        return "Put this card onto the battlefield tapped and attacking".to_string();
    }
    if let Some(remove_from_combat) =
        effect.downcast_ref::<crate::effects::RemoveFromCombatEffect>()
    {
        return format!(
            "Remove {} from combat",
            describe_choose_spec(&remove_from_combat.spec)
        );
    }
    if let Some(renown) = effect.downcast_ref::<crate::effects::RenownEffect>() {
        return format!(
            "If this creature isn't renowned, put {} +1/+1 counter{} on it and it becomes renowned",
            renown.amount,
            if renown.amount == 1 { "" } else { "s" }
        );
    }
    if let Some(return_from_graveyard_or_exile) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardOrExileToBattlefieldEffect>()
    {
        return format!(
            "Return this card from your graveyard or from exile to the battlefield{}",
            if return_from_graveyard_or_exile.tapped {
                " tapped"
            } else {
                ""
            }
        );
    }
    if let Some(sac_source_when_tagged_leaves) =
        effect.downcast_ref::<crate::effects::SacrificeSourceWhenTaggedLeavesEffect>()
    {
        return format!(
            "When tagged '{}' object leaves the battlefield, sacrifice this source",
            sac_source_when_tagged_leaves.tag.as_str()
        );
    }
    if let Some(saddle) = effect.downcast_ref::<crate::effects::SaddleCostEffect>() {
        return format!(
            "Tap any number of untapped creatures you control other than this permanent with total power {} or more",
            saddle.required_power
        );
    }
    if let Some(schedule_tagged_leaves) =
        effect.downcast_ref::<crate::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
    {
        return format!(
            "When tagged '{}' object leaves the battlefield, {}",
            schedule_tagged_leaves.tag.as_str(),
            describe_effect_list(&schedule_tagged_leaves.effects)
        );
    }
    if effect
        .downcast_ref::<crate::effects::SoulbondPairEffect>()
        .is_some()
    {
        return "Pair this creature with another unpaired creature you control".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::UnearthEffect>()
        .is_some()
    {
        return "Unearth".to_string();
    }
    if let Some(may_move) = effect.downcast_ref::<crate::effects::MayMoveToZoneEffect>() {
        let destination = match may_move.zone {
            crate::zone::Zone::Hand => "into your hand",
            crate::zone::Zone::Exile => "into exile",
            crate::zone::Zone::Graveyard => "into its owner's graveyard",
            crate::zone::Zone::Library => "into its owner's library",
            crate::zone::Zone::Battlefield => "onto the battlefield",
            crate::zone::Zone::Command => "into the command zone",
            crate::zone::Zone::Ante => "into ante",
            crate::zone::Zone::Stack => "onto the stack",
            crate::zone::Zone::OutsideGame => "outside the game",
        };
        return format!(
            "{} may put {} {}",
            describe_player_filter(&may_move.decider),
            describe_choose_spec(&may_move.target),
            destination
        );
    }
    if let Some(vote) = effect.downcast_ref::<crate::effects::VoteEffect>() {
        if let Some(compact) = describe_named_vote_per_vote_effects(vote) {
            return compact;
        }
        let choices = match &vote.choice {
            crate::effects::VoteChoice::NamedOptions(options) => join_with_or(
                &options
                    .iter()
                    .map(|option| option.name.to_ascii_lowercase())
                    .collect::<Vec<_>>(),
            ),
            crate::effects::VoteChoice::Objects { filter, count } => match (count.min, count.max) {
                (1, Some(1)) => filter.description(),
                (0, Some(1)) => {
                    format!("up to one {}", strip_leading_article(&filter.description()))
                }
                _ => format!(
                    "{} {}",
                    describe_choice_count(count),
                    pluralize_noun_phrase(&strip_leading_article(&filter.description()))
                ),
            },
            crate::effects::VoteChoice::Players {
                filter,
                exclude_voter,
            } => {
                let filter_already_excludes_voter = matches!(
                    filter,
                    PlayerFilter::Excluding { base, excluded }
                        if **base == PlayerFilter::Any
                            && **excluded == PlayerFilter::IteratedPlayer
                );
                if *exclude_voter
                    && (*filter == PlayerFilter::Any || filter_already_excludes_voter)
                {
                    "another player".to_string()
                } else {
                    strip_leading_article(&describe_player_filter(filter)).to_string()
                }
            }
        };
        let mut suffix = String::new();
        if vote.controller_extra_votes > 0 {
            suffix.push_str(&format!(
                "; you vote an additional {} time{}",
                vote.controller_extra_votes,
                if vote.controller_extra_votes == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if vote.controller_optional_extra_votes > 0 {
            suffix.push_str(&format!(
                "; you may vote an additional {} time{}",
                vote.controller_optional_extra_votes,
                if vote.controller_optional_extra_votes == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }
        if vote.secret {
            if matches!(vote.choice, crate::effects::VoteChoice::Players { .. }) {
                return format!(
                    "Secret council — Each player secretly votes for {}, then those votes are revealed{}",
                    choices, suffix
                );
            }
            return format!(
                "Each player secretly votes for {}, then those votes are revealed{}",
                choices, suffix
            );
        }
        let starting = if vote.starting_with_controller {
            "Starting with you, "
        } else {
            ""
        };
        return format!("{starting}each player votes for {}{}", choices, suffix);
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
        let repeated = describe_effect_list(&repeat.effects);
        let repeated = repeated.trim();
        if repeated.is_empty() {
            return String::new();
        }
        let repeated = lowercase_first(repeated.trim_end_matches('.'));
        if let Value::VoteCount(option) = &repeat.count {
            return format!(
                "For each {} vote, {}",
                option.to_ascii_lowercase(),
                repeated
            );
        }
        if let Value::PlayerVoteCount(filter) = &repeat.count {
            let player = match filter {
                PlayerFilter::IteratedPlayer => "that player".to_string(),
                PlayerFilter::You => "you".to_string(),
                _ => strip_leading_article(&describe_player_filter(filter)).to_string(),
            };
            return format!("For each vote {player} received, {repeated}");
        }
        if repeat
            .count
            .has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay)
            && let Value::DividedRoundedDown(value, divisor) = repeat.count.unhinted()
            && matches!(value.as_ref(), Value::X)
            && *divisor > 0
        {
            let group = small_number_word(*divisor as u32).unwrap_or_else(|| divisor.to_string());
            return format!("For each {group} counters removed this way, {repeated}");
        }
        let affected_card_basis = if repeat
            .count
            .has_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay)
        {
            describe_create_for_each_count(&repeat.count)
                .or_else(|| Some("card discarded this way".to_string()))
        } else if repeat
            .count
            .has_surface_hint(ValueSurfaceHint::CardsRevealedThisWay)
        {
            describe_create_for_each_count(&repeat.count)
                .or_else(|| Some("card revealed this way".to_string()))
        } else {
            None
        };
        if let Some(basis) = affected_card_basis {
            if repeat
                .count
                .has_surface_hint(ValueSurfaceHint::CardsRevealedThisWay)
                && repeat.count.has_surface_hint(ValueSurfaceHint::ForEach)
            {
                return format!("For each {basis}, {repeated}");
            }
            return format!("{repeated} for each {basis}");
        }
        let mana_symbol_group = if repeat.count.has_surface_hint(ValueSurfaceHint::ForEach) {
            match repeat.count.unhinted() {
                Value::ManaSymbolSpentToCastThisSpell { symbol, reference } => {
                    Some((symbol, reference, 1usize))
                }
                Value::DividedRoundedDown(value, divisor) if *divisor > 0 => {
                    match value.as_ref() {
                        Value::ManaSymbolSpentToCastThisSpell { symbol, reference } => {
                            Some((symbol, reference, *divisor as usize))
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        } else {
            None
        };
        if let Some((symbol, reference, group_size)) = mana_symbol_group {
            let group = describe_mana_symbol(*symbol).repeat(group_size);
            return format!(
                "For each {group} spent to cast {}, {repeated}",
                reference.text()
            );
        }
        if repeat.count.has_surface_hint(ValueSurfaceHint::ForEach)
            && let Some((multiplier, basis)) =
                crate::compiled_text::describe_counter_for_each_basis(&repeat.count)
        {
            if multiplier == 1 {
                return format!("For each {basis}, {repeated}");
            }
            return format!("For each {basis}, {repeated} {multiplier} times");
        }
        if let Some(basis) = describe_turn_history_for_each_basis(&repeat.count) {
            return format!("For each {basis}, {repeated}");
        }
        if repeat.count.has_surface_hint(ValueSurfaceHint::ForEach)
            && let Some(basis) = describe_create_for_each_count(&repeat.count)
        {
            return format!("{repeated} for each {basis}");
        }
        if repeat
            .count
            .has_surface_hint(ValueSurfaceHint::RepeatThisProcessOnce)
        {
            if let Some(repeated) = describe_repeated_choose_copy_with_cleanup(repeat) {
                return format!("{repeated}. Repeat this process once");
            }
            let repeated = repeated
                .strip_prefix("you ")
                .map(normalize_you_verb_phrase)
                .unwrap_or_else(|| repeated.to_string());
            return format!(
                "{}. Repeat this process once",
                capitalize_first(repeated.trim_end_matches('.'))
            );
        }
        return match repeat.count.unhinted() {
            Value::Fixed(1) => repeated,
            Value::Fixed(2) => format!("{repeated} twice"),
            _ => format!("{repeated} {} times", describe_value(&repeat.count)),
        };
    }
    if let Some(keyword) = effect.downcast_ref::<crate::effects::EmitKeywordActionEffect>() {
        if keyword.action == crate::events::KeywordActionKind::Forage && keyword.amount == 1 {
            return "forage".to_string();
        }
        if keyword.action == crate::events::KeywordActionKind::Planeswalk && keyword.amount == 1 {
            return "planeswalk".to_string();
        }
        if keyword.action == crate::events::KeywordActionKind::AssembleContraption
            && keyword.amount == 1
        {
            return "assemble a Contraption".to_string();
        }
        if keyword.action == crate::events::KeywordActionKind::ChaosEnsues && keyword.amount == 1 {
            return "chaos ensues".to_string();
        }
        // Runtime keyword-action events are instrumentation; they should not leak
        // into oracle-like rendered rules text.
        return String::new();
    }
    if effect
        .downcast_ref::<crate::effects::EmitGiftGivenEffect>()
        .is_some()
    {
        // Gift trigger instrumentation is runtime-only and should stay hidden.
        return String::new();
    }
    "Unsupported effect".to_string()
}
