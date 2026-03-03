/// Compile a list of effects to human-readable text (for stack ability display).
pub fn compile_effect_list(effects: &[Effect]) -> String {
    describe_effect_list(effects)
}

fn describe_effect_list(effects: &[Effect]) -> String {
    let has_non_target_only = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
    });
    let filtered = effects
        .iter()
        .filter(|effect| {
            !(has_non_target_only
                && effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .is_some())
        })
        .collect::<Vec<_>>();

    fn apply_continuous_for_compaction(
        effect: &Effect,
    ) -> Option<&crate::effects::ApplyContinuousEffect> {
        if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
            return Some(apply);
        }
        if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>()
            && is_implicit_reference_tag(tag_all.tag.as_str())
            && let Some(apply) = tag_all
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        {
            return Some(apply);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            && is_implicit_reference_tag(tagged.tag.as_str())
            && let Some(apply) = tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        {
            return Some(apply);
        }
        None
    }

    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < filtered.len() {
        fn unwrap_implicit_tag_all<'a>(effect: &'a Effect) -> &'a Effect {
            if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>()
                && is_implicit_reference_tag(tag_all.tag.as_str())
            {
                return &tag_all.effect;
            }
            effect
        }

        fn is_exile_up_to_one_target_type(
            effect: &Effect,
            card_type: crate::types::CardType,
        ) -> bool {
            let effect = unwrap_implicit_tag_all(effect);
            let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
            else {
                return false;
            };
            if move_to_zone.zone != Zone::Exile {
                return false;
            }
            let ChooseSpec::WithCount(inner, count) = &move_to_zone.target else {
                return false;
            };
            if count.min != 0 || count.max != Some(1) {
                return false;
            }
            let ChooseSpec::Target(target_inner) = inner.as_ref() else {
                return false;
            };
            let ChooseSpec::Object(filter) = target_inner.as_ref() else {
                return false;
            };
            filter.zone == Some(Zone::Battlefield) && filter.card_types == vec![card_type]
        }

        // Compact Chaotic Transformation-style prefix:
        // Exile up to one target [type] ... and/or ... then ForEachTagged exiled_0 reveal-until.
        if idx + 5 < filtered.len()
            && is_exile_up_to_one_target_type(filtered[idx], crate::types::CardType::Artifact)
            && is_exile_up_to_one_target_type(filtered[idx + 1], crate::types::CardType::Creature)
            && is_exile_up_to_one_target_type(
                filtered[idx + 2],
                crate::types::CardType::Enchantment,
            )
            && is_exile_up_to_one_target_type(
                filtered[idx + 3],
                crate::types::CardType::Planeswalker,
            )
            && is_exile_up_to_one_target_type(filtered[idx + 4], crate::types::CardType::Land)
            && let Some(for_each) =
                filtered[idx + 5].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && for_each.tag.as_str() == "exiled_0"
            && for_each.effects.len() == 1
            && for_each.effects[0]
                .downcast_ref::<crate::effects::SequenceEffect>()
                .is_some()
        {
            parts.push("Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles".to_string());
            idx += 6;
            continue;
        }

        if idx + 1 < filtered.len()
            && let Some(first_apply) = apply_continuous_for_compaction(filtered[idx])
            && let Some(second_apply) = apply_continuous_for_compaction(filtered[idx + 1])
            && let Some(compact) = describe_compact_apply_continuous_pair(first_apply, second_apply)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tag_attached) =
                filtered[idx].downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            && let Some(compact) =
                describe_tag_attached_then_tap_or_untap(tag_attached, filtered[idx + 1])
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(move_back) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_exile_then_return(tagged, move_back)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(reveal_top) =
                filtered[idx].downcast_ref::<crate::effects::RevealTopEffect>()
            && let Some(conditional) =
                filtered[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(compact) =
                describe_reveal_top_then_if_put_into_hand(reveal_top, conditional)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(choose_new) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
            && let Some(compact) = describe_with_id_then_choose_new_targets(with_id, choose_new)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(with_id) = filtered[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = filtered[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && let Some(compact) = describe_with_id_then_if(with_id, if_effect)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(deal) = filtered[idx + 1].downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(compact) = describe_tagged_target_then_power_damage(tagged, deal)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) = describe_choose_then_for_each_copy(choose, for_each)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                filtered[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            let shuffle = filtered
                .get(idx + 2)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(compact) = describe_search_choose_for_each(choose, for_each, shuffle, false)
            {
                parts.push(compact);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
        }
        if idx + 2 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(shuffle) =
                filtered[idx + 1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
            && let Some(for_each) =
                filtered[idx + 2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
            && let Some(compact) =
                describe_search_choose_for_each(choose, for_each, Some(shuffle), true)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 1].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) = describe_choose_then_exile(choose, exile)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) =
                filtered[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_choose_then_move_to_library(choose, move_to_zone)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(move_to_zone) =
                filtered[idx + 2].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) =
                describe_look_at_top_then_choose_move_to_exile(look_at_top, choose, move_to_zone)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 2 < filtered.len()
            && let Some(look_at_top) =
                filtered[idx].downcast_ref::<crate::effects::LookAtTopCardsEffect>()
            && let Some(choose) =
                filtered[idx + 1].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(exile) = filtered[idx + 2].downcast_ref::<crate::effects::ExileEffect>()
            && let Some(compact) =
                describe_look_at_top_then_choose_exile(look_at_top, choose, exile)
        {
            parts.push(compact);
            idx += 3;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(with_id) = filtered[idx + 1].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(sacrifice) = with_id
                .effect
                .downcast_ref::<crate::effects::SacrificeEffect>()
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) =
                filtered[idx + 1].downcast_ref::<crate::effects::SacrificeEffect>()
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(destroy) = filtered[idx + 1].downcast_ref::<crate::effects::DestroyEffect>()
            && let Some(compact) = describe_choose_then_destroy(choose, destroy)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(tagged) = filtered[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_tagged_target_then_cant_block(tagged, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(choose) =
                filtered[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(cant) = filtered[idx + 1].downcast_ref::<crate::effects::CantEffect>()
            && let Some(compact) = describe_choose_then_cant_block(choose, cant)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(draw) = filtered[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(discard) = filtered[idx + 1].downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(compact) = describe_draw_then_discard(draw, discard)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(mill) = filtered[idx].downcast_ref::<crate::effects::MillEffect>()
            && let Some(may) = filtered[idx + 1].downcast_ref::<crate::effects::MayEffect>()
            && let Some(compact) = describe_mill_then_may_return(mill, may)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(for_players) =
                filtered[idx].downcast_ref::<crate::effects::ForPlayersEffect>()
            && for_players.filter == PlayerFilter::Opponent
            && for_players.effects.len() == 1
            && let Some(deal) =
                for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()
            && matches!(
                deal.target,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer)
            )
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && matches!(gain.player, ChooseSpec::Player(PlayerFilter::You))
            && gain.amount == deal.amount
            && !deal.source_is_combat
            && matches!(deal.amount, Value::Count(_))
        {
            let amount_text = describe_value(&deal.amount);
            parts.push(format!(
                "it deals X damage to each opponent and you gain X life, where X is {amount_text}"
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(deal) = filtered[idx].downcast_ref::<crate::effects::DealDamageEffect>()
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(compact) = describe_deal_damage_then_gain_life(deal, gain)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(lose) = filtered[idx].downcast_ref::<crate::effects::LoseLifeEffect>()
            && let Some(gain) = filtered[idx + 1].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(compact) = describe_lose_life_then_gain_life(lose, gain)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(gain) = filtered[idx].downcast_ref::<crate::effects::GainLifeEffect>()
            && let Some(scry) = filtered[idx + 1].downcast_ref::<crate::effects::ScryEffect>()
            && let Some(compact) = describe_gain_life_then_scry(gain, scry)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < filtered.len()
            && let Some(scry) = filtered[idx].downcast_ref::<crate::effects::ScryEffect>()
            && let Some(draw) = filtered[idx + 1].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(compact) = describe_scry_then_draw(scry, draw)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        let rendered = describe_effect(filtered[idx]);
        if !rendered.is_empty() {
            parts.push(rendered);
        }
        idx += 1;
    }
    let text = parts.join(". ");
    cleanup_decompiled_text(&text)
}

fn describe_exile_then_return(
    tagged: &crate::effects::TaggedEffect,
    move_back: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    let crate::target::ChooseSpec::Tagged(return_tag) = &move_back.target else {
        return None;
    };
    if !return_tag.as_str().starts_with("exiled_") || return_tag != &tagged.tag {
        return None;
    }
    let exile_move = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_move.zone != Zone::Exile {
        return None;
    }
    let target = describe_choose_spec(&exile_move.target);
    let return_object = if choose_spec_allows_multiple(&exile_move.target) {
        "those cards"
    } else {
        "that card"
    };
    let owner_control_suffix = if choose_spec_allows_multiple(&exile_move.target) {
        " under their owners' control"
    } else {
        " under its owner's control"
    };
    let tapped_suffix = if move_back.enters_tapped {
        " tapped"
    } else {
        ""
    };
    let controller_suffix = match move_back.battlefield_controller {
        crate::effects::BattlefieldController::Preserve => "",
        crate::effects::BattlefieldController::Owner => owner_control_suffix,
        crate::effects::BattlefieldController::You => " under your control",
    };
    Some(format!(
        "Exile {target}, then return {return_object} to the battlefield{tapped_suffix}{controller_suffix}"
    ))
}

fn describe_reveal_top_then_if_put_into_hand(
    reveal_top: &crate::effects::RevealTopEffect,
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let reveal_tag = reveal_top.tag.as_ref()?;
    let Condition::TaggedObjectMatches(cond_tag, filter) = &conditional.condition else {
        return None;
    };
    if cond_tag != reveal_tag {
        return None;
    }
    let move_to_zone = conditional.if_true[0].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand {
        return None;
    }
    if !matches!(
        move_to_zone.target.base(),
        ChooseSpec::Tagged(tag) if tag == reveal_tag
    ) {
        return None;
    }

    let subject = describe_player_filter(&reveal_top.player);
    let is_you = subject == "you";
    let reveal_sentence = if is_you {
        "Reveal the top card of your library".to_string()
    } else {
        let mut reveal_subject = subject;
        if matches!(
            reveal_top.player,
            PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::DamagedPlayer
        ) {
            if let Some(rest) = reveal_subject.strip_prefix("the ") {
                reveal_subject = rest.to_string();
            }
        }
        let verb = player_verb(&reveal_subject, "reveal", "reveals");
        format!("{reveal_subject} {verb} the top card of their library")
    };

    // Match the common oracle pattern for "if it's a <type> card".
    let desc = filter.description();
    let stripped = strip_leading_article(&desc).trim().to_ascii_lowercase();
    let noun_phrase = if stripped.ends_with(" card") {
        stripped.clone()
    } else if matches!(
        stripped.as_str(),
        "land"
            | "creature"
            | "artifact"
            | "enchantment"
            | "planeswalker"
            | "battle"
            | "instant"
            | "sorcery"
    ) {
        format!("{stripped} card")
    } else {
        return None;
    };
    let condition_text = format!("it's {}", with_indefinite_article(&noun_phrase));

    let move_sentence = if is_you {
        "put it into your hand".to_string()
    } else {
        "that player puts it into their hand".to_string()
    };

    Some(format!(
        "{reveal_sentence}. If {condition_text}, {move_sentence}"
    ))
}

fn describe_tagged_target_then_power_damage(
    tagged: &crate::effects::TaggedEffect,
    deal: &crate::effects::DealDamageEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let Value::PowerOf(source_spec) = &deal.amount else {
        return None;
    };
    let source_tag = match source_spec.as_ref() {
        ChooseSpec::Tagged(tag) => tag,
        _ => return None,
    };
    if source_tag.as_str() != tagged.tag.as_str() {
        return None;
    }

    let source_text = describe_choose_spec(&target_only.target);
    if matches!(
        deal.target,
        ChooseSpec::Tagged(ref target_tag) if target_tag == source_tag
    ) {
        return Some(format!(
            "{source_text} deals damage to itself equal to its power"
        ));
    }

    let target_text = describe_choose_spec(&deal.target);
    Some(format!(
        "{source_text} deals damage equal to its power to {target_text}"
    ))
}

fn cleanup_decompiled_text(text: &str) -> String {
    let mut out = text.to_string();
    for (from, to) in [
        ("you gets", "you get"),
        ("you puts", "you put"),
        ("a artifact", "an artifact"),
        ("a Assassin", "an Assassin"),
        ("a another", "another"),
        ("a enchantment", "an enchantment"),
        ("a untapped", "an untapped"),
        ("a opponent", "an opponent"),
        (" ors ", " or "),
        ("creature are", "creatures are"),
        ("Creatures token", "Creature tokens"),
        ("creatures token", "creature tokens"),
        ("target any target", "any target"),
    ] {
        while out.contains(from) {
            out = out.replace(from, to);
        }
    }
    while out.contains("target target") {
        out = out.replace("target target", "target");
    }
    while out.contains("Target target") {
        out = out.replace("Target target", "Target");
    }
    out
}

fn describe_inline_ability(ability: &Ability) -> String {
    if let Some(text) = &ability.text
        && !text.trim().is_empty()
    {
        return normalize_inline_ability_text(ability, text.trim());
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => static_ability.display(),
        AbilityKind::Triggered(triggered) => {
            format!("a triggered ability ({})", triggered.trigger.display())
        }
        AbilityKind::Activated(activated) if activated.is_mana_ability() => {
            let mut line = String::new();
            if !activated.mana_cost.costs().is_empty() {
                line.push_str(&describe_cost_list(activated.mana_cost.costs()));
            }
            let mut payload = String::new();
            let mana_symbols = activated.mana_symbols();
            if !mana_symbols.is_empty() {
                payload.push_str("Add ");
                payload.push_str(
                    &mana_symbols
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join(""),
                );
            } else if !activated.effects.is_empty() {
                payload.push_str(&describe_effect_list(&activated.effects));
            }
            if !payload.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(&payload);
            }
            if let Some(condition) = &activated.activation_condition {
                let clause = describe_mana_activation_condition(condition);
                if !clause.is_empty() {
                    if !line.is_empty() {
                        line.push_str(". ");
                    }
                    line.push_str(&clause);
                }
            }
            if line.is_empty() {
                "a mana ability".to_string()
            } else {
                line
            }
        }
        AbilityKind::Activated(activated) => {
            let mut line = String::new();
            let mut pre = Vec::new();
            if !activated.mana_cost.costs().is_empty() {
                pre.push(describe_cost_list(activated.mana_cost.costs()));
            }
            if !activated.choices.is_empty() {
                pre.push(format!(
                    "choose {}",
                    activated
                        .choices
                        .iter()
                        .map(describe_choose_spec)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !pre.is_empty() {
                line.push_str(&pre.join(", "));
            }
            if !activated.effects.is_empty() {
                if !line.is_empty() {
                    line.push_str(": ");
                }
                line.push_str(&describe_effect_list(&activated.effects));
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            if !restriction_clauses.is_empty() {
                if !line.is_empty() {
                    line.push_str(". ");
                }
                line.push_str(&join_activation_restriction_clauses(&restriction_clauses));
            }
            if line.is_empty() {
                "an activated ability".to_string()
            } else {
                line
            }
        }
    }
}

fn activated_ability_has_source_tap_cost(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.costs().iter().any(|cost| {
        cost.requires_tap()
            || cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::TapEffect>()
                    .is_some_and(|tap| matches!(tap.spec, ChooseSpec::Source))
            })
    })
}

fn activated_ability_has_source_untap_cost(activated: &crate::ability::ActivatedAbility) -> bool {
    activated.mana_cost.costs().iter().any(|cost| {
        cost.requires_untap()
            || cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::UntapEffect>()
                    .is_some_and(|untap| matches!(untap.spec, ChooseSpec::Source))
            })
    })
}

fn normalize_inline_ability_text(ability: &Ability, text: &str) -> String {
    let trimmed = text.trim();
    let lower = trimmed.to_ascii_lowercase();
    match &ability.kind {
        AbilityKind::Activated(activated) => {
            if activated_ability_has_source_tap_cost(activated)
                && lower.starts_with("t ")
                && let Some((_, rest)) = trimmed.split_once(' ')
                && !rest.trim().is_empty()
            {
                if activated.is_mana_ability() {
                    let mut body = capitalize_first(rest.trim());
                    if !body.ends_with('.') {
                        body.push('.');
                    }
                    return format!("{{T}}: {body}");
                }
                return format!("{{T}}: {}", rest.trim());
            }
            if activated_ability_has_source_untap_cost(activated)
                && lower.starts_with("q ")
                && let Some((_, rest)) = trimmed.split_once(' ')
                && !rest.trim().is_empty()
            {
                if activated.is_mana_ability() {
                    let mut body = capitalize_first(rest.trim());
                    if !body.ends_with('.') {
                        body.push('.');
                    }
                    return format!("{{Q}}: {body}");
                }
                return format!("{{Q}}: {}", rest.trim());
            }
            trimmed.to_string()
        }
        _ => trimmed.to_string(),
    }
}

fn normalize_cost_phrase(text: &str) -> String {
    if let Some(rest) = text.strip_prefix("you ") {
        let normalized = normalize_you_verb_phrase(rest);
        return capitalize_first(&normalized);
    }
    if let Some(rest) = text.strip_prefix("You ") {
        let normalized = normalize_you_verb_phrase(rest);
        return capitalize_first(&normalized);
    }
    text.to_string()
}

fn describe_cost_component(cost: &crate::costs::Cost) -> String {
    if let Some(mana_cost) = cost.mana_cost_ref() {
        return mana_cost.to_oracle();
    }
    if let Some(effect) = cost.effect_ref() {
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>()
            && matches!(tap.spec, ChooseSpec::Source)
        {
            return "{T}".to_string();
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>()
            && matches!(untap.spec, ChooseSpec::Source)
        {
            return "{Q}".to_string();
        }
        return normalize_cost_phrase(&describe_effect(effect));
    }
    if cost.requires_tap() {
        return "{T}".to_string();
    }
    if cost.requires_untap() {
        return "{Q}".to_string();
    }
    if let Some(amount) = cost.life_amount() {
        return if amount == 1 {
            "Pay 1 life".to_string()
        } else {
            format!("Pay {amount} life")
        };
    }
    if cost.is_sacrifice_self() {
        return "Sacrifice this source".to_string();
    }
    let display = cost.display().trim().to_string();
    if display.is_empty() {
        format!("{cost:?}")
    } else {
        display
    }
}

fn describe_cost_list(costs: &[crate::costs::Cost]) -> String {
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < costs.len() {
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(tap) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::TapEffect>())
            && let Some(compact) = describe_choose_then_tap_cost(choose, tap)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(exile) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ExileEffect>())
            && let Some(compact) = describe_choose_then_exile(choose, exile)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(sacrifice) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice)
        {
            parts.push(normalize_cost_phrase(&compact));
            idx += 2;
            continue;
        }
        parts.push(describe_cost_component(&costs[idx]));
        idx += 1;
    }
    parts.join(", ")
}

fn with_indefinite_article(noun: &str) -> String {
    let trimmed = noun.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }
    for prefix in [
        "the active player's ",
        "that player's ",
        "target player's ",
        "an opponent's ",
        "opponent's ",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return with_indefinite_article(rest);
        }
    }
    if trimmed.starts_with("a ") || trimmed.starts_with("an ") {
        let (article, rest) = if let Some(rest) = trimmed.strip_prefix("an ") {
            ("an", rest)
        } else if let Some(rest) = trimmed.strip_prefix("a ") {
            ("a", rest)
        } else {
            ("", trimmed)
        };
        if let Some(first) = rest.chars().next() {
            let should_be_an = matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u');
            if should_be_an && article == "a" {
                return format!("an {rest}");
            }
            if !should_be_an && article == "an" {
                return format!("a {rest}");
            }
        }
        return trimmed.to_string();
    }
    if trimmed.starts_with("another ")
        || trimmed.starts_with("target ")
        || trimmed.starts_with("each ")
        || trimmed.starts_with("all ")
        || trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }
    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn ensure_indefinite_article(noun: &str) -> String {
    let trimmed = noun.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }

    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("another ")
        || lower.starts_with("each ")
        || lower.starts_with("all ")
        || lower.starts_with("this ")
        || lower.starts_with("that ")
        || lower.starts_with("those ")
        || lower.starts_with("target ")
        || lower.starts_with("any ")
        || lower.starts_with("up to ")
        || lower.starts_with("at least ")
        || lower.chars().next().is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }

    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn describe_for_each_double_counters(for_each: &crate::effects::ForEachObject) -> Option<String> {
    if for_each.effects.len() != 1 {
        return None;
    }
    let put = for_each.effects[0].downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.distributed {
        return None;
    }
    if !matches!(put.target.base(), ChooseSpec::Iterated) {
        return None;
    }
    let Value::CountersOn(source, Some(counter_type)) = &put.count else {
        return None;
    };
    if !matches!(source.base(), ChooseSpec::Iterated) {
        return None;
    }

    let filter_description = for_each.filter.description();
    let filter_text = strip_indefinite_article(&filter_description);
    let has_tagged_iterated_reference =
        for_each.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        });
    if has_tagged_iterated_reference {
        let plural = pluralize_noun_phrase(filter_text);
        return Some(format!(
            "Double the number of {} counters on each of those {}",
            describe_counter_type(*counter_type),
            plural
        ));
    }

    Some(format!(
        "Double the number of {} counters on each {}",
        describe_counter_type(*counter_type),
        filter_text
    ))
}

fn describe_for_each_tagged_this_way_subject(filter: &ObjectFilter) -> Option<String> {
    let action = filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        let tag = constraint.tag.as_str();
        if tag.starts_with("exiled_") {
            Some("exiled")
        } else if tag.starts_with("destroyed_") {
            Some("destroyed")
        } else if tag.starts_with("sacrificed_") {
            Some("sacrificed")
        } else if tag.starts_with("revealed_") {
            Some("revealed")
        } else if tag.starts_with("discarded_") {
            Some("discarded")
        } else if tag.starts_with("milled_") {
            Some("milled")
        } else {
            None
        }
    })?;

    let mut subject = strip_indefinite_article(&filter.description()).to_string();
    if action == "exiled" {
        if let Some(head) = subject.strip_suffix(" in exile") {
            subject = head.trim().to_string();
        } else if let Some((head, tail)) = subject.split_once(" in exile ") {
            subject = format!("{} {}", head.trim(), tail.trim());
        }
    } else if action == "revealed" {
        if let Some(head) = subject.strip_suffix(" permanent") {
            subject = format!("{} card", head.trim());
        } else if let Some(head) = subject.strip_suffix(" permanents") {
            subject = format!("{} cards", head.trim());
        }
    }
    let subject = subject.trim();
    if subject.is_empty() {
        return None;
    }

    Some(format!("For each {subject} {action} this way"))
}

fn normalize_put_counter_number_for_each(line: &str) -> Option<String> {
    let (before, after) = split_once_ascii_ci(line, "put the number of ")?;
    let (count_and_counter, after_target) = after.split_once(" counter(s) on ")?;
    let (target, suffix) = after_target
        .split_once(". ")
        .map(|(target, tail)| (target, format!(". {tail}")))
        .unwrap_or_else(|| (after_target.trim_end_matches('.'), String::new()));
    let target = target.trim();

    let count_filter = if let Some(filter) = count_and_counter.strip_suffix(" +1/+1") {
        format!("a +1/+1 counter on {target} for each {filter}")
    } else if let Some(filter) = count_and_counter.strip_suffix(" -1/-1") {
        format!("a -1/-1 counter on {target} for each {filter}")
    } else {
        return None;
    };

    let mut rewritten = String::with_capacity(line.len());
    rewritten.push_str(before);
    if !before.is_empty() && !before.ends_with(' ') {
        rewritten.push(' ');
    }
    rewritten.push_str("Put ");
    rewritten.push_str(&count_filter);
    if suffix.is_empty() {
        rewritten.push('.');
    } else {
        rewritten.push_str(&suffix);
    }
    Some(rewritten)
}

fn strip_indefinite_article(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed
        .strip_prefix("a ")
        .or_else(|| trimmed.strip_prefix("A "))
    {
        return rest;
    }
    if let Some(rest) = trimmed
        .strip_prefix("an ")
        .or_else(|| trimmed.strip_prefix("An "))
    {
        return rest;
    }
    trimmed
}

fn pluralize_word(word: &str) -> String {
    if word.chars().last().is_some_and(|ch| ch.is_ascii_digit()) {
        return word.to_string();
    }
    if let Some((prefix, last)) = word.rsplit_once(' ')
        && !prefix.is_empty()
        && !last.is_empty()
    {
        return format!("{prefix} {}", pluralize_word(last));
    }
    let lower = word.to_ascii_lowercase();
    if lower == "less" {
        return word.to_string();
    }
    if lower == "plains" || lower == "urzas" {
        return word.to_string();
    }
    if lower == "elf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Elves".to_string()
        } else {
            "elves".to_string()
        };
    }
    if lower == "dwarf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Dwarves".to_string()
        } else {
            "dwarves".to_string()
        };
    }
    if lower == "myr" {
        return word.to_string();
    }
    if lower == "mouse" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Mice".to_string()
        } else {
            "mice".to_string()
        };
    }
    if lower.ends_with('y')
        && lower.len() > 1
        && !matches!(
            lower.chars().nth(lower.len() - 2),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        )
    {
        return format!("{}ies", &word[..word.len() - 1]);
    }
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        return format!("{word}es");
    }
    format!("{word}s")
}

fn pluralize_noun_phrase(phrase: &str) -> String {
    let mut base = strip_indefinite_article(phrase).trim();
    let mut trailing = "";
    if let Some(stripped) = base.strip_suffix('.') {
        base = stripped.trim_end();
        trailing = ".";
    }
    if base.contains(" or ") {
        let parts = base
            .split(" or ")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if parts.len() > 1 {
            let plural_parts = parts
                .iter()
                .map(|part| pluralize_noun_phrase(part))
                .collect::<Vec<_>>();
            return format!("{}{}", plural_parts.join(" or "), trailing);
        }
    }
    if let Some((head, tail)) = base.split_once(" with ") {
        return format!(
            "{} with {}{}",
            pluralize_noun_phrase(head),
            tail.trim(),
            trailing
        );
    }
    if let Some((head, tail)) = base.split_once(" without ") {
        return format!(
            "{} without {}{}",
            pluralize_noun_phrase(head),
            tail.trim(),
            trailing
        );
    }
    for suffix in [
        " you control",
        " you own",
        " an opponent controls",
        " an opponent owns",
        " target opponent controls",
        " target player controls",
        " target player owns",
        " that player controls",
        " that player owns",
        " active player controls",
        " active player owns",
        " defending player controls",
        " defending player owns",
        " attacking player controls",
        " attacking player owns",
        " damaged player controls",
        " damaged player owns",
        " a teammate controls",
        " a teammate owns",
        " in your graveyard",
        " in target player's graveyard",
        " in that player's graveyard",
        " in single graveyard",
        " in a graveyard",
        " in graveyard",
        " in your hand",
        " in target player's hand",
        " in that player's hand",
        " in a hand",
        " in your library",
        " in target player's library",
        " in that player's library",
        " in a library",
        " in exile",
    ] {
        if let Some(head) = base.strip_suffix(suffix) {
            let head = head.trim_end();
            let head_plural = pluralize_word(head);
            return format!("{head_plural}{suffix}{trailing}");
        }
    }
    if base.ends_with('s') {
        format!("{base}{trailing}")
    } else {
        format!("{}{}", pluralize_word(base), trailing)
    }
}

fn sacrifice_uses_chosen_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

fn describe_for_players_choose_types_then_sacrifice_rest(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let (tail, choose_effects) = for_players.effects.split_last()?;
    let sacrifice = tail.downcast_ref::<crate::effects::SacrificeEffect>()?;
    if sacrifice.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let Value::Count(count_filter) = &sacrifice.count else {
        return None;
    };
    if count_filter != &sacrifice.filter {
        return None;
    }

    let mut chooses = Vec::new();
    for effect in choose_effects {
        let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        chooses.push(choose);
    }
    if chooses.len() < 2 {
        return None;
    }

    let keep_tag = chooses.first()?.tag.as_str().to_string();
    let has_sacrifice_keep_guard = sacrifice
        .filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag.as_str() == keep_tag
        });
    if !has_sacrifice_keep_guard {
        return None;
    }

    let mut chosen_types = Vec::new();
    for choose in chooses {
        if choose.zone != Zone::Battlefield
            || choose.is_search
            || !choose.count.is_single()
            || choose.chooser != PlayerFilter::IteratedPlayer
            || choose.tag.as_str() != keep_tag
        {
            return None;
        }
        let has_keep_guard = choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                && constraint.tag.as_str() == keep_tag
        });
        if !has_keep_guard || choose.filter.card_types.len() != 1 {
            return None;
        }
        let card_type = *choose.filter.card_types.iter().next()?;
        let phrase = with_indefinite_article(describe_card_type_word_local(card_type));
        if !chosen_types.iter().any(|existing| existing == &phrase) {
            chosen_types.push(phrase);
        }
    }
    if chosen_types.len() < 2 {
        return None;
    }

    let list = join_with_and(&chosen_types);
    let (subject, choose_verb, sacrifice_verb, controls) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "chooses", "sacrifices", "they control"),
        PlayerFilter::Opponent => ("Each opponent", "chooses", "sacrifices", "they control"),
        PlayerFilter::You => ("You", "choose", "sacrifice", "you control"),
        _ => return None,
    };
    Some(format!(
        "{subject} {choose_verb} {list} from among permanents {controls}, then {sacrifice_verb} the rest"
    ))
}

fn describe_for_players_choose_then_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = for_players.effects[1].downcast_ref::<crate::effects::SacrificeEffect>()?;
    if choose.zone != Zone::Battlefield
        || choose.is_search
        || !choose.count.is_single()
        || choose.chooser != PlayerFilter::IteratedPlayer
        || !matches!(sacrifice.count, Value::Fixed(1))
        || sacrifice.player != PlayerFilter::IteratedPlayer
        || !sacrifice_uses_chosen_tag(&sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let (subject, verb, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "sacrifices", "their"),
        PlayerFilter::Opponent => ("Each opponent", "sacrifices", "their"),
        PlayerFilter::You => ("You", "sacrifice", "your"),
        _ => return None,
    };
    let chosen = with_indefinite_article(&choose.filter.description());
    Some(format!("{subject} {verb} {chosen} of {possessive} choice"))
}

fn describe_choose_then_sacrifice(
    choose: &crate::effects::ChooseObjectsEffect,
    sacrifice: &crate::effects::SacrificeEffect,
) -> Option<String> {
    let choose_exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    let sacrifice_count = match sacrifice.count {
        Value::Fixed(value) if value > 0 => value as usize,
        _ => return None,
    };
    if choose.zone != Zone::Battlefield
        || choose.is_search
        || choose_exact != sacrifice_count
        || sacrifice.player != choose.chooser
        || !sacrifice_uses_chosen_tag(&sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let player = describe_player_filter(&choose.chooser);
    let verb = player_verb(&player, "sacrifice", "sacrifices");
    let refers_to_triggering_object = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && matches!(constraint.tag.as_str(), "triggering" | "damaged")
    });
    let chosen = choose.filter.description();
    if sacrifice_count == 1 {
        if refers_to_triggering_object {
            return Some(format!("{player} {verb} it"));
        }
        if let Some(rest) = chosen.strip_prefix(&format!("{player}'s ")) {
            let chosen_kind = with_indefinite_article(rest);
            return Some(format!("{player} {verb} {chosen_kind} of their choice"));
        }
        let chosen = with_indefinite_article(&chosen);
        Some(format!("{player} {verb} {chosen}"))
    } else {
        let count_text = number_word(sacrifice_count as i32)
            .map(str::to_string)
            .unwrap_or_else(|| sacrifice_count.to_string());
        let chosen = pluralize_noun_phrase(&chosen);
        Some(format!("{player} {verb} {count_text} {chosen}"))
    }
}

fn describe_choose_then_destroy(
    choose: &crate::effects::ChooseObjectsEffect,
    destroy: &crate::effects::DestroyEffect,
) -> Option<String> {
    if choose.zone != Zone::Battlefield || choose.is_search || !choose.count.is_single() {
        return None;
    }
    let ChooseSpec::Tagged(tag) = &destroy.spec else {
        return None;
    };
    if tag.as_str() != choose.tag.as_str() {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let description = choose.filter.description();
    let chosen = if let Some(rest) = description.strip_prefix("target player's ") {
        format!("a {} they control", rest)
    } else if let Some(rest) = description.strip_prefix("that player's ") {
        format!("a {} they control", rest)
    } else {
        with_indefinite_article(&description)
    };
    Some(format!("{chooser} {choose_verb} {chosen}. Destroy it"))
}

fn describe_choose_then_for_each_copy(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
) -> Option<String> {
    if choose.is_search || choose.count.is_single() {
        return None;
    }
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let create_copy =
        for_each.effects[0].downcast_ref::<crate::effects::CreateTokenCopyEffect>()?;
    if !matches!(create_copy.target, ChooseSpec::Tagged(ref tag) if tag == &choose.tag) {
        return None;
    }
    if create_copy.controller != PlayerFilter::You
        || create_copy.enters_tapped
        || create_copy.has_haste
        || create_copy.enters_attacking
        || create_copy.attack_target_mode.is_some()
        || create_copy.exile_at_end_of_combat
        || create_copy.sacrifice_at_next_end_step
        || create_copy.exile_at_next_end_step
        || create_copy.pt_adjustment.is_some()
        || !create_copy.added_card_types.is_empty()
        || !create_copy.added_subtypes.is_empty()
        || !create_copy.removed_supertypes.is_empty()
        || create_copy.set_base_power_toughness.is_some()
        || create_copy.set_colors.is_some()
        || create_copy.set_card_types.is_some()
        || create_copy.set_subtypes.is_some()
        || !create_copy.granted_static_abilities.is_empty()
    {
        return None;
    }

    let selected = describe_choose_spec(
        &ChooseSpec::target(ChooseSpec::Object(choose.filter.clone())).with_count(choose.count),
    );
    Some(format!(
        "For each of {selected}, create {} tokens that are copies of that permanent",
        describe_value(&create_copy.count)
    ))
}

fn describe_choose_then_cant_block(
    choose: &crate::effects::ChooseObjectsEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let crate::effect::Restriction::Block(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != crate::effect::Until::EndOfTurn {
        return None;
    }
    if choose.zone != Zone::Battlefield || choose.is_search {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == choose.tag.as_str()
    }) {
        return None;
    }

    let choose_desc = choose.filter.description();
    let base = strip_leading_article(&choose_desc);
    let plural = pluralize_noun_phrase(base);
    let count_text = |n: usize| {
        number_word(n as i32)
            .map(str::to_string)
            .unwrap_or_else(|| n.to_string())
    };
    let sentence = if choose.count.is_dynamic_x() {
        format!("X target {plural} can't block this turn")
    } else {
        match (choose.count.min, choose.count.max) {
            (0, Some(max)) => {
                if max == 1 {
                    format!("Up to one target {base} can't block this turn")
                } else {
                    format!(
                        "Up to {} target {plural} can't block this turn",
                        count_text(max)
                    )
                }
            }
            (min, Some(max)) if min == max => {
                if min == 1 {
                    format!("Target {base} can't block this turn")
                } else {
                    format!("{} target {plural} can't block this turn", count_text(min))
                }
            }
            (0, None) => format!("Any number of target {plural} can't block this turn"),
            (min, None) => format!("At least {min} target {plural} can't block this turn"),
            (min, Some(max)) => format!("{min} to {max} target {plural} can't block this turn"),
        }
    };
    Some(sentence)
}

fn describe_tagged_target_then_cant_block(
    tagged: &crate::effects::TaggedEffect,
    cant: &crate::effects::CantEffect,
) -> Option<String> {
    let target_only = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let crate::effect::Restriction::Block(filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != crate::effect::Until::EndOfTurn {
        return None;
    }
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tagged.tag.as_str()
    }) {
        return None;
    }

    let subject = capitalize_first(&describe_choose_spec(&target_only.target));
    Some(format!("{subject} can't block this turn"))
}

fn tap_uses_chosen_tag(spec: &ChooseSpec, tag: &str) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_choose_then_tap_cost(
    choose: &crate::effects::ChooseObjectsEffect,
    tap: &crate::effects::TapEffect,
) -> Option<String> {
    if choose.zone != Zone::Battlefield || choose.is_search {
        return None;
    }
    if !tap_uses_chosen_tag(&tap.spec, choose.tag.as_str()) {
        return None;
    }

    if choose.count.is_single() {
        return Some(format!(
            "Tap {}",
            with_indefinite_article(&choose.filter.description())
        ));
    }

    let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
    let count_text = number_word(exact as i32)
        .map(str::to_string)
        .unwrap_or_else(|| exact.to_string());
    Some(format!(
        "Tap {} {}",
        count_text,
        pluralize_noun_phrase(&choose.filter.description())
    ))
}

fn exile_uses_chosen_tag(spec: &ChooseSpec, tag: &str) -> bool {
    matches!(spec.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn move_to_exile_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Exile
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_for_each_filter(filter: &ObjectFilter) -> String {
    let mut base_filter = filter.clone();
    base_filter.controller = None;

    let description = base_filter.description();
    let mut base = strip_indefinite_article(&description).to_string();
    if let Some(rest) = base.strip_prefix("another ") {
        base = format!("other {rest}");
    }
    if let Some(rest) = base.strip_prefix("permanent ")
        && matches!(filter.zone, None | Some(Zone::Battlefield))
    {
        if filter.controller.is_some() {
            base = rest.to_string();
        } else {
            base = format!("{rest} on the battlefield");
        }
    }
    if let Some(action) = describe_tagged_this_way_action(filter) {
        if action == "exiled" {
            if let Some(head) = base.strip_suffix(" in exile") {
                base = head.trim().to_string();
            } else if let Some((head, tail)) = base.split_once(" in exile ") {
                base = format!("{} {}", head.trim(), tail.trim());
            }
        } else if action == "revealed" {
            if let Some(head) = base.strip_suffix(" permanent") {
                base = format!("{} card", head.trim());
            } else if let Some(head) = base.strip_suffix(" permanents") {
                base = format!("{} cards", head.trim());
            }
        }
        base = format!("{base} {action} this way");
    }

    if let Some(controller) = &filter.controller {
        if matches!(controller, PlayerFilter::You) {
            return format!("{base} you control");
        }
        return format!("{base} {} controls", describe_player_filter(controller));
    }
    base
}

fn describe_tagged_this_way_action(filter: &ObjectFilter) -> Option<&'static str> {
    filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        let tag = constraint.tag.as_str();
        if tag == "__it__" && filter.zone == Some(Zone::Exile) {
            return Some("exiled");
        }
        if tag.starts_with("exiled_") {
            Some("exiled")
        } else if tag.starts_with("revealed_") {
            Some("revealed")
        } else if tag.starts_with("destroyed_") {
            Some("destroyed")
        } else if tag.starts_with("sacrificed_") {
            Some("sacrificed")
        } else if tag.starts_with("discarded_") {
            Some("discarded")
        } else if tag.starts_with("milled_") {
            Some("milled")
        } else {
            None
        }
    })
}

fn describe_each_controlled_by_iterated(filter: &ObjectFilter) -> Option<String> {
    if filter.controller != Some(PlayerFilter::IteratedPlayer) {
        return None;
    }
    if !filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.colors.is_none()
        && filter.excluded_card_types.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.excluded_colors.is_empty()
        && !filter.token
        && !filter.nontoken
        && !filter.tapped
        && !filter.untapped
        && !filter.attacking
        && !filter.nonattacking
        && !filter.blocking
        && !filter.nonblocking
        && !filter.blocked
        && !filter.unblocked
        && filter.zone.is_none()
        && filter.tagged_constraints.is_empty()
        && filter.targets_object.is_none()
        && filter.targets_player.is_none()
        && filter.ability_markers.is_empty()
        && filter.excluded_ability_markers.is_empty()
        && !filter.noncommander
    {
        let words = filter
            .card_types
            .iter()
            .map(|card_type| format!("{:?}", card_type).to_ascii_lowercase())
            .collect::<Vec<_>>();
        let list = match words.len() {
            0 => String::new(),
            1 => words[0].clone(),
            2 => format!("{} and {}", words[0], words[1]),
            _ => {
                let mut out = words[..words.len() - 1].join(", ");
                out.push_str(", and ");
                out.push_str(&words[words.len() - 1]);
                out
            }
        };
        return Some(format!("each {list} they control"));
    }
    None
}

fn describe_for_players_damage_and_controlled_damage(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let deal_player = for_players.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        deal_player.target,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }
    let for_each = for_players.effects[1].downcast_ref::<crate::effects::ForEachObject>()?;
    if for_each.effects.len() != 1 {
        return None;
    }
    let deal_object = for_each.effects[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if deal_object.amount != deal_player.amount {
        return None;
    }
    if !matches!(deal_object.target, ChooseSpec::Iterated) {
        return None;
    }
    let objects = describe_each_controlled_by_iterated(&for_each.filter)?;
    let player_filter_text = describe_player_filter(&for_players.filter);
    let each_player = strip_leading_article(&player_filter_text);
    Some(format!(
        "Deal {} damage to each {} and {}",
        describe_value(&deal_player.amount),
        each_player,
        objects
    ))
}

fn describe_for_players_reveal_top_mana_value_life_then_put_into_hand(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    let (subject, possessive) = match for_players.filter {
        PlayerFilter::Any => ("Each player", "their"),
        PlayerFilter::Opponent => ("Each opponent", "their"),
        _ => return None,
    };
    if for_players.effects.len() != 3 {
        return None;
    }
    let reveal = for_players.effects[0].downcast_ref::<crate::effects::RevealTopEffect>()?;
    let reveal_tag = reveal.tag.as_ref()?;
    if reveal.player != PlayerFilter::IteratedPlayer
        || !reveal_tag.as_str().starts_with("revealed_")
    {
        return None;
    }
    let lose = for_players.effects[1].downcast_ref::<crate::effects::LoseLifeEffect>()?;
    if lose.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer) {
        return None;
    }
    let Value::ManaValueOf(spec) = &lose.amount else {
        return None;
    };
    if !matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == reveal_tag) {
        return None;
    }
    let move_to_zone = for_players.effects[2].downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Hand {
        return None;
    }
    if !matches!(
        move_to_zone.target.base(),
        ChooseSpec::Tagged(tag) if tag == reveal_tag
    ) {
        return None;
    }

    Some(format!(
        "{subject} reveals the top card of {possessive} library, loses life equal to that card's mana value, then puts it into {possessive} hand"
    ))
}

fn describe_draw_for_each(draw: &crate::effects::DrawCardsEffect) -> Option<String> {
    let player = describe_player_filter(&draw.player);
    let verb = player_verb(&player, "draw", "draws");
    match &draw.count {
        Value::Count(filter) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_for_each_filter(filter)
        )),
        Value::CreaturesDiedThisTurnControlledBy(controller) => {
            let suffix = match controller {
                PlayerFilter::You => "under your control this turn".to_string(),
                PlayerFilter::Opponent => "under an opponent's control this turn".to_string(),
                PlayerFilter::Any => "this turn".to_string(),
                other => format!(
                    "under {} control this turn",
                    describe_possessive_player_filter(other)
                ),
            };
            Some(format!(
                "{player} {verb} a card for each creature that died {suffix}"
            ))
        }
        Value::SpellsCastThisTurn(spell_caster) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_spells_cast_this_turn_each(spell_caster)
        )),
        Value::SpellsCastThisTurnMatching {
            player: spell_caster,
            filter,
            exclude_source,
        } => {
            let base = describe_for_each_filter(filter);
            let prefix = if *exclude_source { "other " } else { "" };
            let tail = match spell_caster {
                PlayerFilter::You => "you've cast this turn".to_string(),
                PlayerFilter::Opponent => "an opponent has cast this turn".to_string(),
                PlayerFilter::Any => "cast this turn".to_string(),
                other => format!(
                    "cast this turn by {}",
                    strip_leading_article(&describe_player_filter(other))
                ),
            };
            Some(format!(
                "{player} {verb} a card for each {prefix}{base} {tail}"
            ))
        }
        Value::CountersOnSource(counter_type) => Some(format!(
            "{player} {verb} a card for each {} counter on this permanent",
            describe_counter_type(*counter_type)
        )),
        Value::CountersOn(spec, Some(counter_type)) => Some(format!(
            "{player} {verb} a card for each {} counter on {}",
            describe_counter_type(*counter_type),
            describe_choose_spec(spec)
        )),
        Value::CountersOn(spec, None) => Some(format!(
            "{player} {verb} a card for each counter on {}",
            describe_choose_spec(spec)
        )),
        Value::BasicLandTypesAmong(filter) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_basic_land_types_among(filter)
        )),
        Value::ColorsAmong(filter) => Some(format!(
            "{player} {verb} a card for each {}",
            describe_colors_among(filter)
        )),
        _ => None,
    }
}

fn describe_create_for_each_count(value: &Value) -> Option<String> {
    match value {
        Value::Count(filter) => Some(describe_for_each_filter(filter)),
        Value::BasicLandTypesAmong(filter) => Some(describe_basic_land_types_among(filter)),
        Value::ColorsAmong(filter) => Some(describe_colors_among(filter)),
        Value::ColorsOfManaSpentToCastThisSpell => {
            Some("color of mana spent to cast this spell".to_string())
        }
        Value::CreaturesDiedThisTurn => Some("creature that died this turn".to_string()),
        _ => None,
    }
}

fn value_is_iterated_object_count(value: &Value) -> bool {
    let Value::Count(filter) = value else {
        return false;
    };
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == "__it__"
    })
}

fn describe_compact_token_count(value: &Value, token_name: &str) -> String {
    match value {
        Value::Fixed(1) => format!("a {token_name} token"),
        Value::Fixed(n) => format!("{n} {token_name} tokens"),
        Value::Count(filter) => {
            format!(
                "a {token_name} token for each {}",
                describe_for_each_count_filter(filter)
            )
        }
        Value::CountScaled(filter, multiplier) => {
            if *multiplier == 1 {
                format!(
                    "a {token_name} token for each {}",
                    describe_for_each_count_filter(filter)
                )
            } else {
                format!(
                    "{multiplier} {token_name} tokens for each {}",
                    describe_for_each_count_filter(filter)
                )
            }
        }
        Value::BasicLandTypesAmong(filter) => {
            let lands = describe_for_each_filter(filter);
            let lands = if lands == "land" {
                "lands".to_string()
            } else if let Some(rest) = lands.strip_prefix("land ") {
                format!("lands {rest}")
            } else {
                lands
            };
            format!("a {token_name} token for each basic land type among {lands}")
        }
        Value::ColorsAmong(filter) => {
            format!(
                "a {token_name} token for each color among {}",
                describe_for_each_filter(filter)
            )
        }
        Value::CreaturesDiedThisTurn => {
            format!("a {token_name} token for each creature that died this turn")
        }
        Value::CreaturesDiedThisTurnControlledBy(filter) => {
            let suffix = match filter {
                PlayerFilter::You => "under your control this turn".to_string(),
                PlayerFilter::Opponent => "under an opponent's control this turn".to_string(),
                PlayerFilter::Any => "this turn".to_string(),
                other => format!(
                    "under {} control this turn",
                    describe_possessive_player_filter(other)
                ),
            };
            format!("a {token_name} token for each creature that died {suffix}")
        }
        Value::ColorsOfManaSpentToCastThisSpell => {
            format!("a {token_name} token for each color of mana spent to cast this spell")
        }
        _ => format!("{} {token_name} token(s)", describe_value(value)),
    }
}

fn describe_compact_create_token(
    create_token: &crate::effects::CreateTokenEffect,
) -> Option<String> {
    if create_token.exile_at_end_of_combat
        || create_token.sacrifice_at_end_of_combat
        || create_token.sacrifice_at_next_end_step
        || create_token.exile_at_next_end_step
    {
        return None;
    }

    let token_name = create_token.token.name();
    let is_compact_named_token = matches!(
        token_name,
        "Treasure" | "Clue" | "Food" | "Blood" | "Gold" | "Powerstone"
    );
    if !is_compact_named_token {
        return None;
    }

    let mut amount = describe_compact_token_count(&create_token.count, token_name);
    let state = if create_token.enters_tapped && create_token.enters_attacking {
        Some("tapped and attacking")
    } else if create_token.enters_tapped {
        Some("tapped")
    } else if create_token.enters_attacking {
        Some("attacking")
    } else {
        None
    };
    if let Some(state) = state {
        amount = amount.replacen(token_name, &format!("{state} {token_name}"), 1);
    }

    if matches!(create_token.controller, PlayerFilter::You) {
        Some(format!("Create {amount}"))
    } else {
        Some(format!(
            "Create {amount} under {} control",
            describe_possessive_player_filter(&create_token.controller)
        ))
    }
}

fn choose_exact_count(choose: &crate::effects::ChooseObjectsEffect) -> Option<usize> {
    choose.count.max.filter(|max| *max == choose.count.min)
}

fn describe_choose_selection(choose: &crate::effects::ChooseObjectsEffect) -> String {
    if choose.top_only {
        if let Some(exact) = choose_exact_count(choose) {
            if exact > 1 {
                let count_text = number_word(exact as i32)
                    .map(str::to_string)
                    .unwrap_or_else(|| exact.to_string());
                return format!("the top {count_text} cards");
            }
        }
        return "the top card".to_string();
    }

    let filter_text = choose.filter.description();
    let mut card_desc = filter_text
        .split(" in ")
        .next()
        .unwrap_or(filter_text.as_str())
        .trim()
        .to_string();
    for owner_prefix in [
        "target player's ",
        "that player's ",
        "their ",
        "your ",
        "an opponent's ",
    ] {
        if let Some(rest) = card_desc.strip_prefix(owner_prefix) {
            card_desc = rest.to_string();
            break;
        }
    }
    if let Some(rest) = card_desc.strip_suffix(" hands") {
        card_desc = format!("{rest} hand");
    }
    if let Some(rest) = card_desc.strip_prefix("card ") {
        card_desc = format!("{rest} card");
    }

    if choose.count.is_single() {
        return with_indefinite_article(&card_desc);
    }
    if let Some(exact) = choose_exact_count(choose) {
        let count_text = number_word(exact as i32)
            .map(str::to_string)
            .unwrap_or_else(|| exact.to_string());
        return format!("{count_text} {}", pluralize_noun_phrase(&card_desc));
    }
    format!(
        "{} {}",
        describe_choice_count(&choose.count),
        pluralize_noun_phrase(&card_desc)
    )
}

fn describe_choose_then_exile(
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if choose.is_search || !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str()) {
        return None;
    }

    let zone_text = match choose.zone {
        Zone::Hand => "hand",
        Zone::Graveyard => "graveyard",
        Zone::Library => "library",
        _ => return None,
    };
    let chooser = describe_player_filter(&choose.chooser);
    let verb = player_verb(&chooser, "exile", "exiles");
    let chosen = describe_choose_selection(choose);
    let origin = if choose.zone == Zone::Library && choose.top_only {
        format!(
            "of {} {zone_text}",
            describe_possessive_player_filter(&choose.chooser)
        )
    } else {
        format!(
            "from {} {zone_text}",
            describe_possessive_player_filter(&choose.chooser)
        )
    };
    let face_down_suffix = if exile.face_down { " face down" } else { "" };
    Some(format!(
        "{chooser} {verb} {chosen} {origin}{face_down_suffix}"
    ))
}

fn move_to_library_uses_chosen_tag(
    move_to_zone: &crate::effects::MoveToZoneEffect,
    tag: &str,
) -> bool {
    move_to_zone.zone == Zone::Library
        && matches!(move_to_zone.target.base(), ChooseSpec::Tagged(t) if t.as_str() == tag)
}

fn describe_choose_zone_origin(
    choose: &crate::effects::ChooseObjectsEffect,
    zone_text: &str,
) -> String {
    match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => format!("from their {zone_text}"),
        Some(owner) => format!(
            "from {} {zone_text}",
            describe_possessive_player_filter(owner)
        ),
        None => format!("from a {zone_text}"),
    }
}

fn describe_choose_then_move_to_library(
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if !move_to_library_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }

    let origin = match choose.zone {
        Zone::Hand => describe_choose_zone_origin(choose, "hand"),
        Zone::Graveyard => describe_choose_zone_origin(choose, "graveyard"),
        Zone::Library => {
            if choose.top_only {
                match choose.filter.owner.as_ref() {
                    Some(PlayerFilter::IteratedPlayer) => {
                        "from the top of their library".to_string()
                    }
                    Some(owner) => format!(
                        "from the top of {} library",
                        describe_possessive_player_filter(owner)
                    ),
                    None => "from the top of a library".to_string(),
                }
            } else {
                describe_choose_zone_origin(choose, "library")
            }
        }
        _ => return None,
    };

    let chooser = describe_player_filter(&choose.chooser);
    let choose_verb = player_verb(&chooser, "choose", "chooses");
    let put_verb = player_verb(&chooser, "put", "puts");
    let chosen = describe_choose_selection(choose);
    let moved_ref = if choose.count.is_single() {
        "it"
    } else {
        "them"
    };

    let destination = match choose.filter.owner.as_ref() {
        Some(PlayerFilter::IteratedPlayer) => "their library".to_string(),
        Some(owner) => format!("{} library", describe_possessive_player_filter(owner)),
        None => owner_library_phrase_for_spec(&move_to_zone.target).to_string(),
    };
    let placement = if move_to_zone.to_top {
        "on top of"
    } else {
        "on the bottom of"
    };

    Some(format!(
        "{chooser} {choose_verb} {chosen} {origin}, then {put_verb} {moved_ref} {placement} {destination}"
    ))
}

fn describe_look_at_top_then_choose_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    exile: &crate::effects::ExileEffect,
) -> Option<String> {
    if !exile_uses_chosen_tag(&exile.spec, choose.tag.as_str()) {
        return None;
    }
    describe_look_at_top_then_choose_exile_text(look_at_top, choose, exile.face_down)
}

fn describe_look_at_top_then_choose_move_to_exile(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    move_to_zone: &crate::effects::MoveToZoneEffect,
) -> Option<String> {
    if !move_to_exile_uses_chosen_tag(move_to_zone, choose.tag.as_str()) {
        return None;
    }
    describe_look_at_top_then_choose_exile_text(look_at_top, choose, false)
}

fn describe_look_at_top_then_choose_exile_text(
    look_at_top: &crate::effects::LookAtTopCardsEffect,
    choose: &crate::effects::ChooseObjectsEffect,
    face_down: bool,
) -> Option<String> {
    if choose.zone != Zone::Library || choose.is_search || !choose.count.is_single() {
        return None;
    }
    let references_looked = choose.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) && constraint.tag.as_str() == look_at_top.tag.as_str()
    });
    if !references_looked {
        return None;
    }

    let owner = describe_possessive_player_filter(&look_at_top.player);
    let (count_text, noun, singular_count) = describe_look_count_and_noun(&look_at_top.count);
    let exile_ref = if singular_count { "it" } else { "one of them" };
    let face_down_suffix = if face_down { " face down" } else { "" };
    Some(format!(
        "Look at the top {count_text} {noun} of {owner} library, then exile {exile_ref}{face_down_suffix}"
    ))
}

fn describe_look_count_and_noun(count: &Value) -> (String, &'static str, bool) {
    if let Value::Fixed(n) = count
        && *n >= 0
    {
        let count_u32 = *n as u32;
        let text = small_number_word(count_u32)
            .map(str::to_string)
            .unwrap_or_else(|| n.to_string());
        let singular = *n == 1;
        return (text, if singular { "card" } else { "cards" }, singular);
    }
    (describe_value(count), "cards", false)
}

fn describe_draw_then_discard(
    draw: &crate::effects::DrawCardsEffect,
    discard: &crate::effects::DiscardEffect,
) -> Option<String> {
    if draw.player != discard.player {
        return None;
    }
    let player = describe_player_filter(&draw.player);
    let mut text = format!(
        "{player} {} {}, then {} {}",
        player_verb(&player, "draw", "draws"),
        describe_card_count(&draw.count),
        player_verb(&player, "discard", "discards"),
        describe_discard_count(&discard.count, discard.card_filter.as_ref())
    );
    if discard.random {
        text.push_str(" at random");
    }
    Some(text)
}

fn describe_mill_then_may_return(
    mill: &crate::effects::MillEffect,
    may: &crate::effects::MayEffect,
) -> Option<String> {
    if may.effects.len() != 1 {
        return None;
    }
    let return_effect = may.effects.first()?;
    let is_return_to_hand = return_effect
        .downcast_ref::<crate::effects::ReturnToHandEffect>()
        .is_some()
        || return_effect
            .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
            .is_some();
    if !is_return_to_hand {
        return None;
    }

    let decider = may.decider.as_ref().unwrap_or(&mill.player);
    if decider != &mill.player {
        return None;
    }

    let player = describe_player_filter(&mill.player);
    let mill_clause = format!(
        "{player} {} {}",
        player_verb(&player, "mill", "mills"),
        describe_card_count(&mill.count)
    );
    let return_clause = lowercase_first(&describe_effect(return_effect));
    Some(format!("{mill_clause}, then {player} may {return_clause}"))
}

fn describe_gain_life_then_scry(
    gain: &crate::effects::GainLifeEffect,
    scry: &crate::effects::ScryEffect,
) -> Option<String> {
    let crate::target::ChooseSpec::Player(gain_player) = gain.player.base() else {
        return None;
    };
    if gain_player != &scry.player {
        return None;
    }
    if !matches!(gain.amount, Value::Fixed(_) | Value::X) {
        return None;
    }

    let player = describe_player_filter(gain_player);
    let gain_clause = format!(
        "{player} {} {} life",
        player_verb(&player, "gain", "gains"),
        describe_value(&gain.amount)
    );
    if *gain_player == PlayerFilter::You {
        return Some(format!(
            "{gain_clause} and scry {}",
            describe_value(&scry.count)
        ));
    }
    Some(format!(
        "{gain_clause} and {player} {} {}",
        player_verb(&player, "scry", "scries"),
        describe_value(&scry.count)
    ))
}

fn describe_scry_then_draw(
    scry: &crate::effects::ScryEffect,
    draw: &crate::effects::DrawCardsEffect,
) -> Option<String> {
    if scry.player != draw.player {
        return None;
    }
    if scry.player == PlayerFilter::You {
        return Some(format!(
            "Scry {}, then draw {}",
            describe_value(&scry.count),
            describe_card_count(&draw.count)
        ));
    }

    let player = describe_player_filter(&scry.player);
    Some(format!(
        "{player} {} {}, then {} {}",
        player_verb(&player, "scry", "scries"),
        describe_value(&scry.count),
        player_verb(&player, "draw", "draws"),
        describe_card_count(&draw.count)
    ))
}

fn describe_where_x_basis(value: &Value) -> Option<String> {
    match value {
        Value::Count(filter) => Some(format!(
            "the number of {}",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        )),
        Value::BasicLandTypesAmong(filter) => Some(format!(
            "the number of {}",
            describe_basic_land_types_among(filter)
        )),
        Value::ColorsAmong(filter) => {
            Some(format!("the number of {}", describe_colors_among(filter)))
        }
        Value::CountScaled(filter, multiplier) if *multiplier == 1 => Some(format!(
            "the number of {}",
            pluralize_noun_phrase(&describe_for_each_count_filter(filter))
        )),
        _ => {
            let rendered = describe_value(value);
            if rendered.starts_with("the number of ") {
                Some(rendered)
            } else {
                None
            }
        }
    }
}

fn describe_deal_damage_then_gain_life(
    deal: &crate::effects::DealDamageEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    let where_x = if deal.amount == gain.amount {
        describe_where_x_basis(&deal.amount)
    } else if matches!(deal.amount, Value::X) {
        describe_where_x_basis(&gain.amount)
    } else if matches!(gain.amount, Value::X) {
        describe_where_x_basis(&deal.amount)
    } else {
        None
    }?;

    let target = describe_choose_spec(&deal.target);
    let player = describe_choose_spec(&gain.player);
    Some(format!(
        "Deal X damage to {target} and {player} {} X life, where X is {where_x}",
        player_verb(&player, "gain", "gains")
    ))
}

fn describe_lose_life_then_gain_life(
    lose: &crate::effects::LoseLifeEffect,
    gain: &crate::effects::GainLifeEffect,
) -> Option<String> {
    let where_x = if lose.amount == gain.amount {
        describe_where_x_basis(&lose.amount)
    } else if matches!(lose.amount, Value::X) {
        describe_where_x_basis(&gain.amount)
    } else if matches!(gain.amount, Value::X) {
        describe_where_x_basis(&lose.amount)
    } else {
        None
    }?;

    let lose_player = describe_choose_spec(&lose.player);
    let gain_player = describe_choose_spec(&gain.player);
    Some(format!(
        "{lose_player} {} X life and {gain_player} {} X life, where X is {where_x}",
        player_verb(&lose_player, "lose", "loses"),
        player_verb(&gain_player, "gain", "gains")
    ))
}

fn describe_with_id_then_if(
    with_id: &crate::effects::WithIdEffect,
    if_effect: &crate::effects::IfEffect,
) -> Option<String> {
    if if_effect.condition != with_id.id {
        return None;
    }

    let setup = describe_effect(&with_id.effect);
    let then_text = describe_effect_list(&if_effect.then);
    let else_text = describe_effect_list(&if_effect.else_);

    let condition = if let Some(may) = with_id.effect.downcast_ref::<crate::effects::MayEffect>() {
        let who = may
            .decider
            .as_ref()
            .map(describe_player_filter)
            .unwrap_or_else(|| "you".to_string());
        match if_effect.predicate {
            EffectPredicate::DidNotHappen => {
                if who == "you" {
                    "If you don't".to_string()
                } else {
                    format!("If {who} doesn't")
                }
            }
            _ => {
                if who == "you" {
                    "If you do".to_string()
                } else {
                    format!("If {who} does")
                }
            }
        }
    } else {
        match if_effect.predicate {
            EffectPredicate::Happened => "If it happened".to_string(),
            EffectPredicate::HappenedNotReplaced => {
                let setup_is_destroy = with_id
                    .effect
                    .downcast_ref::<crate::effects::DestroyEffect>()
                    .is_some()
                    || with_id
                        .effect
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::DestroyEffect>()
                        })
                        .is_some();
                if setup_is_destroy {
                    "If that permanent dies this way".to_string()
                } else {
                    "If it happened and wasn't replaced".to_string()
                }
            }
            _ => format!("If {}", describe_effect_predicate(&if_effect.predicate)),
        }
    };

    if else_text.is_empty() {
        Some(format!("{setup}. {condition}, {then_text}"))
    } else {
        Some(format!(
            "{setup}. {condition}, {then_text}. Otherwise, {else_text}"
        ))
    }
}

fn describe_with_id_then_choose_new_targets(
    with_id: &crate::effects::WithIdEffect,
    choose_new: &crate::effects::ChooseNewTargetsEffect,
) -> Option<String> {
    if choose_new.from_effect != with_id.id {
        return None;
    }

    let base = describe_effect(&with_id.effect);
    let chooser = choose_new
        .chooser
        .as_ref()
        .map(describe_player_filter)
        .unwrap_or_else(|| "you".to_string());
    let choose_phrase = if choose_new.may {
        if chooser == "you" {
            "You may choose new targets for the copy".to_string()
        } else {
            format!("{chooser} may choose new targets for the copy")
        }
    } else if chooser == "you" {
        "You choose new targets for the copy".to_string()
    } else {
        format!("{chooser} chooses new targets for the copy")
    };

    Some(format!("{base}. {choose_phrase}"))
}

enum SearchDestination {
    Battlefield {
        tapped: bool,
        controller: PlayerFilter,
    },
    Hand,
    Graveyard,
    Exile,
    LibraryTop,
}

fn describe_search_choose_for_each(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
    shuffle_before_move: bool,
) -> Option<String> {
    let search_like = choose.is_search
        || (choose.zone == Zone::Library && choose.tag.as_str().starts_with("searched_"));
    if !search_like || choose.zone != Zone::Library {
        return None;
    }
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }
    let library_owner_filter = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);

    let destination = if let Some(put) =
        for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        if !matches!(put.target, ChooseSpec::Iterated) {
            return None;
        }
        SearchDestination::Battlefield {
            tapped: put.tapped,
            controller: put.controller.clone(),
        }
    } else if let Some(return_to_hand) =
        for_each.effects[0].downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        if !matches!(return_to_hand.spec, ChooseSpec::Iterated) {
            return None;
        }
        SearchDestination::Hand
    } else if let Some(move_to_zone) =
        for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        if !matches!(move_to_zone.target, ChooseSpec::Iterated) {
            return None;
        }
        if move_to_zone.zone == Zone::Battlefield {
            SearchDestination::Battlefield {
                tapped: false,
                controller: choose.chooser.clone(),
            }
        } else if move_to_zone.zone == Zone::Hand {
            SearchDestination::Hand
        } else if move_to_zone.zone == Zone::Graveyard {
            SearchDestination::Graveyard
        } else if move_to_zone.zone == Zone::Exile {
            SearchDestination::Exile
        } else if move_to_zone.zone == Zone::Library && move_to_zone.to_top {
            SearchDestination::LibraryTop
        } else {
            return None;
        }
    } else {
        return None;
    };

    if let Some(shuffle) = shuffle
        && shuffle.player != *library_owner_filter
    {
        return None;
    }

    let mut implied_filter = choose.filter.clone();
    // The searched library owner is already called out by "Search ... library".
    implied_filter.owner = None;
    let implied_filter_text = if implied_filter == ObjectFilter::default() {
        "card".to_string()
    } else {
        implied_filter.description()
    };
    let filter_text = if choose.description.trim().is_empty()
        || choose.description.trim().eq_ignore_ascii_case("objects")
    {
        implied_filter_text
    } else {
        choose.description.trim().to_string()
    };
    let selection_text = if choose.count.max == Some(1) {
        with_indefinite_article(&filter_text)
    } else {
        let mut count_text = describe_choice_count(&choose.count);
        if count_text == "any number" {
            count_text = "any number of".to_string();
        }
        format!("{count_text} {filter_text}")
    };
    let selection_text = describe_search_selection_with_cards(&selection_text);
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

    let mut text;
    match destination {
        SearchDestination::Battlefield { tapped, controller } => {
            let control_suffix = if controller == *library_owner_filter {
                String::new()
            } else {
                format!(
                    " under {} control",
                    describe_possessive_player_filter(&controller)
                )
            };
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, shuffle, then put {} onto the battlefield{}",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    control_suffix
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} onto the battlefield{}",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    control_suffix
                )
            };
            if tapped {
                text.push_str(" tapped");
            }
        }
        SearchDestination::Hand => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, shuffle, then put {} into {} hand",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(library_owner_filter)
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} into {} hand",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(library_owner_filter)
                )
            };
        }
        SearchDestination::Graveyard => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, shuffle, then put {} into {} graveyard",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(library_owner_filter)
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} into {} graveyard",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(library_owner_filter)
                )
            };
        }
        SearchDestination::Exile => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, shuffle, then exile {}",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                )
            } else {
                format!(
                    "Search {} library for {}{}, exile {}",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                )
            };
        }
        SearchDestination::LibraryTop => {
            text = if shuffle.is_some() && shuffle_before_move {
                format!(
                    "Search {} library for {}{}, then shuffle and put {} on top of {} library",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(library_owner_filter)
                )
            } else {
                format!(
                    "Search {} library for {}{}, put {} on top of {} library",
                    describe_possessive_player_filter(library_owner_filter),
                    selection_text,
                    reveal_clause,
                    pronoun,
                    describe_possessive_player_filter(library_owner_filter)
                )
            };
            if !choose.count.is_single() {
                text.push_str(" in any order");
            }
        }
    }
    if shuffle.is_some() && !shuffle_before_move {
        text.push_str(", then shuffle");
    }
    Some(text)
}

fn describe_search_sequence(sequence: &crate::effects::SequenceEffect) -> Option<String> {
    if sequence.effects.len() < 2 || sequence.effects.len() > 3 {
        return None;
    }
    let choose = sequence.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if let Some(for_each) =
        sequence.effects[1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        let shuffle = if sequence.effects.len() == 3 {
            Some(sequence.effects[2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?)
        } else {
            None
        };
        return describe_search_choose_for_each(choose, for_each, shuffle, false);
    }
    if sequence.effects.len() == 3
        && let Some(shuffle) =
            sequence.effects[1].downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(for_each) =
            sequence.effects[2].downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        return describe_search_choose_for_each(choose, for_each, Some(shuffle), true);
    }
    None
}

fn describe_reveal_until_sequence(sequence: &crate::effects::SequenceEffect) -> Option<String> {
    if sequence.effects.len() != 3 {
        return None;
    }
    let choose = sequence.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = sequence.effects[1].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = sequence.effects[2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    if choose.zone != Zone::Library || !choose.top_only || !choose.reveal || choose.is_search {
        return None;
    }
    if for_each.tag != choose.tag {
        return None;
    }
    if shuffle.player != choose.chooser {
        return None;
    }
    if for_each.effects.len() != 1 {
        return None;
    }
    let put = for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    if !matches!(put.target, ChooseSpec::Iterated) || put.tapped {
        return None;
    }
    if put.controller != choose.chooser {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser);
    let library_owner = describe_possessive_player_filter(&choose.chooser);

    let shares_card_type_with_it = choose.filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::target::TaggedOpbjectRelation::SharesCardType
            && constraint.tag.as_str() == "__it__"
    });
    let selection = if shares_card_type_with_it {
        "a card that shares a card type with it".to_string()
    } else {
        strip_leading_article(&choose.filter.description()).to_string()
    };

    Some(format!(
        "{chooser} reveals cards from the top of {library_owner} library until they reveal {selection}, puts that card onto the battlefield, then shuffles"
    ))
}

fn describe_effect(effect: &Effect) -> String {
    with_effect_render_depth(|| describe_effect_impl(effect))
}

fn describe_tap_or_untap_mode(choose_mode: &crate::effects::ChooseModeEffect) -> Option<String> {
    if choose_mode.modes.len() != 2 {
        return None;
    }
    let is_choose_one = matches!(choose_mode.choose_count, Value::Fixed(1))
        && choose_mode
            .min_choose_count
            .as_ref()
            .is_none_or(|value| matches!(value, Value::Fixed(1)));
    if !is_choose_one {
        return None;
    }
    let mut shared_target: Option<String> = None;
    let mut saw_tap = false;
    let mut saw_untap = false;
    let mut terse_mode_labels = true;
    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let mode_label = mode
            .description
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if mode_label != "tap" && mode_label != "untap" {
            terse_mode_labels = false;
        }
        let effect = &mode.effects[0];
        if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
            saw_tap = true;
            let candidate = describe_choose_spec(&tap.spec);
            if let Some(existing) = &shared_target {
                if existing != &candidate {
                    return None;
                }
            } else {
                shared_target = Some(candidate);
            }
            continue;
        }
        if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
            saw_untap = true;
            let candidate = describe_choose_spec(&untap.spec);
            if let Some(existing) = &shared_target {
                if existing != &candidate {
                    return None;
                }
            } else {
                shared_target = Some(candidate);
            }
            continue;
        }
        return None;
    }
    if saw_tap && saw_untap {
        let target = shared_target.unwrap_or_else(|| "that object".to_string());
        if terse_mode_labels {
            return Some(format!("Tap or untap {target}"));
        }
        return Some(format!("Choose one — Tap {target}. • Untap {target}."));
    }
    None
}

fn describe_put_or_remove_counter_mode(
    choose_mode: &crate::effects::ChooseModeEffect,
) -> Option<String> {
    if choose_mode.modes.len() != 2 {
        return None;
    }
    let is_choose_one = matches!(choose_mode.choose_count, Value::Fixed(1))
        && choose_mode
            .min_choose_count
            .as_ref()
            .is_none_or(|value| matches!(value, Value::Fixed(1)));
    if !is_choose_one {
        return None;
    }

    let mut put_mode: Option<(&crate::effects::PutCountersEffect, String)> = None;
    let mut remove_mode: Option<(&crate::effects::RemoveCountersEffect, String)> = None;
    let mut remove_followup_mode: Option<crate::effects::ChooseModeEffect> = None;

    for mode in &choose_mode.modes {
        let description = if mode.description.trim().is_empty() {
            describe_effect_list(&mode.effects)
        } else {
            mode.description.trim().to_string()
        };
        if mode.effects.len() == 1 {
            let effect = &mode.effects[0];
            if let Some(put) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
                put_mode = Some((put, description));
                continue;
            }
            if let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
                remove_mode = Some((remove, description));
                continue;
            }
            return None;
        }

        if mode.effects.len() == 2
            && let Some(with_id) = mode.effects[0].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(remove) = with_id
                .effect
                .downcast_ref::<crate::effects::RemoveCountersEffect>()
            && let Some(if_effect) = mode.effects[1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && matches!(if_effect.predicate, EffectPredicate::Happened)
            && if_effect.else_.is_empty()
            && if_effect.then.len() == 1
            && let Some(followup_choose) =
                if_effect.then[0].downcast_ref::<crate::effects::ChooseModeEffect>()
        {
            remove_mode = Some((remove, description));
            remove_followup_mode = Some(followup_choose.clone());
            continue;
        }

        return None;
    }

    let (put_effect, put_description) = put_mode?;
    let (remove_effect, remove_description) = remove_mode?;
    if put_effect.target != remove_effect.target {
        return None;
    }

    let put_clause = put_description.trim().trim_end_matches('.');
    let remove_clause = lowercase_first(remove_description.trim().trim_end_matches('.'));
    if !put_clause.to_ascii_lowercase().starts_with("put ") || !remove_clause.starts_with("remove ")
    {
        return None;
    }

    if let Some(followup_choose) = remove_followup_mode {
        let followup_text = describe_effect(&Effect::new(followup_choose));
        let followup_clause = lowercase_first(followup_text.trim());
        let removed_counter =
            describe_put_counter_phrase(&remove_effect.count, remove_effect.counter_type);
        return Some(format!(
            "{put_clause} or {remove_clause}. When you remove {removed_counter} this way, {followup_clause}"
        ));
    }

    Some(format!("{put_clause} or {remove_clause}"))
}

fn describe_conditional_damage_instead(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if conditional.if_true.len() != 1 || conditional.if_false.len() != 1 {
        return None;
    }
    let true_damage = conditional.if_true[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    let false_damage =
        conditional.if_false[0].downcast_ref::<crate::effects::DealDamageEffect>()?;
    if true_damage.source_is_combat || false_damage.source_is_combat {
        return None;
    }
    if true_damage.target != false_damage.target {
        return None;
    }

    let base_amount = describe_value(&false_damage.amount);
    let instead_amount = describe_value(&true_damage.amount);
    let target = describe_choose_spec(&true_damage.target);
    let condition = describe_condition(&conditional.condition);
    Some(format!(
        "Deal {base_amount} damage to {target}. It deals {instead_amount} damage instead if {condition}"
    ))
}

fn describe_conditional_choose_both_instead(
    conditional: &crate::effects::ConditionalEffect,
) -> Option<String> {
    if conditional.if_true.len() != 1 || conditional.if_false.len() != 1 {
        return None;
    }
    let choose_true = conditional.if_true[0].downcast_ref::<crate::effects::ChooseModeEffect>()?;
    let choose_false =
        conditional.if_false[0].downcast_ref::<crate::effects::ChooseModeEffect>()?;

    if choose_true.modes.len() != choose_false.modes.len()
        || choose_true
            .modes
            .iter()
            .zip(choose_false.modes.iter())
            .any(|(left, right)| left.description.trim() != right.description.trim())
    {
        return None;
    }

    // Pattern: "Choose one. If <condition>, you may choose both instead."
    if choose_true.choose_count != Value::Fixed(2)
        || choose_true.min_choose_count.as_ref() != Some(&Value::Fixed(1))
        || choose_false.choose_count != Value::Fixed(1)
        || choose_false.min_choose_count.is_some()
    {
        return None;
    }

    let condition = describe_condition(&conditional.condition);
    let mut out = format!("Choose one. If {condition}, you may choose both instead.");
    for mode in &choose_true.modes {
        let description = ensure_trailing_period(mode.description.trim());
        if description.trim().is_empty() {
            continue;
        }
        out.push('\n');
        out.push_str("• ");
        out.push_str(description.trim());
    }
    Some(out)
}

fn describe_effect_impl(effect: &Effect) -> String {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        if let Some(compact) = describe_reveal_until_sequence(sequence) {
            return compact;
        }
        if let Some(compact) = describe_search_sequence(sequence) {
            return compact;
        }
        return describe_effect_list(&sequence.effects);
    }
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        if let Some(compact) = describe_for_each_double_counters(for_each) {
            return compact;
        }
        if for_each.effects.len() == 1
            && let Some(put) =
                for_each.effects[0].downcast_ref::<crate::effects::PutCountersEffect>()
            && matches!(put.target, ChooseSpec::Iterated)
            && put.target_count.is_none()
            && !put.distributed
        {
            let description = for_each.filter.description();
            let filter_text = strip_indefinite_article(&description);
            return format!(
                "Put {} on each {}",
                describe_put_counter_phrase(&put.count, put.counter_type),
                filter_text
            );
        }
        if let Some(subject) = describe_for_each_tagged_this_way_subject(&for_each.filter) {
            return format!("{subject}, {}", describe_effect_list(&for_each.effects));
        }
        let description = for_each.filter.description();
        let filter_text = strip_indefinite_article(&description);
        return format!(
            "For each {}, {}",
            filter_text,
            describe_effect_list(&for_each.effects)
        );
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        let tag = for_each_tagged.tag.as_str();
        let subject = if tag.starts_with("destroyed_") {
            "For each object destroyed this way".to_string()
        } else if tag.starts_with("exiled_") {
            "For each object exiled this way".to_string()
        } else if tag.starts_with("sacrificed_") {
            "For each object sacrificed this way".to_string()
        } else if tag.is_empty() {
            "For each tagged object".to_string()
        } else {
            format!("For each tagged '{tag}' object")
        };
        return format!(
            "{subject}, {}",
            describe_effect_list(&for_each_tagged.effects)
        );
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if let Some(compact) =
            describe_for_players_reveal_top_mana_value_life_then_put_into_hand(for_players)
        {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_types_then_sacrifice_rest(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_choose_then_sacrifice(for_players) {
            return compact;
        }
        if let Some(compact) = describe_for_players_damage_and_controlled_damage(for_players) {
            return compact;
        }
        if for_players.effects.len() == 1
            && let Some(conditional) =
                for_players.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
            && conditional.if_false.is_empty()
            && let Some(relative) = describe_player_relative_condition(&conditional.condition)
        {
            let player_filter_text = describe_player_filter(&for_players.filter);
            let each_player = strip_leading_article(&player_filter_text);
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
            let player_filter_text = describe_player_filter(&for_players.filter);
            let each_player = strip_leading_article(&player_filter_text);
            let mut inner = describe_effect_list(&may.effects);
            if let Some(rest) = inner.strip_prefix("that player ") {
                inner = rest.to_string();
            }
            if let Some(rest) = inner.strip_prefix("you ") {
                inner = rest.to_string();
            }
            inner = normalize_you_verb_phrase(&inner);
            return format!("For each {each_player}, that player may {inner}");
        }
        let player_filter_text = describe_player_filter(&for_players.filter);
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
            || (choose.zone == Zone::Library && choose.tag.as_str().starts_with("searched_"));
        let filter_text = choose.filter.description();
        let choice_text = if choose.top_only {
            if let Some(exact) = choose_exact_count(choose) {
                if exact > 1 {
                    let count_text = number_word(exact as i32)
                        .map(str::to_string)
                        .unwrap_or_else(|| exact.to_string());
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
            format!("{} {}", describe_choice_count(&choose.count), filter_text)
        };
        let (zone_phrase, zone_keyword) = match choose.zone {
            Zone::Battlefield => ("the battlefield", "battlefield"),
            Zone::Hand => ("a hand", "hand"),
            Zone::Graveyard => ("a graveyard", "graveyard"),
            Zone::Library => ("a library", "library"),
            Zone::Stack => ("the stack", "stack"),
            Zone::Exile => ("exile", "exile"),
            Zone::Command => ("the command zone", "command"),
        };
        let filter_lower = filter_text.to_ascii_lowercase();
        let includes_zone_already = filter_lower.contains(zone_keyword);
        let location_suffix = if includes_zone_already {
            String::new()
        } else {
            format!(" in {zone_phrase}")
        };
        return format!(
            "{} {} {}{} and tags it as '{}'",
            chooser,
            if search_like {
                "searches for"
            } else {
                choose_verb
            },
            choice_text,
            location_suffix,
            choose.tag.as_str()
        );
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        let target = describe_choose_spec(&move_to_zone.target);
        return match move_to_zone.zone {
            Zone::Exile => format!("Exile {target}"),
            Zone::Graveyard => format!("Put {target} into its owner's graveyard"),
            Zone::Hand => {
                if let ChooseSpec::Tagged(tag) = move_to_zone.target.base()
                    && (tag.as_str().starts_with("revealed_")
                        || tag.as_str().starts_with("searched_")
                        || tag.as_str().starts_with("milled_")
                        || tag.as_str().starts_with("discarded_"))
                {
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
                    let cards = describe_card_choice_count(move_to_zone.target.count());
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
                    if move_to_zone.to_top {
                        return format!("Put {cards} from {from_zone} on top of {library}");
                    }
                    return format!("Put {cards} from {from_zone} on the bottom of {library}");
                }
                if let Some(owner) = graveyard_owner_from_spec(&move_to_zone.target) {
                    let cards = describe_choose_spec_without_graveyard_zone(&move_to_zone.target);
                    let from_zone = match &owner {
                        Some(owner) => {
                            format!("{} graveyard", describe_possessive_player_filter(owner))
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
                        return format!("Put {cards} from {from_zone} on top of {library}");
                    }
                    return format!("Put {cards} from {from_zone} on the bottom of {library}");
                }
                if move_to_zone.to_top {
                    format!(
                        "Put {target} on top of {}",
                        owner_library_phrase_for_spec(&move_to_zone.target)
                    )
                } else {
                    format!(
                        "Put {target} on the bottom of {}",
                        owner_library_phrase_for_spec(&move_to_zone.target)
                    )
                }
            }
            Zone::Battlefield => {
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
                let controller_suffix = match move_to_zone.battlefield_controller {
                    crate::effects::BattlefieldController::Preserve => "",
                    crate::effects::BattlefieldController::Owner => owner_control_suffix,
                    crate::effects::BattlefieldController::You => " under your control",
                };
                if let crate::target::ChooseSpec::Tagged(tag) = &move_to_zone.target
                    && tag.as_str().starts_with("exiled_")
                {
                    format!("Return {target} to the battlefield{tapped_suffix}{controller_suffix}")
                } else {
                    format!("Put {target} onto the battlefield{tapped_suffix}{controller_suffix}")
                }
            }
            Zone::Stack => format!("Put {target} on the stack"),
            Zone::Command => format!("Move {target} to the command zone"),
        };
    }
    if let Some(move_to_second) =
        effect.downcast_ref::<crate::effects::MoveToLibrarySecondFromTopEffect>()
    {
        let target = describe_choose_spec(&move_to_second.target);
        return format!(
            "Put {target} into {} second from the top",
            owner_library_phrase_for_spec(&move_to_second.target)
        );
    }
    if let Some(put_onto_battlefield) =
        effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        let target = describe_choose_spec(&put_onto_battlefield.target);
        let mut text = format!("Put {target} onto the battlefield");
        if put_onto_battlefield.tapped {
            text.push_str(" tapped");
        }
        return text;
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        let face_down_suffix = if exile.face_down { " face down" } else { "" };
        return format!(
            "Exile {}{face_down_suffix}",
            describe_choose_spec(&exile.spec)
        );
    }
    if let Some(exile_until) = effect.downcast_ref::<crate::effects::ExileUntilEffect>() {
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
        return format!(
            "Exile {}{face_down_suffix} {duration}",
            describe_choose_spec(&exile_until.spec)
        );
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
        let base = format!("Destroy {}", describe_choose_spec(&destroy.spec));
        let tail = if choose_spec_allows_multiple(&destroy.spec) {
            "They can't be regenerated"
        } else {
            "It can't be regenerated"
        };
        return format!("{base}. {tail}");
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        if let ChooseSpec::All(filter) = &destroy.spec
            && filter.card_types.as_slice() == [crate::types::CardType::Creature]
            && filter.all_colors == Some(false)
        {
            return "Destroy each creature that isn't all colors".to_string();
        }
        return format!("Destroy {}", describe_choose_spec(&destroy.spec));
    }
    if let Some(deal_damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        if let Value::PowerOf(source) | Value::ToughnessOf(source) = &deal_damage.amount {
            let mut subject = describe_choose_spec(source);
            if subject == "this source" {
                subject = "this creature".to_string();
            } else if subject == "it" {
                subject = "that creature".to_string();
            }
            let mut target = describe_choose_spec(&deal_damage.target);
            if target == "this source" {
                target = "this creature".to_string();
            } else if target == "it" {
                target = "that creature".to_string();
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
                describe_choose_spec(&deal_damage.target)
            );
        }
        return format!(
            "Deal {} damage to {}",
            describe_value(&deal_damage.amount),
            describe_choose_spec(&deal_damage.target)
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
        return format!("Counter {}", describe_choose_spec(&counter_spell.target));
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        let payer = describe_player_filter(&unless_pays.player);
        let pay_verb = player_verb(&payer, "pay", "pays");
        let mana_text = unless_pays
            .mana
            .iter()
            .copied()
            .map(describe_mana_symbol)
            .collect::<Vec<_>>()
            .join("");
        let has_base_mana = !mana_text.is_empty();
        let additional_subject = |filter: &ObjectFilter| {
            let mut subject = filter.description();
            if filter.zone == Some(Zone::Graveyard)
                && filter.owner.is_none()
                && subject.ends_with(" in graveyard")
            {
                subject = subject.replacen(" in graveyard", " in each graveyard", 1);
            }
            subject
                .strip_prefix("a ")
                .or_else(|| subject.strip_prefix("an "))
                .unwrap_or(subject.as_str())
                .to_string()
        };
        let additional_text = match unless_pays.additional_generic.as_ref() {
            Some(Value::Count(filter)) => {
                let subject = additional_subject(filter);
                if has_base_mana {
                    format!("plus an additional {{1}} for each {subject}")
                } else {
                    format!("{{1}} for each {subject}")
                }
            }
            Some(Value::CountScaled(filter, multiplier)) if *multiplier > 0 => {
                let subject = additional_subject(filter);
                if has_base_mana {
                    format!("plus an additional {{{multiplier}}} for each {subject}")
                } else {
                    format!("{{{multiplier}}} for each {subject}")
                }
            }
            Some(Value::PartySize(PlayerFilter::You)) => {
                if has_base_mana {
                    "plus an additional {1} for each creature in your party".to_string()
                } else {
                    "{1} for each creature in your party".to_string()
                }
            }
            Some(Value::BasicLandTypesAmong(filter)) => {
                let lands = describe_basic_land_type_scope(filter);
                if has_base_mana {
                    format!("plus {{1}} for each basic land type among {lands}")
                } else {
                    format!("{{1}} for each basic land type among {lands}")
                }
            }
            Some(Value::ColorsAmong(filter)) => {
                let scope = describe_for_each_filter(filter);
                if has_base_mana {
                    format!("plus {{1}} for each color among {scope}")
                } else {
                    format!("{{1}} for each color among {scope}")
                }
            }
            Some(value) => format!("plus {}", describe_value(value)),
            None => String::new(),
        };
        let mut payment_text = mana_text.clone();
        if !additional_text.is_empty() {
            if payment_text.is_empty() {
                payment_text = additional_text.clone();
            } else {
                payment_text.push(' ');
                payment_text.push_str(&additional_text);
            }
        }
        if let Some(life) = &unless_pays.life {
            if payment_text.is_empty() {
                payment_text = format!("{} life", describe_value(life));
            } else {
                payment_text = format!("{payment_text} and {} life", describe_value(life));
            }
        }
        if unless_pays.effects.len() == 1
            && let Some(counter) =
                unless_pays.effects[0].downcast_ref::<crate::effects::CounterEffect>()
        {
            return format!(
                "Counter {} unless {} {} {}",
                describe_choose_spec(&counter.target),
                payer,
                pay_verb,
                payment_text
            );
        }

        let inner_text = describe_effect_list(&unless_pays.effects);
        return format!(
            "{} unless {} {} {}",
            inner_text, payer, pay_verb, payment_text
        );
    }
    if let Some(unless_action) = effect.downcast_ref::<crate::effects::UnlessActionEffect>() {
        let inner_text = describe_effect_list(&unless_action.effects);
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
        let alt_text = describe_effect_list(&unless_action.alternative);
        let player = describe_player_filter(&unless_action.player);
        let unless_clause = if alt_text == player || alt_text.starts_with(&format!("{player} ")) {
            alt_text
        } else {
            format!("{player} {alt_text}")
        };
        return format!("{} unless {}", inner_text, unless_clause);
    }
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        if put_counters.distributed {
            return format!(
                "Distribute {} among {}",
                describe_put_counter_phrase(&put_counters.count, put_counters.counter_type),
                describe_choose_spec(&put_counters.target)
            );
        }
        let mut target = describe_choose_spec(&put_counters.target);
        if let ChooseSpec::WithCount(inner, count) = &put_counters.target
            && matches!(inner.as_ref(), ChooseSpec::Target(_))
            && !count.is_single()
            && !target.starts_with("each of ")
        {
            target = format!("each of {target}");
        }
        if let Value::Count(filter) = &put_counters.count
            && matches!(
                put_counters.counter_type,
                crate::object::CounterType::PlusOnePlusOne
                    | crate::object::CounterType::MinusOneMinusOne
            )
        {
            return format!(
                "Put a {} counter on {target} for each {}",
                describe_counter_type(put_counters.counter_type),
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(
            put_counters.counter_type,
            crate::object::CounterType::PlusOnePlusOne
                | crate::object::CounterType::MinusOneMinusOne
        ) && let Value::Add(left, right) = &put_counters.count
            && left == right
        {
            let per_text = match left.as_ref() {
                Value::Count(filter) => Some(describe_for_each_count_filter(filter)),
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
        return format!(
            "Put {} on {}",
            describe_put_counter_phrase(&put_counters.count, put_counters.counter_type),
            target
        );
    }
    if let Some(remove_counters) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
        return format!(
            "Remove {} from {}",
            describe_put_counter_phrase(&remove_counters.count, remove_counters.counter_type),
            describe_choose_spec(&remove_counters.target)
        );
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
    if let Some(move_counters) = effect.downcast_ref::<crate::effects::MoveAllCountersEffect>() {
        return format!(
            "Move all counters from {} to {}",
            describe_choose_spec(&move_counters.from),
            describe_choose_spec(&move_counters.to)
        );
    }
    if let Some(proliferate) = effect.downcast_ref::<crate::effects::ProliferateEffect>() {
        let _ = proliferate;
        return "Proliferate".to_string();
    }
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
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
            return format!(
                "Return {} {} to the battlefield{}",
                target_text,
                from_text,
                if return_to_battlefield.tapped {
                    " tapped"
                } else {
                    ""
                }
            );
        }
        return format!(
            "Return {} from graveyard to the battlefield{}",
            describe_choose_spec(&return_to_battlefield.target),
            if return_to_battlefield.tapped {
                " tapped"
            } else {
                ""
            }
        );
    }
    if let Some(return_all_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnAllToBattlefieldEffect>()
    {
        return format!(
            "Return all {} to the battlefield{}",
            describe_for_each_filter(&return_all_to_battlefield.filter),
            if return_all_to_battlefield.tapped {
                " tapped"
            } else {
                ""
            }
        );
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
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
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        let player = describe_choose_spec(&gain.player);
        if let Value::CountersOnSource(counter_type) = &gain.amount {
            return format!(
                "{} {} 1 life for each {} counter on this permanent",
                player,
                player_verb(&player, "gain", "gains"),
                describe_counter_type(*counter_type)
            );
        }
        if let Value::Add(left, right) = &gain.amount
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
        if let Value::Count(filter) = &gain.amount {
            return format!(
                "{} {} 1 life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_for_each_count_filter(filter)
            );
        }
        if let Value::CountScaled(filter, multiplier) = &gain.amount {
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "gain", "gains"),
                multiplier,
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(gain.amount, Value::CreaturesDiedThisTurn) {
            return format!(
                "{} {} 1 life for each creature that died this turn",
                player,
                player_verb(&player, "gain", "gains")
            );
        }
        if matches!(
            gain.amount,
            Value::SourcePower
                | Value::SourceToughness
                | Value::PowerOf(_)
                | Value::ToughnessOf(_)
                | Value::ManaValueOf(_)
        ) {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "gain", "gains"),
                describe_value(&gain.amount)
            );
        }
        return format!(
            "{} {} {} life",
            player,
            player_verb(&player, "gain", "gains"),
            describe_value(&gain.amount)
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
    if let Some(lose) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        let player = describe_choose_spec(&lose.player);
        if let Value::CountersOnSource(counter_type) = &lose.amount {
            return format!(
                "{} {} 1 life for each {} counter on this permanent",
                player,
                player_verb(&player, "lose", "loses"),
                describe_counter_type(*counter_type)
            );
        }
        if let Value::Add(left, right) = &lose.amount
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
        if let Value::Count(filter) = &lose.amount {
            return format!(
                "{} {} 1 life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_for_each_count_filter(filter)
            );
        }
        if let Value::CountScaled(filter, multiplier) = &lose.amount {
            return format!(
                "{} {} {} life for each {}",
                player,
                player_verb(&player, "lose", "loses"),
                multiplier,
                describe_for_each_count_filter(filter)
            );
        }
        if matches!(lose.amount, Value::CreaturesDiedThisTurn) {
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
        ) {
            return format!(
                "{} {} life equal to {}",
                player,
                player_verb(&player, "lose", "loses"),
                describe_value(&lose.amount)
            );
        }
        return format!(
            "{} {} {} life",
            player,
            player_verb(&player, "lose", "loses"),
            describe_value(&lose.amount)
        );
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        let player = describe_player_filter(&discard.player);
        let random_suffix = if discard.random { " at random" } else { "" };
        return format!(
            "{} {} {}{}",
            player,
            player_verb(&player, "discard", "discards"),
            describe_discard_count(&discard.count, discard.card_filter.as_ref()),
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
            "Add {} to {}",
            if mana.is_empty() { "{0}" } else { &mana },
            describe_mana_pool_owner(&add_mana.player)
        );
    }
    if let Some(add_colorless) = effect.downcast_ref::<crate::effects::AddColorlessManaEffect>() {
        return format!(
            "Add {} colorless mana to {}",
            describe_value(&add_colorless.amount),
            describe_mana_pool_owner(&add_colorless.player)
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
        if let Value::Count(filter) = &add_scaled.amount {
            return format!(
                "Add {} for each {} to {}",
                mana_text,
                filter.description(),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::CountersOnSource(counter_type) = &add_scaled.amount {
            return format!(
                "Add {} for each {} counter on this source to {}",
                mana_text,
                describe_counter_type(*counter_type),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::CountersOn(spec, Some(counter_type)) = &add_scaled.amount {
            return format!(
                "Add {} for each {} counter on {} to {}",
                mana_text,
                describe_counter_type(*counter_type),
                describe_choose_spec(spec),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::CountersOn(spec, None) = &add_scaled.amount {
            return format!(
                "Add {} for each counter on {} to {}",
                mana_text,
                describe_choose_spec(spec),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Some((party_filter, multiplier)) = party_size_multiplier(&add_scaled.amount) {
            let party_owner = describe_possessive_player_filter(&party_filter);
            if multiplier <= 1 {
                return format!(
                    "Add {} for each creature in {} party to {}",
                    mana_text,
                    party_owner,
                    describe_mana_pool_owner(&add_scaled.player)
                );
            }
            return format!(
                "Add {} {} times for each creature in {} party to {}",
                mana_text,
                multiplier,
                party_owner,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::Devotion { player, color } = &add_scaled.amount {
            let color_name = format!("{color:?}").to_ascii_lowercase();
            return format!(
                "Add an amount of {} equal to {} devotion to {}",
                mana_text,
                describe_possessive_player_filter(player),
                color_name
            );
        }
        if let Value::PowerOf(spec) = &add_scaled.amount {
            return format!(
                "Add an amount of {} equal to the power of {} to {}",
                mana_text,
                describe_choose_spec(spec),
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::ManaValueOf(spec) = &add_scaled.amount {
            let amount_text = {
                let described = describe_choose_spec(spec);
                if described == "it" {
                    "its mana value".to_string()
                } else {
                    format!("the mana value of {described}")
                }
            };
            return format!(
                "Add an amount of {} equal to {} to {}",
                mana_text,
                amount_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EffectValue(_) = &add_scaled.amount {
            return format!(
                "Add that much {} to {}",
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) = &add_scaled.amount
        {
            return format!(
                "Add that much {} to {}",
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EffectValueOffset(_, offset) = &add_scaled.amount {
            let amount_text = if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            };
            return format!(
                "Add {} {} to {}",
                amount_text,
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        if let Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) = &add_scaled.amount
        {
            let amount_text = if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            };
            return format!(
                "Add {} {} to {}",
                amount_text,
                mana_text,
                describe_mana_pool_owner(&add_scaled.player)
            );
        }
        return format!(
            "Add {} {} time(s) to {}",
            mana_text,
            describe_value(&add_scaled.amount),
            describe_mana_pool_owner(&add_scaled.player)
        );
    }
    if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        let player = describe_player_filter(&mill.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "mill", "mills"),
            describe_card_count(&mill.count)
        );
    }
    if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
        return format!("Tap {}", describe_choose_spec(&tap.spec));
    }
    if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        return format!("Untap {}", describe_choose_spec(&untap.spec));
    }
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachToEffect>() {
        return format!(
            "Attach this source to {}",
            describe_choose_spec(&attach.target)
        );
    }
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
        return format!(
            "Attach {} to {}",
            describe_attach_objects_spec(&attach.objects),
            describe_choose_spec(&attach.target)
        );
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        let player = describe_player_filter(&sacrifice.player);
        let verb = player_verb(&player, "sacrifice", "sacrifices");
        if let Value::Count(count_filter) = &sacrifice.count
            && count_filter == &sacrifice.filter
        {
            let mut noun = sacrifice.filter.description();
            if let Some(rest) = noun.strip_prefix("target player's ") {
                noun = rest.to_string();
            } else if let Some(rest) = noun.strip_prefix("that player's ") {
                noun = rest.to_string();
            } else if let Some(rest) = noun.strip_prefix("the active player's ") {
                noun = rest.to_string();
            }
            if let Some(rest) = noun.strip_prefix("a ") {
                noun = rest.to_string();
            } else if let Some(rest) = noun.strip_prefix("an ") {
                noun = rest.to_string();
            }
            return format!("{player} {verb} all {}", pluralize_noun_phrase(&noun));
        }
        if matches!(sacrifice.count, Value::Fixed(1)) {
            if let Some(rest) = sacrifice
                .filter
                .description()
                .strip_prefix("target player's ")
            {
                return format!("{player} {verb} {}", with_indefinite_article(rest));
            }
            if let Some(rest) = sacrifice
                .filter
                .description()
                .strip_prefix("that player's ")
            {
                return format!("{player} {verb} {}", with_indefinite_article(rest));
            }
            if let Some(rest) = sacrifice
                .filter
                .description()
                .strip_prefix("the active player's ")
            {
                return format!("{player} {verb} {}", with_indefinite_article(rest));
            }
        }
        return format!(
            "{} {} {} {}",
            player,
            verb,
            describe_object_count(&sacrifice.count),
            sacrifice.filter.description()
        );
    }
    if let Some(sacrifice_target) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
        return format!(
            "Sacrifice {}",
            describe_choose_spec(&sacrifice_target.target)
        );
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        if let Some(owner) = graveyard_owner_from_spec(&return_to_hand.spec) {
            let target_text = describe_choose_spec_without_graveyard_zone(&return_to_hand.spec);
            let from_text = match &owner {
                Some(owner) => format!("{} graveyard", describe_possessive_player_filter(owner)),
                None => "a graveyard".to_string(),
            };
            let to_text = match &owner {
                Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
                None => owner_hand_phrase_for_spec(&return_to_hand.spec).to_string(),
            };
            return format!("Return {target_text} from {from_text} to {to_text}");
        }
        if is_you_owned_battlefield_object_spec(&return_to_hand.spec) {
            return format!(
                "Return {} to your hand",
                describe_choose_spec(&return_to_hand.spec)
            );
        }
        return format!(
            "Return {} to {}",
            describe_choose_spec(&return_to_hand.spec),
            owner_hand_phrase_for_spec(&return_to_hand.spec)
        );
    }
    if let Some(return_from_gy) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        let random_suffix = if return_from_gy.random {
            " at random"
        } else {
            ""
        };
        if let Some(owner) = graveyard_owner_from_spec(&return_from_gy.target) {
            let target_text = describe_choose_spec_without_graveyard_zone(&return_from_gy.target);
            let from_text = match &owner {
                Some(owner) => format!("{} graveyard", describe_possessive_player_filter(owner)),
                None => "a graveyard".to_string(),
            };
            let to_text = match &owner {
                Some(owner) => format!("{} hand", describe_possessive_player_filter(owner)),
                None => owner_hand_phrase_for_spec(&return_from_gy.target).to_string(),
            };
            return format!("Return {target_text}{random_suffix} from {from_text} to {to_text}");
        }
        return format!(
            "Return {}{} from a graveyard to {}",
            describe_choose_spec_without_graveyard_zone(&return_from_gy.target),
            random_suffix,
            owner_hand_phrase_for_spec(&return_from_gy.target)
        );
    }
    if let Some(shuffle_library) = effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>() {
        return format!(
            "Shuffle {} library",
            describe_possessive_player_filter(&shuffle_library.player)
        );
    }
    if let Some(shuffle_gy) =
        effect.downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()
    {
        let subject = describe_player_filter(&shuffle_gy.player);
        let verb = player_verb(&subject, "shuffle", "shuffles");
        // Use possessive pronoun ("your"/"their") instead of repeating the subject
        let possessive = if subject == "you" { "your" } else { "their" };
        return format!(
            "{} {} {} graveyard into {} library",
            subject, verb, possessive, possessive
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
        let destination = match search_library.destination {
            Zone::Hand => "into hand",
            Zone::Battlefield => "onto the battlefield",
            Zone::Library => "on top of library",
            Zone::Graveyard => "into their graveyard",
            Zone::Exile => "into exile",
            Zone::Stack => "onto the stack",
            Zone::Command => "into the command zone",
        };
        let filter_desc = if is_generic_owned_card_search_filter(&search_library.filter) {
            "a card".to_string()
        } else {
            describe_search_selection_with_cards(&search_library.filter.description())
        };
        if search_library.reveal && search_library.destination != Zone::Battlefield {
            return format!(
                "Search {} library for {}, reveal it, put it {}, then shuffle",
                describe_possessive_player_filter(&search_library.player),
                filter_desc,
                destination
            );
        }
        return format!(
            "Search {} library for {}, put it {}, then shuffle",
            describe_possessive_player_filter(&search_library.player),
            filter_desc,
            destination
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
        let owner = describe_possessive_player_filter(&look_at_top.player);
        let (count_text, noun, _) = describe_look_count_and_noun(&look_at_top.count);
        return format!("Look at the top {count_text} {noun} of {owner} library");
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
    if let Some(apply_continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if let Some(text) = describe_apply_continuous_effect(apply_continuous) {
            return text;
        }
    }
    if let Some(grant_all) = effect.downcast_ref::<crate::effects::GrantAbilitiesAllEffect>() {
        return format!(
            "{} gains {} {}",
            grant_all.filter.description(),
            grant_all
                .abilities
                .iter()
                .map(|ability| ability.display())
                .collect::<Vec<_>>()
                .join(", "),
            describe_until(&grant_all.duration)
        );
    }
    if let Some(grant_target) = effect.downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
    {
        return format!(
            "{} gains {} {}",
            describe_choose_spec(&grant_target.target),
            grant_target
                .abilities
                .iter()
                .map(|ability| ability.display())
                .collect::<Vec<_>>()
                .join(", "),
            describe_until(&grant_target.duration)
        );
    }
    if let Some(grant_object) = effect.downcast_ref::<crate::effects::GrantObjectAbilityEffect>() {
        return format!(
            "Grant {} to {}",
            describe_inline_ability(&grant_object.ability),
            describe_choose_spec(&grant_object.target)
        );
    }
    if let Some(modify_pt) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        let power_text = describe_value(&modify_pt.power);
        let toughness_text = describe_value(&modify_pt.toughness);
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
        let each_text = match &modify_pt_each.count {
            Value::Count(filter) => describe_for_each_count_filter(filter),
            Value::BasicLandTypesAmong(filter) => describe_basic_land_types_among(filter),
            Value::ColorsAmong(filter) => describe_colors_among(filter),
            _ => describe_value(&modify_pt_each.count),
        };
        return format!(
            "{} {} {}/{} for each {} {}",
            target_text,
            gets_verb,
            describe_signed_i32(modify_pt_each.power_per),
            describe_signed_i32(modify_pt_each.toughness_per),
            each_text,
            describe_until(&modify_pt_each.duration)
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
    if let Some(transform) = effect.downcast_ref::<crate::effects::TransformEffect>() {
        return format!("Transform {}", describe_transform_target(&transform.target));
    }
    if let Some(flip) = effect.downcast_ref::<crate::effects::FlipEffect>() {
        return format!("Flip {}", describe_flip_target(&flip.target));
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if is_implicit_reference_tag(tagged.tag.as_str()) {
            return describe_effect(&tagged.effect);
        }
        return format!(
            "Tag '{}' then {}",
            tagged.tag.as_str(),
            describe_effect(&tagged.effect)
        );
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
        return format!(
            "Tag the object attached to this source as '{}'",
            tag_attached.tag.as_str()
        );
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return describe_effect(&with_id.effect);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        if let Some(compact) = describe_conditional_damage_instead(conditional) {
            return compact;
        }
        if let Some(compact) = describe_conditional_choose_both_instead(conditional) {
            return compact;
        }
        let true_branch = describe_effect_list(&conditional.if_true);
        let false_branch = describe_effect_list(&conditional.if_false);
        if false_branch.is_empty() {
            return format!(
                "If {}, {}",
                describe_condition(&conditional.condition),
                true_branch
            );
        }
        return format!(
            "If {}, {}. Otherwise, {}",
            describe_condition(&conditional.condition),
            true_branch,
            false_branch
        );
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        let then_text = describe_effect_list(&if_effect.then);
        let else_text = describe_effect_list(&if_effect.else_);
        if else_text.is_empty() {
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
    if let Some(cast_tagged) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
        let verb = if cast_tagged.allow_land {
            "play"
        } else {
            "cast"
        };
        let spec = crate::target::ChooseSpec::Tagged(cast_tagged.tag.clone());
        let target = if cast_tagged.as_copy {
            let tag = cast_tagged.tag.as_str();
            let tag_is_numbered = tag.rsplit_once('_').is_some_and(|(_, suffix)| {
                !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit())
            });
            if tag == "it" || tag_is_numbered {
                "the copy".to_string()
            } else {
                format!("a copy of {}", describe_choose_spec(&spec))
            }
        } else {
            describe_choose_spec(&spec)
        };
        let mut text = format!("{verb} {target}");
        if cast_tagged.without_paying_mana_cost {
            text.push_str(" without paying its mana cost");
        }
        return text;
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        if let Some(decider) = may.decider.as_ref() {
            let who = describe_player_filter(decider);
            let mut inner = describe_effect_list(&may.effects);
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
                inner = normalize_you_verb_phrase(&inner);
            }
            inner = lowercase_may_clause(&inner);
            return format!("{who} may {inner}");
        }

        if may.effects.len() == 1
            && let Some(cast_tagged) =
                may.effects[0].downcast_ref::<crate::effects::CastTaggedEffect>()
            && cast_tagged.as_copy
        {
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
        inner = normalize_you_verb_phrase(&inner);
        inner = lowercase_may_clause(&inner);
        return format!("You may {inner}");
    }
    if let Some(target_only) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
        return format!("Choose {}", describe_choose_spec(&target_only.target));
    }
    if let Some(compact) = describe_compact_protection_choice(effect) {
        return compact;
    }
    if let Some(compact) = describe_compact_keyword_choice(effect) {
        return compact;
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        if let Some(compact) = describe_tap_or_untap_mode(choose_mode) {
            return compact;
        }
        if let Some(compact) = describe_put_or_remove_counter_mode(choose_mode) {
            return compact;
        }
        let mut header = describe_mode_choice_header(
            &choose_mode.choose_count,
            choose_mode.min_choose_count.as_ref(),
        );
        if choose_mode.disallow_previously_chosen_modes {
            header = if choose_mode.disallow_previously_chosen_modes_this_turn {
                "Choose one that hasn't been chosen this turn —".to_string()
            } else {
                "Choose one that hasn't been chosen —".to_string()
            };
        }
        let modes = choose_mode
            .modes
            .iter()
            .map(|mode| {
                let description_raw = mode.description.trim();
                let description = ensure_trailing_period(description_raw);
                let mode_effects = describe_effect_list(&mode.effects);
                if !description.trim().is_empty() {
                    let mode_effects_trimmed = mode_effects.trim();
                    if mode_effects_trimmed.is_empty() {
                        return description;
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
                        return description;
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
                        description
                    } else {
                        format!(
                            "{} {}",
                            description_head,
                            ensure_trailing_period(followup.trim())
                        )
                    }
                } else {
                    ensure_trailing_period(mode_effects.trim())
                }
            })
            .collect::<Vec<_>>()
            .join(" • ");
        if choose_mode.disallow_previously_chosen_modes {
            return format!("{header}\n• {}", modes.replace(" • ", "\n• "));
        }
        if choose_mode.allow_repeated_modes {
            let normalized_header = header
                .trim_end_matches('-')
                .trim_end_matches('—')
                .trim()
                .trim_end_matches('.')
                .to_string();
            return format!(
                "{normalized_header}. You may choose the same mode more than once. • {modes}"
            );
        }
        return format!("{header} {modes}");
    }
    if let Some(create_token) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        if let Some(compact) = describe_compact_create_token(create_token) {
            return compact;
        }
        let token_pronoun = if matches!(create_token.count, Value::Fixed(1)) {
            "it"
        } else {
            "them"
        };
        let append_token_cleanup_sentences = |mut text: String| {
            if create_token.exile_at_end_of_combat {
                text.push_str(&format!(". Exile {token_pronoun} at end of combat"));
            }
            if create_token.sacrifice_at_end_of_combat {
                text.push_str(&format!(". Sacrifice {token_pronoun} at end of combat"));
            }
            if create_token.sacrifice_at_next_end_step {
                text.push_str(&format!(
                    ". Sacrifice {token_pronoun} at the beginning of the next end step"
                ));
            }
            if create_token.exile_at_next_end_step {
                text.push_str(&format!(
                    ". Exile {token_pronoun} at the beginning of the next end step"
                ));
            }
            text
        };
        let append_token_entry_flags = |mut text: String| {
            if create_token.enters_tapped && create_token.enters_attacking {
                text.push_str(" that's tapped and attacking");
                return text;
            }
            if create_token.enters_tapped {
                text.push_str(", tapped");
            }
            if create_token.enters_attacking {
                text.push_str(", attacking");
            }
            text
        };
        if value_is_iterated_object_count(&create_token.count) {
            let token_blueprint = describe_token_blueprint(&create_token.token);
            let mut text = if matches!(create_token.controller, PlayerFilter::You) {
                format!("Create 1 {token_blueprint}")
            } else {
                format!(
                    "Create 1 {} under {} control",
                    token_blueprint,
                    describe_possessive_player_filter(&create_token.controller)
                )
            };
            text = append_token_entry_flags(text);
            return append_token_cleanup_sentences(text);
        }
        if let Some(for_each_count) = describe_create_for_each_count(&create_token.count) {
            let token_blueprint = describe_token_blueprint(&create_token.token);
            let mut text = if matches!(create_token.controller, PlayerFilter::You) {
                format!("Create 1 {} for each {}", token_blueprint, for_each_count)
            } else {
                format!(
                    "Create 1 {} under {} control for each {}",
                    token_blueprint,
                    describe_possessive_player_filter(&create_token.controller),
                    for_each_count
                )
            };
            text = append_token_entry_flags(text);
            return append_token_cleanup_sentences(text);
        }
        let token_blueprint = describe_token_blueprint(&create_token.token);
        let count_text = describe_effect_count_backref(&create_token.count)
            .unwrap_or_else(|| describe_value(&create_token.count));
        let mut text = if matches!(create_token.controller, PlayerFilter::You) {
            format!("Create {} {}", count_text, token_blueprint)
        } else {
            format!(
                "Create {} {} under {} control",
                count_text,
                token_blueprint,
                describe_possessive_player_filter(&create_token.controller)
            )
        };
        text = append_token_entry_flags(text);
        return append_token_cleanup_sentences(text);
    }
    if let Some(create_copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        let target = match &create_copy.target {
            ChooseSpec::Tagged(tag) if tag.as_str().starts_with("exile_cost_") => {
                "the exiled card".to_string()
            }
            _ => describe_choose_spec(&create_copy.target),
        };
        let mut text = match create_copy.count {
            Value::Fixed(1) => format!("Create a token that's a copy of {target}"),
            Value::Fixed(n) => format!("Create {n} tokens that are copies of {target}"),
            _ => format!(
                "Create {} tokens that are copies of {target}",
                describe_value(&create_copy.count)
            ),
        };
        if !matches!(create_copy.controller, PlayerFilter::You) {
            text.push_str(&format!(
                " under {} control",
                describe_possessive_player_filter(&create_copy.controller)
            ));
        }
        if create_copy.enters_tapped {
            text.push_str(", tapped");
        }
        if create_copy.has_haste {
            text.push_str(", with haste");
        }
        if create_copy.enters_attacking {
            if let Some(crate::effects::CopyAttackTargetMode::PlayerOrPlaneswalkerControlledBy(
                player_filter,
            )) = &create_copy.attack_target_mode
            {
                let player = describe_player_filter(player_filter);
                text.push_str(&format!(
                    ", attacking {player} or a planeswalker they control"
                ));
            } else {
                text.push_str(", attacking");
            }
        }
        if create_copy.exile_at_end_of_combat {
            text.push_str(", and exile at end of combat");
        }
        if create_copy.sacrifice_at_next_end_step {
            text.push_str(", and sacrifice it at the beginning of the next end step");
        }
        if create_copy.exile_at_next_end_step {
            text.push_str(", and exile it at the beginning of the next end step");
        }
        if create_copy.pt_adjustment.is_some() {
            if matches!(create_copy.count, Value::Fixed(1)) {
                text.push_str(
                    ", except its power and toughness are each half that permanent's power and toughness, rounded up",
                );
            } else {
                text.push_str(
                    ", except their power and toughness are each half that permanent's power and toughness, rounded up",
                );
            }
        }
        if let Some((power, toughness)) = create_copy.set_base_power_toughness {
            text.push_str(&format!(
                ", with base power and toughness {power}/{toughness}"
            ));
        }
        if create_copy.set_colors.is_some()
            || create_copy.set_card_types.is_some()
            || create_copy.set_subtypes.is_some()
        {
            let mut words = Vec::new();
            if let Some(colors) = create_copy.set_colors {
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
            }
            if let Some(subtypes) = &create_copy.set_subtypes {
                words.extend(
                    subtypes
                        .iter()
                        .map(|subtype| format!("{subtype:?}").to_ascii_lowercase()),
                );
            }
            if let Some(card_types) = &create_copy.set_card_types {
                words.extend(
                    card_types
                        .iter()
                        .map(|card_type| format!("{card_type:?}").to_ascii_lowercase()),
                );
            }
            if !words.is_empty() {
                text.push_str(", and it's ");
                text.push_str(&words.join(" "));
            }
        }
        if !create_copy.added_card_types.is_empty() || !create_copy.added_subtypes.is_empty() {
            let mut type_words: Vec<String> = create_copy
                .added_card_types
                .iter()
                .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
                .collect();
            type_words.extend(
                create_copy
                    .added_subtypes
                    .iter()
                    .map(|subtype| format!("{subtype:?}").to_ascii_lowercase()),
            );
            if !type_words.is_empty() {
                text.push_str(", and it's ");
                text.push_str(&type_words.join(" "));
                text.push_str(" in addition to its other types");
            }
        }
        if create_copy
            .removed_supertypes
            .iter()
            .any(|supertype| *supertype == Supertype::Legendary)
        {
            text.push_str(", and it isn't legendary");
        }
        if !create_copy.granted_static_abilities.is_empty() {
            let mut granted = Vec::new();
            for ability in &create_copy.granted_static_abilities {
                let normalized = normalize_token_granted_static_ability_text(&ability.display());
                let quoted = quote_token_granted_ability_text(&normalized);
                if !granted.contains(&quoted) {
                    granted.push(quoted);
                }
            }
            text.push_str(", and it has ");
            text.push_str(&join_with_and(&granted));
        }
        return text;
    }
    if let Some(earthbend) = effect.downcast_ref::<crate::effects::EarthbendEffect>() {
        return format!(
            "Earthbend {} with {} +1/+1 counter(s)",
            describe_choose_spec(&earthbend.target),
            earthbend.counters
        );
    }
    if let Some(explore) = effect.downcast_ref::<crate::effects::ExploreEffect>() {
        if matches!(explore.target.base(), ChooseSpec::Source) {
            return "it explores".to_string();
        }
        let subject = capitalize_first(&describe_choose_spec(&explore.target));
        return format!("{subject} explores");
    }
    if let Some(behold) = effect.downcast_ref::<crate::effects::BeholdEffect>() {
        let subtype_name = format!("{:?}", behold.subtype);
        if behold.count == 1 {
            return format!("Behold {}", with_indefinite_article(&subtype_name));
        }
        let count_text = small_number_word(behold.count)
            .map(str::to_string)
            .unwrap_or_else(|| behold.count.to_string());
        return format!("Behold {count_text} {subtype_name}s");
    }
    if effect
        .downcast_ref::<crate::effects::OpenAttractionEffect>()
        .is_some()
    {
        return "Open an Attraction".to_string();
    }
    if effect
        .downcast_ref::<crate::effects::ManifestDreadEffect>()
        .is_some()
    {
        return "Manifest dread".to_string();
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
        if regenerate.duration == Until::EndOfTurn {
            return format!("Regenerate {target}");
        }
        return format!(
            "Regenerate {target} {}",
            describe_until(&regenerate.duration)
        );
    }
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
        if cant.duration == Until::EndOfTurn {
            return format!("{} this turn", describe_restriction(&cant.restriction));
        }
        return format!(
            "{} {}",
            describe_restriction(&cant.restriction),
            describe_until(&cant.duration)
        );
    }
    if let Some(remove_up_to_any) =
        effect.downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
    {
        return format!(
            "Remove up to {} counters from {}",
            describe_value(&remove_up_to_any.max_count),
            describe_choose_spec(&remove_up_to_any.target)
        );
    }
    if let Some(surveil) = effect.downcast_ref::<crate::effects::SurveilEffect>() {
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
    if let Some(scry) = effect.downcast_ref::<crate::effects::ScryEffect>() {
        if scry.player == PlayerFilter::You {
            return format!("Scry {}", describe_value(&scry.count));
        }
        if scry.player == PlayerFilter::Opponent {
            return format!("Fateseal {}", describe_value(&scry.count));
        }
        let player = describe_player_filter(&scry.player);
        return format!(
            "{} {} {}",
            player,
            player_verb(&player, "scry", "scries"),
            describe_value(&scry.count)
        );
    }
    if let Some(discover) = effect.downcast_ref::<crate::effects::DiscoverEffect>() {
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
    if let Some(become_basic) =
        effect.downcast_ref::<crate::effects::BecomeBasicLandTypeChoiceEffect>()
    {
        let target = describe_choose_spec(&become_basic.target);
        if become_basic.until == Until::EndOfTurn {
            return format!(
                "{} becomes the basic land type of your choice until end of turn",
                target
            );
        }
        return format!(
            "{} becomes the basic land type of your choice {}",
            target,
            describe_until(&become_basic.until)
        );
    }
    if let Some(investigate) = effect.downcast_ref::<crate::effects::InvestigateEffect>() {
        return format!("Investigate {}", describe_value(&investigate.count));
    }
    if let Some(poison) = effect.downcast_ref::<crate::effects::PoisonCountersEffect>() {
        let amount = match poison.count {
            Value::Fixed(1) => "a poison counter".to_string(),
            _ => format!("{} poison counters", describe_value(&poison.count)),
        };
        return format!("{} gets {}", describe_player_filter(&poison.player), amount);
    }
    if let Some(pay_energy) = effect.downcast_ref::<crate::effects::PayEnergyEffect>() {
        let payer = describe_choose_spec(&pay_energy.player);
        let amount = describe_energy_payment_amount(&pay_energy.amount);
        if payer == "you" {
            return format!("Pay {amount}");
        }
        return format!("{payer} {} {amount}", player_verb(&payer, "pay", "pays"));
    }
    if let Some(energy) = effect.downcast_ref::<crate::effects::EnergyCountersEffect>() {
        let player = describe_player_filter(&energy.player);
        let verb = player_verb(&player, "get", "gets");
        return match &energy.count {
            Value::Fixed(amount) if *amount > 0 => format!(
                "{player} {verb} {}",
                repeated_energy_symbols(*amount as usize)
            ),
            Value::Count(filter) => format!(
                "{player} {verb} {{E}} for each {}",
                describe_for_each_count_filter(filter)
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
    if let Some(connive) = effect.downcast_ref::<crate::effects::ConniveEffect>() {
        return format!("{} connives", describe_choose_spec(&connive.target));
    }
    if let Some(goad) = effect.downcast_ref::<crate::effects::GoadEffect>() {
        return format!("Goad {}", describe_goad_target(&goad.target));
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
    if let Some(lose_game) = effect.downcast_ref::<crate::effects::LoseTheGameEffect>() {
        let player = describe_player_filter(&lose_game.player);
        return format!(
            "{} {} the game",
            player,
            player_verb(&player, "lose", "loses")
        );
    }
    if let Some(skip_draw) = effect.downcast_ref::<crate::effects::SkipDrawStepEffect>() {
        return format!(
            "{} skips their next draw step",
            describe_player_filter(&skip_draw.player)
        );
    }
    if let Some(skip_turn) = effect.downcast_ref::<crate::effects::SkipTurnEffect>() {
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
    if let Some(monstrosity) = effect.downcast_ref::<crate::effects::MonstrosityEffect>() {
        return format!("Monstrosity {}", describe_value(&monstrosity.n));
    }
    if let Some(copy_spell) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        if matches!(copy_spell.target, ChooseSpec::Source)
            && matches!(
                copy_spell.count,
                Value::SpellsCastBeforeThisTurn(PlayerFilter::You)
            )
        {
            return "Copy this spell for each spell cast before it this turn".to_string();
        }
        if matches!(copy_spell.target, ChooseSpec::Source)
            && matches!(copy_spell.count, Value::Fixed(1))
        {
            return "Copy this spell".to_string();
        }
        if matches!(copy_spell.target, ChooseSpec::Source)
            && let Value::Count(filter) = &copy_spell.count
        {
            let mut each_filter = filter.description();
            if each_filter.ends_with('s') {
                each_filter = each_filter.trim_end_matches('s').to_string();
            }
            return format!("Copy this spell for each {each_filter}");
        }
        return format!(
            "Copy {} {} time(s)",
            describe_choose_spec(&copy_spell.target),
            describe_value(&copy_spell.count)
        );
    }
    if let Some(choose_new) = effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>() {
        let chooser_text = choose_new
            .chooser
            .as_ref()
            .map(describe_player_filter)
            .unwrap_or_else(|| "you".to_string());
        return format!(
            "{} {} new targets for the copy",
            chooser_text,
            if choose_new.may {
                "may choose"
            } else {
                "chooses"
            },
        );
    }
    if let Some(retarget) = effect.downcast_ref::<crate::effects::RetargetStackObjectEffect>() {
        let target_text = describe_choose_spec(&retarget.target);
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
    if let Some(set_life) = effect.downcast_ref::<crate::effects::SetLifeTotalEffect>() {
        return format!(
            "{}'s life total becomes {}",
            describe_player_filter(&set_life.player),
            describe_value(&set_life.amount)
        );
    }
    if let Some(pay_mana) = effect.downcast_ref::<crate::effects::PayManaEffect>() {
        return format!(
            "{} pays {}",
            describe_choose_spec(&pay_mana.player),
            pay_mana.cost.to_oracle()
        );
    }
    if let Some(add_any) = effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>() {
        if let Some(colors) = &add_any.available_colors {
            if matches!(add_any.amount, Value::Fixed(1)) {
                let options = colors
                    .iter()
                    .copied()
                    .map(crate::mana::ManaSymbol::from_color)
                    .collect::<Vec<_>>();
                return format!(
                    "Add {} to {}",
                    describe_mana_alternatives(&options),
                    describe_mana_pool_owner(&add_any.player)
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
                "Add {} mana in any combination of {} to {}",
                describe_value(&add_any.amount),
                options,
                describe_mana_pool_owner(&add_any.player)
            );
        }
        return format!(
            "Add {} mana of any color to {}",
            describe_value(&add_any.amount),
            describe_mana_pool_owner(&add_any.player)
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
                "Add X mana of any one color to {}, where X is the number of {}",
                describe_mana_pool_owner(&add_one.player),
                count_subject
            );
        }
        return format!(
            "Add {} mana of any one color to {}",
            describe_value(&add_one.amount),
            describe_mana_pool_owner(&add_one.player)
        );
    }
    if let Some(add_chosen) =
        effect.downcast_ref::<crate::effects::mana::AddManaOfChosenColorEffect>()
    {
        let pool = describe_mana_pool_owner(&add_chosen.player);
        let amount = describe_value(&add_chosen.amount);
        if let Some(fixed) = add_chosen.fixed_option {
            let fixed_symbol = describe_mana_symbol(crate::mana::ManaSymbol::from_color(fixed));
            if matches!(add_chosen.amount, Value::Fixed(1)) {
                return format!(
                    "Add {} or one mana of the chosen color to {}",
                    fixed_symbol, pool
                );
            }
            return format!(
                "Add {} or {} mana of the chosen color to {}",
                fixed_symbol, amount, pool
            );
        }
        if matches!(add_chosen.amount, Value::Fixed(1)) {
            return format!("Add one mana of the chosen color to {}", pool);
        }
        return format!("Add {} mana of the chosen color to {}", amount, pool);
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
        return format!(
            "Add {} mana of any{} {} to {} that {} could produce",
            describe_value(&add_land_produced.amount),
            one_word,
            any_word,
            describe_mana_pool_owner(&add_land_produced.player),
            add_land_produced.land_filter.description()
        );
    }
    if let Some(add_commander) =
        effect.downcast_ref::<crate::effects::AddManaFromCommanderColorIdentityEffect>()
    {
        return format!(
            "Add {} mana of commander's color identity to {}",
            describe_value(&add_commander.amount),
            describe_mana_pool_owner(&add_commander.player)
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
        return format!(
            "Prevent the next {} {} that would be dealt to {} {}",
            describe_value(&prevent_damage.amount),
            damage_text,
            describe_choose_spec(&prevent_damage.target),
            timing
        );
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
        return format!(
            "Prevent all {} that would be dealt to {} {}",
            damage_text,
            describe_choose_spec(&prevent_all_target.target),
            timing
        );
    }
    if let Some(prevent_next_time) =
        effect.downcast_ref::<crate::effects::PreventNextTimeDamageEffect>()
    {
        let source_text = match &prevent_next_time.source {
            crate::effects::PreventNextTimeDamageSource::Choice => {
                "a source of your choice".to_string()
            }
            crate::effects::PreventNextTimeDamageSource::Filter(filter) => {
                let desc = filter.description();
                if desc.is_empty() {
                    "a source".to_string()
                } else {
                    format!("{desc} source")
                }
            }
        };
        let target_text = match prevent_next_time.target {
            crate::effects::PreventNextTimeDamageTarget::AnyTarget => "any target".to_string(),
            crate::effects::PreventNextTimeDamageTarget::You => "you".to_string(),
        };
        return format!(
            "Prevent the next time {source_text} would deal damage to {target_text} this turn"
        );
    }
    if let Some(redirect_next) =
        effect.downcast_ref::<crate::effects::RedirectNextDamageToTargetEffect>()
    {
        return format!(
            "The next {} damage that would be dealt to this creature this turn is dealt to {} instead",
            describe_value(&redirect_next.amount),
            describe_choose_spec(&redirect_next.target)
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
        };
        return format!(
            "The next time {source_text} would deal damage to {} this turn, that damage is dealt to this creature instead",
            describe_choose_spec(&redirect_next_time.target)
        );
    }
    if let Some(prevent_from) =
        effect.downcast_ref::<crate::effects::PreventAllCombatDamageFromEffect>()
    {
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
    if let Some(prevent_all) = effect.downcast_ref::<crate::effects::PreventAllDamageEffect>() {
        let damage_type = describe_damage_filter(&prevent_all.damage_filter);
        let protected = describe_prevention_target(&prevent_all.target);
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
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        let trigger_display = schedule.trigger.display();
        let mut trigger_text = trigger_display.trim().trim_end_matches('.').to_string();
        if schedule.until_end_of_turn {
            let trigger_lower = trigger_text.to_ascii_lowercase();
            if !trigger_lower.contains(" this turn") {
                trigger_text.push_str(" this turn");
            }
        }
        trigger_text = cleanup_decompiled_text(&trigger_text);
        let trigger_lower = trigger_text.to_ascii_lowercase();
        let delayed_text = lowercase_first(&describe_effect_list(&schedule.effects));
        if schedule.one_shot && schedule.start_next_turn {
            if trigger_lower.contains("that player's end step")
                || trigger_lower.contains("target player's end step")
            {
                return format!(
                    "At the beginning of the end step of that player's next turn, {delayed_text}"
                );
            }
            if trigger_lower.contains("your end step") {
                return format!("At the beginning of your next end step, {delayed_text}");
            }
            return format!("At the beginning of the next end step, {delayed_text}");
        }
        if schedule.one_shot
            && (trigger_lower.contains("beginning of each player's end step")
                || trigger_lower.contains("beginning of end step"))
        {
            return format!("At the beginning of the next end step, {delayed_text}");
        }
        if schedule.one_shot && trigger_lower.contains("beginning of your end step") {
            return format!("At the beginning of your next end step, {delayed_text}");
        }
        if schedule.one_shot && trigger_lower.contains("when this creature dies") {
            if let Some(filter) = &schedule.target_filter {
                let subject = with_indefinite_article(&describe_for_each_filter(filter));
                return format!(
                    "When {subject} dealt damage this way dies this turn, {delayed_text}"
                );
            }
            return format!("When that creature dies this turn, {delayed_text}");
        }
        if trigger_lower.starts_with("when ")
            || trigger_lower.starts_with("whenever ")
            || trigger_lower.starts_with("if ")
        {
            return format!("{trigger_text}, {delayed_text}");
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
    if let Some(grant_play) = effect.downcast_ref::<crate::effects::GrantPlayFromGraveyardEffect>()
    {
        let player = describe_player_filter(&grant_play.player);
        let graveyard_owner = describe_possessive_player_filter(&grant_play.player);
        return format!(
            "Until end of turn, {player} may play lands and cast spells from {graveyard_owner} graveyard"
        );
    }
    if let Some(control_player) = effect.downcast_ref::<crate::effects::ControlPlayerEffect>() {
        return format!(
            "Control {} during their next turn",
            describe_player_filter(&control_player.player)
        );
    }
    if let Some(exile_hand) = effect.downcast_ref::<crate::effects::ExileFromHandAsCostEffect>() {
        return capitalize_first(&describe_exile_from_hand_as_cost_phrase(exile_hand));
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
        return format!(
            "{} becomes the color of {} choice {}",
            describe_choose_spec(&become_color.target),
            describe_possessive_player_filter(&become_color.chooser),
            describe_until(&become_color.until)
        );
    }
    if let Some(become_type) =
        effect.downcast_ref::<crate::effects::BecomeCreatureTypeChoiceEffect>()
    {
        if become_type.excluded_subtypes.is_empty() {
            return format!(
                "{} becomes the creature type of {} choice {}",
                describe_choose_spec(&become_type.target),
                describe_possessive_player_filter(&become_type.chooser),
                describe_until(&become_type.until)
            );
        }
        let excluded = become_type
            .excluded_subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}").to_ascii_lowercase())
            .collect::<Vec<_>>();
        return format!(
            "{} becomes the creature type of {} choice (other than {}) {}",
            describe_choose_spec(&become_type.target),
            describe_possessive_player_filter(&become_type.chooser),
            join_with_or(&excluded),
            describe_until(&become_type.until)
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
    if effect
        .downcast_ref::<crate::effects::ClashEffect>()
        .is_some()
    {
        return "Clash with an opponent".to_string();
    }
    if let Some(clear_damage) = effect.downcast_ref::<crate::effects::ClearDamageEffect>() {
        return format!(
            "Remove all damage from {}",
            describe_choose_spec(&clear_damage.target)
        );
    }
    if let Some(create_emblem) = effect.downcast_ref::<crate::effects::CreateEmblemEffect>() {
        return format!("Create an emblem named {}", create_emblem.emblem.name);
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
    if let Some(exchange_life) = effect.downcast_ref::<crate::effects::ExchangeLifeTotalsEffect>() {
        return format!(
            "Exchange life totals of {} and {}",
            describe_player_filter(&exchange_life.player1),
            describe_player_filter(&exchange_life.player2)
        );
    }
    if let Some(exile_top) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() {
        let (count_text, noun, _) = describe_look_count_and_noun(&exile_top.count);
        return format!(
            "Exile the top {count_text} {noun} of {} library",
            describe_possessive_player_filter(&exile_top.player)
        );
    }
    if let Some(experience) = effect.downcast_ref::<crate::effects::ExperienceCountersEffect>() {
        let player = describe_player_filter(&experience.player);
        return format!(
            "{player} {} {} experience counter{}",
            player_verb(&player, "get", "gets"),
            describe_value(&experience.count),
            if matches!(&experience.count, Value::Fixed(1)) {
                ""
            } else {
                "s"
            }
        );
    }
    if let Some(for_each_counter_kind) =
        effect.downcast_ref::<crate::effects::ForEachCounterKindPutOrRemoveEffect>()
    {
        return format!(
            "For each kind of counter on {}, choose to put or remove one of that kind",
            describe_choose_spec(&for_each_counter_kind.target)
        );
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantEffect>() {
        let duration = match grant.duration {
            crate::grant::GrantDuration::UntilEndOfTurn => " until end of turn",
            crate::grant::GrantDuration::Forever => "",
        };
        return format!(
            "{} gains {}{}",
            describe_choose_spec(&grant.target),
            grant.grantable.display(),
            duration
        );
    }
    if let Some(grant_play_tagged) = effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
    {
        let timing = match grant_play_tagged.duration {
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn => "until end of turn",
            crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                "until the end of your next turn"
            }
        };
        return format!(
            "{} may play tagged '{}' cards {timing}",
            describe_player_filter(&grant_play_tagged.player),
            grant_play_tagged.tag.as_str()
        );
    }
    if let Some(grant_tagged_spell_life) =
        effect.downcast_ref::<crate::effects::GrantTaggedSpellLifeCostByManaValueEffect>()
    {
        return format!(
            "{} may cast tagged '{}' spells from exile this turn by paying life equal to their mana value",
            describe_player_filter(&grant_tagged_spell_life.player),
            grant_tagged_spell_life.tag.as_str()
        );
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
    if effect
        .downcast_ref::<crate::effects::NinjutsuCostEffect>()
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
            "Return this card from your graveyard or exile to the battlefield{}",
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
    if let Some(vote) = effect.downcast_ref::<crate::effects::VoteEffect>() {
        let choices = vote
            .options
            .iter()
            .map(|option| option.name.to_ascii_lowercase())
            .collect::<Vec<_>>();
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
        return format!("Each player votes for {}{}", join_with_or(&choices), suffix);
    }
    if effect
        .downcast_ref::<crate::effects::EmitKeywordActionEffect>()
        .is_some()
    {
        // Runtime keyword-action events are instrumentation; they should not leak
        // into oracle-like rendered rules text.
        return String::new();
    }
    "Unsupported effect".to_string()
}

fn describe_activation_timing_clause(timing: &ActivationTiming) -> Option<&'static str> {
    match timing {
        ActivationTiming::AnyTime => None,
        ActivationTiming::SorcerySpeed => Some("Activate only as a sorcery"),
        ActivationTiming::DuringCombat => Some("Activate only during combat"),
        ActivationTiming::OncePerTurn => Some("Activate only once each turn"),
        ActivationTiming::DuringYourTurn => Some("Activate only during your turn"),
        ActivationTiming::DuringOpponentsTurn => Some("Activate only during an opponent's turn"),
    }
}

fn normalize_activation_restriction_clause(raw: &str) -> String {
    let mut clause = raw.trim().trim_end_matches('.').to_string();
    if clause.is_empty() {
        return clause;
    }
    let lower = clause.to_ascii_lowercase();
    if lower == "activate only as a sorcery and only once each turn" {
        return "Activate only once each turn".to_string();
    }
    clause = clause.replace("activate only as sorcery", "activate only as a sorcery");
    clause = clause.replace("activate only once turn", "activate only once each turn");
    if clause.starts_with("activate ") {
        clause = capitalize_first(&clause);
    }
    clause
}

fn collect_activation_restriction_clauses(
    timing: &ActivationTiming,
    additional_restrictions: &[String],
) -> Vec<String> {
    let mut clauses = Vec::new();

    if let Some(timing_clause) = describe_activation_timing_clause(timing) {
        let normalized = normalize_activation_restriction_clause(timing_clause);
        push_activation_restriction_clause(&mut clauses, normalized);
    }

    for raw in additional_restrictions {
        let normalized = normalize_activation_restriction_clause(raw);
        push_activation_restriction_clause(&mut clauses, normalized);
    }

    clauses
}

fn push_activation_restriction_clause(clauses: &mut Vec<String>, clause: String) {
    if clause.is_empty() {
        return;
    }
    let clause_lower = clause.to_ascii_lowercase();
    let mut remove_indices = Vec::new();
    for (idx, existing) in clauses.iter().enumerate() {
        let existing_lower = existing.to_ascii_lowercase();
        if existing_lower == clause_lower
            || activation_clause_is_more_specific(&existing_lower, &clause_lower)
        {
            return;
        }
        if activation_clause_is_more_specific(&clause_lower, &existing_lower) {
            remove_indices.push(idx);
        }
    }
    for idx in remove_indices.into_iter().rev() {
        clauses.remove(idx);
    }
    clauses.push(clause);
}

fn activation_clause_is_more_specific(candidate: &str, base: &str) -> bool {
    if candidate.len() <= base.len() || !candidate.starts_with(base) {
        return false;
    }
    let tail = candidate[base.len()..].trim_start();
    tail.starts_with(',')
        || tail.starts_with("and ")
        || tail.starts_with("before ")
        || tail.starts_with("after ")
        || tail.starts_with("if ")
        || tail.starts_with("unless ")
}

fn join_activation_restriction_clauses(clauses: &[String]) -> String {
    let mut iter = clauses.iter();
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut line = first.clone();
    for clause in iter {
        if let Some(rest) = clause.strip_prefix("Activate only ") {
            line.push_str(" and ");
            line.push_str(rest);
        } else {
            line.push_str(" and ");
            line.push_str(clause);
        }
    }
    line
}

fn describe_keyword_ability(ability: &Ability) -> Option<String> {
    let raw_text = ability.text.as_deref()?.trim();
    let text = raw_text.to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.len() == 4
        && words[0] == "cycling"
        && words[1] == "pay"
        && words[3] == "life"
        && (words[2].parse::<u32>().is_ok() || words[2] == "x")
    {
        return Some(format!("Cycling—Pay {} life", words[2]));
    }
    let is_equip_keyword = words.first().is_some_and(|word| {
        *word == "equip" || word.starts_with("equip—") || word.starts_with("equip-")
    });
    if is_equip_keyword {
        let mut rendered = if raw_text.eq_ignore_ascii_case("equip") {
            "Equip".to_string()
        } else {
            raw_text.to_string()
        };
        if let AbilityKind::Activated(activated) = &ability.kind {
            let mut restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            // Equip implies sorcery-speed by default; only surface extra restrictions.
            restriction_clauses
                .retain(|clause| !clause.eq_ignore_ascii_case("Activate only as a sorcery"));
            if !restriction_clauses.is_empty() {
                rendered.push_str(". ");
                rendered.push_str(&join_activation_restriction_clauses(&restriction_clauses));
            }
        }
        return Some(rendered);
    }
    if words.len() >= 2 && words[0] == "level" && words[1] == "up" {
        return Some(raw_text.to_string());
    }
    if text == "storm" {
        return Some("Storm".to_string());
    }
    if text == "toxic" || text.starts_with("toxic ") {
        return Some(raw_text.to_string());
    }
    let first_cycling_idx = words
        .iter()
        .position(|word| trim_cycling_punctuation(word).ends_with("cycling"));
    let is_cycling_clause = first_cycling_idx.is_some_and(|idx| {
        !words[..idx]
            .iter()
            .any(|word| matches!(*word, "has" | "have"))
    });
    if is_cycling_clause {
        let mut cycling_rendered = Vec::new();
        for (idx, word) in words.iter().enumerate() {
            let keyword = trim_cycling_punctuation(word);
            if !keyword.ends_with("cycling") {
                continue;
            }
            let next = words
                .get(idx + 1)
                .map(|next| trim_cycling_punctuation(next));
            let has_cost = next.is_none_or(is_cycling_cost_word);
            if !has_cost {
                continue;
            }
            let mut chars = keyword.chars();
            let mut base = match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => "Cycling".to_string(),
            };
            if keyword == "landcycling"
                && idx > 0
                && trim_cycling_punctuation(words[idx - 1]) == "basic"
            {
                base = "Basic landcycling".to_string();
            }
            let mut cost_tokens = Vec::new();
            let mut j = idx + 1;
            while let Some(word) = words.get(j).map(|word| trim_cycling_punctuation(word)) {
                if is_cycling_cost_word(word) {
                    cost_tokens.push(word);
                    j += 1;
                } else {
                    break;
                }
            }
            if cost_tokens.is_empty() {
                cycling_rendered.push(base);
            } else {
                let cost = cost_tokens
                    .iter()
                    .map(|word| render_cycling_cost_token(word))
                    .collect::<Vec<_>>()
                    .join("");
                cycling_rendered.push(format!("{} {}", base, cost));
            }
        }
        if !cycling_rendered.is_empty() {
            return Some(cycling_rendered.join(", "));
        }
    }
    if text == "prowess" {
        return Some("Prowess".to_string());
    }
    if text == "exalted" {
        return Some("Exalted".to_string());
    }
    if text == "persist" {
        return Some("Persist".to_string());
    }
    if text == "undying" {
        return Some("Undying".to_string());
    }
    if text.starts_with("bushido ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("rampage ") {
        return Some(raw_text.to_string());
    }
    if text == "extort" {
        return Some("Extort".to_string());
    }
    if text == "partner" {
        return Some("Partner".to_string());
    }
    if text == "assist" {
        return Some("Assist".to_string());
    }
    if text.starts_with("soulshift ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("outlast ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("modular ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("graft ") {
        return Some(raw_text.to_string());
    }
    if text == "sunburst" {
        return Some("Sunburst".to_string());
    }
    if text.starts_with("fading ") {
        return Some(raw_text.to_string());
    }
    if text.starts_with("vanishing ") {
        return Some(raw_text.to_string());
    }
    None
}

fn trim_cycling_punctuation(word: &str) -> &str {
    word.trim_matches(|ch: char| matches!(ch, ',' | '.' | ';'))
}

fn render_cycling_cost_token(word: &str) -> String {
    let upper = word.to_ascii_uppercase();
    if upper.starts_with('{') && upper.ends_with('}') {
        upper
    } else {
        format!("{{{upper}}}")
    }
}

fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x'
                )
        })
}

fn choices_are_simple_targets(choices: &[ChooseSpec]) -> bool {
    fn is_simple_target(choice: &ChooseSpec) -> bool {
        match choice {
            ChooseSpec::Target(_) | ChooseSpec::AnyTarget | ChooseSpec::PlayerOrPlaneswalker(_) => {
                true
            }
            ChooseSpec::WithCount(inner, _) => is_simple_target(inner),
            _ => false,
        }
    }

    choices.iter().all(is_simple_target)
}

fn flatten_condition_and_expr(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_condition_and_expr(left, out);
            flatten_condition_and_expr(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

fn fold_condition_exprs(conditions: Vec<crate::ConditionExpr>) -> Option<crate::ConditionExpr> {
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        crate::ConditionExpr::And(Box::new(acc), Box::new(next))
    }))
}

fn split_trigger_intervening_if(
    condition: &crate::ConditionExpr,
) -> (Option<crate::ConditionExpr>, Option<u32>) {
    let mut flat = Vec::new();
    flatten_condition_and_expr(condition, &mut flat);

    let mut non_limit = Vec::new();
    let mut max_times_each_turn: Option<u32> = None;
    for item in flat {
        match item {
            crate::ConditionExpr::MaxTimesEachTurn(limit) => {
                max_times_each_turn = Some(match max_times_each_turn {
                    Some(existing) => existing.min(limit),
                    None => limit,
                });
            }
            other => non_limit.push(other),
        }
    }

    (fold_condition_exprs(non_limit), max_times_each_turn)
}

fn describe_ability(
    index: usize,
    ability: &Ability,
    subject: &str,
    rewrite_it_deals: bool,
) -> Vec<String> {
    if let Some(keyword) = describe_keyword_ability(ability) {
        return vec![format!("Keyword ability {index}: {keyword}")];
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            if let Some(levels) = static_ability.level_abilities()
                && !levels.is_empty()
            {
                let mut lines = Vec::new();
                for level in levels {
                    let range = match level.max_level {
                        Some(max) if max == level.min_level => format!("Level {}", level.min_level),
                        Some(max) => format!("Level {}-{}", level.min_level, max),
                        None => format!("Level {}+", level.min_level),
                    };
                    lines.push(format!("Static ability {index}: {range}"));
                    if let Some((power, toughness)) = level.power_toughness {
                        lines.push(format!("Static ability {index}: {power}/{toughness}"));
                    }
                    for granted in &level.abilities {
                        lines.push(format!("Static ability {index}: {}", granted.display()));
                    }
                }
                return lines;
            }
            if matches!(
                static_ability.id(),
                crate::static_abilities::StaticAbilityId::KeywordMarker
                    | crate::static_abilities::StaticAbilityId::RuleTextPlaceholder
                    | crate::static_abilities::StaticAbilityId::KeywordFallbackText
                    | crate::static_abilities::StaticAbilityId::RuleFallbackText
                    | crate::static_abilities::StaticAbilityId::UnsupportedParserLine
            ) && let Some(text) = ability.text.as_deref()
            {
                let normalized = normalize_sentence_surface_style(text.trim());
                if !normalized.is_empty() {
                    return vec![format!("Static ability {index}: {normalized}")];
                }
            }
            vec![format!(
                "Static ability {index}: {}",
                static_ability.display()
            )]
        }
        AbilityKind::Triggered(triggered) => {
            if let Some(text) = ability.text.as_deref() {
                let normalized = normalize_sentence_surface_style(text.trim());
                if normalized.to_ascii_lowercase().starts_with("annihilator ")
                    || normalized
                        .trim_end_matches('.')
                        .eq_ignore_ascii_case("haunt")
                    || normalized
                        .to_ascii_lowercase()
                        .starts_with("cumulative upkeep")
                {
                    return vec![format!("Keyword ability {index}: {normalized}")];
                }
            }
            let mut line = format!("Triggered ability {index}: {}", triggered.trigger.display());
            let (intervening_condition, max_times_each_turn) = triggered
                .intervening_if
                .as_ref()
                .map(split_trigger_intervening_if)
                .unwrap_or((None, None));
            if let Some(condition) = intervening_condition {
                line.push_str(", if ");
                line.push_str(&describe_condition(&condition));
            }
            let mut clauses = Vec::new();
            if !triggered.choices.is_empty()
                && !(!triggered.effects.is_empty()
                    && choices_are_simple_targets(&triggered.choices))
            {
                let choices = triggered
                    .choices
                    .iter()
                    .map(describe_choose_spec)
                    .collect::<Vec<_>>()
                    .join(", ");
                clauses.push(format!("choose {choices}"));
            }
            if !triggered.effects.is_empty() {
                let effects = describe_effect_list(&triggered.effects);
                clauses.push(rewrite_damage_phrases_for_permanent_abilities(
                    &effects,
                    subject,
                    rewrite_it_deals,
                ));
            }
            if !clauses.is_empty() {
                // Oracle-style: "Whenever ..., if ..., ..." rather than "Whenever ...: If ..."
                if clauses.len() == 1 {
                    let only = clauses[0].trim_start();
                    if let Some(rest) = only.strip_prefix("If ") {
                        line.push_str(", if ");
                        line.push_str(rest.trim_start());
                    } else if let Some(rest) = only.strip_prefix("if ") {
                        line.push_str(", if ");
                        line.push_str(rest.trim_start());
                    } else {
                        line.push_str(": ");
                        line.push_str(only);
                    }
                } else {
                    line.push_str(": ");
                    line.push_str(&clauses.join(": "));
                }
            }
            if let Some(max) = max_times_each_turn {
                if max == 1 {
                    line.push_str(". This ability triggers only once each turn");
                } else if max == 2 {
                    line.push_str(". This ability triggers only twice each turn");
                } else {
                    line.push_str(". This ability triggers only ");
                    line.push_str(&max.to_string());
                    line.push_str(" times");
                    line.push_str(" each turn");
                }
            }
            vec![line]
        }
        AbilityKind::Activated(activated) if activated.is_mana_ability() => {
            let mut line = format!("Mana ability {index}");
            let cost_text = if !activated.mana_cost.costs().is_empty() {
                Some(describe_cost_list(activated.mana_cost.costs()))
            } else {
                None
            };
            let mana_symbols = activated.mana_symbols();
            let add_text = if !mana_symbols.is_empty() {
                Some(format!(
                    "Add {}",
                    mana_symbols
                        .iter()
                        .copied()
                        .map(describe_mana_symbol)
                        .collect::<Vec<_>>()
                        .join("")
                ))
            } else {
                None
            };
            if let (Some(cost), Some(add)) = (&cost_text, &add_text) {
                line.push_str(": ");
                line.push_str(cost);
                line.push_str(": ");
                line.push_str(add);
            } else if let Some(cost) = &cost_text {
                line.push_str(": ");
                line.push_str(cost);
            } else if let Some(add) = &add_text {
                line.push_str(": ");
                line.push_str(add);
            }
            if !activated.effects.is_empty() {
                line.push_str(": ");
                let effects = describe_effect_list(&activated.effects);
                line.push_str(&rewrite_damage_phrases_for_permanent_abilities(
                    &effects,
                    subject,
                    rewrite_it_deals,
                ));
            }
            if let Some(condition) = &activated.activation_condition {
                let clause = describe_mana_activation_condition(condition);
                if !clause.is_empty() {
                    line.push_str(". ");
                    line.push_str(&clause);
                }
            }
            vec![line]
        }
        AbilityKind::Activated(activated) => {
            if let Some(text) = ability.text.as_deref() {
                let normalized = normalize_sentence_surface_style(text.trim());
                if normalized.to_ascii_lowercase().starts_with("crew ") {
                    return vec![format!("Keyword ability {index}: {normalized}")];
                }
            }
            let mut line = format!("Activated ability {index}");
            let mut pre = Vec::new();
            let has_boast_label = ability
                .text
                .as_deref()
                .is_some_and(|text| text.eq_ignore_ascii_case("boast"));
            let has_renew_label = ability
                .text
                .as_deref()
                .is_some_and(|text| text.eq_ignore_ascii_case("renew"));
            if has_boast_label {
                let mut label = "Boast".to_string();
                if !activated.mana_cost.costs().is_empty() {
                    label.push(' ');
                    label.push_str(&describe_cost_list(activated.mana_cost.costs()));
                }
                pre.push(label);
            } else if has_renew_label {
                let mut label = "Renew".to_string();
                if !activated.mana_cost.costs().is_empty() {
                    label.push_str(" \u{2014} ");
                    label.push_str(&describe_cost_list(activated.mana_cost.costs()));
                }
                pre.push(label);
            } else if !activated.mana_cost.costs().is_empty() {
                pre.push(describe_cost_list(activated.mana_cost.costs()));
            }
            if !activated.choices.is_empty()
                && !(!activated.effects.is_empty()
                    && choices_are_simple_targets(&activated.choices))
            {
                pre.push(format!(
                    "choose {}",
                    activated
                        .choices
                        .iter()
                        .map(describe_choose_spec)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !pre.is_empty() {
                line.push_str(": ");
                line.push_str(&pre.join(", "));
            }
            if !activated.effects.is_empty() {
                line.push_str(": ");
                let effects = describe_effect_list(&activated.effects);
                line.push_str(&rewrite_damage_phrases_for_permanent_abilities(
                    &effects,
                    subject,
                    rewrite_it_deals,
                ));
            }
            if let Some(x_clause) = extract_activated_x_is_clause(ability.text.as_deref()) {
                let line_lower = line.to_ascii_lowercase();
                let clause_lower = x_clause.to_ascii_lowercase();
                let line_already_has_x_clause = line_lower.contains("where x is ")
                    || line_lower.contains(". x is ")
                    || line_lower.ends_with(" x is");
                if !line_already_has_x_clause && !line_lower.contains(clause_lower.as_str()) {
                    if !inject_x_clause_into_modal_heading(&mut line, &x_clause) {
                        while line.ends_with('.') {
                            line.pop();
                        }
                        line.push_str(". ");
                        line.push_str(&x_clause);
                    }
                }
            }
            let restriction_clauses = collect_activation_restriction_clauses(
                &activated.timing,
                &activated.additional_restrictions,
            );
            if !restriction_clauses.is_empty() {
                line.push_str(". ");
                line.push_str(&join_activation_restriction_clauses(&restriction_clauses));
            }
            vec![line]
        }
    }
}

fn rewrite_damage_phrases_for_permanent_abilities(
    effect_text: &str,
    subject: &str,
    rewrite_it_deals: bool,
) -> String {
    if let Some(rest) = effect_text.strip_prefix("Deal ") {
        return format!("{subject} deals {rest}");
    }
    if let Some(rest) = effect_text.strip_prefix("deal ") {
        return format!("{subject} deals {rest}");
    }
    if rewrite_it_deals {
        if let Some(rest) = effect_text.strip_prefix("It deals ") {
            return format!("{subject} deals {rest}");
        }
        if let Some(rest) = effect_text.strip_prefix("it deals ") {
            return format!("{subject} deals {rest}");
        }
    }

    let mut out = effect_text.to_string();
    // Common oracle phrasing: "you may have this creature deal ..."
    out = out.replace("You may Deal ", &format!("You may have {subject} deal "));
    out = out.replace("you may Deal ", &format!("you may have {subject} deal "));
    out = out.replace("You may deal ", &format!("You may have {subject} deal "));
    out = out.replace("you may deal ", &format!("you may have {subject} deal "));
    out
}

fn card_self_reference_phrase_for_card(card: &crate::card::Card) -> &'static str {
    if card.is_instant() || card.is_sorcery() {
        return "this spell";
    }
    if card.subtypes.contains(&Subtype::Aura) {
        return "this Aura";
    }
    if card.subtypes.contains(&Subtype::Equipment) {
        return "this Equipment";
    }
    if card.subtypes.contains(&Subtype::Fortification) {
        return "this Fortification";
    }
    if card.subtypes.contains(&Subtype::Saga) {
        return "this Saga";
    }
    if card.subtypes.contains(&Subtype::Vehicle) {
        return "this Vehicle";
    }

    let card_types = &card.card_types;
    if card_types.contains(&CardType::Creature) {
        "this creature"
    } else if card_types.contains(&CardType::Enchantment) {
        "this enchantment"
    } else if card_types.contains(&CardType::Battle) {
        "this battle"
    } else if card_types.contains(&CardType::Land) {
        "this land"
    } else if card_types.contains(&CardType::Artifact) {
        "this artifact"
    } else if card_types.contains(&CardType::Planeswalker) {
        "this planeswalker"
    } else {
        "this permanent"
    }
}

fn subject_for_card(card: &crate::card::Card) -> &'static str {
    card_self_reference_phrase_for_card(card)
}

fn extract_activated_x_is_clause(text: Option<&str>) -> Option<String> {
    let text = text?.trim();
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("x is ")?;
    let clause = text[idx..].trim().trim_end_matches('.').trim();
    if clause.is_empty() {
        None
    } else {
        Some(clause.to_string())
    }
}

fn inject_x_clause_into_modal_heading(line: &mut String, x_clause: &str) -> bool {
    let direct_replacements = vec![
        ("Choose one —\n• ", format!("Choose one. {x_clause}\n• ")),
        ("Choose one -\n• ", format!("Choose one. {x_clause}\n• ")),
        ("choose one —\n• ", format!("choose one. {x_clause}\n• ")),
        ("choose one -\n• ", format!("choose one. {x_clause}\n• ")),
        (
            "Choose one or both —\n• ",
            format!("Choose one or both. {x_clause}\n• "),
        ),
        (
            "Choose one or both -\n• ",
            format!("Choose one or both. {x_clause}\n• "),
        ),
        (
            "Choose one or more —\n• ",
            format!("Choose one or more. {x_clause}\n• "),
        ),
        (
            "Choose one or more -\n• ",
            format!("Choose one or more. {x_clause}\n• "),
        ),
    ];
    for (marker, replacement) in direct_replacements {
        if line.contains(marker) {
            *line = line.replacen(marker, replacement.as_str(), 1);
            return true;
        }
    }

    for marker in [
        "Choose one —",
        "Choose one -",
        "choose one —",
        "choose one -",
        "Choose one or both —",
        "Choose one or both -",
        "choose one or both —",
        "choose one or both -",
        "Choose one or more —",
        "Choose one or more -",
        "choose one or more —",
        "choose one or more -",
    ] {
        if line.contains(marker) {
            let replacement = if marker.contains(" —") {
                marker.replacen(" —", format!(". {x_clause} —").as_str(), 1)
            } else {
                marker.replacen(" -", format!(". {x_clause} -").as_str(), 1)
            };
            *line = line.replacen(marker, replacement.as_str(), 1);
            return true;
        }
    }
    false
}

fn describe_mana_activation_condition(condition: &crate::ConditionExpr) -> String {
    fn flatten(condition: &crate::ConditionExpr, out: &mut Vec<crate::ConditionExpr>) {
        match condition {
            crate::ConditionExpr::And(left, right) => {
                flatten(left, out);
                flatten(right, out);
            }
            _ => out.push(condition.clone()),
        }
    }

    match condition {
        crate::ConditionExpr::And(_, _) => {
            let mut conditions = Vec::new();
            flatten(condition, &mut conditions);
            let clauses = conditions
                .iter()
                .map(describe_mana_activation_condition)
                .collect::<Vec<_>>();
            match clauses.len() {
                0 => String::new(),
                1 => clauses[0].clone(),
                _ => {
                    let mut iter = clauses.into_iter();
                    let first = iter.next().unwrap_or_default();
                    let mut line = first;
                    for clause in iter {
                        if let Some(rest) = clause.strip_prefix("Activate only ") {
                            line.push_str(" and ");
                            line.push_str(rest);
                        } else {
                            line.push_str(" and ");
                            line.push_str(&clause);
                        }
                    }
                    line
                }
            }
        }
        crate::ConditionExpr::ControlCreaturesTotalPowerAtLeast(power) => {
            format!("Activate only if creatures you control have total power {power} or greater")
        }
        crate::ConditionExpr::CardInYourGraveyard {
            card_types,
            subtypes,
        } => {
            let mut descriptors: Vec<String> = Vec::new();
            for subtype in subtypes {
                descriptors.push(format!("{subtype:?}"));
            }
            for card_type in card_types {
                descriptors.push(format!("{card_type:?}").to_ascii_lowercase());
            }
            descriptors.retain(|entry| !entry.is_empty());
            descriptors.dedup();

            if descriptors.is_empty() {
                "Activate only if there is a card in your graveyard".to_string()
            } else if descriptors.len() == 1 {
                format!(
                    "Activate only if there is an {} card in your graveyard",
                    descriptors[0]
                )
            } else {
                let head = descriptors[..descriptors.len() - 1].join(" ");
                let tail = descriptors.last().expect("descriptor tail");
                format!("Activate only if there is a {head} {tail} card in your graveyard")
            }
        }
        crate::ConditionExpr::ActivationTiming(timing) => match timing {
            ActivationTiming::AnyTime => "Activate only as an instant".to_string(),
            ActivationTiming::SorcerySpeed => "Activate only as a sorcery".to_string(),
            ActivationTiming::DuringCombat => "Activate only during combat".to_string(),
            ActivationTiming::OncePerTurn => "Activate only once each turn".to_string(),
            ActivationTiming::DuringYourTurn => "Activate only during your turn".to_string(),
            ActivationTiming::DuringOpponentsTurn => {
                "Activate only during an opponent's turn".to_string()
            }
        },
        crate::ConditionExpr::MaxActivationsPerTurn(limit) => {
            if *limit == 1 {
                "Activate only once each turn".to_string()
            } else {
                format!("Activate only up to {limit} times each turn")
            }
        }
        crate::ConditionExpr::Unmodeled(restriction) => {
            let suffix = restriction
                .trim_start_matches("activate only ")
                .trim_start_matches("Activate only ")
                .trim_start_matches("activate ")
                .trim_start_matches("Activate ");
            if suffix.is_empty() {
                "Activate only".to_string()
            } else {
                format!("Activate only {suffix}")
            }
        }
        _ => {
            let described = describe_condition(condition);
            let described = described.trim().trim_end_matches('.');
            if described.is_empty() {
                "Activate only if this condition is met".to_string()
            } else {
                format!("Activate only if {}", lowercase_first(described))
            }
        }
    }
}

#[allow(dead_code)]
fn collapse_redundant_keyword_tail(line: &str) -> String {
    let mut normalized = line.trim().to_string();
    loop {
        let lower = normalized.to_ascii_lowercase();
        let Some(split_idx) = lower.rfind(" and ") else {
            break;
        };
        let prefix = normalized[..split_idx].trim_end();
        let tail = normalized[split_idx + 5..].trim_start();

        // Only collapse trailing keyword duplication that appears as a second
        // sentence/clause after punctuation (e.g. ". and haste." or
        // "). and vigilance.").
        let Some(last_prefix_char) = prefix.chars().rev().find(|ch| !ch.is_whitespace()) else {
            break;
        };
        if !matches!(last_prefix_char, '.' | ')' | '"') {
            break;
        }

        let tail_no_reminder = tail
            .split_once('(')
            .map(|(head, _)| head.trim())
            .unwrap_or(tail)
            .trim_end_matches('.')
            .trim();
        let keyword_tail = if is_keyword_phrase(tail_no_reminder) {
            Some(tail_no_reminder.to_ascii_lowercase())
        } else {
            normalize_keyword_list_phrase(tail_no_reminder)
        };

        let Some(keyword_tail) = keyword_tail else {
            break;
        };

        let prefix_lower = strip_parenthetical_segments(prefix).to_ascii_lowercase();
        let duplicate = prefix_lower.contains(&format!(" has {keyword_tail}"))
            || prefix_lower.contains(&format!(" gain {keyword_tail}"))
            || prefix_lower.contains(&format!(" gains {keyword_tail}"));
        if !duplicate {
            break;
        }

        let collapsed = prefix.trim_end_matches('.').trim_end();
        normalized = if collapsed.ends_with('"') || collapsed.ends_with(')') {
            collapsed.to_string()
        } else {
            format!("{collapsed}.")
        };
    }
    normalized
}

fn describe_enchant_filter(filter: &ObjectFilter) -> String {
    let aura_creature_gate = filter.card_types.len() == 1
        && filter.card_types[0] == CardType::Creature
        && filter.subtypes.len() == 1
        && filter.subtypes[0] == crate::types::Subtype::Aura
        && filter.controller.is_none()
        && filter.owner.is_none()
        && filter.zone == Some(Zone::Battlefield);
    if aura_creature_gate {
        return "creature with another Aura attached to it".to_string();
    }
    let desc = filter.description();
    if let Some(stripped) = desc.strip_prefix("a ") {
        stripped.to_string()
    } else if let Some(stripped) = desc.strip_prefix("an ") {
        stripped.to_string()
    } else {
        desc
    }
}

fn ability_can_render_as_keyword_group(ability: &Ability) -> bool {
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            static_ability.is_keyword()
                || static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
                || static_ability.id()
                    == crate::static_abilities::StaticAbilityId::KeywordFallbackText
        }
        _ => false,
    }
}

fn describe_additional_cost_effects(effects: &[Effect]) -> String {
    if effects.len() == 1
        && let Some(choose_mode) = effects[0].downcast_ref::<crate::effects::ChooseModeEffect>()
    {
        let min = choose_mode
            .min_choose_count
            .clone()
            .unwrap_or_else(|| choose_mode.choose_count.clone());
        if choose_mode.choose_count == Value::Fixed(1) && min == Value::Fixed(1) {
            let mut options = Vec::new();
            for mode in &choose_mode.modes {
                let mut text = mode.description.trim().to_string();
                if text.is_empty() {
                    text = describe_effect_list(&mode.effects);
                }
                text = text.trim().trim_end_matches('.').to_string();
                if let Some(rest) = text.strip_prefix("you ") {
                    text = normalize_you_verb_phrase(rest);
                }
                if let Some(rest) = text.strip_prefix("pay ") {
                    let normalized_cost = normalize_cost_amount_token(rest);
                    text = format!("pay {normalized_cost}");
                }
                if text.is_empty() {
                    continue;
                }
                options.push(text);
            }
            if options.len() >= 2 {
                return join_with_or(&options);
            }
        }
    }

    describe_effect_list(effects)
}

fn describe_alternative_cost_effects(cost_effects: &[Effect]) -> String {
    if cost_effects.len() == 2
        && let Some(choose) = cost_effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(return_to_hand) =
            cost_effects[1].downcast_ref::<crate::effects::ReturnToHandEffect>()
        && let ChooseSpec::Target(inner) = &return_to_hand.spec
        && let ChooseSpec::Object(filter) = inner.as_ref()
    {
        let references_chosen = filter.tagged_constraints.len() == 1
            && filter.tagged_constraints[0].tag == choose.tag
            && filter.tagged_constraints[0].relation
                == crate::filter::TaggedOpbjectRelation::IsTaggedObject;
        if references_chosen {
            let mut described = choose.filter.clone();
            if described.zone == Some(Zone::Battlefield) {
                described.zone = None;
            }
            return format!("return {} to its owner's hand", described.description());
        }
    }

    if cost_effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some()
    }) {
        return describe_effect_list(cost_effects);
    }

    let mut clauses = Vec::new();
    for effect in cost_effects {
        if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>()
            && lose_life.player == ChooseSpec::Player(PlayerFilter::You)
        {
            clauses.push(format!("pay {} life", describe_value(&lose_life.amount)));
            continue;
        }
        if let Some(exile_hand) = effect.downcast_ref::<crate::effects::ExileFromHandAsCostEffect>()
        {
            clauses.push(describe_exile_from_hand_as_cost_phrase(exile_hand));
            continue;
        }

        let mut clause = describe_effect(effect)
            .trim()
            .trim_end_matches('.')
            .to_string();
        if let Some(rest) = clause.strip_prefix("you ") {
            clause = normalize_you_verb_phrase(rest);
        } else if let Some(rest) = clause.strip_prefix("You ") {
            clause = normalize_you_verb_phrase(rest);
        }
        if clause.is_empty() {
            continue;
        }
        clauses.push(clause);
    }

    if clauses.is_empty() {
        describe_effect_list(cost_effects)
    } else {
        join_with_and(&clauses)
    }
}

fn describe_exile_from_hand_as_cost_phrase(
    exile_hand: &crate::effects::ExileFromHandAsCostEffect,
) -> String {
    let count = exile_hand.count.max(1);
    let card_word = if count == 1 { "card" } else { "cards" };
    let amount = if count == 1 {
        "a".to_string()
    } else {
        small_number_word(count)
            .map(str::to_string)
            .unwrap_or_else(|| count.to_string())
    };
    let color_prefix = exile_hand
        .color_filter
        .map(|colors| describe_token_color_words(colors, false))
        .filter(|text| !text.is_empty())
        .map(|text| format!("{text} "))
        .unwrap_or_default();
    format!("exile {amount} {color_prefix}{card_word} from your hand")
}

fn describe_imprint_from_hand_phrase(
    imprint: &crate::effects::cards::ImprintFromHandEffect,
) -> String {
    let mut card_text = imprint.filter.description();
    if let Some((subject, zone_phrase)) = card_text.rsplit_once(" in ")
        && zone_phrase.to_ascii_lowercase().contains("hand")
    {
        card_text = format!("{subject} from {zone_phrase}");
    }
    format!("imprint, you may exile {card_text}")
}

fn describe_optional_cost_line(cost: &crate::cost::OptionalCost) -> String {
    let label = cost.label;
    let cost_text = describe_cost_list(cost.cost.costs());
    match label {
        "Replicate" => format!("Replicate—{}.", cost_text.trim_end_matches('.')),
        // Most optional-cost keywords render with a space-separated payload.
        "Kicker" | "Multikicker" | "Buyback" | "Entwine" => {
            if cost_text.trim().is_empty() {
                label.to_string()
            } else {
                format!("{label} {cost_text}")
            }
        }
        other if cost.repeatable => {
            if cost_text.trim().is_empty() {
                other.to_string()
            } else {
                format!("{other}—{}.", cost_text.trim_end_matches('.'))
            }
        }
        other => {
            if cost_text.trim().is_empty() {
                other.to_string()
            } else {
                format!("{other} {cost_text}")
            }
        }
    }
}
