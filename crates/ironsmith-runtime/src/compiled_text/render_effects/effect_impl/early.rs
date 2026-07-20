{
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>()
        && let Some(text) = describe_untap_restriction_oracle(cant)
    {
        return text;
    }
    if effect
        .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
        .is_some_and(|tag| is_implicit_reference_tag(tag.tag.as_str()))
    {
        return String::new();
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        if let Some(compact) = describe_coordinated_sequence(sequence) {
            return compact;
        }
        if let Some(compact) = describe_reveal_until_sequence(sequence) {
            return compact;
        }
        if let Some(compact) = describe_search_sequence(sequence) {
            return compact;
        }
        return describe_effect_list(&sequence.effects);
    }
    if let Some(create_emblem) = effect.downcast_ref::<crate::effects::CreateEmblemEffect>()
        && let Some(emblem_text) = stored_emblem_rules_text(&create_emblem.emblem)
    {
        return format!("You get an emblem with \"{emblem_text}\"");
    }
    if let Some(restricted) = effect.downcast_ref::<crate::effects::ManaRestrictedEffect>() {
        let mut parts = Vec::new();
        let effect_text = describe_effect_list(&restricted.effects);
        if !effect_text.trim().is_empty() {
            parts.push(effect_text);
        }
        parts.extend(
            restricted
                .restrictions
                .iter()
                .filter_map(|restriction| describe_mana_usage_restriction(restriction, None)),
        );
        return cleanup_decompiled_text(&parts.join(". "));
    }
    if let Some(retained) = effect.downcast_ref::<crate::effects::ManaRetainedEffect>() {
        let effect_text = describe_effect_list(&retained.effects);
        let duration = match retained.duration {
            ironsmith_core::ManaRetentionDuration::EndOfCombat => "Until end of combat",
            ironsmith_core::ManaRetentionDuration::EndOfTurn => "Until end of turn",
        };
        return cleanup_decompiled_text(&format!(
            "{effect_text}. {duration}, you don't lose this mana as steps and phases end"
        ));
    }
    if effect
        .downcast_ref::<crate::effects::LearnEffect>()
        .is_some()
    {
        return "Learn".to_string();
    }
    if let Some(double_counters) = effect.downcast_ref::<crate::effects::DoubleCountersEffect>() {
        let counter_text = match double_counters.counter_type {
            Some(counter_type) => format!("{} counters", describe_counter_type(counter_type)),
            None => "each kind of counter".to_string(),
        };
        if let ChooseSpec::All(filter) = double_counters.target.base() {
            let filter_description = filter.description();
            let filter_text = strip_indefinite_article(&filter_description);
            let has_tagged_iterated_reference =
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                });
            if has_tagged_iterated_reference {
                return format!(
                    "Double the number of {counter_text} on each of those {}",
                    pluralize_noun_phrase(filter_text)
                );
            }
            return format!("Double the number of {counter_text} on each {filter_text}");
        }
        if matches!(
            double_counters.target.base(),
            ChooseSpec::Player(_)
                | ChooseSpec::SpecificPlayer(_)
                | ChooseSpec::EachPlayer(_)
                | ChooseSpec::SourceController
                | ChooseSpec::SourceOwner
        ) {
            let player_text = describe_choose_spec(&double_counters.target);
            let verb = if player_text == "you" { "have" } else { "has" };
            return format!("Double the number of {counter_text} {player_text} {verb}");
        }
        return format!(
            "Double the number of {counter_text} on {}",
            describe_choose_spec(&double_counters.target)
        );
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        // Some plural copy statements lower through a ForEach wrapper even
        // though the contained continuous effect already targets the same
        // complete filter. Rendering both levels produces the redundant
        // "For each ..., each ..." surface.
        if let [inner] = for_each.effects.as_slice() {
            let inner = unwrap_basic_tag_wrappers(inner);
            if let Some(remove) =
                inner.downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
                && matches!(remove.target.unhinted(), ChooseSpec::Iterated)
                && matches!(
                    &remove.max_count,
                    Value::CountersOn(spec, None)
                        if matches!(spec.unhinted(), ChooseSpec::Iterated)
                )
            {
                let mut subject_filter = for_each.filter.clone();
                subject_filter.zone = None;
                let subject = describe_count_filter_value_subject(&subject_filter);
                return format!("Remove all counters from all {subject}");
            }
            if let Some(apply) = inner.downcast_ref::<crate::effects::ApplyContinuousEffect>()
                && apply.runtime_modifications.iter().any(|modification| {
                    matches!(
                        modification,
                        crate::effects::continuous::RuntimeModification::CopyOf { .. }
                    )
                })
                && matches!(
                    &apply.target,
                    crate::continuous::EffectTarget::Filter(filter) if filter == &for_each.filter
                )
            {
                return describe_effect(inner);
            }
        }
        if let Some(compact) = describe_for_each_double_stat(for_each) {
            return compact;
        }
        if let Some(compact) = describe_for_each_iterated_damage(for_each, None) {
            return compact;
        }
        if let Some(compact) =
            describe_divided_evenly_x_damage_to_target_opponent_creatures(for_each)
        {
            return compact;
        }
        if let Some(compact) = describe_for_each_double_counters(for_each) {
            return compact;
        }
        if let Some(compact) = describe_for_each_put_counters_then_untap(for_each) {
            return compact;
        }
        if let Some(compact) = describe_for_each_created_token_attachment(for_each) {
            return compact;
        }
        if let Some(compact) = describe_for_each_devotion_damage(for_each) {
            return compact;
        }
        if let Some(compact) = describe_for_each_sacrifice_by_controller(for_each) {
            return compact;
        }
        if let Some(compact) = describe_for_each_prevent_combat_damage_unless_pays(for_each) {
            return compact;
        }
        if let Some(compact) = describe_source_and_blocked_creatures_top_library_shuffle(for_each) {
            return compact;
        }
        if let Some(compact) = describe_optional_basic_land_search_effects(&for_each.effects) {
            return compact;
        }
        if let [copy_effect] = for_each.effects.as_slice()
            && let Some(create_copy) = unwrap_tag_wrapped_effect(copy_effect)
                .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
            && (create_copy.exile_at_end_of_combat
                || create_copy.sacrifice_at_next_end_step
                || create_copy.exile_at_next_end_step)
        {
            let mut creation_only = create_copy.clone();
            creation_only.exile_at_end_of_combat = false;
            creation_only.sacrifice_at_next_end_step = false;
            creation_only.exile_at_next_end_step = false;
            let creation = lowercase_first(&describe_effect(&Effect::new(creation_only)));
            let filter_text = describe_for_each_filter(&for_each.filter)
                .replace("modified attacking creature", "attacking modified creature");
            let mut text = format!("For each {filter_text}, {creation}");
            if create_copy.exile_at_end_of_combat {
                text.push_str(". Exile those tokens at end of combat");
            }
            if create_copy.sacrifice_at_next_end_step {
                let timing =
                    describe_next_end_step_cleanup_timing(&create_copy.next_end_step_player);
                text.push_str(&format!(
                    ". Sacrifice those tokens at the beginning of {timing}"
                ));
            }
            if create_copy.exile_at_next_end_step {
                let timing =
                    describe_next_end_step_cleanup_timing(&create_copy.next_end_step_player);
                text.push_str(&format!(
                    ". Exile those tokens at the beginning of {timing}"
                ));
            }
            return text;
        }
        if let [create_effect] = for_each.effects.as_slice()
            && let Some(create) = unwrap_tag_wrapped_effect(create_effect)
                .downcast_ref::<crate::effects::CreateTokenEffect>()
            && (create.exile_at_end_of_combat
                || create.sacrifice_at_end_of_combat
                || create.sacrifice_at_next_end_step
                || create.exile_at_next_end_step)
        {
            let mut creation_only = create.clone();
            creation_only.exile_at_end_of_combat = false;
            creation_only.sacrifice_at_end_of_combat = false;
            creation_only.sacrifice_at_next_end_step = false;
            creation_only.exile_at_next_end_step = false;
            let creation = lowercase_first(&describe_effect(&Effect::new(creation_only)));
            let filter_text = describe_for_each_filter(&for_each.filter);
            let mut text = format!("For each {filter_text}, {creation}");
            if create.exile_at_end_of_combat {
                text.push_str(". Exile those tokens at end of combat");
            }
            if create.sacrifice_at_end_of_combat {
                text.push_str(". Sacrifice those tokens at end of combat");
            }
            if create.sacrifice_at_next_end_step {
                let timing =
                    describe_next_end_step_cleanup_timing(&create.next_end_step_player);
                text.push_str(&format!(
                    ". Sacrifice those tokens at the beginning of {timing}"
                ));
            }
            if create.exile_at_next_end_step {
                let timing =
                    describe_next_end_step_cleanup_timing(&create.next_end_step_player);
                text.push_str(&format!(
                    ". Exile those tokens at the beginning of {timing}"
                ));
            }
            return text;
        }
        if for_each.effects.len() == 1
            && let Some(put) =
                for_each.effects[0].downcast_ref::<crate::effects::PutCountersEffect>()
            && matches!(put.target, ChooseSpec::Iterated)
            && put.target_count.is_none()
            && !put.distributed
        {
            // A set defined only by a "<verbed> this way" tag back-references
            // the objects the previous sentence just acted on; oracle uses the
            // partitive pronoun ("Put a stun counter on each of them").
            let filter_text = if this_way_back_reference_filter(&for_each.filter) {
                "of them".to_string()
            } else {
                describe_for_each_filter(&for_each.filter)
            };
            if let Some((counter_text, where_x)) =
                describe_counter_count_with_where_x(&put.amount, put.counter_type)
            {
                return format!(
                    "Put {counter_text} on each {filter_text}, where X is {where_x}"
                );
            }
            return format!(
                "Put {} on each {}",
                describe_put_counter_phrase(&put.amount, put.counter_type),
                filter_text
            );
        }
        if let Some(subject) = describe_for_each_tagged_this_way_subject(&for_each.filter) {
            let effect_text = describe_effect_list(&for_each.effects);
            if subject.ends_with(" tapped this way")
                && let Some(action) = effect_text.strip_prefix("that object ")
                && let Some(iterated_subject) = subject.strip_prefix("For each ")
            {
                return format!("Each {iterated_subject} {action}");
            }
            if subject == "For each land destroyed this way"
                && effect_text.contains("basic land card")
                && effect_text.contains("For each tagged 'searched' object")
                && effect_text.contains("put them onto the battlefield")
            {
                return "For each land destroyed this way, its controller may search their library for a basic land card and put it onto the battlefield".to_string();
            }
            return format!("{subject}, {effect_text}");
        }
        if for_each.effects.len() == 1
            && let Some(gain_control) =
                for_each.effects[0].downcast_ref::<crate::effects::GainControlEffect>()
            && matches!(gain_control.target, ChooseSpec::Iterated)
        {
            let filter_text = describe_for_each_filter(&for_each.filter);
            return format!(
                "Gain control of each {} {}",
                filter_text,
                describe_until(&gain_control.duration)
            );
        }
        if for_each.effects.len() == 1 {
            let deal = if let Some(deal) =
                for_each.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()
            {
                Some(deal)
            } else if let Some(tagged) =
                for_each.effects[0].downcast_ref::<crate::effects::TaggedEffect>()
            {
                tagged
                    .effect
                    .downcast_ref::<crate::effects::DealDamageEffect>()
            } else {
                None
            };
            if let Some(deal) = deal
                && matches!(deal.target, ChooseSpec::Iterated)
            {
                let effect_text = describe_effect_list(&for_each.effects);
                let replacements = [
                    " to that object",
                    " to that creature",
                    " to that permanent",
                    " to that artifact",
                    " to that enchantment",
                    " to that land",
                    " to that spell",
                    " to that card",
                ];
                if let Some(suffix) = replacements
                    .iter()
                    .find(|suffix| effect_text.ends_with(**suffix))
                {
                    let subject_text = effect_text.trim_end_matches(*suffix);
                    let filter_text = strip_battlefield_zone_suffix(describe_for_each_filter(
                        &for_each.filter,
                    ));
                    return format!("{subject_text} to each {filter_text}");
                }
            }
        }
        // Battlefield is the implicit zone for an iterated permanent noun
        // ("For each land, ..."); the explicit suffix belongs to count
        // surfaces ("for each creature on the battlefield").
        let filter_text =
            strip_battlefield_zone_suffix(describe_for_each_filter(&for_each.filter));
        // Inside an explicit per-object loop, the generic object reference is
        // unambiguously the current iterand. Keep this scoped to ForEachObject:
        // outside a loop, "that object" can be the only clear antecedent.
        let effect_text = describe_effect_list(&for_each.effects)
            .replace("That object's", "Its")
            .replace("that object's", "its")
            .replace("That object", "It")
            .replace("that object", "it");
        return format!(
            "For each {}, {}",
            filter_text,
            lowercase_first(&effect_text)
        );
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        if for_each_tagged.tag.as_str().starts_with("searched")
            && for_each_tagged.effects.len() == 1
            && let Some(put) = for_each_tagged.effects[0]
                .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
            && matches!(&put.target, ChooseSpec::Tagged(tag) if tag == &for_each_tagged.tag)
            && !put.tapped
        {
            return "put it onto the battlefield".to_string();
        }
        if let Some(compact) = describe_for_each_tagged_optional_basic_land_search(for_each_tagged)
        {
            return compact;
        }
        if let Some(compact) = describe_for_each_tagged_shuffle_into_owner_library(for_each_tagged)
        {
            return compact;
        }
        if let Some(compact) = describe_for_each_tagged_created_token_attachment(for_each_tagged) {
            return compact;
        }
        if for_each_tagged.tag.as_str().starts_with("targeted_")
            && for_each_tagged.effects.len() == 1
            && let Some(villainous) =
                for_each_tagged.effects[0].downcast_ref::<crate::effects::VillainousChoiceEffect>()
        {
            return format!(
                "For each of them, {}",
                describe_villainous_choice(villainous)
            );
        }
        let tag = for_each_tagged.tag.as_str();
        let subject = if tag.starts_with("destroyed_")
            || crate::cards::is_sentence_helper_tag(tag, "destroyed")
        {
            "For each object destroyed this way".to_string()
        } else if tag.starts_with("exiled_") || crate::cards::is_sentence_helper_tag(tag, "exiled")
        {
            "For each object exiled this way".to_string()
        } else if tag.starts_with("revealed_")
            || tag.contains("revealed_this_way")
            || crate::cards::is_sentence_helper_tag(tag, "revealed")
        {
            "For each card revealed this way".to_string()
        } else if tag.starts_with("looked_")
            || tag.contains("looked_this_way")
            || crate::cards::is_sentence_helper_tag(tag, "looked")
        {
            "For each card looked at this way".to_string()
        } else if tag.starts_with("chosen_")
            || tag.contains("chosen_this_way")
            || crate::cards::is_sentence_helper_tag(tag, "chosen")
        {
            "For each card chosen this way".to_string()
        } else if tag.starts_with("searched_")
            || tag.contains("searched_this_way")
            || crate::cards::is_sentence_helper_tag(tag, "searched")
        {
            "For each card searched for this way".to_string()
        } else if tag.starts_with("sacrificed_")
            || crate::cards::is_sentence_helper_tag(tag, "sacrificed")
        {
            "For each object sacrificed this way".to_string()
        } else if tag == crate::tag::SOURCE_EXILED_TAG
            || tag.starts_with("exiled_")
            || crate::cards::is_sentence_helper_tag(tag, "exiled")
        {
            "For each card exiled this way".to_string()
        } else if let Some(action) = this_way_action_from_tag(&for_each_tagged.tag) {
            let noun = if matches!(action, "milled" | "revealed" | "discarded" | "exiled") {
                "card"
            } else {
                "object"
            };
            format!("For each {noun} {action} this way")
        } else {
            // Tags are internal referents, never an oracle-text surface. When
            // no action provenance is available, retain the collection
            // relationship without exposing the implementation marker.
            "For each of those objects".to_string()
        };
        return format!(
            "{subject}, {}",
            describe_effect_list(&for_each_tagged.effects)
        );
    }
    if effect
        .downcast_ref::<crate::effects::AscendEffect>()
        .is_some()
    {
        return "Ascend".to_string();
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if let [inner] = for_players.effects.as_slice()
            && let Some(create_emblem) = inner.downcast_ref::<crate::effects::CreateEmblemEffect>()
        {
            let subject = match &for_players.filter {
                PlayerFilter::Target(inner) if **inner == PlayerFilter::Any => {
                    "Target player".to_string()
                }
                filter => capitalize_first(&describe_player_filter(filter)),
            };
            if let Some(emblem_text) = stored_emblem_rules_text(&create_emblem.emblem) {
                return format!("{subject} gets an emblem with \"{emblem_text}\"");
            }
            return format!(
                "{subject} gets an emblem named {}",
                create_emblem.emblem.name
            );
        }
        if let Some(compact) = describe_each_player_return_from_graveyard_to_hand(for_players) {
            return compact;
        }
        if let Some(compact) =
            describe_each_player_choose_type_return_from_graveyard_to_hand(for_players)
        {
            return compact;
        }
        if let Some(compact) =
            describe_each_player_may_discard_hand_draw_commander_value(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_each_player_may_discard_hand_draw(for_players) {
            return compact;
        }
        if let Some(compact) = describe_each_player_may_discard_card_then_draw(for_players) {
            return compact;
        }
        if let Some(compact) = describe_each_player_shuffle_hand_then_draw(for_players) {
            return compact;
        }
        // Preserve the explicit searched-card partition before the generic
        // correlated-loop renderers flatten the per-player collection into
        // independent choose and move sentences.
        if let Some(compact) =
            describe_for_players_optional_search_battlefield_partition(for_players)
        {
            return compact;
        }
        // The reveal/lose-life/put-in-hand bundle has a dedicated surface;
        // check it before the generic correlated-loop renderer flattens it.
        if let Some(compact) =
            describe_for_players_reveal_top_mana_value_life_then_put_into_hand(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_for_players_correlated_result_loop(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_may_happened_sequence(for_players) {
            return compact;
        }
        if let Some(compact) = describe_sequential_any_player_may_action(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_may_search_library_then_shuffle(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_search_library_then_shuffle(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_nonland_put_counter(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_bend_or_break(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_shuffle_then_conditional_consult(for_players) {
            return compact;
        }
        if let Some(compact) =
            describe_for_players_shuffle_reveal_permanents_put_rest_bottom(for_players)
        {
            return compact;
        }
        if let Some(compact) =
            describe_for_players_reveal_top_mana_value_life_then_put_into_hand(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_types_then_sacrifice_rest(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_split_piles_then_choose_sacrifice(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_split_piles_then_choose_restriction(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_then_sacrifice(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_then_exile(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_then_untap_chosen(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_controls_no_lose_game(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_may_choose_then_move_to_battlefield(for_players)
        {
            return compact;
        }
        if let Some(compact) =
            describe_for_players_history_damage_and_controlled_damage(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_for_players_damage_and_controlled_damage(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_target_return_unless_draw(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_vote_received_repeat(for_players) {
            return compact;
        }
        if let Some(compact) =
            describe_each_player_return_all_from_their_graveyard_with_counters(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_each_player_return_all_from_their_graveyard(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_single_iterated_animation(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_coordinated_actions(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_iterated_action_sequence(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_simple_iterated_action(for_players) {
            return compact;
        }
        if for_players.filter == PlayerFilter::Any
            && for_players.effects.len() == 1
            && let Some(cant) = for_players.effects[0].downcast_ref::<crate::effects::CantEffect>()
            && matches!(
                cant.restriction,
                crate::effect::Restriction::CastMoreThanOneSpellEachTurn(PlayerFilter::Any, _)
            )
        {
            let restriction_text = describe_restriction(&cant.restriction);
            if let Some(rest) = restriction_text
                .strip_prefix("players can't")
                .or_else(|| restriction_text.strip_prefix("Players can't"))
            {
                return format!("Each player can't{rest}");
            }
        }
        if for_players.filter == PlayerFilter::Any
            && for_players.effects.len() == 1
            && let Some(return_to_battlefield) =
                for_players.effects[0]
                    .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
            && graveyard_owner_from_spec(&return_to_battlefield.target)
                == Some(Some(PlayerFilter::IteratedPlayer))
        {
            let mut target_text =
                describe_choose_spec_without_graveyard_zone(&return_to_battlefield.target);
            if target_text.starts_with("all ")
                && let Some(base) = target_text.strip_suffix(" card")
            {
                target_text = format!("{base} cards");
            }
            return format!(
                "Each player returns {target_text} from their graveyard to the battlefield{}",
                if return_to_battlefield.tapped {
                    " tapped"
                } else {
                    ""
                }
            );
        }
        if for_players.effects.len() == 1
            && let Some(conditional) =
                for_players.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
            && conditional.if_false.is_empty()
            && let Some(relative) = describe_player_relative_condition(&conditional.condition)
        {
            let player_filter_text = describe_for_each_player_filter(&for_players.filter);
            let each_player = strip_leading_article(&player_filter_text);
            if conditional.if_true.len() == 1
                && let Some(draw) =
                    conditional.if_true[0].downcast_ref::<crate::effects::DrawCardsEffect>()
                && draw.player == PlayerFilter::You
                && draw.count == Value::Fixed(1)
            {
                return format!("you draw a card for each {each_player} who {relative}");
            }
            if conditional.if_true.len() == 1
                && let Some(create) =
                    conditional.if_true[0].downcast_ref::<crate::effects::CreateTokenEffect>()
                && create.controller == PlayerFilter::You
                && create.count == Value::Fixed(1)
            {
                let token_text = describe_effect(&conditional.if_true[0]);
                return format!("{token_text} for each {each_player} who {relative}");
            }
            if conditional.if_true.len() == 1
                && let Some(damage) =
                    conditional.if_true[0].downcast_ref::<crate::effects::DealDamageEffect>()
                && matches!(
                    damage.target,
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer)
                )
            {
                let amount_text = describe_value(&damage.amount);
                return format!("Deal {amount_text} damage to each {each_player} who {relative}");
            }
            let mut inner = describe_effect_list(&conditional.if_true);
            if let Some(rest) = inner.strip_prefix("that player ") {
                inner = rest.to_string();
            }
            if let Some(rest) = inner.strip_prefix("you ") {
                inner = rest.to_string();
            }
            inner = normalize_third_person_verb_phrase(&inner);
            return format!("Each {each_player} who {relative} {inner}");
        }
        if for_players.effects.len() == 1
            && let Some(may) = for_players.effects[0].downcast_ref::<crate::effects::MayEffect>()
            && may.decider.is_none()
        {
            let player_filter_text = describe_for_each_player_filter(&for_players.filter);
            let each_player = strip_leading_article(&player_filter_text);
            if may.effects.len() == 1
                && let Some(create_copy) =
                    may.effects[0].downcast_ref::<crate::effects::CreateTokenCopyEffect>()
                && create_copy.controller == PlayerFilter::You
                && create_copy.enters_attacking
                && matches!(
                    create_copy.attack_target_mode,
                    Some(
                        crate::effects::CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                            PlayerFilter::IteratedPlayer
                        )
                    )
                )
            {
                let inner = normalize_you_verb_phrase(&describe_effect_list(&may.effects));
                return format!("For each {each_player}, you may {inner}");
            }
            let mut inner = describe_effect_list(&may.effects);
            if let Some(rest) = inner.strip_prefix("that player ") {
                inner = rest.to_string();
            }
            if let Some(rest) = inner.strip_prefix("you ") {
                inner = rest.to_string();
            }
            inner = normalize_you_verb_phrase(&inner);
            inner = lowercase_first(&inner);
            return format!("For each {each_player}, that player may {inner}");
        }
        let player_filter_text = describe_for_each_player_filter(&for_players.filter);
        let each_player = strip_leading_article(&player_filter_text);
        return format!(
            "For each {}, {}",
            each_player,
            describe_effect_list(&for_players.effects)
        );
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        let chooser = describe_player_filter(&choose.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let search_like = choose.is_search
            || (choose_primary_zone(choose) == Some(Zone::Library)
                && choose.tag.as_str().starts_with("searched"));
        let filter_text = choose.filter.description();
        let search_origin = if search_like {
            describe_search_origin_zones(choose)
        } else {
            None
        };
        let choice_text = if search_like
            && choose.search_mode == SearchSelectionMode::Exact
            && choose.count.min == 0
            && choose.count.max == Some(1)
        {
            describe_choose_selection(&crate::effects::ChooseObjectsEffect {
                count: ChoiceCount::exactly(1),
                ..choose.clone()
            })
        } else if choose.top_only {
            if let Some(exact) = choose_exact_count(choose) {
                if exact > 1 {
                    let count_text = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
                    format!(
                        "the top {count_text} {}",
                        pluralize_noun_phrase(&filter_text)
                    )
                } else {
                    format!("the top {filter_text}")
                }
            } else {
                format!("the top {filter_text}")
            }
        } else {
            describe_choose_selection(choose)
        };
        if search_like && let Some(search_origin) = search_origin {
            let pronoun = if choose.count.max == Some(1) {
                "it"
            } else {
                "them"
            };
            let reveal_clause = if choose.reveal {
                format!(", reveal {pronoun}")
            } else {
                String::new()
            };
            return format!(
                "{} {} {} for {}{}",
                chooser,
                player_verb(&chooser, "search", "searches"),
                search_origin,
                choice_text,
                reveal_clause
            );
        }
        let zone_location = |zone| match zone {
            Zone::Battlefield => ("on the battlefield", "battlefield"),
            Zone::Hand => ("in a hand", "hand"),
            Zone::Graveyard => ("in a graveyard", "graveyard"),
            Zone::Library => ("in a library", "library"),
            Zone::Stack => ("on the stack", "stack"),
            Zone::Exile => ("in exile", "exile"),
            Zone::Command => ("in the command zone", "command"),
            Zone::Ante => ("in ante", "ante"),
            Zone::OutsideGame => ("outside the game", "outside"),
        };
        let filter_lower = filter_text.to_ascii_lowercase();
        // Oracle leaves the battlefield implicit for permanent choices
        // regardless of a controller constraint ("Choose up to one
        // creature", not "... on the battlefield").
        let controlled_battlefield_choice = choose_primary_zone(choose) == Some(Zone::Battlefield)
            && choose.filter.zone == Some(Zone::Battlefield)
            && !choose.count.random;
        let suppress_location_suffix =
            revealed_keyword_choice_label(choose).is_some() || controlled_battlefield_choice;
        let location_suffix = if suppress_location_suffix {
            String::new()
        } else if let Some(zones) = choose_search_zones(choose)
            && zones.len() > 1
        {
            let parts = zones
                .iter()
                .filter_map(|zone| {
                    let (phrase, keyword) = zone_location(*zone);
                    (!filter_lower.contains(keyword)).then(|| phrase.to_string())
                })
                .collect::<Vec<_>>();
            if parts.is_empty() {
                String::new()
            } else {
                format!(" {}", join_with_or(&parts))
            }
        } else {
            let (zone_phrase, zone_keyword) = choose_primary_zone(choose)
                .map(zone_location)
                .unwrap_or(("in an unspecified zone", ""));
            // For graveyard chooses, check the actual selection text —
            // describe_choose_selection may have stripped the zone that
            // filter_text still mentions, and oracle spells it out. Hand and
            // library chooses keep the filter-based check: their downstream
            // reveal-compaction gates ("You choose ... from it") key on the
            // suffix-free surface.
            let includes_zone_already = if choose_primary_zone(choose) == Some(Zone::Graveyard) {
                let choice_lower = choice_text.to_ascii_lowercase();
                zone_keyword.is_empty()
                    || choice_lower.contains(zone_keyword)
                    || choice_lower.contains("from it")
                    || choice_lower.contains("from among")
                    || choice_lower.contains("of them")
                    || choice_lower.contains("in it")
            } else {
                zone_keyword.is_empty() || filter_lower.contains(zone_keyword)
            };
            if includes_zone_already {
                String::new()
            } else {
                format!(" {zone_phrase}")
            }
        };
        return format!(
            "{} {} {}{}",
            chooser,
            if search_like {
                "searches for"
            } else {
                choose_verb
            },
            choice_text,
            location_suffix
        );
    }
    if let Some(choose_name) = effect.downcast_ref::<crate::effects::ChooseCardNameEffect>() {
        let chooser = describe_player_filter(&choose_name.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let selection = describe_choose_card_name_selection(choose_name);
        return format!("{chooser} {choose_verb} {selection} name");
    }
    if let Some(choose_player) = effect.downcast_ref::<crate::effects::ChoosePlayerEffect>() {
        let chooser = describe_player_filter(&choose_player.chooser);
        let choose_verb = if choose_player.random {
            player_verb(&chooser, "chooses at random", "chooses at random")
        } else {
            player_verb(&chooser, "choose", "chooses")
        };
        let filtered = describe_player_filter(&choose_player.filter);
        return format!(
            "{chooser} {choose_verb} {}",
            with_indefinite_article(strip_leading_article(&filtered))
        );
    }
    if let Some(put_sticker) = effect.downcast_ref::<crate::effects::PutStickerEffect>() {
        return format!(
            "Put {} on {}",
            match put_sticker.action {
                crate::events::KeywordActionKind::Sticker => "a sticker",
                crate::events::KeywordActionKind::NameSticker => "a name sticker",
                crate::events::KeywordActionKind::ArtSticker => "an art sticker",
                crate::events::KeywordActionKind::AbilitySticker => "an ability sticker",
                crate::events::KeywordActionKind::PowerToughnessSticker => {
                    "a power and toughness sticker"
                }
                _ => "a sticker",
            },
            describe_choose_spec(&put_sticker.target)
        );
    }
    if let Some(choose_spell) =
        effect.downcast_ref::<crate::effects::ChooseSpellCastHistoryEffect>()
    {
        let chooser = describe_player_filter(&choose_spell.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let filter_text = choose_spell.filter.description();
        let cast_by = describe_player_filter(&choose_spell.cast_by);
        return format!(
            "{chooser} {choose_verb} one of {} cast this turn by {cast_by}",
            pluralize_noun_phrase(strip_leading_article(&filter_text))
        );
    }
    if let Some(choose_color) = effect.downcast_ref::<crate::effects::ChooseColorEffect>() {
        let chooser = describe_player_filter(&choose_color.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        return format!("{chooser} {choose_verb} a color");
    }
    if let Some(choose_land_type) = effect.downcast_ref::<crate::effects::ChooseLandTypeEffect>() {
        let chooser = describe_player_filter(&choose_land_type.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let kind = if choose_land_type.exclude_basic {
            "a nonbasic land type"
        } else {
            "a land type"
        };
        return format!("{chooser} {choose_verb} {kind}");
    }
    if let Some(choose_card_type) = effect.downcast_ref::<crate::effects::ChooseCardTypeEffect>() {
        let chooser = describe_player_filter(&choose_card_type.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let permanent_type_options = [
            crate::types::CardType::Artifact,
            crate::types::CardType::Creature,
            crate::types::CardType::Enchantment,
            crate::types::CardType::Land,
            crate::types::CardType::Planeswalker,
            crate::types::CardType::Battle,
        ];
        if choose_card_type.options == permanent_type_options {
            return format!("{chooser} {choose_verb} a permanent type");
        }
        if choose_card_type.options.is_empty() {
            return format!("{chooser} {choose_verb} a card type");
        }
        let options = choose_card_type
            .options
            .iter()
            .map(|card_type| card_type.to_string().to_ascii_lowercase())
            .collect::<Vec<_>>();
        return format!("{chooser} {choose_verb} {}", join_with_or(&options));
    }
    if let Some(choose_named_option) =
        effect.downcast_ref::<crate::effects::ChooseNamedOptionEffect>()
    {
        let chooser = describe_player_filter(&choose_named_option.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        let options = choose_named_option
            .options
            .iter()
            .map(|option| option.to_ascii_lowercase())
            .collect::<Vec<_>>();
        return format!("{chooser} {choose_verb} {}", join_with_or(&options));
    }
    if let Some(directional_control) =
        effect.downcast_ref::<crate::effects::DirectionalAdjacentPlayerControlEffect>()
    {
        let object = with_indefinite_article(&directional_control.filter.description());
        let chosen_object = strip_leading_article(&object);
        return format!(
            "Starting with you and proceeding in the chosen direction, each player chooses {object} controlled by the next player in that direction. Each player gains control of the {chosen_object} they chose"
        );
    }
    if let Some(choose_creature_type) =
        effect.downcast_ref::<crate::effects::ChooseCreatureTypeEffect>()
    {
        let chooser = describe_player_filter(&choose_creature_type.chooser);
        let choose_verb = player_verb(&chooser, "choose", "chooses");
        if choose_creature_type.excluded_subtypes.is_empty() {
            return format!("{chooser} {choose_verb} a creature type");
        }
        let excluded = choose_creature_type
            .excluded_subtypes
            .iter()
            .map(|subtype| subtype.to_string().to_ascii_lowercase())
            .collect::<Vec<_>>();
        return format!(
            "{chooser} {choose_verb} a creature type other than {}",
            join_with_or(&excluded)
        );
    }
    if let Some(move_choice) =
        effect.downcast_ref::<crate::effects::MoveToLibraryTopOrBottomChoiceEffect>()
    {
        let target = describe_choose_spec(&move_choice.target);
        if let Some(chooser) = &move_choice.chooser
            && !matches!(
                chooser,
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
                    | PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(_))
            )
        {
            return format!(
                "Put {target} on {} choice of the top or bottom of {}",
                describe_possessive_player_filter(chooser),
                owner_library_phrase_for_spec(&move_choice.target)
            );
        }
        if choose_spec_is_plural(&move_choice.target) {
            return format!(
                "Put {target} on their owners' choice of the top or bottom of their owners' libraries"
            );
        }
        return format!(
            "{} owner puts it on their choice of the top or bottom of their library",
            describe_possessive_choose_spec(&move_choice.target)
        );
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        if move_to_zone.actor_surface.is_some()
            || move_to_zone.target_plural_surface
            || move_to_zone.verb_surface
                != ironsmith_core::MoveToZoneVerbSurface::Canonical
        {
            let mut canonical_move = move_to_zone.clone();
            canonical_move.actor_surface = None;
            canonical_move.target_plural_surface = false;
            canonical_move.verb_surface = ironsmith_core::MoveToZoneVerbSurface::Canonical;
            let mut rendered = describe_effect(&Effect::new(canonical_move));

            rendered = match move_to_zone.verb_surface {
                ironsmith_core::MoveToZoneVerbSurface::Canonical => rendered,
                ironsmith_core::MoveToZoneVerbSurface::Put => {
                    if move_to_zone.zone == Zone::Exile
                        && let Some(rest) = rendered.strip_prefix("Exile ")
                    {
                        format!("Put {rest} into exile")
                    } else {
                        let mut put = rendered
                            .strip_prefix("Return ")
                            .or_else(|| rendered.strip_prefix("Move "))
                            .map(|rest| format!("Put {rest}"))
                            .unwrap_or(rendered);
                        put = match move_to_zone.zone {
                            Zone::Battlefield => put.replacen(
                                " to the battlefield",
                                " onto the battlefield",
                                1,
                            ),
                            Zone::Hand | Zone::Graveyard => put.replacen(" to ", " into ", 1),
                            _ => put,
                        };
                        put
                    }
                }
                ironsmith_core::MoveToZoneVerbSurface::Return => {
                    let mut returned = rendered
                        .strip_prefix("Put ")
                        .or_else(|| rendered.strip_prefix("Move "))
                        .map(|rest| format!("Return {rest}"))
                        .unwrap_or(rendered);
                    returned = match move_to_zone.zone {
                        Zone::Battlefield => returned.replacen(
                            " onto the battlefield",
                            " to the battlefield",
                            1,
                        ),
                        Zone::Hand | Zone::Graveyard => {
                            returned.replacen(" into ", " to ", 1)
                        }
                        _ => returned,
                    };
                    returned
                }
            };

            if move_to_zone.target_plural_surface {
                rendered = rendered
                    .replacen("the exiled card", "the exiled cards", 1)
                    .replacen("that card", "those cards", 1)
                    .replacen(" it ", " them ", 1)
                    .replacen("Put it ", "Put them ", 1)
                    .replacen("Return it ", "Return them ", 1)
                    .replacen("Move it ", "Move them ", 1)
                    .replace("its owner's", "their owners'")
                    .replace("its owner", "their owners");
            }

            if let Some(actor) = &move_to_zone.actor_surface {
                let subject = if move_to_zone.destination_player_surface.as_ref() == Some(actor)
                    && move_to_zone.destination_player_reference_surface.is_some()
                {
                    "that player".to_string()
                } else {
                    describe_player_filter(actor)
                };
                let action = if subject == "that player"
                    && move_to_zone.destination_player_reference_surface
                        == Some(ironsmith_core::DestinationPlayerReferenceSurface::Pronoun)
                {
                    rendered.replacen("that player's", "their", 1)
                } else {
                    rendered
                };
                let action = lowercase_first(&action);
                let action = if subject == "you" {
                    normalize_you_verb_phrase(&action)
                } else {
                    normalize_third_person_verb_phrase(&action)
                };
                rendered = format!("{} {action}", capitalize_first(&subject));
            }
            return rendered;
        }
        if let Some(surface) = &move_to_zone.exiled_with_source_surface {
            let rendered = describe_exiled_with_source_move(
                surface,
                move_to_zone.zone,
                move_to_zone.destination_player_surface.as_ref(),
                Some(&move_to_zone.battlefield_controller),
                move_to_zone.enters_tapped,
            );
            return if move_to_zone.zone == Zone::Battlefield {
                append_battlefield_entry_counter_surface(
                    rendered,
                    &move_to_zone.enters_with_counters,
                )
            } else {
                rendered
            };
        }
        let order_suffix = match move_to_zone.library_order.as_ref() {
            Some(crate::effects::LibraryPlacementOrder::Random) => " in a random order",
            Some(crate::effects::LibraryPlacementOrder::ChosenBy(_)) => " in any order",
            None => "",
        };
        let target = if move_to_zone.zone == Zone::Battlefield
            && let Some(target) = describe_hand_or_graveyard_choice_target(&move_to_zone.target)
        {
            target
        } else if move_to_zone.library_order.is_some()
            && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(_))
        {
            "those cards".to_string()
        } else {
            describe_choose_spec(&move_to_zone.target)
        };
        let contextual_destination = match (
            move_to_zone.destination_player_surface.as_ref(),
            move_to_zone.destination_player_reference_surface,
            move_to_zone.target.base(),
        ) {
            (
                None,
                Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer),
                ChooseSpec::Tagged(_),
            ) => Some("that player's".to_string()),
            (Some(player), reference_surface, _) => Some(match reference_surface {
                Some(ironsmith_core::DestinationPlayerReferenceSurface::Pronoun) => {
                    "their".to_string()
                }
                Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer) => {
                    "that player's".to_string()
                }
                None if matches!(player, PlayerFilter::AliasedTarget(_)) => {
                    "that player's".to_string()
                }
                None => describe_possessive_player_filter(player),
            }),
            _ => None,
        };
        let rendered = match move_to_zone.zone {
            Zone::Exile => {
                if let Some(owner) = graveyard_owner_from_spec(&move_to_zone.target) {
                    let mut target_text =
                        describe_choose_spec_without_graveyard_zone(&move_to_zone.target);
                    if move_to_zone.target.count().is_random()
                        && !target_text.to_ascii_lowercase().contains(" at random")
                    {
                        target_text.push_str(" at random");
                    }
                    let from_text = match owner {
                        Some(owner) => {
                            format!(
                                "{} graveyard",
                                describe_possessive_graveyard_owner_filter(&owner)
                            )
                        }
                        None => "a graveyard".to_string(),
                    };
                    format!("Exile {target_text} from {from_text}")
                } else {
                    format!("Exile {target}")
                }
            }
            Zone::Graveyard => {
                if let ChooseSpec::Tagged(tag) = move_to_zone.target.base()
                    && crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled")
                {
                    "Return those cards to their owners' graveyards".to_string()
                } else if let ChooseSpec::All(filter) = move_to_zone.target.base()
                    && is_source_exiled_cards_filter(filter)
                {
                    "Put each card exiled with this artifact into its owner's graveyard".to_string()
                } else {
                    let target = describe_simple_exiled_card_target(&move_to_zone.target)
                        .unwrap_or_else(|| target.clone());
                    if let Some(owner) = &contextual_destination {
                        format!("Put {target} into {owner} graveyard")
                    } else {
                        format!("Put {target} into its owner's graveyard")
                    }
                }
            }
            Zone::Hand => {
                let is_put = matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag)
                    if tag.as_str().starts_with("revealed_")
                        || crate::cards::is_sentence_helper_tag(tag.as_str(), "revealed")
                        || tag.as_str().starts_with("searched_")
                        || crate::cards::is_sentence_helper_tag(tag.as_str(), "searched")
                        || tag.as_str().starts_with("milled_")
                        || crate::cards::is_sentence_helper_tag(tag.as_str(), "milled")
                        || tag.as_str().starts_with("discarded_"));
                if let Some(owner) = &contextual_destination {
                    if is_put {
                        format!("Put {target} into {owner} hand")
                    } else {
                        format!("Return {target} to {owner} hand")
                    }
                } else if is_put {
                    format!(
                        "Put {target} into {}",
                        owner_hand_phrase_for_spec(&move_to_zone.target)
                    )
                } else {
                    format!(
                        "Return {target} to {}",
                        owner_hand_phrase_for_spec(&move_to_zone.target)
                    )
                }
            }
            Zone::Library => {
                if let Some(owner) = hand_owner_from_spec(&move_to_zone.target) {
                    let from_zone = match &owner {
                        Some(owner) => {
                            format!("{} hand", describe_possessive_player_filter(owner))
                        }
                        None => "a hand".to_string(),
                    };
                    let library = match &owner {
                        Some(owner) => {
                            format!("{} library", describe_possessive_player_filter(owner))
                        }
                        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
                    };
                    if matches!(move_to_zone.target.base(), ChooseSpec::All(_)) {
                        if move_to_zone.to_top {
                            return format!(
                                "Put the cards in {from_zone} on top of {library}{order_suffix}"
                            );
                        }
                        return format!(
                            "Put the cards in {from_zone} on the bottom of {library}{order_suffix}"
                        );
                    }
                    let cards = describe_card_choice_count(move_to_zone.target.count());
                    if move_to_zone.to_top {
                        return format!(
                            "Put {cards} from {from_zone} on top of {library}{order_suffix}"
                        );
                    }
                    return format!(
                        "Put {cards} from {from_zone} on the bottom of {library}{order_suffix}"
                    );
                }
                if let Some(owner) = graveyard_owner_from_spec(&move_to_zone.target) {
                    let cards = describe_choose_spec_without_graveyard_zone(&move_to_zone.target);
                    let from_zone = match &owner {
                        Some(owner) => {
                            format!(
                                "{} graveyard",
                                describe_possessive_graveyard_owner_filter(owner)
                            )
                        }
                        None => "a graveyard".to_string(),
                    };
                    let library = match &owner {
                        Some(owner) => {
                            format!("{} library", describe_possessive_player_filter(owner))
                        }
                        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
                    };
                    if move_to_zone.to_top {
                        return format!(
                            "Put {cards} from {from_zone} on top of {library}{order_suffix}"
                        );
                    }
                    return format!(
                        "Put {cards} from {from_zone} on the bottom of {library}{order_suffix}"
                    );
                }
                if move_to_zone.to_top {
                    format!(
                        "Put {target} on top of {}{order_suffix}",
                        contextual_destination
                            .as_ref()
                            .map(|owner| format!("{owner} library"))
                            .unwrap_or_else(|| owner_library_phrase_for_spec(&move_to_zone.target).to_string())
                    )
                } else {
                    format!(
                        "Put {target} on the bottom of {}{order_suffix}",
                        contextual_destination
                            .as_ref()
                            .map(|owner| format!("{owner} library"))
                            .unwrap_or_else(|| owner_library_phrase_for_spec(&move_to_zone.target).to_string())
                    )
                }
            }
            Zone::Battlefield => {
                let source_from_exile_target =
                    describe_source_card_from_exile_target(&move_to_zone.target);
                let target = if let Some(target) = source_from_exile_target {
                    target.to_string()
                } else if let ChooseSpec::All(filter) = &move_to_zone.target
                    && filter.tagged_constraints.iter().any(|constraint| {
                        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                            && crate::cards::is_sentence_helper_tag(
                                constraint.tag.as_str(),
                                "exiled",
                            )
                    })
                {
                    "the exiled cards".to_string()
                } else if matches!(&move_to_zone.target, ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
                {
                    "it".to_string()
                } else if is_source_damaged_death_graveyard_card_spec(&move_to_zone.target) {
                    "that card".to_string()
                } else {
                    target
                };
                let owner_control_suffix = if choose_spec_allows_multiple(&move_to_zone.target) {
                    " under their owners' control"
                } else {
                    " under its owner's control"
                };
                let tapped_suffix = if move_to_zone.enters_tapped {
                    " tapped"
                } else {
                    ""
                };
                let attacking_suffix = if move_to_zone.enters_attacking {
                    " and attacking"
                } else {
                    ""
                };
                let face_down_suffix = if move_to_zone.enters_face_down {
                    " face down"
                } else {
                    ""
                };
                let controller_suffix = match move_to_zone.battlefield_controller {
                    crate::effects::BattlefieldController::Preserve => "",
                    crate::effects::BattlefieldController::Owner => owner_control_suffix,
                    crate::effects::BattlefieldController::You => " under your control",
                };
                if source_from_exile_target.is_some() {
                    if matches!(
                        move_to_zone.battlefield_controller,
                        crate::effects::BattlefieldController::Owner
                    ) {
                        format!(
                            "Return {target} to the battlefield{tapped_suffix}{attacking_suffix}{face_down_suffix}{controller_suffix}"
                        )
                    } else {
                        format!(
                            "Put {target} onto the battlefield{tapped_suffix}{attacking_suffix}{face_down_suffix}{controller_suffix}"
                        )
                    }
                } else if let crate::target::ChooseSpec::Tagged(tag) = &move_to_zone.target
                    && (tag.as_str() == "triggering"
                        || tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                        || tag.as_str().starts_with("exiled_")
                        || crate::cards::is_sentence_helper_tag(tag.as_str(), "exiled"))
                {
                    format!(
                        "Return {target} to the battlefield{tapped_suffix}{attacking_suffix}{face_down_suffix}{controller_suffix}"
                    )
                } else {
                    format!(
                        "Put {target} onto the battlefield{tapped_suffix}{attacking_suffix}{face_down_suffix}{controller_suffix}"
                    )
                }
            }
            Zone::Stack => format!("Put {target} on the stack"),
            Zone::Command => format!("Move {target} to the command zone"),
            Zone::Ante => format!("Ante {target}"),
            Zone::OutsideGame => format!("Move {target} outside the game"),
        };
        return if move_to_zone.zone == Zone::Battlefield {
            append_battlefield_entry_counter_surface(
                rendered,
                &move_to_zone.enters_with_counters,
            )
        } else {
            rendered
        };
    }
    let describe_library_top_position = |position: &crate::effect::Value| match position {
        crate::effect::Value::Add(left, right)
            if matches!(right.as_ref(), crate::effect::Value::Fixed(1))
                && value_prefers_where_x(left) =>
        {
            format!(
                "just beneath the top X cards, where X is {}",
                describe_where_x_basis(left).unwrap_or_else(|| describe_value(left))
            )
        }
        crate::effect::Value::Add(left, right)
            if matches!(left.as_ref(), crate::effect::Value::Fixed(1))
                && value_prefers_where_x(right) =>
        {
            format!(
                "just beneath the top X cards, where X is {}",
                describe_where_x_basis(right).unwrap_or_else(|| describe_value(right))
            )
        }
        crate::effect::Value::Add(left, right)
            if matches!(
                (left.as_ref(), right.as_ref()),
                (crate::effect::Value::X, crate::effect::Value::Fixed(1))
                    | (crate::effect::Value::Fixed(1), crate::effect::Value::X)
            ) =>
        {
            "just beneath the top X cards".to_string()
        }
        _ => library_position_from_top_text(position, false),
    };
    if let Some(move_to_nth) =
        effect.downcast_ref::<crate::effects::MoveToLibraryNthFromTopEffect>()
    {
        let target = describe_choose_spec(&move_to_nth.target);
        return format!(
            "Put {target} into {} {}",
            owner_library_phrase_for_spec(&move_to_nth.target),
            describe_library_top_position(&move_to_nth.position)
        );
    }
    if let Some(put_onto_battlefield) =
        effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        let target = describe_source_card_from_exile_target(&put_onto_battlefield.target)
            .map(str::to_string)
            .unwrap_or_else(|| describe_choose_spec(&put_onto_battlefield.target));
        let mut text = format!("Put {target} onto the battlefield");
        if put_onto_battlefield.tapped {
            text.push_str(" tapped");
        }
        return text;
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        let face_down_suffix = if exile.face_down { " face down" } else { "" };
        if let ChooseSpec::All(filter) = &exile.spec
            && has_vote_winners_tag(filter)
        {
            return format!(
                "Exile each permanent with the most votes or tied for most votes{face_down_suffix}"
            );
        }
        if let ChooseSpec::All(filter) = &exile.spec
            && filter.zone == Some(Zone::Graveyard)
            && filter.owner == Some(PlayerFilter::Opponent)
            && is_whole_graveyard_exile_filter(filter)
        {
            return format!("Exile each opponent's graveyard{face_down_suffix}");
        }
        if let ChooseSpec::All(filter) = &exile.spec
            && filter.zone == Some(Zone::Graveyard)
            && is_whole_graveyard_exile_filter(filter)
            && let Some(owner) = filter.owner.as_ref()
        {
            return format!(
                "Exile {} graveyard{face_down_suffix}",
                describe_possessive_graveyard_owner_filter(owner)
            );
        }
        if let ChooseSpec::All(filter) = &exile.spec
            && filter.zone == Some(Zone::Graveyard)
            && filter.owner.is_none()
            && !filter.single_graveyard
            && is_whole_graveyard_exile_filter(filter)
        {
            return format!("Exile all graveyards{face_down_suffix}");
        }
        let target = describe_choose_spec(&exile.spec);
        if let ChooseSpec::All(filter) = &exile.spec
            && filter.zone == Some(Zone::Graveyard)
            && filter.owner.is_none()
            && !filter.single_graveyard
            && target.contains(" in a graveyard")
        {
            let target = target
                .replacen(" in a graveyard", " in all graveyards", 1)
                .replacen(" that was put", " that were put", 1);
            return format!("Exile {target}{face_down_suffix}");
        }
        if let Some(owner) = graveyard_owner_from_spec(&exile.spec) {
            let target = describe_choose_spec_without_graveyard_zone(&exile.spec);
            let from = match owner {
                Some(owner) => format!(
                    "{} graveyard",
                    describe_possessive_graveyard_owner_filter(&owner)
                ),
                None if graveyard_spec_is_single(&exile.spec) => {
                    "a single graveyard".to_string()
                }
                None if choose_spec_allows_multiple(&exile.spec) => "graveyards".to_string(),
                None => "a graveyard".to_string(),
            };
            return format!("Exile {target} from {from}{face_down_suffix}");
        }
        if let Some(rest) = target.strip_prefix("all cards in ") {
            return format!("Exile all cards from {rest}{face_down_suffix}");
        }
        return format!("Exile {}{face_down_suffix}", target);
    }
    if let Some(exile_until) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
        if exile_until.explicit_return_surface
            && exile_until.duration
                == crate::effects::ExileUntilDuration::SourceLeavesBattlefield
            && exile_until.return_zone == Zone::Battlefield
        {
            let target = describe_choose_spec(&exile_until.spec);
            let plural = choose_spec_allows_multiple(&exile_until.spec);
            let return_object = if plural { "those cards" } else { "that card" };
            let owner_control = if plural {
                "under their owners' control"
            } else {
                "under its owner's control"
            };
            let face_down_suffix = if exile_until.face_down { " face down" } else { "" };
            return format!(
                "Exile {target}{face_down_suffix}. Return {return_object} to the battlefield {owner_control} when this permanent leaves the battlefield"
            );
        }
        let duration = match exile_until.duration {
            crate::effects::ExileUntilDuration::SourceLeavesBattlefield => {
                "until this permanent leaves the battlefield"
            }
            crate::effects::ExileUntilDuration::NextEndStep => "until the next end step",
            crate::effects::ExileUntilDuration::EndOfCombat => "until end of combat",
        };
        let face_down_suffix = if exile_until.face_down {
            " face down"
        } else {
            ""
        };
        let mut target = describe_choose_spec(&exile_until.spec);
        if matches!(&exile_until.spec, ChooseSpec::All(_)) {
            target = target.replace("artifacts or creatures", "artifacts and creatures");
        }
        return format!("Exile {target}{face_down_suffix} {duration}");
    }
    if let Some(_haunt_exile) = effect.downcast_ref::<crate::effects::HauntExileEffect>() {
        return "Exile it haunting target creature".to_string();
    }
    if let Some(exile_when_source_leaves) =
        effect.downcast_ref::<crate::effects::ExileTaggedWhenSourceLeavesEffect>()
    {
        if exile_when_source_leaves
            .tag
            .as_str()
            .starts_with("created_")
        {
            return "Exile that token when this permanent leaves the battlefield".to_string();
        }
        let tagged = ChooseSpec::Tagged(exile_when_source_leaves.tag.clone());
        return format!(
            "Exile {} when this permanent leaves the battlefield",
            describe_choose_spec(&tagged)
        );
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        let where_clause = choose_spec_dynamic_count_value_where_clause(&destroy.spec)
            .or_else(|| choose_spec_filter_where_x_clause(&destroy.spec))
            .unwrap_or_default();
        let base = format!("Destroy {}{where_clause}", describe_choose_spec(&destroy.spec));
        let tail = if choose_spec_allows_multiple(&destroy.spec) {
            "They can't be regenerated"
        } else {
            "It can't be regenerated"
        };
        return format!("{base}. {tail}");
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        let destroy_count = destroy.spec.count();
        if let ChooseSpec::All(filter) = &destroy.spec
            && has_vote_winners_tag(filter)
        {
            return "Destroy each creature with the most votes or tied for most votes".to_string();
        }
        if destroy_count.is_single()
            && destroy_count.is_random()
            && let Some(filter) = match destroy.spec.base() {
                ChooseSpec::All(filter) | ChooseSpec::Object(filter) => Some(filter),
                _ => None,
            }
            && filter.with_counter.is_some()
            && filter.controller == Some(PlayerFilter::NotYou)
            && (filter.card_types.is_empty()
                || filter.card_types.iter().any(|card_type| {
                    matches!(
                        card_type,
                        CardType::Artifact
                            | CardType::Creature
                            | CardType::Enchantment
                            | CardType::Land
                            | CardType::Planeswalker
                            | CardType::Battle
                    )
                }))
        {
            return "Destroy one of those permanents at random".to_string();
        }
        if let ChooseSpec::All(filter) = &destroy.spec
            && filter.card_types.as_slice() == [crate::types::CardType::Creature]
            && filter.all_colors == Some(false)
        {
            return "Destroy each creature that isn't all colors".to_string();
        }
        if let ChooseSpec::All(filter) = &destroy.spec
            && filter.card_types.as_slice() == [crate::types::CardType::Creature]
            && let Some(crate::filter::Comparison::GreaterThanOrEqualExpr(value)) = &filter.power
            && matches!(value.unhinted(), Value::EffectValue(_))
        {
            return "Destroy each creature with power greater than or equal to that result"
                .to_string();
        }
        if let ChooseSpec::All(filter) = &destroy.spec
            && filter.card_types.as_slice() == [crate::types::CardType::Creature]
            && filter.with_counter.is_none()
            && matches!(
                filter.without_counter,
                Some(crate::filter::CounterConstraint::Any)
            )
        {
            return "Destroy all creatures with no counters on them".to_string();
        }
        if let ChooseSpec::All(filter) = &destroy.spec
            && let Some(counter) = filter.with_counter
            && filter.card_types.iter().any(|card_type| {
                matches!(
                    card_type,
                    CardType::Artifact
                        | CardType::Creature
                        | CardType::Enchantment
                        | CardType::Land
                        | CardType::Planeswalker
                        | CardType::Battle
                )
            })
        {
            return format!(
                "Destroy each permanent with {} on it",
                describe_counter_constraint(counter)
            );
        }
        if let ChooseSpec::All(filter) = &destroy.spec
            && filter.card_types.len() == 3
            && filter.card_types.contains(&CardType::Artifact)
            && filter.card_types.contains(&CardType::Creature)
            && filter.card_types.contains(&CardType::Enchantment)
            && matches!(
                filter.mana_value,
                Some(crate::filter::Comparison::LessThanOrEqualExpr(ref value))
                    if matches!(value.as_ref(), Value::EffectValue(_) | Value::EffectValueOffset(_, 0))
            )
        {
            return "Destroy each artifact, creature, and enchantment with mana value less than or equal to the amount of {E} paid this way".to_string();
        }
        let where_clause = choose_spec_dynamic_count_value_where_clause(&destroy.spec)
            .or_else(|| choose_spec_filter_where_x_clause(&destroy.spec))
            .unwrap_or_default();
        return format!("Destroy {}{where_clause}", describe_choose_spec(&destroy.spec));
    }
    if let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        if let Some(for_each) = unwrap_basic_tag_wrappers(&with_source.effect)
            .downcast_ref::<crate::effects::ForEachObject>()
            && let Some(compact) =
                describe_for_each_iterated_damage(for_each, Some(&with_source.source))
        {
            return compact;
        }
        if with_source
            .effect
            .downcast_ref::<crate::effects::BecomeSaddledUntilEotEffect>()
            .is_some()
        {
            let mut subject = describe_choose_spec(&with_source.source);
            if subject == "it" {
                subject = "that permanent".to_string();
            } else if subject == "this source" {
                subject = "this permanent".to_string();
            }
            return format!("{subject} becomes saddled until end of turn");
        }
        if let Some(deal_damage) = with_source
            .effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
        {
            let has_explicit_source_surface =
                with_source.source.source_reference_surface().is_some();
            let mut subject = describe_choose_spec(&with_source.source);
            if subject == "this source" {
                subject = "this creature".to_string();
            } else if subject == "it" && !has_explicit_source_surface {
                subject = "that creature".to_string();
            } else if subject.eq_ignore_ascii_case("target creature") {
                // The targeting happened in an earlier clause; this is a
                // back-reference ("That creature deals ...").
                subject = "that creature".to_string();
            }
            let mut target = describe_damage_target(&deal_damage.target);
            if target == "this source" {
                target = "this creature".to_string();
            } else if target == "it" {
                target = "that creature".to_string();
            }
            // Multi-target full damage reads "each of two other target
            // creatures" in oracle.
            if let Some((count_word, rest)) = target.clone().split_once(" target other ") {
                target = format!("each of {count_word} other target {rest}");
            }

            if matches!(
                with_source.source.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            ) && matches!(
                deal_damage.amount.unhinted(),
                Value::SourcePower | Value::SourceToughness
            ) {
                let stat = if matches!(deal_damage.amount.unhinted(), Value::SourceToughness) {
                    "toughness"
                } else {
                    "power"
                };
                return format!(
                    "This spell deals damage equal to the exiled card's {stat} to {target}"
                );
            }

            if let Value::PowerOf(spec) | Value::ToughnessOf(spec) = deal_damage.amount.unhinted()
                && matches!(
                    spec.base(),
                    ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                )
            {
                let stat = if matches!(deal_damage.amount.unhinted(), Value::ToughnessOf(_)) {
                    "toughness"
                } else {
                    "power"
                };
                return format!(
                    "This spell deals damage equal to the exiled card's {stat} to {target}"
                );
            }

            if let Value::PowerOf(_) | Value::ToughnessOf(_) = &deal_damage.amount {
                let stat = if matches!(&deal_damage.amount, Value::ToughnessOf(_)) {
                    "toughness"
                } else {
                    "power"
                };
                if subject.eq_ignore_ascii_case(&target) {
                    return format!("{subject} deals damage to itself equal to its {stat}");
                }
                let verb = if choose_spec_is_plural(&with_source.source) {
                    "deal"
                } else {
                    "deals"
                };
                return format!("{subject} {verb} damage equal to its {stat} to {target}");
            }

            if matches!(
                deal_damage.amount.unhinted(),
                Value::TurnHistoryCount(_)
            ) {
                let verb = if choose_spec_is_plural(&with_source.source) {
                    "deal"
                } else {
                    "deals"
                };
                return format!(
                    "{subject} {verb} damage to {target} equal to {}",
                    describe_value(&deal_damage.amount)
                );
            }

            if deal_damage
                .amount
                .has_surface_hint(ValueSurfaceHint::EqualTo)
                && let Some(basis) = describe_prior_effect_count_basis_for_action(
                    &deal_damage.amount,
                    crate::effect::PriorEffectAction::Tapped,
                    true,
                )
            {
                let verb = if choose_spec_is_plural(&with_source.source) {
                    "deal"
                } else {
                    "deals"
                };
                return format!(
                    "{subject} {verb} damage to {target} equal to the number of {basis}"
                );
            }
            if deal_damage
                .amount
                .has_surface_hint(ValueSurfaceHint::ForEach)
                && let Some(basis) = describe_prior_effect_count_basis_for_action(
                    &deal_damage.amount,
                    crate::effect::PriorEffectAction::Tapped,
                    false,
                )
            {
                let verb = if choose_spec_is_plural(&with_source.source) {
                    "deal"
                } else {
                    "deals"
                };
                return format!("{subject} {verb} 1 damage to {target} for each {basis}");
            }

            let (amount, where_x) = describe_damage_amount_clause(&deal_damage.amount);
            let mut text = format!("{subject} deals {amount} to {target}");
            if let Some(where_x) = where_x {
                text.push_str(&format!(", where X is {where_x}"));
            }
            return text;
        }
        return describe_effect(&with_source.effect);
    }
    if let Some(deal_damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        if deal_damage.unpreventable {
            let mut preventable = deal_damage.clone();
            preventable.unpreventable = false;
            let base = describe_effect_impl(&Effect::new(preventable));
            return format!(
                "{}. The damage can't be prevented",
                base.trim_end_matches('.')
            );
        }
        if let Value::PowerOf(source) | Value::ToughnessOf(source) = &deal_damage.amount {
            if matches!(
                source.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            ) {
                let stat = if matches!(&deal_damage.amount, Value::ToughnessOf(_)) {
                    "toughness"
                } else {
                    "power"
                };
                return format!(
                    "Deal damage equal to the exiled card's {stat} to {}",
                    describe_damage_target(&deal_damage.target)
                );
            }
            let target_matches_power_source = matches!(
                (source.base(), deal_damage.target.base()),
                (
                    ChooseSpec::Tagged(source_tag),
                    ChooseSpec::Player(
                        PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(target_tag))
                            | PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(target_tag))
                    )
                ) if source_tag.as_str() == target_tag.as_str()
            );
            if target_matches_power_source {
                let stat = if matches!(&deal_damage.amount, Value::ToughnessOf(_)) {
                    "toughness"
                } else {
                    "power"
                };
                let amount_text = describe_dynamic_counter_basis(source, stat);
                return format!(
                    "Deal damage equal to {amount_text} to {}",
                    describe_damage_target(&deal_damage.target)
                );
            }
            let mut subject = describe_choose_spec(source);
            if subject == "this source" {
                subject = "this creature".to_string();
            } else if subject == "it" {
                subject = "that creature".to_string();
            } else if subject.eq_ignore_ascii_case("target creature") {
                // Back-reference to an already-targeted creature.
                subject = "that creature".to_string();
            }
            let mut target = describe_damage_target(&deal_damage.target);
            if target == "this source" {
                target = "this creature".to_string();
            } else if target == "it" {
                target = "that creature".to_string();
            }
            // Multi-target full damage reads "each of two other target
            // creatures" in oracle.
            if let Some((count_word, rest)) = target.clone().split_once(" target other ") {
                target = format!("each of {count_word} other target {rest}");
            }
            let stat = if matches!(&deal_damage.amount, Value::ToughnessOf(_)) {
                "toughness"
            } else {
                "power"
            };
            if subject.eq_ignore_ascii_case(&target) {
                let lower_subject = subject.to_ascii_lowercase();
                let should_render_each = !lower_subject.starts_with("target ")
                    && !lower_subject.starts_with("this ")
                    && !lower_subject.starts_with("that ")
                    && !lower_subject.starts_with("another ");
                if should_render_each {
                    return format!("Each {subject} deals damage to itself equal to its {stat}");
                }
                if choose_spec_is_plural(source) {
                    let each_subject = if subject.to_ascii_lowercase().starts_with("each ") {
                        subject.clone()
                    } else {
                        format!("Each {subject}")
                    };
                    return format!("{each_subject} deals damage to itself equal to its {stat}");
                }
                return format!("{subject} deals damage to itself equal to its {stat}");
            }
            let verb = if choose_spec_is_plural(source) {
                "deal"
            } else {
                "deals"
            };
            return format!("{subject} {verb} damage equal to its {stat} to {target}");
        }
        if let Value::ManaValueOf(spec) = &deal_damage.amount {
            let amount_text = {
                let described = describe_choose_spec(spec);
                if described == "it" {
                    "its mana value".to_string()
                } else {
                    format!("the mana value of {described}")
                }
            };
            return format!(
                "Deal damage equal to {} to {}",
                amount_text,
                describe_damage_target(&deal_damage.target)
            );
        }
        if let Value::Add(left, right) = &deal_damage.amount {
            let mana_value_spec = match (left.as_ref(), right.as_ref()) {
                (Value::Fixed(n), Value::ManaValueOf(spec))
                | (Value::ManaValueOf(spec), Value::Fixed(n)) => Some((*n, spec)),
                _ => None,
            };
            if let Some((offset, spec)) = mana_value_spec {
                let described = describe_choose_spec(spec);
                let amount_text = if described == "it" {
                    if offset == 0 {
                        "its mana value".to_string()
                    } else {
                        format!("{offset} plus its mana value")
                    }
                } else if offset == 0 {
                    format!("the mana value of {described}")
                } else {
                    format!("{offset} plus the mana value of {described}")
                };
                return format!(
                    "Deal damage equal to {} to {}",
                    amount_text,
                    describe_damage_target(&deal_damage.target)
                );
            }
        }
        if matches!(
            &deal_damage.amount,
            Value::SpellsCastThisTurn(_)
                | Value::SpellsCastBeforeThisTurn(_)
                | Value::SpellsCastThisTurnMatching { .. }
        ) {
            return format!(
                "Deal damage to {} equal to {}",
                describe_damage_target(&deal_damage.target),
                describe_value(&deal_damage.amount)
            );
        }
        if deal_damage
            .amount
            .has_surface_hint(ValueSurfaceHint::EqualTo)
            && let Some(basis) = describe_prior_effect_count_basis_for_action(
                &deal_damage.amount,
                crate::effect::PriorEffectAction::Tapped,
                true,
            )
        {
            return format!(
                "Deal damage to {} equal to the number of {basis}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if deal_damage
            .amount
            .has_surface_hint(ValueSurfaceHint::ForEach)
            && let Some(basis) = describe_prior_effect_count_basis_for_action(
                &deal_damage.amount,
                crate::effect::PriorEffectAction::Tapped,
                false,
            )
        {
            return format!(
                "Deal 1 damage to {} for each {basis}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if let Some((amount, where_x)) = describe_where_x_offset_value(&deal_damage.amount) {
            return format!(
                "Deal {amount} damage to {}, where X is {where_x}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if let Some((amount_text, where_x)) =
            describe_damage_amount_with_revealed_count_where_x(&deal_damage.amount)
        {
            return format!(
                "Deal {amount_text} damage to {}, where X is {where_x}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if !value_prefers_where_x(&deal_damage.amount)
            && count_damage_prefers_equal_to(&deal_damage.amount)
        {
            return format!(
                "Deal damage to {} equal to {}",
                describe_damage_target(&deal_damage.target),
                describe_value(&deal_damage.amount)
            );
        }
        if let Some(where_x) = describe_where_x_basis(&deal_damage.amount) {
            return format!(
                "Deal X damage to {}, where X is {where_x}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if is_effect_count_reference(&deal_damage.amount, None) {
            return format!(
                "Deal that much damage to {}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if matches!(deal_damage.amount.unhinted(), Value::XTimes(2)) {
            return format!(
                "Deal twice X damage to {}",
                describe_damage_target(&deal_damage.target)
            );
        }
        if value_prefers_equal_to(&deal_damage.amount)
            || power_damage_prefers_equal_to(&deal_damage.amount)
        {
            return format!(
                "Deal damage to {} equal to {}",
                describe_damage_target(&deal_damage.target),
                describe_value(&deal_damage.amount)
            );
        }
        return format!(
            "Deal {} damage to {}",
            describe_value(&deal_damage.amount),
            describe_damage_target(&deal_damage.target)
        );
    }
    if let Some(distributed) = effect.downcast_ref::<crate::effects::DealDistributedDamageEffect>()
    {
        if !matches!(distributed.source.base(), ChooseSpec::Source) {
            let source = capitalize_first(&describe_choose_spec(&distributed.source));
            let chooser = if matches!(
                &distributed.chooser,
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
                    | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Target)
            ) {
                "its controller".to_string()
            } else {
                describe_player_filter(&distributed.chooser)
            };
            let (amount, where_clause) = if let Some(where_x) =
                describe_where_x_basis(&distributed.amount)
            {
                ("X damage".to_string(), format!(", where X is {where_x}"))
            } else if matches!(
                distributed.amount.unhinted(),
                Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Source)
            ) {
                ("damage equal to its power".to_string(), String::new())
            } else {
                (
                    format!(
                        "{} damage",
                        describe_distributed_damage_amount(&distributed.amount)
                    ),
                    String::new(),
                )
            };
            let choose_verb = player_verb(&chooser, "choose", "chooses");
            return format!(
                "{source} deals {amount} divided as {chooser} {choose_verb} among {}{where_clause}",
                describe_distributed_damage_target(&distributed.target),
            );
        }
        if let Some(where_x) = describe_where_x_basis(&distributed.amount) {
            if matches!(distributed.amount.unhinted(), Value::SourcePower)
                || matches!(
                    distributed.amount.unhinted(),
                    Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Source)
                )
            {
                return format!(
                    "Deal damage equal to its power divided as you choose among {}",
                    describe_distributed_damage_target(&distributed.target)
                );
            }
            return format!(
                "Deal X damage divided as you choose among {}, where X is {where_x}",
                describe_distributed_damage_target(&distributed.target)
            );
        }
        let amount = describe_distributed_damage_amount(&distributed.amount);
        if amount.contains("mana value") {
            return format!(
                "Deal damage equal to {amount} divided as you choose among {}",
                describe_distributed_damage_target(&distributed.target)
            );
        }
        return format!(
            "Deal {} damage divided as you choose among {}",
            amount,
            describe_distributed_damage_target(&distributed.target)
        );
    }
    if let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() {
        return format!(
            "{} fights {}",
            describe_choose_spec(&fight.creature1),
            describe_choose_spec(&fight.creature2)
        );
    }
    if let Some(counter_spell) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        if let ChooseSpec::Target(inner) = &counter_spell.target
            && let ChooseSpec::Object(filter) = inner.as_ref()
            && filter.any_of.len() == 2
            && filter.any_of.iter().any(|candidate| {
                candidate.zone == Some(Zone::Stack)
                    && candidate.stack_kind == Some(StackObjectKind::TriggeredAbility)
            })
            && filter.any_of.iter().any(|candidate| {
                candidate.zone == Some(Zone::Stack)
                    && candidate.stack_kind == Some(StackObjectKind::Spell)
                    && candidate.colorless
            })
        {
            return "Counter target triggered ability or colorless spell".to_string();
        }
        if let Some(target_text) = describe_counter_all_stack_abilities(&counter_spell.target) {
            return format!("Counter {target_text}");
        }
        let target_text = describe_choose_spec(&counter_spell.target)
            .replace("instant spell spell", "instant spell")
            .replace("sorcery spell spell", "sorcery spell");
        return format!("Counter {target_text}");
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        let payer = match unless_pays.player {
            PlayerFilter::Any => "any player".to_string(),
            _ => describe_player_filter(&unless_pays.player),
        };
        let pay_verb = player_verb(&payer, "pay", "pays");
        let counter_target = if let [counter_effect] = unless_pays.effects.as_slice() {
            counter_effect
                .downcast_ref::<crate::effects::CounterEffect>()
                .map(|counter| &counter.target)
        } else {
            None
        };
        let display = if let Some(target) = counter_target {
            describe_total_cost_payment_for_same_sole_target(&unless_pays.cost, target)
        } else {
            describe_total_cost_payment(&unless_pays.cost)
        };
        let payment_text = display.strip_prefix("Pay ").unwrap_or(&display).to_string();
        let action_payment_text = |payment: &str| -> Option<String> {
            let (base, third_person, rest) = if let Some(rest) = payment.strip_prefix("Sacrifice ")
            {
                ("sacrifice", "sacrifices", rest)
            } else if let Some(rest) = payment.strip_prefix("Discard ") {
                ("discard", "discards", rest)
            } else if let Some(rest) = payment.strip_prefix("Exile ") {
                ("exile", "exiles", rest)
            } else if let Some(rest) = payment.strip_prefix("Mill ") {
                ("mill", "mills", rest)
            } else {
                return None;
            };
            let verb = if payer == "you" { base } else { third_person };
            Some(format!("{verb} {rest}"))
        };
        if let ironsmith_core::TotalCostKind::OneOf(branches) = unless_pays.cost.kind() {
            let branch_texts = branches
                .iter()
                .map(|branch| {
                    let branch_display = if let Some(target) = counter_target {
                        describe_total_cost_payment_for_same_sole_target(branch, target)
                    } else {
                        describe_total_cost_payment(branch)
                    };
                    let branch_payment_text = branch_display
                        .strip_prefix("Pay ")
                        .unwrap_or(&branch_display)
                        .to_string();
                    action_payment_text(&branch_payment_text)
                        .unwrap_or_else(|| format!("{pay_verb} {branch_payment_text}"))
                })
                .collect::<Vec<_>>();
            if !branch_texts.is_empty() {
                let inner_text = describe_effect_list(&unless_pays.effects);
                return format!(
                    "{} unless {} {}",
                    inner_text,
                    payer,
                    branch_texts.join(" or ")
                );
            }
        }
        if unless_pays.effects.len() == 1
            && let Some(counter) =
                unless_pays.effects[0].downcast_ref::<crate::effects::CounterEffect>()
        {
            if let Some(action_text) = action_payment_text(&payment_text) {
                return format!(
                    "Counter {} unless {} {}",
                    describe_choose_spec(&counter.target),
                    payer,
                    action_text
                );
            }
            return format!(
                "Counter {} unless {} {} {}",
                describe_choose_spec(&counter.target),
                payer,
                pay_verb,
                payment_text
            );
        }

        if unless_pays.player == PlayerFilter::IteratedPlayer
            && unless_pays.effects.len() == 1
            && let Some(for_players) =
                unless_pays.effects[0].downcast_ref::<crate::effects::ForPlayersEffect>()
            && for_players.filter == PlayerFilter::Any
            && for_players.effects.len() == 1
        {
            let inner_text = describe_effect_list(&for_players.effects);
            return format!("For each player, {inner_text} unless they pay {payment_text}");
        }

        let inner_text = describe_effect_list(&unless_pays.effects);
        if let Some(action_text) = action_payment_text(&payment_text) {
            return format!("{} unless {} {}", inner_text, payer, action_text);
        }
        if let Some(prefix) =
            describe_unless_any_player_pays_search_prefix(unless_pays, &payment_text)
        {
            return prefix;
        }
        return format!(
            "{} unless {} {} {}",
            inner_text, payer, pay_verb, payment_text
        );
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        if let Some(compact) = describe_typed_unless_source_damage(unless_action) {
            return compact;
        }
        let inner_text = describe_effect_list(&unless_action.effects);
        if unless_action.alternative.len() == 1
            && let Some(pay_mana) =
                unless_action.alternative[0].downcast_ref::<crate::effects::PayManaEffect>()
            && let ChooseSpec::Player(alternative_player) = &pay_mana.player
            && *alternative_player == unless_action.player
        {
            let payment_text = pay_mana.cost.to_oracle();
            return format!("{} or pay {}", inner_text, payment_text);
        }
        if unless_action.alternative.len() == 1
            && let Some(lose_life) =
                unless_action.alternative[0].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let ChooseSpec::Player(alternative_player) = &lose_life.player
            && *alternative_player == unless_action.player
        {
            let payer = match alternative_player {
                PlayerFilter::Any => "any player".to_string(),
                _ => describe_player_filter(alternative_player),
            };
            let pay_verb = player_verb(&payer, "pay", "pays");
            return format!(
                "{} unless {} {} {} life",
                inner_text,
                payer,
                pay_verb,
                describe_value(&lose_life.amount)
            );
        }
        if unless_action.alternative.len() == 1
            && let Some(or_choice) =
                unless_action.alternative[0].downcast_ref::<crate::effects::UnlessActionEffect>()
            && or_choice.player == unless_action.player
        {
            let player = describe_player_filter(&unless_action.player);
            let strip_player_prefix = |text: String| {
                if text == player {
                    return text;
                }
                let prefix = format!("{player} ");
                text.strip_prefix(&prefix)
                    .map(str::to_string)
                    .unwrap_or(text)
            };
            let first_choice = strip_player_prefix(describe_effect_list(&or_choice.effects));
            let second_choice = strip_player_prefix(describe_effect_list(&or_choice.alternative));
            return format!(
                "{} unless {} {} or {}",
                inner_text, player, first_choice, second_choice
            );
        }
        let alt_text = describe_effect_list(&unless_action.alternative);
        let player = describe_player_filter(&unless_action.player);
        if inner_text.contains("to that player")
            && alt_text.contains("sacrifices a creature")
            && alt_text.starts_with("target player")
        {
            let sacrifice = alt_text
                .trim_start_matches("target player")
                .trim_start()
                .trim_start_matches("target player")
                .trim_start();
            return format!("{inner_text} unless that player {sacrifice}");
        }
        let unless_clause = if alt_text == player || alt_text.starts_with(&format!("{player} ")) {
            alt_text
        } else {
            format!("{player} {alt_text}")
        };
        return format!("{} unless {}", inner_text, unless_clause);
    }
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        if value_has_surface_hint(&put_counters.amount, ValueSurfaceHint::BlightKeywordAction)
            && put_counters.counter_type == CounterType::MinusOneMinusOne
            && put_counters.target == ChooseSpec::Object(ObjectFilter::creature().you_control())
            && put_counters.target_count.is_none()
            && !put_counters.distributed
        {
            return format!("Blight {}", describe_value(&put_counters.amount));
        }
        if put_counters.distributed {
            if let Some((counter_text, where_x)) =
                describe_counter_count_with_where_x(&put_counters.amount, put_counters.counter_type)
            {
                return format!(
                    "Distribute {counter_text} among {}, where X is {where_x}",
                    describe_choose_spec(&put_counters.target)
                );
            }
            return format!(
                "Distribute {} among {}",
                describe_put_counter_phrase(&put_counters.amount, put_counters.counter_type),
                describe_choose_spec(&put_counters.target)
            );
        }
        let mut target = describe_choose_spec(&put_counters.target);
        if let ChooseSpec::WithCount(inner, count) = &put_counters.target
            && matches!(inner.as_ref(), ChooseSpec::Target(_))
            && !count.is_single()
            && count.max != Some(1)
            && !target.starts_with("each of ")
        {
            target = format!("each of {target}");
        }
        if let Value::Count(filter) = &put_counters.amount
            && matches!(
                put_counters.counter_type,
                crate::object::CounterType::PlusOnePlusOne
                    | crate::object::CounterType::MinusOneMinusOne
            )
        {
            return format!(
                "Put a {} counter on {target} for each {}",
                describe_counter_type(put_counters.counter_type),
                describe_for_each_count_filter(&filter)
            );
        }
        if put_counters
            .amount
            .has_surface_hint(ValueSurfaceHint::EqualToAfterTarget)
        {
            let amount = put_counters
                .amount
                .clone()
                .without_surface_hint(ValueSurfaceHint::EqualToAfterTarget)
                .without_surface_hint(ValueSurfaceHint::EqualTo);
            return format!(
                "Put a number of {} counters on {target} equal to {}",
                describe_counter_type(put_counters.counter_type),
                describe_value(&amount)
            );
        }
        if put_counters
            .amount
            .has_surface_hint(ValueSurfaceHint::ForEach)
            && let Some(for_each) = describe_create_for_each_count(&put_counters.amount)
        {
            return format!(
                "Put a {} counter on {target} for each {for_each}",
                describe_counter_type(put_counters.counter_type),
            );
        }
        if let Some(group_size) = life_lost_this_way_group_size(&put_counters.amount) {
            return format!(
                "Put {} on {target} for each {group_size} life lost this way",
                describe_put_counter_phrase(&Value::Fixed(1), put_counters.counter_type),
            );
        }
        if let Some((counter_text, where_x)) =
            describe_counter_count_with_where_x(&put_counters.amount, put_counters.counter_type)
        {
            return format!("Put {counter_text} on {target}, where X is {where_x}");
        }
        if let Value::ManaValueOf(spec) = &put_counters.amount
            && matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
        {
            return format!(
                "Put a number of {} counters on {target} equal to the mana value of the exiled card",
                describe_counter_type(put_counters.counter_type)
            );
        }
        if let Value::CountersOn(spec, Some(counter_type)) = &put_counters.amount
            && is_graveyard_same_stable_tagged_spec(spec)
        {
            return format!(
                "Put its {} counters on {target}",
                describe_counter_type(counter_type.clone()),
            );
        }
        if let Some((amount_text, basis_text)) = describe_dynamic_counter_amount_phrase(
            &put_counters.amount,
            put_counters.counter_type,
            &target,
        ) {
            return format!("Put {amount_text} on {target}, where X is {basis_text}");
        }
        if let Value::Add(left, right) = &put_counters.amount
            && left == right
        {
            let per_text = match left.as_ref() {
                Value::Count(filter) => Some(describe_for_each_count_filter(filter)),
                Value::EffectMetric { metric, .. } | Value::PendingEffectMetric { metric, .. } => {
                    match metric {
                        crate::effect::EffectMetric::Count
                        | crate::effect::EffectMetric::AffectedCount => {
                            Some("object affected this way".to_string())
                        }
                        crate::effect::EffectMetric::ChosenCount => {
                            Some("object chosen this way".to_string())
                        }
                        crate::effect::EffectMetric::LifeLost => {
                            Some("1 life lost this way".to_string())
                        }
                        _ => None,
                    }
                }
                Value::SpellsCastThisTurn(player) => {
                    Some(describe_for_each_spells_cast_this_turn(player, false))
                }
                Value::SpellsCastBeforeThisTurn(player) => {
                    Some(describe_for_each_spells_cast_this_turn(player, true))
                }
                Value::Add(inner, offset)
                    if matches!(offset.as_ref(), Value::Fixed(n) if *n == -1)
                        && matches!(inner.as_ref(), Value::SpellsCastThisTurn(_)) =>
                {
                    if let Value::SpellsCastThisTurn(player) = inner.as_ref() {
                        Some(describe_for_each_spells_cast_this_turn(player, true))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(per_text) = per_text {
                return format!(
                    "Put two {} counters on {target} for each {per_text}",
                    describe_counter_type(put_counters.counter_type),
                );
            }
        }
        let target_where_clause =
            choose_spec_dynamic_count_value_where_clause(&put_counters.target)
                .or_else(|| choose_spec_filter_where_x_clause(&put_counters.target))
                .unwrap_or_default();
        return format!(
            "Put {} on {}{}",
            describe_put_counter_phrase(&put_counters.amount, put_counters.counter_type),
            target,
            target_where_clause
        );
    }
    if let Some(remove_counters) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
        let target = if matches!(remove_counters.target, ChooseSpec::Source) {
            "it".to_string()
        } else {
            describe_choose_spec(&remove_counters.target)
        };
        let counter_phrase = describe_remove_counter_phrase(
            &remove_counters.count,
            remove_counters.counter_type,
            &remove_counters.target,
        );
        return format!("Remove {} from {}", counter_phrase, target);
    }
    if let Some(remove_counters_among) =
        effect.downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
    {
        return crate::effects::remove_any_counters_among_cost_display(remove_counters_among);
    }
    if let Some(remove_any_from_source) =
        effect.downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
    {
        return remove_any_from_source.cost_display();
    }
    if let Some(remove_up_to_counters) =
        effect.downcast_ref::<crate::effects::RemoveUpToCountersEffect>()
    {
        return format!(
            "Remove up to {} {} counter(s) from {}",
            describe_value(&remove_up_to_counters.max_count),
            describe_counter_type(remove_up_to_counters.counter_type),
            describe_choose_spec(&remove_up_to_counters.target)
        );
    }
    if let Some(remove_up_to_any_counters) =
        effect.downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
    {
        let target = describe_choose_spec(&remove_up_to_any_counters.target);
        if let Value::CountersOn(counter_source, None) = &remove_up_to_any_counters.max_count
            && counter_source.unhinted() == remove_up_to_any_counters.target.unhinted()
        {
            return format!("Remove all counters from {target}");
        }
        return format!(
            "Remove up to {} counters from {}",
            describe_value(&remove_up_to_any_counters.max_count),
            target
        );
    }
    if let Some(move_counters) = effect.downcast_ref::<crate::effects::MoveAllCountersEffect>() {
        let from_text = describe_choose_spec(&move_counters.from);
        if matches!(move_counters.from, ChooseSpec::Source) || from_text == "it" {
            return format!(
                "Put its counters on {}",
                describe_choose_spec(&move_counters.to)
            );
        }
        return format!(
            "Move all counters from {} onto {}",
            from_text,
            describe_choose_spec(&move_counters.to)
        );
    }
    if let Some(proliferate) = effect.downcast_ref::<crate::effects::ProliferateEffect>() {
        if let Some(where_x) = describe_where_x_basis(&proliferate.count) {
            return format!("Proliferate X times, where X is {where_x}");
        }
        return match &proliferate.count {
            crate::effect::Value::Fixed(1) => "Proliferate".to_string(),
            crate::effect::Value::Fixed(2) => "Proliferate twice".to_string(),
            count => format!("Proliferate {} times", describe_value(count)),
        };
    }
    if let Some(reveal_from_hand) = effect.downcast_ref::<crate::effects::RevealFromHandEffect>() {
        return reveal_from_hand.cost_display();
    }
    if effect
        .downcast_ref::<crate::effects::RevealSourceFromHandEffect>()
        .is_some()
    {
        return "Reveal this card from your hand".to_string();
    }
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        let where_clause = choose_spec_dynamic_count_value_where_clause(&return_to_battlefield.target)
            .or_else(|| choose_spec_filter_where_x_clause(&return_to_battlefield.target))
            .unwrap_or_default();
        if matches!(return_to_battlefield.target.unhinted(), ChooseSpec::Source) {
            return append_battlefield_entry_counter_surface(
                format!(
                    "Return {} from your graveyard to the battlefield{}{where_clause}",
                    describe_choose_spec(&return_to_battlefield.target),
                    if return_to_battlefield.tapped {
                        " tapped"
                    } else {
                        ""
                    }
                ),
                &return_to_battlefield.enters_with_counters,
            );
        }
        if return_to_battlefield.target.source_reference_surface().is_some()
            && matches!(
                return_to_battlefield.target.unhinted(),
                ChooseSpec::Tagged(_) | ChooseSpec::Iterated
            )
        {
            return append_battlefield_entry_counter_surface(
                format!(
                    "Return {} to the battlefield{}{where_clause}",
                    describe_choose_spec(&return_to_battlefield.target),
                    if return_to_battlefield.tapped {
                        " tapped"
                    } else {
                        ""
                    }
                ),
                &return_to_battlefield.enters_with_counters,
            );
        }
        if let Some(owner) = graveyard_owner_from_spec(&return_to_battlefield.target) {
            let target_text =
                describe_choose_spec_without_graveyard_zone(&return_to_battlefield.target);
            let from_text = match owner {
                Some(owner) => format!(
                    "from {} graveyard",
                    describe_possessive_player_filter(&owner)
                ),
                None => "from graveyard".to_string(),
            };
            return append_battlefield_entry_counter_surface(
                format!(
                    "Return {} {} to the battlefield{}{where_clause}",
                    target_text,
                    from_text,
                    if return_to_battlefield.tapped {
                        " tapped"
                    } else {
                        ""
                    }
                ),
                &return_to_battlefield.enters_with_counters,
            );
        }
        return append_battlefield_entry_counter_surface(
            format!(
                "Return {} from graveyard to the battlefield{}{where_clause}",
                describe_choose_spec(&return_to_battlefield.target),
                if return_to_battlefield.tapped {
                    " tapped"
                } else {
                    ""
                }
            ),
            &return_to_battlefield.enters_with_counters,
        );
    }
    if let Some(return_all_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        return describe_return_all_to_battlefield_effect(return_all_to_battlefield);
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        if let Some(dynamic_for_each) = describe_draw_for_each_turn_history(draw) {
            return dynamic_for_each;
        }
        if value_prefers_where_x(&draw.count)
            && let Some(where_x) = describe_where_x_basis(&draw.count)
        {
            let player = describe_player_filter(&draw.player);
            return format!(
                "{player} {} X cards, where X is {where_x}",
                player_verb(&player, "draw", "draws")
            );
        }
        if let Some(dynamic_for_each) = describe_draw_for_each(draw) {
            return dynamic_for_each;
        }
        let player = describe_player_filter(&draw.player);
        return format!(
            "{player} {} {}",
            player_verb(&player, "draw", "draws"),
            describe_card_count(&draw.count)
        );
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawForEachTaggedMatchingEffect>() {
        let player = describe_player_filter(&draw.player);
        let counted = if draw.filter.zone == Some(Zone::Hand)
            && draw.filter.controller.is_none()
            && (draw.tag.as_str().starts_with("searched")
                || draw.tag.as_str().starts_with("exiled")
                || crate::cards::is_sentence_helper_tag(draw.tag.as_str(), "exiled"))
        {
            match &draw.filter.owner {
                Some(PlayerFilter::You) => "card exiled from your hand this way".to_string(),
                Some(PlayerFilter::NotYou) => {
                    "card exiled from your opponent's hand this way".to_string()
                }
                Some(PlayerFilter::Specific(_))
                | Some(PlayerFilter::Target(_))
                | Some(PlayerFilter::IteratedPlayer)
                | Some(PlayerFilter::ControllerOf(_)) => {
                    "card exiled from their hand this way".to_string()
                }
                Some(owner) => format!(
                    "card exiled from {} hand this way",
                    describe_possessive_player_filter(owner)
                ),
                None => "card exiled from hand this way".to_string(),
            }
        } else {
            describe_for_each_count_filter(&draw.filter)
        };
        return format!(
            "{player} {} a card for each {}",
            player_verb(&player, "draw", "draws"),
            counted
        );
    }
    if let Some(speed) = effect.downcast_ref::<crate::effects::IncreaseSpeedEffect>() {
        return format!(
            "{} speed increases by {}",
            describe_possessive_player_filter(&speed.player),
            describe_value(&speed.amount)
        );
    }
    if let Some(speed) = effect.downcast_ref::<crate::effects::ReduceSpeedEffect>() {
        return format!(
            "{} speed decreases by {}",
            describe_possessive_player_filter(&speed.player),
            describe_value(&speed.amount)
        );
    }
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        let player = describe_choose_spec(&gain.player);
        if let Some(multiplier) = counters_removed_this_way_multiplier(&gain.amount)
            && multiplier > 0
        {
            return format!(
                "{} {} {} life for each counter removed this way",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier
            );
        }
        if value_has_surface_hint(&gain.amount, ValueSurfaceHint::EqualTo) {
            let amount = gain
                .amount
                .clone()
                .without_surface_hint(ValueSurfaceHint::EqualTo);
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_value(&amount)
            );
        }
        if gain.amount.has_surface_hint(ValueSurfaceHint::ForEach) {
            let (basis, multiplier) = match gain.amount.unhinted() {
                Value::Add(left, right) if left == right => (left.as_ref(), 2),
                Value::Scaled(value, multiplier) if *multiplier > 0 => {
                    (value.as_ref(), *multiplier)
                }
                basis => (basis, 1),
            };
            if let Some(for_each) = describe_create_for_each_count(basis) {
                return format!(
                    "{} {} {} life for each {for_each}",
                    player,
                    player_verb(&player, "gain", "gains"),
                    multiplier
                );
            }
        }
        if let Some((amount, where_x)) = describe_where_x_offset_value(&gain.amount) {
            return format!(
                "{} {} {amount} life, where X is {where_x}",
                player,
                player_verb(&player, "gain", "gains")
            );
        }
        if value_prefers_where_x(&gain.amount)
            && let Some(where_x) = describe_where_x_basis(&gain.amount)
        {
            return format!(
                "{} {} X life, where X is {where_x}",
                player,
                player_verb(&player, "gain", "gains")
            );
        }
        if let Value::CountersOnSource(counter_type) = gain.amount.unhinted() {
            return format!(
                "{} {} 1 life for each {} counter on this permanent",
                player,
                player_verb(&player, "gain", "gains"),
                describe_counter_type(*counter_type)
            );
        }
        if let Value::Add(left, right) = gain.amount.unhinted()
            && let (Value::CountersOnSource(left_counter), Value::CountersOnSource(right_counter)) =
                (left.as_ref(), right.as_ref())
            && left_counter == right_counter
        {
            return format!(
                "{} {} 2 life for each {} counter on this permanent",
                player,
                player_verb(&player, "gain", "gains"),
                describe_counter_type(*left_counter)
            );
        }
        if let Some((party_filter, multiplier)) = party_size_multiplier(&gain.amount) {
            let party_owner = describe_possessive_player_filter(&party_filter);
            if multiplier <= 1 {
                return format!(
                    "{} {} 1 life for each creature in {} party",
                    player,
                    player_verb(&player, "gain", "gains"),
                    party_owner
                );
            }
            return format!(
                "{} {} {} life for each creature in {} party",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier,
                party_owner
            );
        }
        if let Some((filter, multiplier)) = basic_land_types_multiplier(&gain.amount) {
            let among = describe_basic_land_types_among(filter);
            if multiplier <= 1 {
                return format!(
                    "{} {} 1 life for each {}",
                    player,
                    player_verb(&player, "gain", "gains"),
                    among
                );
            }
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier,
                among
            );
        }
        if let Some((spells_filter, multiplier)) = spells_cast_this_turn_multiplier(&gain.amount) {
            let each = describe_spells_cast_this_turn_each(&spells_filter);
            if multiplier <= 1 {
                return format!(
                    "{} {} 1 life for each {}",
                    player,
                    player_verb(&player, "gain", "gains"),
                    each
                );
            }
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier,
                each
            );
        }
        if let Value::Count(filter) = gain.amount.unhinted() {
            return format!(
                "{} {} 1 life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_for_each_count_filter(filter)
            );
        }
        if let Value::CountScaled(filter, multiplier) = gain.amount.unhinted() {
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier,
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(gain.amount.unhinted(), Value::CreaturesDiedThisTurn) {
            return format!(
                "{} {} 1 life for each creature that died this turn",
                player,
                player_verb(&player, "gain", "gains")
            );
        }
        if value_is_source_exiled_mana_value(&gain.amount) {
            return format!(
                "{} {} life equal to its mana value",
                player,
                player_verb(&player, "gain", "gains")
            );
        }
        if let Value::ManaValueOf(spec) = gain.amount.unhinted() {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_dynamic_counter_basis(spec, "mana value")
            );
        }
        if matches!(
            gain.amount,
            Value::Add(_, _)
                | Value::CountScaled(_, _)
                | Value::TotalPower(_)
                | Value::TotalToughness(_)
                | Value::TotalManaValue(_)
                | Value::GreatestPower(_)
                | Value::GreatestToughness(_)
                | Value::GreatestManaValue(_)
                | Value::LeastPower(_)
                | Value::LeastToughness(_)
                | Value::LeastManaValue(_)
                | Value::SourcePower
                | Value::SourceToughness
                | Value::PowerOf(_)
                | Value::ToughnessOf(_)
                | Value::Speed(_)
                | Value::LifeTotal(_)
                | Value::LifeTotalAsTurnBegan(_)
                | Value::LifeTotalDifference(_)
                | Value::StartingLifeTotal(_)
                | Value::HalfLifeTotalRoundedUp(_)
                | Value::HalfLifeTotalRoundedDown(_)
                | Value::HalfStartingLifeTotalRoundedUp(_)
                | Value::HalfStartingLifeTotalRoundedDown(_)
                | Value::LifeGainedThisTurn(_)
                | Value::LifeLostThisTurn(_)
                | Value::DamageDealtToPlayersThisTurn(_)
                | Value::CardsDiscardedThisTurn(_)
                | Value::EffectMetric { .. }
                | Value::EffectMetricOffset { .. }
                | Value::PendingEffectMetric { .. }
                | Value::PendingEffectMetricOffset { .. }
        ) {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_value(&gain.amount)
            );
        }
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "gain", "gains"),
            describe_life_amount_phrase(&gain.amount)
        );
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantManaAbilityUntilEotEffect>() {
        let mut cost = describe_cost_list(grant.ability.mana_cost.costs());
        cost = lowercase_first(cost.trim());
        let cost = cost.trim_end_matches('.');
        let mana = grant
            .ability
            .mana_symbols()
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        let mana = if mana.is_empty() {
            "{0}".to_string()
        } else {
            mana
        };
        return format!(
            "Until end of turn, any time you could activate a mana ability, you may {cost}. If you do, add {mana}."
        );
    }
    if let Some(pay) = effect.downcast_ref::<crate::effects::PayLifeEffect>() {
        let player = describe_choose_spec(&pay.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "pay", "pays"),
            describe_life_amount_phrase(&pay.amount)
        );
    }
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        let player = describe_choose_spec(&lose.player);
        if value_has_surface_hint(&lose.amount, ValueSurfaceHint::EqualTo) {
            let amount = lose
                .amount
                .clone()
                .without_surface_hint(ValueSurfaceHint::EqualTo);
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_value(&amount)
            );
        }
        if lose.amount.has_surface_hint(ValueSurfaceHint::ForEach) {
            let (basis, multiplier) = match lose.amount.unhinted() {
                Value::Add(left, right) if left == right => (left.as_ref(), 2),
                basis => (basis, 1),
            };
            if let Some(for_each) = describe_create_for_each_count(basis) {
                return format!(
                    "{} {} {} life for each {for_each}",
                    player,
                    player_verb(&player, "lose", "loses"),
                    multiplier
                );
            }
        }
        if let ChooseSpec::Player(player_filter) = &lose.player
            && let Some(amount) =
                describe_half_life_amount_for_same_player(&lose.amount, player_filter)
        {
            return format!(
                "{} {} {amount}",
                player,
                player_verb(&player, "lose", "loses")
            );
        }
        if let Value::CountersOn(spec, Some(counter_type)) = lose.amount.unhinted() {
            return format!(
                "{} {} 1 life for each {} counter on {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_counter_type(*counter_type),
                describe_choose_spec(spec)
            );
        }
        if value_prefers_where_x(&lose.amount)
            && let Some(where_x) = describe_where_x_basis(&lose.amount)
        {
            return format!(
                "{} {} X life, where X is {where_x}",
                player,
                player_verb(&player, "lose", "loses")
            );
        }
        if let Value::CountersOnSource(counter_type) = lose.amount.unhinted() {
            return format!(
                "{} {} 1 life for each {} counter on this permanent",
                player,
                player_verb(&player, "lose", "loses"),
                describe_counter_type(*counter_type)
            );
        }
        if let Value::Add(left, right) = lose.amount.unhinted()
            && let (Value::CountersOnSource(left_counter), Value::CountersOnSource(right_counter)) =
                (left.as_ref(), right.as_ref())
            && left_counter == right_counter
        {
            return format!(
                "{} {} 2 life for each {} counter on this permanent",
                player,
                player_verb(&player, "lose", "loses"),
                describe_counter_type(*left_counter)
            );
        }
        if let Some((party_filter, multiplier)) = party_size_multiplier(&lose.amount) {
            let party_owner = describe_possessive_player_filter(&party_filter);
            if multiplier <= 1 {
                return format!(
                    "{} {} 1 life for each creature in {} party",
                    player,
                    player_verb(&player, "lose", "loses"),
                    party_owner
                );
            }
            return format!(
                "{} {} {} life for each creature in {} party",
                player,
                player_verb(&player, "lose", "loses"),
                multiplier,
                party_owner
            );
        }
        if let Some((spells_filter, multiplier)) = spells_cast_this_turn_multiplier(&lose.amount) {
            let each = describe_spells_cast_this_turn_each(&spells_filter);
            if multiplier <= 1 {
                return format!(
                    "{} {} 1 life for each {}",
                    player,
                    player_verb(&player, "lose", "loses"),
                    each
                );
            }
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                multiplier,
                each
            );
        }
        if let Value::Count(filter) = lose.amount.unhinted() {
            return format!(
                "{} {} 1 life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_for_each_count_filter(filter)
            );
        }
        if let Value::CountScaled(filter, multiplier) = lose.amount.unhinted() {
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                multiplier,
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(lose.amount.unhinted(), Value::CreaturesDiedThisTurn) {
            return format!(
                "{} {} 1 life for each creature that died this turn",
                player,
                player_verb(&player, "lose", "loses")
            );
        }
        if matches!(
            lose.amount,
            Value::SourcePower
                | Value::SourceToughness
                | Value::PowerOf(_)
                | Value::ToughnessOf(_)
                | Value::ManaValueOf(_)
                | Value::Speed(_)
                | Value::LifeGainedThisTurn(_)
                | Value::LifeLostThisTurn(_)
                | Value::DamageDealtToPlayersThisTurn(_)
        ) {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_value(&lose.amount)
            );
        }
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "lose", "loses"),
            describe_life_amount_phrase(&lose.amount)
        );
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        let player = describe_player_filter(&discard.player);
        let random_suffix = if discard.random { " at random" } else { "" };
        if discard_count_covers_entire_hand(discard) {
            let possessive = if discard.player == PlayerFilter::You {
                "your"
            } else {
                "their"
            };
            return format!(
                "{} {} all the cards in {possessive} hand",
                player,
                player_verb(&player, "discard", "discards")
            );
        }
        if !discard.any_number
            && discard.count.has_surface_hint(ValueSurfaceHint::ForEach)
            && let Some(for_each) = describe_create_for_each_count(&discard.count)
        {
            return format!(
                "{} {} a card for each {for_each}{random_suffix}",
                player,
                player_verb(&player, "discard", "discards")
            );
        }
        if !discard.any_number
            && additional_cost_color_discard_surface(
                &discard.count,
                discard.card_filter.as_ref(),
            )
            .is_none()
            && let Some(where_x) = describe_where_x_basis(&discard.count)
        {
            return format!(
                "{} {} X cards{random_suffix}, where X is {where_x}",
                player,
                player_verb(&player, "discard", "discards")
            );
        }
        let discard_count = discard_sequence_count(discard);
        return format!(
            "{} {} {}{}",
            player,
            player_verb(&player, "discard", "discards"),
            discard_count,
            random_suffix
        );
    }
    if let Some(discard_hand) = effect.downcast_ref::<crate::effects::DiscardHandEffect>() {
        let player = describe_player_filter(&discard_hand.player);
        let hand = if player == "you" {
            "your hand"
        } else {
            "their hand"
        };
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "discard", "discards"),
            hand
        );
    }
    if let Some(add_mana) = effect.downcast_ref::<crate::effects::AddManaEffect>() {
        let mana = add_mana
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        if matches!(add_mana.player, PlayerFilter::ChosenPlayer) {
            return format!(
                "A player of your choice adds {}",
                if mana.is_empty() { "{0}" } else { &mana }
            );
        }
        if !matches!(add_mana.player, PlayerFilter::You) {
            let player = describe_player_filter(&add_mana.player);
            return format!(
                "{} {} {}",
                player,
                player_verb(&player, "add", "adds"),
                if mana.is_empty() { "{0}" } else { &mana }
            );
        }
        return format!(
            "Add {}{}",
            if mana.is_empty() { "{0}" } else { &mana },
            describe_add_mana_destination_suffix(&add_mana.player)
        );
    }
    if let Some(add_colorless) = effect.downcast_ref::<crate::effects::AddColorlessManaEffect>() {
        return format!(
            "Add {} colorless mana{}",
            describe_value(&add_colorless.amount),
            describe_add_mana_destination_suffix(&add_colorless.player)
        );
    }
    if let Some(add_scaled) = effect.downcast_ref::<crate::effects::AddScaledManaEffect>() {
        let mana = add_scaled
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        let mana_text = if mana.is_empty() { "{0}" } else { &mana };
        let amount = add_scaled.amount.unhinted();
        if add_scaled
            .amount
            .has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay)
            && matches!(add_scaled.amount.unhinted(), Value::X)
        {
            return format!(
                "Add {mana_text} for each counter removed this way{}",
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::Count(filter) = amount {
            let has_tagged_shared_subtype = filter.tagged_constraints.iter().any(|constraint| {
                matches!(constraint.tag.as_str(), "__it__" | "triggering")
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::SharesSubtypeWithTagged
            });
            if has_tagged_shared_subtype {
                let count_subject = pluralize_noun_phrase(&describe_for_each_count_filter(filter))
                    .replace(" that shares ", " that share ")
                    .replace(" that object", " it");
                return format!(
                    "Add an amount of {} equal to the number of {}{}",
                    mana_text,
                    count_subject,
                    describe_add_mana_destination_suffix(&add_scaled.player)
                );
            }
            return format!(
                "Add {} for each {}{}",
                mana_text,
                describe_for_each_count_filter(filter),
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::CountersOnSource(counter_type) = amount {
            return format!(
                "Add {} for each {} counter on this source{}",
                mana_text,
                describe_counter_type(*counter_type),
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::CountersOn(spec, Some(counter_type)) = amount {
            return format!(
                "Add {} for each {} counter on {}{}",
                mana_text,
                describe_counter_type(*counter_type),
                describe_choose_spec(spec),
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::CountersOn(spec, None) = amount {
            return format!(
                "Add {} for each counter on {}{}",
                mana_text,
                describe_choose_spec(spec),
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Some((party_filter, multiplier)) = party_size_multiplier(amount) {
            let party_owner = describe_possessive_player_filter(&party_filter);
            if multiplier <= 1 {
                return format!(
                    "Add {} for each creature in {} party{}",
                    mana_text,
                    party_owner,
                    describe_add_mana_destination_suffix(&add_scaled.player)
                );
            }
            return format!(
                "Add {} {} times for each creature in {} party{}",
                mana_text,
                multiplier,
                party_owner,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::Devotion { player, color } = amount {
            let color_name = color.name().to_string();
            return format!(
                "Add an amount of {} equal to {} devotion to {}",
                mana_text,
                describe_possessive_player_filter(player),
                color_name
            );
        }
        if let Value::LifeLostThisTurn(player) = amount {
            let life_text = match player {
                PlayerFilter::You => "for each 1 life you have lost this turn".to_string(),
                PlayerFilter::Opponent => {
                    "for each 1 life your opponents have lost this turn".to_string()
                }
                _ => format!(
                    "for each 1 life {} lost this turn",
                    describe_player_filter(player)
                ),
            };
            return format!(
                "Add {} {}{}",
                mana_text,
                life_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::MaxCardsDrawnThisTurn(player) = amount {
            let drawn_text = match player {
                PlayerFilter::You => "for each card you've drawn this turn".to_string(),
                PlayerFilter::Opponent => {
                    "for each card the opponent who drew the most cards has drawn this turn"
                        .to_string()
                }
                _ => format!("equal to {}", describe_value(amount)),
            };
            return format!(
                "Add {} {}{}",
                mana_text,
                drawn_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::PowerOf(spec) = amount {
            return format!(
                "Add an amount of {} equal to the power of {}{}",
                mana_text,
                describe_choose_spec(spec),
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::ManaValueOf(spec) = amount {
            let amount_text = if add_scaled.amount.surface_hints().iter().any(|hint| {
                matches!(hint, ValueSurfaceHint::SacrificedObject(_))
            }) {
                describe_value(&add_scaled.amount)
            } else {
                let described = describe_choose_spec(spec);
                if described == "it" {
                    "its mana value".to_string()
                } else {
                    format!("the mana value of {described}")
                }
            };
            return format!(
                "Add an amount of {} equal to {}{}",
                mana_text,
                amount_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if is_effect_count_reference(amount, None) {
            return format!(
                "Add that much {}{}",
                mana_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if matches!(
            amount,
            Value::EventValue(EventValueSpec::LifeAmount)
        ) {
            return format!(
                "Add that much {}{}",
                mana_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Some(offset) = effect_count_reference_offset(amount, None) {
            let amount_text = if offset == 0 {
                "that much".to_string()
            } else if offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            };
            return format!(
                "Add {} {}{}",
                amount_text,
                mana_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        if let Value::EventValueOffset(EventValueSpec::LifeAmount, offset) = amount {
            let amount_text = if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            };
            return format!(
                "Add {} {}{}",
                amount_text,
                mana_text,
                describe_add_mana_destination_suffix(&add_scaled.player)
            );
        }
        return format!(
            "Add an amount of {} equal to {}{}",
            mana_text,
            describe_value(amount),
            describe_add_mana_destination_suffix(&add_scaled.player)
        );
    }
    if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        if mill.player == PlayerFilter::You {
            if mill.count == Value::X {
                return "Mill X cards".to_string();
            }
            if value_prefers_where_x(&mill.count)
                && let Some(where_x) =
                    describe_mill_where_x_basis_for_player(&mill.count, &mill.player)
            {
                return format!("Mill X cards, where X is {where_x}");
            }
            let count_text = describe_mill_count_for_player(&mill.count, &mill.player);
            if let Some(rest) = count_text.strip_prefix("the number of ") {
                let basis = singularize_for_each_basis(rest.strip_suffix(" cards").unwrap_or(rest));
                return format!("Mill a card for each {basis}");
            }
            return format!("Mill {count_text}");
        }
        let player = describe_player_filter(&mill.player);
        if mill.count == Value::X {
            return format!(
                "{} {} X cards",
                player,
                player_verb(&player, "mill", "mills")
            );
        }
        if value_prefers_where_x(&mill.count)
            && let Some(where_x) =
                describe_mill_where_x_basis_for_player(&mill.count, &mill.player)
        {
            return format!(
                "{} {} X cards, where X is {where_x}",
                player,
                player_verb(&player, "mill", "mills")
            );
        }
        let count_text = describe_mill_count_for_player(&mill.count, &mill.player);
        if let Some(rest) = count_text.strip_prefix("the number of ") {
            let basis = singularize_for_each_basis(rest.strip_suffix(" cards").unwrap_or(rest));
            return format!(
                "{} {} a card for each {}",
                player,
                player_verb(&player, "mill", "mills"),
                basis
            );
        }
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "mill", "mills"),
            count_text
        );
    }
    if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
        if let Some(text) = describe_dynamic_count_tap(tap) {
            return text;
        }
        let where_clause = choose_spec_dynamic_count_value_where_clause(&tap.target)
            .or_else(|| choose_spec_filter_where_x_clause(&tap.target))
            .unwrap_or_default();
        return format!("Tap {}{where_clause}", describe_choose_spec(&tap.target));
    }
    if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        let where_clause = choose_spec_dynamic_count_value_where_clause(&untap.target)
            .or_else(|| choose_spec_filter_where_x_clause(&untap.target))
            .unwrap_or_default();
        return format!("Untap {}{where_clause}", describe_choose_spec(&untap.target));
    }
    if let Some(phase_out) = effect.downcast_ref::<crate::effects::PhaseOutEffect>() {
        if let ChooseSpec::All(filter) = phase_out.spec.base()
            && filter
                .static_abilities
                .contains(&crate::static_abilities::StaticAbilityId::Phasing)
        {
            let mut without_phasing = filter.clone();
            without_phasing
                .static_abilities
                .retain(|ability| *ability != crate::static_abilities::StaticAbilityId::Phasing);
            let desc = describe_choose_spec(&ChooseSpec::All(without_phasing));
            return format!("{} with phasing phase out", capitalize_first(&desc));
        }
        let target = describe_choose_spec(&phase_out.spec);
        if phase_out.duration == crate::effects::PhaseOutDuration::UntilSourceLeaves {
            let source = phase_out
                .source_surface
                .as_ref()
                .map(crate::target::SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this permanent".to_string());
            return format!("Phase out {target} until {source} leaves the battlefield");
        }
        if target == "equipped creature" {
            return "Equipped creature phases out".to_string();
        }
        return format!("Phase out {target}");
    }
    if let Some(phase_in) = effect.downcast_ref::<crate::effects::PhaseInEffect>() {
        if matches!(phase_in.spec.base(), ChooseSpec::All(_)) {
            let desc = describe_choose_spec(&phase_in.spec);
            let base = desc.strip_prefix("all ").unwrap_or(desc.as_str());
            return format!("Phase in all phased-out {base}");
        }
        return format!("Phase in {}", describe_choose_spec(&phase_in.spec));
    }
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachToEffect>() {
        return format!(
            "Attach this source to {}",
            describe_choose_spec(&attach.target)
        );
    }
    if let Some(reconfigure) = effect.downcast_ref::<crate::effects::ReconfigureEffect>() {
        return format!(
            "Attach this source to {} or unattach it",
            describe_choose_spec(&reconfigure.target)
        );
    }
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
        let triggering_tag = TagKey::from("triggering");
        if choose_spec_references_exact_tag(&attach.objects, &triggering_tag)
            && matches!(&attach.target, ChooseSpec::Tagged(tag) if tag.as_str().starts_with("created_"))
        {
            return "Attach it to the token".to_string();
        }
        let target = describe_choose_spec(&attach.target);
        let target = if attach.target.is_target() {
            target
        } else {
            target
                .strip_suffix(" of your choice")
                .unwrap_or(&target)
                .to_string()
        };
        return format!(
            "Attach {} to {}",
            describe_attach_objects_spec(&attach.objects),
            target
        );
    }
    if let Some(unattach) = effect.downcast_ref::<crate::effects::UnattachObjectsEffect>() {
        if let Some(text) = describe_unattach_all_equipment_from_tagged(&unattach.objects) {
            return text;
        }
        return format!("Unattach {}", describe_choose_spec(&unattach.objects));
    }
    if let Some(sacrifice) = sacrifice_view(effect) {
        return describe_sacrifice_effect(sacrifice);
    }
    if let Some(sacrifice_target) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
        if sacrifice_target.target.is_target()
            && sacrifice_target.target.count().is_single()
            && matches!(
                sacrifice_target.target.base(),
                ChooseSpec::Object(filter) if filter.controller.is_none()
            )
        {
            return format!(
                "{}'s controller sacrifices it",
                capitalize_first(&describe_choose_spec(&sacrifice_target.target))
            );
        }
        return format!(
            "Sacrifice {}",
            describe_choose_spec(&sacrifice_target.target)
        );
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        if let Some(surface) = &return_to_hand.exiled_with_source_surface {
            return describe_exiled_with_source_move(
                surface,
                Zone::Hand,
                return_to_hand.destination_player_surface.as_ref(),
                None,
                false,
            );
        }
        let contextual_hand = return_to_hand
            .destination_player_surface
            .as_ref()
            .map(|player| format!("{} hand", describe_possessive_player_filter(player)));
        if let ChooseSpec::Target(inner) = &return_to_hand.spec
            && let ChooseSpec::Object(filter) = inner.as_ref()
            && filter.zone == Some(Zone::Exile)
            && filter.owner == Some(PlayerFilter::You)
            && filter.alternative_cast == Some(crate::filter::AlternativeCastKind::Flashback)
        {
            return "Return target exiled card with flashback you own to your hand".to_string();
        }
        if let ChooseSpec::All(filter) = &return_to_hand.spec {
            if let Some(text) = source_and_source_exiled_return_text(filter) {
                return text;
            }
            if is_source_exiled_cards_filter(filter) {
                return "Return the exiled cards to their owners' hands".to_string();
            }
        }
        if let ChooseSpec::Object(filter) = &return_to_hand.spec
            && let Some(text) = source_and_source_exiled_return_text(filter)
        {
            return text;
        }
        if let Some(owner) = graveyard_owner_from_spec(&return_to_hand.spec) {
            let target_text = describe_choose_spec_without_graveyard_zone(&return_to_hand.spec);
            let from_text = match &owner {
                Some(owner) => format!(
                    "{} graveyard",
                    describe_possessive_graveyard_owner_filter(owner)
                ),
                None => "a graveyard".to_string(),
            };
            let to_text = contextual_hand.clone().unwrap_or_else(|| match &owner {
                Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
                None => owner_hand_phrase_for_spec(&return_to_hand.spec).to_string(),
            });
            return format!("Return {target_text} from {from_text} to {to_text}");
        }
        if contextual_hand.is_none() && is_you_owned_battlefield_object_spec(&return_to_hand.spec) {
            return format!(
                "Return {} to your hand",
                describe_choose_spec(&return_to_hand.spec)
            );
        }
        if let Some(exception_text) = describe_return_to_hand_excluded_subtypes(return_to_hand) {
            return format!("Return {exception_text}");
        }
        let where_clause = choose_spec_dynamic_count_value_where_clause(&return_to_hand.spec)
            .or_else(|| choose_spec_filter_where_x_clause(&return_to_hand.spec))
            .unwrap_or_default();
        let target_text = if matches!(return_to_hand.spec.base(), ChooseSpec::Tagged(_)) {
            return_to_hand
                .set_reference_surface
                .clone()
                .or_else(|| {
                    matches!(
                        return_to_hand.set_quantifier_surface,
                        Some(ironsmith_core::SetQuantifierSurface::Each)
                    )
                    .then(|| "each of them".to_string())
                })
                .unwrap_or_else(|| describe_choose_spec(&return_to_hand.spec))
        } else {
            describe_choose_spec(&return_to_hand.spec)
        };
        return format!(
            "Return {} to {}{}",
            target_text,
            contextual_hand
                .as_deref()
                .unwrap_or_else(|| owner_hand_phrase_for_spec(&return_to_hand.spec)),
            where_clause
        );
    }
    if let Some(return_from_gy) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        let contextual_graveyard = return_from_gy
            .graveyard_player_surface
            .as_ref()
            .map(|player| format!("{} graveyard", describe_possessive_player_filter(player)));
        let contextual_hand = return_from_gy
            .destination_player_surface
            .as_ref()
            .map(|player| format!("{} hand", describe_possessive_player_filter(player)));
        let random_suffix = if return_from_gy.random {
            " at random"
        } else {
            ""
        };
        let where_clause = choose_spec_dynamic_count_value_where_clause(&return_from_gy.target)
            .unwrap_or_default();
        if matches!(return_from_gy.target.base(), ChooseSpec::Source) {
            return format!(
                "Return this card{random_suffix} from {} to {}{where_clause}",
                contextual_graveyard.as_deref().unwrap_or("a graveyard"),
                contextual_hand.as_deref().unwrap_or("its owner's hand"),
            );
        }
        if let Some(owner) = graveyard_owner_from_spec(&return_from_gy.target) {
            let target_text = describe_choose_spec_without_graveyard_zone(&return_from_gy.target);
            let from_text = match &owner {
                Some(owner) => format!(
                    "{} graveyard",
                    describe_possessive_graveyard_owner_filter(owner)
                ),
                None => "a graveyard".to_string(),
            };
            let to_text = contextual_hand.clone().unwrap_or_else(|| match &owner {
                Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
                None => owner_hand_phrase_for_spec(&return_from_gy.target).to_string(),
            });
            return format!(
                "Return {target_text}{random_suffix} from {from_text} to {to_text}{where_clause}"
            );
        }
        return format!(
            "Return {}{} from {} to {}{}",
            describe_choose_spec_without_graveyard_zone(&return_from_gy.target),
            random_suffix,
            contextual_graveyard.as_deref().unwrap_or("a graveyard"),
            contextual_hand
                .as_deref()
                .unwrap_or_else(|| owner_hand_phrase_for_spec(&return_from_gy.target)),
            where_clause
        );
    }
    if let Some(shuffle_library) = effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>() {
        if matches!(
            &shuffle_library.player,
            PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)
        ) {
            return "Target player shuffles".to_string();
        }
        return format!(
            "Shuffle {} library",
            describe_possessive_player_filter(&shuffle_library.player)
        );
    }
    if let Some(shuffle_hand_and_gy) =
        effect.downcast_ref::<crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect>()
    {
        let subject = describe_player_filter(&shuffle_hand_and_gy.player);
        let verb = player_verb(&subject, "shuffle", "shuffles");
        let possessive = if subject == "you" { "your" } else { "their" };
        let objects = if shuffle_hand_and_gy.include_owned_permanents {
            let owner = if subject == "you" { "you" } else { "they" };
            format!("{} hand, graveyard, and all permanents {owner} own", possessive)
        } else {
            format!("{} hand and graveyard", possessive)
        };
        return format!("{subject} {verb} {objects} into {possessive} library");
    }
    if let Some(shuffle_gy) =
        effect.downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()
    {
        if matches!(
            &shuffle_gy.player,
            PlayerFilter::Target(inner) if matches!(inner.as_ref(), PlayerFilter::Any)
        ) {
            return "Target player shuffles their graveyard into their library".to_string();
        }
        let possessive = describe_possessive_player_filter(&shuffle_gy.player);
        return format!("Shuffle all cards from {possessive} graveyard into {possessive} library");
    }
    if let Some(shuffle_objects) =
        effect.downcast_ref::<crate::effects::ShuffleObjectsIntoLibraryEffect>()
    {
        if shuffle_objects.owner_library_destination {
            let (target_text, singular) = match shuffle_objects.target.base() {
                ChooseSpec::Source | ChooseSpec::Tagged(_) => ("it".to_string(), true),
                _ => (
                    describe_choose_spec(&shuffle_objects.target),
                    shuffle_objects.target.is_single(),
                ),
            };
            let destination = if singular {
                "its owner's library"
            } else {
                "their owners' libraries"
            };
            return format!("Shuffle {target_text} into {destination}");
        }
        if matches!(shuffle_objects.target.base(), ChooseSpec::Source)
            && matches!(
                &shuffle_objects.player,
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
            )
        {
            return "This creature's owner shuffles it into their library".to_string();
        }
        if shuffle_objects.target.is_single()
            && matches!(
                &shuffle_objects.player,
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target)
            )
        {
            return format!(
                "The owner of {} shuffles it into their library",
                describe_choose_spec(&shuffle_objects.target)
            );
        }
        if matches!(shuffle_objects.target.base(), ChooseSpec::Source)
            && matches!(&shuffle_objects.player, PlayerFilter::You)
        {
            return "Shuffle it into its owner's library".to_string();
        }
        if let (
            ChooseSpec::Tagged(target_tag),
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::Tagged(owner_tag)),
        ) = (&shuffle_objects.target, &shuffle_objects.player)
            && target_tag == owner_tag
        {
            return "Shuffle it into its owner's library".to_string();
        }
        if let Some(owner) = graveyard_owner_from_spec(&shuffle_objects.target) {
            let owner_matches = match &owner {
                Some(owner) => owner == &shuffle_objects.player,
                None => true,
            };
            if owner_matches {
                let target_text =
                    describe_choose_spec_without_graveyard_zone(&shuffle_objects.target);
                let from_text = match &owner {
                    Some(owner) => {
                        format!(
                            "{} graveyard",
                            describe_possessive_graveyard_owner_filter(owner)
                        )
                    }
                    None => "a graveyard".to_string(),
                };
                let library = match &owner {
                    Some(owner) => {
                        format!("{} library", describe_possessive_player_filter(owner))
                    }
                    None => owner_library_phrase_for_spec(&shuffle_objects.target).to_string(),
                };
                return format!("Shuffle {target_text} from {from_text} into {library}");
            }
        }
        return format!(
            "Shuffle {} into {} library",
            describe_choose_spec(&shuffle_objects.target),
            describe_possessive_player_filter(&shuffle_objects.player)
        );
    }
    if let Some(reorder_gy) = effect.downcast_ref::<crate::effects::ReorderGraveyardEffect>() {
        return format!(
            "Reorder {} graveyard as you choose",
            describe_possessive_player_filter(&reorder_gy.player)
        );
    }
    if effect
        .downcast_ref::<crate::effects::ReorderLibraryTopEffect>()
        .is_some()
    {
        return "Put them back in any order".to_string();
    }
    if let Some(search_library) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
        let library_position = || {
            search_library
                .library_position_from_top
                .as_ref()
                .map(|position| library_position_from_top_text(position, true))
                .unwrap_or_else(|| "on top".to_string())
        };
        let destination = match search_library.destination {
            Zone::Hand => "into hand".to_string(),
            Zone::Battlefield => "onto the battlefield".to_string(),
            Zone::Library => format!("{} of library", library_position()),
            Zone::Graveyard => "into their graveyard".to_string(),
            Zone::Exile => "into exile".to_string(),
            Zone::Stack => "onto the stack".to_string(),
            Zone::Command => "into the command zone".to_string(),
            Zone::Ante => "into ante".to_string(),
            Zone::OutsideGame => "outside the game".to_string(),
        };
        let mut display_filter = search_library.filter.clone();
        if display_filter.owner.as_ref() == Some(&search_library.player) {
            display_filter.owner = None;
        }
        if display_filter.zone == Some(Zone::Library) {
            display_filter.zone = None;
        }
        let filter_desc = if is_generic_owned_card_search_filter(&display_filter) {
            "a card".to_string()
        } else {
            describe_single_search_filter_in_zone(&display_filter, Zone::Library)
        };
        let chooser = describe_player_filter(&search_library.chooser);
        let chooser_searches_own_library = search_library.chooser == search_library.player
            && search_library.chooser != PlayerFilter::You;
        let search_prefix = if chooser_searches_own_library {
            format!(
                "{} searches their library",
                capitalize_first(&chooser)
            )
        } else if search_library.chooser == PlayerFilter::You {
            format!(
                "Search {} library",
                describe_possessive_player_filter(&search_library.player)
            )
        } else {
            format!(
                "{} searches {} library",
                capitalize_first(&chooser),
                describe_possessive_player_filter(&search_library.player)
            )
        };
        if search_library.destination == Zone::Library {
            let reveal_clause = if search_library.reveal {
                if chooser_searches_own_library {
                    ", reveals it"
                } else {
                    ", reveal it"
                }
            } else {
                ""
            };
            let finish = if chooser_searches_own_library {
                "then shuffles and puts"
            } else {
                "then shuffle and put"
            };
            return format!(
                "{search_prefix} for {filter_desc}{reveal_clause}, {finish} {} {}",
                search_library.result_reference_surface.as_str(),
                library_position()
            );
        }
        if search_library.reveal && search_library.destination != Zone::Battlefield {
            let verbs = if chooser_searches_own_library {
                ("reveals", "puts", "shuffles")
            } else {
                ("reveal", "put", "shuffle")
            };
            return format!(
                "{search_prefix} for {filter_desc}, {} it, {} it {destination}, then {}",
                verbs.0, verbs.1, verbs.2
            );
        }
        let verbs = if chooser_searches_own_library {
            ("puts", "shuffles")
        } else {
            ("put", "shuffle")
        };
        return format!(
            "{search_prefix} for {filter_desc}, {} it {destination}, then {}",
            verbs.0, verbs.1
        );
    }
    if let Some(search_slots) = effect.downcast_ref::<crate::effects::SearchLibrarySlotsEffect>() {
        let destination = match search_slots.destination {
            Zone::Hand => "into your hand",
            Zone::Battlefield => "onto the battlefield",
            Zone::Library => "on top of your library",
            Zone::Graveyard => "into your graveyard",
            Zone::Exile => "into exile",
            Zone::Stack => "onto the stack",
            Zone::Command => "into the command zone",
            Zone::Ante => "into ante",
            Zone::OutsideGame => "outside the game",
        };
        let multi_zone = search_slots
            .slots
            .iter()
            .any(|slot| slot.filter.zone.is_none());
        let search_origin = if multi_zone {
            format!(
                "{} library and graveyard",
                describe_possessive_player_filter(&search_slots.player)
            )
        } else {
            format!(
                "{} library",
                describe_possessive_player_filter(&search_slots.player)
            )
        };
        if let Some(selection) = describe_basic_land_type_search_slots(search_slots) {
            if search_slots.reveal {
                return format!(
                    "Search {search_origin} for {selection}, reveal those cards, put them {destination}, then shuffle"
                );
            }
            return format!(
                "Search {search_origin} for {selection}, put them {destination}, then shuffle"
            );
        }
        let selections: Vec<String> = search_slots
            .slots
            .iter()
            .map(|slot| {
                let mut display_filter = slot.filter.clone();
                if display_filter.owner.as_ref() == Some(&search_slots.player) {
                    display_filter.owner = None;
                }
                if display_filter.zone == Some(Zone::Library) {
                    display_filter.zone = None;
                }
                let mut selection = display_filter.description();
                if let Some(head) = selection.strip_suffix(" card") {
                    let first_word = head.split_whitespace().next().unwrap_or_default();
                    if !matches!(
                        first_word,
                        "a" | "an" | "the" | "target" | "another" | "each" | "all" | "up" | "any"
                    ) {
                        selection = head.to_string();
                    }
                }
                let selection = title_case_named_card_selection(&selection);
                describe_search_selection_with_cards(&selection)
            })
            .collect();
        let joined = join_with_and(&selections);
        if search_slots.reveal {
            let separator = if selections.len() >= 3 && search_slots.destination == Zone::Hand {
                ". "
            } else {
                ", "
            };
            let reveal_clause = if separator == ". " {
                "Reveal those cards"
            } else if multi_zone && selections.len() == 2 {
                "reveal them"
            } else {
                "reveal those cards"
            };
            return format!(
                "Search {search_origin} for {}{}{reveal_clause}, put them {}, then shuffle",
                joined, separator, destination
            );
        }
        return format!(
            "Search {search_origin} for {}, put them {}, then shuffle",
            joined, destination
        );
    }
    if effect
        .downcast_ref::<crate::effects::RevealTaggedEffect>()
        .is_some()
    {
        return "Reveal it".to_string();
    }
    if let Some(reveal_top) = effect.downcast_ref::<crate::effects::RevealTopEffect>() {
        // Revealing the top card is the semantic action; internal tag keys are
        // scaffolding for later "it/that card" references and should not leak
        // into compiled text.
        //
        // For "you", oracle text is typically imperative ("Reveal ..."). For other players,
        // oracle text typically uses a subject ("defending player reveals ...").
        if reveal_top.player == PlayerFilter::You {
            return "Reveal the top card of your library".to_string();
        }
        let mut subject = describe_player_filter(&reveal_top.player);
        if matches!(
            reveal_top.player,
            PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::DamagedPlayer
        ) {
            if let Some(rest) = subject.strip_prefix("the ") {
                subject = rest.to_string();
            }
        }
        let verb = player_verb(&subject, "reveal", "reveals");
        let pronoun = if subject == "you" { "your" } else { "their" };
        return format!("{subject} {verb} the top card of {pronoun} library");
    }
    if let Some(look_at_top) = effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>() {
        let owner = if look_at_top.player == PlayerFilter::DamagedPlayer {
            "that player's".to_string()
        } else {
            describe_possessive_player_filter(&look_at_top.player)
        };
        if matches!(
            look_at_top.count.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
        ) {
            let top_phrase = format!("that many cards from the top of {owner} library");
            if look_at_top.reveal {
                if look_at_top.player == PlayerFilter::You {
                    return "Reveal that many cards from the top of your library".to_string();
                }
                let subject = describe_player_filter(&look_at_top.player);
                let verb = player_verb(&subject, "reveal", "reveals");
                return format!("{subject} {verb} {top_phrase}");
            }
            return format!("Look at {top_phrase}");
        }
        let (count_text, noun, where_clause) =
            describe_top_count_noun_and_where_clause(&look_at_top.count);
        let top_phrase = if look_at_top.count == Value::Fixed(1) {
            format!("top {noun}")
        } else {
            format!("top {count_text} {noun}")
        };
        if look_at_top.reveal {
            if look_at_top.player == PlayerFilter::You {
                return format!("Reveal the {top_phrase} of your library{where_clause}");
            }
            let subject = describe_player_filter(&look_at_top.player);
            let verb = player_verb(&subject, "reveal", "reveals");
            return format!("{subject} {verb} the {top_phrase} of {owner} library{where_clause}");
        }
        return format!("Look at the {top_phrase} of {owner} library{where_clause}");
    }
    if let Some(rearrange) =
        effect.downcast_ref::<crate::effects::RearrangeLookedCardsInLibraryEffect>()
    {
        let count_text = match (rearrange.count.min, rearrange.count.max) {
            (0, Some(1)) => "up to one".to_string(),
            (min, Some(max)) if min == max => {
                small_number_word(max as u32).unwrap_or_else(|| max.to_string())
            }
            (0, Some(max)) => format!("up to {max}"),
            (min, Some(max)) => format!("between {min} and {max}"),
            (min, None) => format!("at least {min}"),
        };
        return format!(
            "Put {count_text} of those cards on top of your library and the rest on the bottom of your library in any order"
        );
    }
    if let Some(look_at_hand) = effect.downcast_ref::<crate::effects::LookAtHandEffect>() {
        if look_at_hand.reveal {
            if matches!(
                look_at_hand.target.base(),
                crate::target::ChooseSpec::Player(PlayerFilter::You)
            ) {
                return "Reveal your hand".to_string();
            }
            let subject = capitalize_first(&describe_choose_spec(&look_at_hand.target));
            let reveal_verb = player_verb(&subject, "reveal", "reveals");
            return format!("{subject} {reveal_verb} their hand");
        }
        let owner = describe_possessive_choose_spec(&look_at_hand.target);
        return format!("Look at {owner} hand");
    }
    if let Some(look_at_objects) = effect.downcast_ref::<crate::effects::LookAtObjectsEffect>() {
        // "Look at any face-down creatures they control" — a target-player
        // face-down creature scope reads as a pronoun back-reference.
        let targets_player_face_down = look_at_objects.filter.face_down == Some(true)
            && look_at_objects.filter.card_types == [CardType::Creature]
            && matches!(
                &look_at_objects.filter.controller,
                Some(PlayerFilter::Target(_))
            );
        if targets_player_face_down {
            return "Look at any face-down creatures they control".to_string();
        }
        return format!("Look at {}", look_at_objects.filter.description());
    }
    if let Some(apply_continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if let Some(text) = describe_apply_continuous_effect(apply_continuous) {
            return text;
        }
    }
    if let Some(grant_all) = effect.downcast_ref::<crate::effects::GrantAbilitiesAllEffect>() {
        if let Some(text) = describe_attack_block_if_able_grant(
            &grant_all.abilities,
            &grant_all.duration,
            &grant_all.filter.description(),
        ) {
            return text;
        }
        let self_subject = granted_ability_self_subject_for_filter(&grant_all.filter);
        return format!(
            "{} gains {} {}",
            grant_all.filter.description(),
            grant_all
                .abilities
                .iter()
                .map(|ability| {
                    ability
                        .granted_inline_ability()
                        .map(|inline| describe_granted_ability_phrase(inline, self_subject))
                        .unwrap_or_else(|| {
                            strip_redundant_granted_subject(
                                lowercase_first(&ability.display()),
                                self_subject,
                            )
                        })
                })
                .collect::<Vec<_>>()
                .join(", "),
            describe_until(&grant_all.duration)
        );
    }
    if let Some(grant_target) = effect.downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
    {
        if let Some(text) = describe_attack_block_if_able_grant(
            &grant_target.abilities,
            &grant_target.duration,
            &describe_choose_spec(&grant_target.target),
        ) {
            return text;
        }
        let self_subject = granted_ability_self_subject_for_choose_spec(&grant_target.target);
        return format!(
            "{} gains {} {}",
            describe_choose_spec(&grant_target.target),
            grant_target
                .abilities
                .iter()
                .map(|ability| {
                    ability
                        .granted_inline_ability()
                        .map(|inline| describe_granted_ability_phrase(inline, self_subject))
                        .unwrap_or_else(|| {
                            strip_redundant_granted_subject(
                                lowercase_first(&ability.display()),
                                self_subject,
                            )
                        })
                })
                .collect::<Vec<_>>()
                .join(", "),
            describe_until(&grant_target.duration)
        );
    }
    if let Some(grant_object) = effect.downcast_ref::<crate::effects::GrantObjectAbilityEffect>() {
        let self_subject = granted_ability_self_subject_for_choose_spec(&grant_object.target);
        return format!(
            "Grant {} to {}",
            describe_inline_ability_with_self_subject(&grant_object.ability, self_subject),
            describe_choose_spec(&grant_object.target)
        );
    }
    if let Some(modify_pt) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        if let Some(scale_text) = describe_dynamic_pt_scale_action(
            &modify_pt.target,
            &modify_pt.power,
            &modify_pt.toughness,
            &modify_pt.duration,
        ) {
            return scale_text;
        }
        let power_text = describe_where_x_basis(&modify_pt.power)
            .unwrap_or_else(|| describe_value(&modify_pt.power));
        let toughness_text = describe_where_x_basis(&modify_pt.toughness)
            .unwrap_or_else(|| describe_value(&modify_pt.toughness));
        let for_each_text = match (&modify_pt.power, &modify_pt.toughness) {
            (Value::Count(power_filter), Value::Count(toughness_filter))
                if power_filter == toughness_filter =>
            {
                Some(describe_for_each_count_filter(power_filter))
            }
            (
                Value::BasicLandTypesAmong(power_filter),
                Value::BasicLandTypesAmong(toughness_filter),
            ) if power_filter == toughness_filter => Some(
                describe_basic_land_types_among(power_filter)
                    .replace("basic land types", "basic land type"),
            ),
            (
                Value::CreatureTypesAmong(power_filter),
                Value::CreatureTypesAmong(toughness_filter),
            ) if power_filter == toughness_filter => Some(format!(
                "creature type among {}",
                describe_count_filter_value_subject(power_filter)
            )),
            (Value::CardTypesAmong(power_filter), Value::CardTypesAmong(toughness_filter))
                if power_filter == toughness_filter =>
            {
                Some(format!(
                    "card type among {}",
                    describe_count_filter_value_subject(power_filter)
                ))
            }
            (
                Value::CountersOnSource(power_counter),
                Value::CountersOnSource(toughness_counter),
            ) if power_counter == toughness_counter => {
                Some(format!("{} counter on it", power_counter.description()))
            }
            _ => None,
        };
        if let Some(for_each_text) = for_each_text {
            return format!(
                "{} gets +1/+1 for each {} {}",
                describe_choose_spec(&modify_pt.target),
                for_each_text,
                describe_until(&modify_pt.duration)
            );
        }
        if !matches!(modify_pt.power, Value::Fixed(_)) && power_text == toughness_text {
            return format!(
                "{} gets +X/+X {}, where X is {}",
                describe_choose_spec(&modify_pt.target),
                describe_until(&modify_pt.duration),
                power_text
            );
        }
        if !matches!(modify_pt.power, Value::Fixed(_))
            && matches!(modify_pt.toughness, Value::Fixed(0))
        {
            if let Value::CountersOnSource(counter_type) = &modify_pt.power {
                return format!(
                    "{} gets +1/+0 for each {} counter on it {}",
                    describe_choose_spec(&modify_pt.target),
                    counter_type.description(),
                    describe_until(&modify_pt.duration)
                );
            }
            return format!(
                "{} gets +X/+0 {}, where X is {}",
                describe_choose_spec(&modify_pt.target),
                describe_until(&modify_pt.duration),
                power_text
            );
        }
        if !matches!(modify_pt.toughness, Value::Fixed(_))
            && matches!(modify_pt.power, Value::Fixed(0))
        {
            return format!(
                "{} gets +0/+X {}, where X is {}",
                describe_choose_spec(&modify_pt.target),
                describe_until(&modify_pt.duration),
                toughness_text
            );
        }
        return format!(
            "{} gets {}/{} {}",
            describe_choose_spec(&modify_pt.target),
            describe_signed_value(&modify_pt.power),
            describe_toughness_delta_with_power_context(&modify_pt.power, &modify_pt.toughness),
            describe_until(&modify_pt.duration)
        );
    }
    if let Some(set_base_pt) = effect.downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()
    {
        if set_base_pt.power.unhinted() == set_base_pt.toughness.unhinted()
            && let Some(where_x) = describe_where_x_basis(&set_base_pt.power)
        {
            return format!(
                "{} has base power and toughness X/X {}, where X is {where_x}",
                describe_choose_spec(&set_base_pt.target),
                describe_until(&set_base_pt.duration)
            );
        }
        return format!(
            "{} has base power and toughness {}/{} {}",
            describe_choose_spec(&set_base_pt.target),
            describe_value(&set_base_pt.power),
            describe_value(&set_base_pt.toughness),
            describe_until(&set_base_pt.duration)
        );
    }
    if let Some(modify_pt_all) =
        effect.downcast_ref::<crate::effects::ModifyPowerToughnessAllEffect>()
    {
        return format!(
            "{} get {}/{} {}",
            modify_pt_all.filter.description(),
            describe_signed_value(&modify_pt_all.power),
            describe_toughness_delta_with_power_context(
                &modify_pt_all.power,
                &modify_pt_all.toughness,
            ),
            describe_until(&modify_pt_all.duration)
        );
    }
    if let Some(modify_pt_each) =
        effect.downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()
    {
        let target_text = describe_choose_spec(&modify_pt_each.target);
        let gets_verb = if choose_spec_is_plural(&modify_pt_each.target) {
            "get"
        } else {
            "gets"
        };
        let each_text = describe_create_for_each_count(&modify_pt_each.count)
            .unwrap_or_else(|| describe_value(&modify_pt_each.count));
        let additional = if value_has_surface_hint(
            &modify_pt_each.count,
            ValueSurfaceHint::AdditionalPowerToughnessModifier,
        ) {
            "an additional "
        } else {
            ""
        };
        let power = describe_signed_i32(modify_pt_each.power_per);
        let toughness = describe_signed_i32(modify_pt_each.toughness_per);
        let duration = describe_until(&modify_pt_each.duration);
        if modify_pt_each
            .count
            .has_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay)
        {
            return format!(
                "{target_text} {gets_verb} {additional}{power}/{toughness} {duration} for each {each_text}"
            );
        }
        return format!(
            "{target_text} {gets_verb} {additional}{power}/{toughness} for each {each_text} {duration}"
        );
    }
    if let Some(bid_life) = effect.downcast_ref::<crate::effects::BidLifeEffect>() {
        let target = describe_choose_spec(&bid_life.target);
        let starting_bid = match bid_life.starting_bid {
            crate::effects::LifeBidStart::Fixed(amount) => amount,
        };
        return format!(
            "Each player may bid life for control of {target}. You start the bidding with a bid of {starting_bid}. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and gains control of the creature. This effect lasts indefinitely"
        );
    }
    if let Some(gain_control) = effect.downcast_ref::<crate::effects::GainControlEffect>() {
        return format!(
            "Gain control of {} {}",
            describe_choose_spec(&gain_control.target),
            describe_until(&gain_control.duration)
        );
    }
    if let Some(exchange_control) = effect.downcast_ref::<crate::effects::ExchangeControlEffect>() {
        let shared_suffix = match exchange_control.shared_type {
            Some(crate::effects::SharedTypeConstraint::CardType) => " that share a card type",
            Some(crate::effects::SharedTypeConstraint::PermanentType) => {
                " that share a permanent type"
            }
            None => "",
        };
        if exchange_control.permanent1.is_target() && !exchange_control.permanent1.is_single() {
            return format!(
                "Exchange control of {}{shared_suffix}",
                describe_choose_spec(&exchange_control.permanent1)
            );
        }
        return format!(
            "Exchange control of {} and {}{shared_suffix}",
            describe_choose_spec(&exchange_control.permanent1),
            describe_choose_spec(&exchange_control.permanent2)
        );
    }
    if let Some(exchange_text_boxes) =
        effect.downcast_ref::<crate::effects::ExchangeTextBoxesEffect>()
    {
        return format!(
            "Exchange the text boxes of {}",
            describe_choose_spec(&exchange_text_boxes.target)
        );
    }
    if let Some(exchange_zones) = effect.downcast_ref::<crate::effects::ExchangeZonesEffect>() {
        return format!(
            "Exchange {} {} and {}",
            describe_possessive_player_filter(&exchange_zones.player),
            exchange_zones.zone1,
            exchange_zones.zone2
        );
    }
    if let Some(transform) = effect.downcast_ref::<crate::effects::TransformEffect>() {
        return format!("Transform {}", describe_transform_target(&transform.target));
    }
    if let Some(meld) = effect.downcast_ref::<crate::effects::MeldEffect>() {
        let result_name = meld
            .result_name
            .split_whitespace()
            .map(capitalize_first)
            .collect::<Vec<_>>()
            .join(" ");
        let mut text = format!("Exile them, then meld them into {result_name}");
        if meld.enters_tapped && meld.enters_attacking {
            text.push_str(". It enters tapped and attacking");
        }
        return text;
    }
    if let Some(convert) = effect.downcast_ref::<crate::effects::ConvertEffect>() {
        return format!("Convert {}", describe_transform_target(&convert.target));
    }
    if let Some(flip) = effect.downcast_ref::<crate::effects::FlipEffect>() {
        return format!("Flip {}", describe_flip_target(&flip.target));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return describe_effect(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        if is_implicit_reference_tag(tag_all.tag.as_str()) {
            return describe_effect(&tag_all.effect);
        }
        return format!(
            "Tag all affected objects as '{}' then {}",
            tag_all.tag.as_str(),
            describe_effect(&tag_all.effect)
        );
    }
    if let Some(tag_triggering) = effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    {
        if is_implicit_reference_tag(tag_triggering.tag.as_str()) {
            return String::new();
        }
        return format!(
            "Tag the triggering object as '{}'",
            tag_triggering.tag.as_str()
        );
    }
    if let Some(tag_blockers) = effect.downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
    {
        if is_implicit_reference_tag(tag_blockers.tag.as_str()) {
            return String::new();
        }
        return format!(
            "Tag the triggering blockers as '{}'",
            tag_blockers.tag.as_str()
        );
    }
    if let Some(tag_damage_target) =
        effect.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>()
    {
        if is_implicit_reference_tag(tag_damage_target.tag.as_str()) {
            return String::new();
        }
        return format!(
            "Tag the triggering damaged object as '{}'",
            tag_damage_target.tag.as_str()
        );
    }
    if let Some(tag_attached) = effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>() {
        if matches!(tag_attached.tag.as_str(), "enchanted" | "equipped") {
            return String::new();
        }
        return format!(
            "Tag the object attached to this source as '{}'",
            tag_attached.tag.as_str()
        );
    }
    if let Some(roll_die) = effect.downcast_ref::<crate::effects::RollDieEffect>() {
        let player = describe_player_filter(&roll_die.player);
        let die_text = roll_die
            .die_text
            .clone()
            .unwrap_or_else(|| format!("d{}", roll_die.sides));
        if player == "you" {
            return format!("Roll a {die_text}");
        }
        return format!(
            "{player} {} a {die_text}",
            player_verb(&player, "roll", "rolls"),
        );
    }
    if let Some(roll_dice) = effect.downcast_ref::<crate::effects::RollDiceChooseResultEffect>() {
        let player = describe_player_filter(&roll_dice.player);
        let die_text = roll_dice
            .die_text
            .clone()
            .unwrap_or_else(|| format!("d{}", roll_dice.sides));
        let count = match roll_dice.count {
            1 => "one".to_string(),
            2 => "two".to_string(),
            n => n.to_string(),
        };
        if player == "you" {
            return format!("Roll {count} {die_text} and choose one result");
        }
        return format!(
            "{player} {} {count} {die_text} and chooses one result",
            player_verb(&player, "roll", "rolls"),
        );
    }
    if let Some(flip_coin) = effect.downcast_ref::<crate::effects::FlipCoinEffect>() {
        let player = describe_player_filter(&flip_coin.player);
        if player == "you" {
            return "Flip a coin".to_string();
        }
        return format!("{player} flips a coin");
    }
    if effect
        .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
        .is_some()
    {
        return String::new();
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return describe_effect(&with_id.effect);
    }
    if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatProcessEffect>() {
        if let Some(rendered) = describe_clash_repeat_process(repeat) {
            return rendered;
        }
        if let Some(rendered) = render_iterative_library_repeat_process(repeat) {
            return rendered;
        }
        let body = describe_effect_list(&repeat.effects);
        let body = body.trim().trim_end_matches('.');
        if body.is_empty() {
            return "Repeat this process".to_string();
        }
        if body.ends_with("You may repeat this process any number of times")
            || body.ends_with("you may repeat this process any number of times")
            || body.ends_with("You may repeat this process")
            || body.ends_with("you may repeat this process")
        {
            return body.to_string();
        }
        if body.contains("If you do,") || body.contains("if you do,") {
            return format!("{body} and repeat this process");
        }
        return format!("{body}. Repeat this process");
    }
    if let Some(prompt) = effect.downcast_ref::<crate::effects::RepeatProcessPromptEffect>() {
        return prompt.description().to_string();
    }
    if let Some(turn_face_up) = effect.downcast_ref::<crate::effects::TurnFaceUpEffect>() {
        let target = match turn_face_up.target.base() {
            ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG => {
                "the exiled card".to_string()
            }
            ChooseSpec::Tagged(_) => "it".to_string(),
            other => describe_choose_spec(other),
        };
        return format!("Turn {target} face up");
    }
    if let Some(retain) = effect.downcast_ref::<crate::effects::RetainManaUntilEndOfTurnEffect>() {
        return match retain.player {
            PlayerFilter::You => {
                "Until end of turn, you don't lose this mana as steps and phases end".to_string()
            }
            _ => "Until end of turn, mana doesn't empty as steps and phases end".to_string(),
        };
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        if conditional.surface == ironsmith_core::ConditionalSurface::TrailingIf {
            let effect_text = describe_effect_clause_list(&conditional.if_true)
                .unwrap_or_else(|| describe_effect_list(&conditional.if_true));
            return format!(
                "{effect_text} if {}",
                describe_condition(&conditional.condition)
            );
        }
        if conditional.surface == ironsmith_core::ConditionalSurface::TrailingUnless {
            let effect_text = describe_effect_clause_list(&conditional.if_true)
                .unwrap_or_else(|| describe_effect_list(&conditional.if_true));
            let positive_condition = match &conditional.condition {
                crate::effect::Condition::Not(inner) => inner.as_ref(),
                condition => condition,
            };
            let mut condition_text = if matches!(
                positive_condition,
                crate::effect::Condition::EnchantedPermanentAttackedThisTurn
            ) {
                "that creature attacked this turn".to_string()
            } else {
                describe_condition(positive_condition)
            };
            if let Some(rest) = condition_text.strip_prefix("that player ") {
                let rest = rest
                    .strip_prefix("controls ")
                    .map(|tail| format!("control {tail}"))
                    .or_else(|| rest.strip_prefix("has ").map(|tail| format!("have {tail}")))
                    .or_else(|| rest.strip_prefix("is ").map(|tail| format!("are {tail}")))
                    .unwrap_or_else(|| rest.to_string());
                condition_text = format!("they {rest}");
            }
            condition_text = condition_text.replace("that player's", "their");
            return format!("{effect_text} unless {condition_text}");
        }
        if let Some(compact) = describe_target_color_set_conditional_destroy(conditional) {
            return compact;
        }
        if let Some(compact) = describe_cards_in_hand_difference_conditional(conditional) {
            return compact;
        }
        if let Some(compact) = describe_conditional_damage_instead(conditional) {
            return compact;
        }
        if let Some(compact) = describe_conditional_choose_both_instead(conditional) {
            return compact;
        }
        if let Some(compact) = describe_conditional_replacement_instead(conditional) {
            return compact;
        }
        if let Some(compact) = describe_no_more_counters_move_then_each_player_return(conditional) {
            return compact;
        }
        if let Some(compact) = describe_delirium_countered_spell_same_name_search(conditional) {
            return compact;
        }
        if let Some(compact) = describe_second_spell_counter_conditional(conditional) {
            return compact;
        }
        if conditional.if_false.is_empty()
            && conditional.surface == ironsmith_core::ConditionalSurface::LeadingIf
        {
            let visible = conditional
                .if_true
                .iter()
                .map(structural_unwrap_render_wrappers)
                .filter(|effect| {
                    !effect
                        .downcast_ref::<crate::effects::TargetOnlyEffect>()
                        .is_some_and(|target| !target.explicit_declaration)
                        && effect
                            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                            .is_none()
                })
                .collect::<Vec<_>>();
            if let Some(inline) = describe_reveal_hand_choose_discard_inline(&visible) {
                return format!(
                    "If {}, {}",
                    describe_condition(&conditional.condition),
                    lowercase_first(&inline)
                );
            }
        }
        if conditional.if_false.is_empty()
            && matches!(
                conditional.condition,
                crate::effect::Condition::AttackedThisTurn
            )
            && let [single] = conditional.if_true.as_slice()
            && let Some(schedule) =
                single.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            && let Some(compact) = describe_next_spell_delayed_trigger(schedule, true)
        {
            return format!(
                "Raid — If you attacked this turn, {}",
                lowercase_first(&compact)
            );
        }
        if let Some(compact) =
            describe_was_declined_optional_battlefield_fallback_conditional(conditional)
        {
            return compact;
        }
        let chosen_collection_branch = match conditional.if_true.as_slice() {
            [producer, consumer] => describe_for_players_choose_then_destroy_chosen_collection_pair(
                producer, consumer,
            )
            .or_else(|| {
                describe_each_player_choose_creature_then_destroy_others_pair(producer, consumer)
            }),
            _ => None,
        };
        let mut true_branch = chosen_collection_branch
            .or_else(|| describe_effect_clause_list(&conditional.if_true))
            .unwrap_or_else(|| describe_effect_list(&conditional.if_true));
        if matches!(
            &conditional.condition,
            crate::effect::Condition::SourceHasCounterAtLeast {
                surface: crate::effect::SourceCounterThresholdSurface::ThereAreOn(_),
                ..
            }
        ) {
            for action in ["Flip", "Sacrifice", "Transform", "Untap"] {
                for source in [
                    "this artifact",
                    "this creature",
                    "this enchantment",
                    "this permanent",
                    "this source",
                ] {
                    true_branch = true_branch.replace(
                        &format!("{action} {source}"),
                        &format!("{action} it"),
                    );
                    true_branch = true_branch.replace(
                        &format!("{} {source}", action.to_ascii_lowercase()),
                        &format!("{} it", action.to_ascii_lowercase()),
                    );
                }
            }
        }
        let false_branch = describe_effect_clause_list(&conditional.if_false)
            .unwrap_or_else(|| describe_effect_list(&conditional.if_false));
        if !false_branch.is_empty()
            && let Some((may_branch, did_not_branch)) = true_branch.split_once(". If you don't, ")
            && did_not_branch.eq_ignore_ascii_case(&false_branch)
        {
            let explicit_may_decline = if let [setup, declined] = conditional.if_true.as_slice()
                && let Some(with_id) = setup.downcast_ref::<crate::effects::WithIdEffect>()
                && let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>()
                && let Some(if_effect) = declined.downcast_ref::<crate::effects::IfEffect>()
            {
                may.fallback == crate::decision::FallbackStrategy::Decline
                    && matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
                    && if_effect.condition == with_id.id
                    && matches!(
                        if_effect.predicate,
                        EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined
                    )
                    && if_effect.else_.is_empty()
            } else {
                false
            };
            let condition_text = describe_condition(&conditional.condition);
            let may_branch = may_branch.replace("put it onto", "put that card onto");
            let false_branch = false_branch
                .replace(
                    "Put it into its owner's hand",
                    "put that card into its owner's hand",
                )
                .replace(
                    "put it into its owner's hand",
                    "put that card into its owner's hand",
                )
                .replace("Put it into your hand", "put that card into your hand")
                .replace("put it into your hand", "put that card into your hand");
            let fallback_condition = if explicit_may_decline {
                "If you don't"
            } else {
                "Otherwise"
            };
            return format!(
                "{may_branch} if {condition_text}. {fallback_condition}, {false_branch}"
            );
        }
        if true_branch.is_empty() && !false_branch.is_empty() {
            return describe_false_only_conditional(&conditional.condition, &false_branch);
        }
        if false_branch.is_empty() {
            if true_branch.eq_ignore_ascii_case("you win the game") {
                return format!(
                    "You win the game if {}",
                    describe_condition(&conditional.condition)
                );
            }
            if let Some(condition_text) =
                describe_sacrificed_tagged_condition(&conditional.condition)
            {
                let mut branch = lowercase_first(true_branch.trim());
                if let Some(rest) = branch.strip_prefix("target player creates") {
                    branch = format!("that player creates{rest}");
                }
                return format!("If {condition_text}, {branch}");
            }
            let condition_text = describe_retained_land_noncreature_condition(conditional)
                .map(str::to_string)
                .unwrap_or_else(|| describe_condition(&conditional.condition));
            if matches!(
                conditional.condition,
                crate::effect::Condition::CountParity { .. }
            ) && !true_branch.contains(". ")
                && !true_branch.contains(": ")
                && !true_branch.starts_with("If ")
                && !true_branch.starts_with("When ")
                && !true_branch.starts_with("Whenever ")
                && !true_branch.starts_with("At ")
            {
                return format!("{true_branch} if {condition_text}");
            }
            if condition_text.starts_with("the sacrificed ")
                && !true_branch.contains(". ")
                && !true_branch.contains(": ")
                && !true_branch.starts_with("If ")
                && !true_branch.starts_with("When ")
                && !true_branch.starts_with("Whenever ")
                && !true_branch.starts_with("At ")
            {
                return format!("{true_branch} if {condition_text}");
            }
            return format!("If {}, {}", condition_text, true_branch);
        }
        let condition_text = describe_condition(&conditional.condition);
        if condition_text == "it was attacking" {
            return format!("{true_branch} if {condition_text}. Otherwise, {false_branch}");
        }
        return format!(
            "If {}, {}. Otherwise, {}",
            condition_text, true_branch, false_branch
        );
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        let then_text = describe_conditional_branch_effect_list(&if_effect.then)
            .unwrap_or_else(|| describe_result_branch_effect_list(&if_effect.then));
        let else_text = describe_effect_list(&if_effect.else_);
        if then_text.trim().is_empty() && else_text.trim().is_empty() {
            return String::new();
        }
        if else_text.is_empty() {
            if if_effect.predicate == EffectPredicate::SearchedLibrary
                && if_effect.then.len() == 1
                && let Some(shuffle) =
                    if_effect.then[0].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            {
                let player = describe_player_filter(&shuffle.player);
                if player == "you" {
                    return "If you search your library this way, shuffle your library".to_string();
                }
                return format!(
                    "If {player} searches their library this way, {player} {}",
                    player_verb(&player, "shuffle", "shuffles")
                );
            }
            if is_reflexive_choose_one_followup(if_effect, &then_text) {
                return format!("When you do, {}", lowercase_first(then_text.trim_start()));
            }
            return format!(
                "If effect #{} {}, {}",
                if_effect.condition.0,
                describe_effect_predicate(&if_effect.predicate),
                then_text
            );
        }
        return format!(
            "If effect #{} {}, {}. Otherwise, {}",
            if_effect.condition.0,
            describe_effect_predicate(&if_effect.predicate),
            then_text,
            else_text
        );
    }
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        let triggered = describe_result_branch_effect_list(&reflexive.effects);
        return format!(
            "When effect #{} {}, {}",
            reflexive.condition.0,
            describe_effect_predicate(&reflexive.predicate),
            triggered
        );
    }
    if let Some(cast_tagged) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
        let verb = if cast_tagged.allow_land {
            "play"
        } else {
            "cast"
        };
        let tag = cast_tagged.tag.as_str();
        let helper_tag = tag.starts_with("targeted_")
            || tag.starts_with("__source_")
            || tag == "__it__"
            || matches!(
                tag,
                "exiled" | "revealed" | "looked" | "chosen" | "searched"
            )
            || crate::cards::is_sentence_helper_tag(tag, "exiled")
            || crate::cards::is_sentence_helper_tag(tag, "revealed")
            || crate::cards::is_sentence_helper_tag(tag, "looked")
            || crate::cards::is_sentence_helper_tag(tag, "chosen")
            || crate::cards::is_sentence_helper_tag(tag, "searched");
        let spec = crate::target::ChooseSpec::Tagged(cast_tagged.tag.clone());
        let target = if cast_tagged.as_copy {
            let tag_is_numbered = tag.rsplit_once('_').is_some_and(|(_, suffix)| {
                !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
            });
            if tag == "it" || tag_is_numbered {
                "the copy".to_string()
            } else {
                format!("a copy of {}", describe_choose_spec(&spec))
            }
        } else if helper_tag {
            "that card".to_string()
        } else {
            describe_choose_spec(&spec)
        };
        let mut text = format!("{verb} {target}");
        if cast_tagged.without_paying_mana_cost {
            text.push_str(" without paying its mana cost");
        }
        if let Some(reduction) = cast_tagged.cost_reduction.as_ref() {
            let copy_ref = if cast_tagged.as_copy {
                "That copy"
            } else {
                "That spell"
            };
            text.push_str(&format!(
                ". {copy_ref} costs {} less to cast",
                reduction.to_oracle()
            ));
        }
        return text;
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        // "untap this creature and remove it from combat" is one optional
        // instruction (Gustcloak); a period join would read the removal as
        // mandatory. Checked before the compact helpers so none of them
        // claims the untap half alone.
        if may.effects.len() == 1
            && let Some(seq) = may.effects[0].downcast_ref::<crate::effects::SequenceEffect>()
            && let [untap_effect, remove_effect] = seq.effects.as_slice()
            && untap_effect
                .downcast_ref::<crate::effects::UntapEffect>()
                .is_some()
            && remove_effect
                .downcast_ref::<crate::effects::RemoveFromCombatEffect>()
                .is_some()
        {
            let untap = lowercase_may_clause(&describe_effect_list(std::slice::from_ref(
                untap_effect,
            )));
            let remove = lowercase_may_clause(&describe_effect_list(std::slice::from_ref(
                remove_effect,
            )));
            return format!("You may {untap} and {remove}");
        }
        if let Some(compact) = describe_may_compound_payment(may) {
            return compact;
        }
        if let Some(compact) = describe_may_have_source_deal_damage_to_decider(may) {
            return compact;
        }
        if let Some(compact) = describe_typed_may_causative(may) {
            return compact;
        }
        if let Some(compact) = describe_may_enlist(may) {
            return compact;
        }
        if let Some(compact) = describe_may_discover_from_triggering_toughness(may) {
            return compact;
        }
        if let Some(compact) = describe_may_have_you_create_tokens(may) {
            return compact;
        }
        if let Some(compact) = describe_may_have_target_block_source(may) {
            return compact;
        }
        if let Some(compact) = describe_may_choose_then_sacrifice(may) {
            return compact;
        }
        if let Some(compact) = describe_may_search_then_put_onto_battlefield(may) {
            return compact;
        }
        if let Some(compact) = describe_may_choose_reveal_and_move_to_hand(may) {
            return compact;
        }
        if let Some(compact) = describe_may_search_library_and_or_nonlibrary(may) {
            return compact;
        }
        if let Some(compact) = describe_may_search_sequence_then_shuffle(may) {
            return compact;
        }
        if may.effects.len() == 1
            && let Some(retarget) =
                may.effects[0].downcast_ref::<crate::effects::RetargetStackObjectEffect>()
            && matches!(retarget.mode, crate::effects::RetargetMode::All)
            && !retarget.require_change
            && matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag.as_str().contains("copied"))
        {
            let who = may
                .decider
                .as_ref()
                .map(describe_player_filter)
                .unwrap_or_else(|| "you".to_string());
            if who == "you" {
                return "You may choose new targets for the copy".to_string();
            }
            return format!("{who} may choose new targets for the copy");
        }
        if let Some(decider) = may.decider.as_ref() {
            // A bare Opponent decider iterates every opponent: oracle says
            // "each opponent may ..." with their-possessives.
            let each_opponent = matches!(decider, PlayerFilter::Opponent);
            let who = if each_opponent {
                "each opponent".to_string()
            } else {
                describe_player_filter(decider)
            };
            let mut inner = describe_effect_list(&may.effects);
            if each_opponent {
                inner = inner
                    .replace("an opponent's library", "their library")
                    .replace("an opponent's hand", "their hand")
                    .replace("an opponent's graveyard", "their graveyard");
                if let Some(rest) = inner.strip_prefix("an opponent may ") {
                    inner = rest.to_string();
                } else if let Some(rest) = inner.strip_prefix("An opponent may ") {
                    inner = rest.to_string();
                }
            }
            let may_prefix = format!("{who} may ");
            if inner.starts_with(&may_prefix) {
                inner = inner[may_prefix.len()..].to_string();
            } else if who == "you" && inner.starts_with("you may ") {
                inner = inner["you may ".len()..].to_string();
            }
            let prefix = format!("{who} ");
            if inner.starts_with(&prefix) {
                inner = inner[prefix.len()..].to_string();
            } else if who == "you" && inner.starts_with("you ") {
                inner = inner["you ".len()..].to_string();
            }
            if who == "you" {
                if let Some(rest) = inner.strip_prefix("that player ") {
                    let normalized = normalize_you_verb_phrase(rest);
                    return format!("you may have that player {normalized}");
                }
                if let Some(rest) = inner.strip_prefix("target player ") {
                    let normalized = normalize_you_verb_phrase(rest);
                    return format!("you may have target player {normalized}");
                }
                if let Some(causative) = may_causative_clause(&inner) {
                    return format!("you may {causative}");
                }
            } else if let Some(rest) = inner.strip_prefix("you ") {
                let normalized = normalize_you_verb_phrase(rest);
                return format!("{who} may have you {normalized}");
            }
            inner = normalize_you_verb_phrase(&inner);
            inner = lowercase_may_clause(&inner);
            return format!("{who} may {inner}");
        }

        if may.effects.len() == 1
            && let Some(cast_tagged) =
                may.effects[0].downcast_ref::<crate::effects::CastTaggedEffect>()
            && cast_tagged.as_copy
        {
            if let Some(reduction) = cast_tagged.cost_reduction.as_ref() {
                return format!(
                    "Copy it. You may cast the copy. That copy costs {} less to cast",
                    reduction.to_oracle()
                );
            }
            let mut inner = describe_effect_list(&may.effects);
            if inner.starts_with("you ") {
                inner = inner["you ".len()..].to_string();
            }
            inner = normalize_you_verb_phrase(&inner);
            inner = lowercase_may_clause(&inner);
            return format!("Copy it. You may {inner}");
        }

        let mut inner = describe_effect_list(&may.effects);
        if inner.starts_with("you ") {
            inner = inner["you ".len()..].to_string();
        }
        if let Some(causative) = may_causative_clause(&inner) {
            return format!("You may {causative}");
        }
        inner = normalize_you_verb_phrase(&inner);
        inner = lowercase_may_clause(&inner);
        return format!("You may {inner}");
    }
}
