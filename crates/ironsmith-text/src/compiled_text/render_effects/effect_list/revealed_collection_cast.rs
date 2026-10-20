use super::*;

/// A public top-library reveal, an optional choice from that exact set, and
/// a free cast whose selected card is excluded from the random-bottom rest.
/// The shared tags and identical dynamic cap prove collection provenance.
pub(super) fn describe_revealed_collection_free_cast(effects: &[&Effect]) -> Option<String> {
    let [look_effect, reveal_effect, choose_effect, cast_effect, rest_effect] = effects else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    let rest = structural_unwrap_render_wrappers(rest_effect)
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    let [membership] = choose.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if look.reveal
        || look.player != PlayerFilter::You
        || look.viewer != PlayerFilter::You
        || !look.count.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs)
        || reveal.tag != look.tag
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.count != crate::effect::ChoiceCount::up_to(1)
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.filter.zone != Some(Zone::Library)
        || choose.filter.excluded_card_types.as_slice() != [CardType::Land]
        || membership.tag != look.tag
        || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let Some(crate::filter::Comparison::LessThanOrEqualExpr(cap)) = &choose.filter.mana_value
    else {
        return None;
    };
    if cap.unhinted() != look.count.unhinted() {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.excluded_card_types.clear();
    filter.tagged_constraints.clear();
    filter.mana_value = None;
    if filter != ObjectFilter::default()
        || cast
            != &crate::effects::CastTaggedEffect::new(choose.tag.clone(), PlayerFilter::You)
                .without_paying_mana_cost()
        || rest
            != &crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                look.tag.clone(),
                Some(choose.tag.clone()),
                crate::effects::consult_helpers::LibraryBottomOrder::Random,
                PlayerFilter::You,
            )
    {
        return None;
    }
    let looked = describe_effect(look_effect);
    let looked = looked.trim_end_matches('.');
    let top = looked.strip_prefix("Look at ")?;
    Some(format!(
        "Reveal {top}. You may cast a spell with mana value X or less from among cards revealed this way without paying its mana cost. Put the rest on the bottom of your library in a random order"
    ))
}

/// Renders a choose-set/iterate-set free cast only when an earlier executable
/// action in this resolution produced the tagged exiled collection. The
/// selection's broad zone list is safe because tag membership still limits it
/// to cards produced by that action.
pub(super) fn describe_prior_exiled_collection_cast(
    prefix: &[&Effect],
    choose_effect: &Effect,
    for_each_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [cast_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    let constraints = choose.filter.tagged_constraints.as_slice();
    let (membership, rest) = constraints.split_first()?;
    // A cast from a captured exile collection must still name the very same
    // exile object, even if the physical card left exile and returned.
    let rest = if let Some(exact) = rest.first()
        && exact.tag == membership.tag
        && exact.relation == crate::filter::TaggedOpbjectRelation::SameObjectId
    {
        &rest[1..]
    } else {
        rest
    };
    let excluded = match rest {
        [] => None,
        [excluded] if excluded.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject => {
            Some(excluded)
        }
        _ => return None,
    };
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.count_value.is_some()
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || !choose_search_zones(choose)?.contains(&Zone::Exile)
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
    {
        return None;
    }
    // The paid reading ("You may cast a spell from among those cards") is
    // the same choice with the ordinary cost; anything else keeps its own
    // renderer.
    let free_cast = if cast
        == &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
            .without_paying_mana_cost()
    {
        true
    } else if cast == &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You) {
        false
    } else {
        return None;
    };
    // An exile action, possibly nested in a sequence or a per-player loop,
    // that tags exactly the face-up cards it moved.
    fn exiles_top_into(effect: &Effect, tag: &crate::TagKey) -> bool {
        let effect = structural_unwrap_render_wrappers(effect);
        if let Some(exile) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() {
            return !exile.face_down
                && (exile.moved_tags.contains(tag) || exile.accumulated_tags.contains(tag));
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().any(|inner| exiles_top_into(inner, tag));
        }
        if let Some(players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            return players.effects.iter().any(|inner| exiles_top_into(inner, tag));
        }
        false
    }
    let sourced_here = prefix.iter().any(|effect| {
        let nested_exile = exiles_top_into(effect, &membership.tag);
        let effect = structural_unwrap_render_wrappers(effect);
        nested_exile
            || effect
            .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            .is_some_and(|exile| {
                !exile.face_down && exile.moved_tags.contains(&membership.tag)
            })
            || effect
                .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
                .is_some_and(|consult| {
                    consult.mode == crate::effects::consult_helpers::LibraryConsultMode::Exile
                        && (consult.all_tag == membership.tag
                            || consult.match_tag == membership.tag)
                })
            || effect
                .downcast_ref::<crate::effects::ForPlayersEffect>()
                .is_some_and(|players| {
                    matches!(players.effects.as_slice(), [inner] if
                            structural_unwrap_render_wrappers(inner)
                                .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
                                .is_some_and(|consult| {
                                    consult.mode == crate::effects::consult_helpers::LibraryConsultMode::Exile
                                        && (consult.all_tag == membership.tag
                                            || (players.filter != PlayerFilter::Any
                                                && consult.match_tag == membership.tag))
                                }))
                })
    });
    // "For each player, search that player's library for a card and exile it"
    // tags exactly the searched cards that moved to exile.
    let searched_exile_source = prefix.iter().any(|effect| {
        structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::ForPlayersEffect>()
            .is_some_and(|players| {
                matches!(players.effects.as_slice(), [inner] if
                    inner
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .is_some_and(|tagged| {
                            tagged.tag == membership.tag
                                && tagged
                                    .effect
                                    .downcast_ref::<crate::effects::SearchLibraryEffect>()
                                    .is_some_and(|search| search.destination == Zone::Exile)
                        }))
            })
    });
    if !sourced_here && !searched_exile_source {
        return None;
    }
    // A consult whose stop filter is nonland already restricts its match set
    // to nonland cards ("until they exile a nonland card ... from among those
    // nonland cards"), so the choice need not repeat the land exclusion.
    let nonland_consult_match = prefix.iter().any(|effect| {
        let effect = structural_unwrap_render_wrappers(effect);
        let consult_matches = |consult: &crate::effects::ConsultTopOfLibraryEffect| {
            consult.mode == crate::effects::consult_helpers::LibraryConsultMode::Exile
                && consult.match_tag == membership.tag
                && consult.filter.excluded_card_types.as_slice() == [CardType::Land]
        };
        effect
            .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
            .is_some_and(consult_matches)
            || effect
                .downcast_ref::<crate::effects::ForPlayersEffect>()
                .is_some_and(|players| {
                    matches!(players.effects.as_slice(), [inner] if
                        structural_unwrap_render_wrappers(inner)
                            .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
                            .is_some_and(consult_matches))
                })
    });
    if let Some(excluded) = excluded {
        let prior_choice = prefix.iter().find_map(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                .filter(|prior| prior.tag == excluded.tag)
        })?;
        let [prior_membership] = prior_choice.filter.tagged_constraints.as_slice() else {
            return None;
        };
        if prior_membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            || !prefix.iter().any(|effect| {
                structural_unwrap_render_wrappers(effect)
                    .downcast_ref::<crate::effects::ForPlayersEffect>()
                    .is_some_and(|players| {
                        matches!(players.effects.as_slice(), [inner] if
                            structural_unwrap_render_wrappers(inner)
                                .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
                                .is_some_and(|consult| {
                                    consult.mode == crate::effects::consult_helpers::LibraryConsultMode::Exile
                                        && consult.all_tag == membership.tag
                                        && consult.match_tag == prior_membership.tag
                                }))
                    })
            })
        {
            return None;
        }
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.clear();
    let prior_exiled_surface = filter.prior_effect_action_surface()
        == Some(crate::effect::PriorEffectAction::Exiled);
    if prior_exiled_surface {
        filter.set_prior_effect_action_surface(None);
    }
    // "nonland" may be recorded once per lowering layer; it is one exclusion.
    filter.excluded_card_types.dedup();
    let bare_nonland_match = nonland_consult_match && filter.excluded_card_types.is_empty();
    if filter.excluded_card_types.as_slice() != [CardType::Land] && !bare_nonland_match {
        return None;
    }
    filter.excluded_card_types.clear();
    let cap = match filter.mana_value.take() {
        None => String::new(),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => {
            let value = if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs) {
                "X".to_string()
            } else {
                describe_value(&value)
            };
            format!(" with mana value {value} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqual(value)) => {
            format!(" with mana value {value} or less")
        }
        _ => return None,
    };
    // A typed pool ("an instant or sorcery spell", "a Vehicle or artifact
    // creature spell") keeps its spell characteristics on the cast noun.
    let typed_spell = if filter == ObjectFilter::default() {
        None
    } else {
        let mut typed = filter.clone();
        typed.card_types.clear();
        typed.all_card_types.clear();
        typed.subtypes.clear();
        typed.any_of.clear();
        typed.set_explicit_card_type_noun(None);
        typed.union_surface = ObjectFilter::default().union_surface;
        if typed != ObjectFilter::default() || !cap.is_empty() {
            return None;
        }
        let spell = describe_cast_spell_filter(&filter, CastSpellFilterContext::EnclosingPermission);
        if spell.contains("card") || !spell.ends_with("spell") {
            return None;
        }
        Some(spell)
    };
    let count = choose.count;
    let (number, possessive) = if count == crate::effect::ChoiceCount::any_number() {
        ("any number of".to_string(), "their")
    } else if count == crate::effect::ChoiceCount::up_to(1) {
        ("a".to_string(), "its")
    } else if count.min == 0
        && count.max.is_some_and(|max| max > 1)
        && !count.dynamic_x
        && !count.up_to_x
        && !count.random
        && !count.explicit_exactly
    {
        let max = count.max?;
        let count_word = small_number_word(u32::try_from(max).ok()?)
            .unwrap_or_else(|| max.to_string());
        (format!("up to {count_word}"), "their")
    } else {
        return None;
    };
    let aggregate_cap = match &choose.aggregate_constraint {
        None => String::new(),
        Some(constraint)
            if constraint.metric == crate::effect::ChoiceAggregateMetric::ManaValue
                && constraint.minimum.is_none() =>
        {
            let maximum = describe_value(&constraint.maximum);
            if matches!(constraint.maximum.unhinted(), Value::Fixed(_)) {
                format!(" with total mana value {maximum} or less")
            } else {
                format!(" with total mana value less than or equal to {maximum}")
            }
        }
        _ => return None,
    };
    if !aggregate_cap.is_empty() && !cap.is_empty() {
        return None;
    }
    if searched_exile_source
        && free_cast
        && typed_spell.is_none()
        && excluded.is_none()
        && count == crate::effect::ChoiceCount::any_number()
        && cap.is_empty()
        && aggregate_cap.is_empty()
    {
        return Some("You may cast those cards without paying their mana costs".to_string());
    }
    let spells = match &typed_spell {
        None if possessive == "its" => "spell".to_string(),
        None => "spells".to_string(),
        Some(spell) if possessive == "its" => spell.clone(),
        // Oracle pluralizes a two-type pool with "and/or".
        Some(spell) if spell == "instant or sorcery spell" => {
            "instant and/or sorcery spells".to_string()
        }
        Some(spell) => pluralize_cast_spell_description(spell),
    };
    let number = if number == "a"
        && spells.starts_with(|first: char| matches!(first, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an".to_string()
    } else {
        number
    };
    let costs = if possessive == "its" { "cost" } else { "costs" };
    let source = if excluded.is_some() {
        "the other cards exiled this way"
    } else if bare_nonland_match {
        "those nonland cards"
    } else if prior_exiled_surface {
        "cards exiled this way"
    } else {
        "them"
    };
    let payment = if free_cast {
        format!(" without paying {possessive} mana {costs}")
    } else {
        String::new()
    };
    Some(format!(
        "You may cast {number} {spells}{cap}{aggregate_cap} from among {source}{payment}"
    ))
}

/// "For each player, you may cast a card that player milled this way without
/// paying its mana cost" (The Ur-Sphinx): an up-to-one choice among the
/// iterated player's cards milled by the earlier per-player mill.
pub(super) fn describe_iterated_player_milled_card_cast(
    choose_effect: &Effect,
    for_each_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [cast_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    let [membership] = choose.filter.tagged_constraints.as_slice() else {
        return None;
    };
    let paid = crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You);
    let free = cast == &paid.clone().without_paying_mana_cost();
    if (!free && cast != &paid)
        || choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.count != crate::effect::ChoiceCount::up_to(1)
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::Graveyard)
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || !membership.tag.as_str().starts_with("milled_")
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
    {
        return None;
    }
    let mut residual = choose.filter.clone();
    residual.zone = None;
    residual.owner = None;
    residual.tagged_constraints.clear();
    residual.excluded_card_types.retain(|card_type| *card_type != CardType::Land);
    residual.union_surface = ObjectFilter::default().union_surface;
    if residual != ObjectFilter::default() {
        return None;
    }
    let payment = if free {
        " without paying its mana cost"
    } else {
        ""
    };
    Some(format!("Cast a card that player milled this way{payment}"))
}

/// "Copy those cards. You may cast any number of spells with total mana
/// value 6 or less from among the copies without paying their mana costs":
/// a choice from an exact exiled set whose every member is cast as a copy.
pub(super) fn describe_copied_exiled_collection_cast(
    choose_effect: &Effect,
    for_each_effect: &Effect,
) -> Option<String> {
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [cast_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    let [membership] = choose.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if choose.is_search
        || choose.chooser != PlayerFilter::You
        || choose.count != crate::effect::ChoiceCount::any_number()
        || choose.count_value.is_some()
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !choose.additional_zones.is_empty()
        || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        || !crate::cards::is_sentence_helper_tag(membership.tag.as_str(), "exiled")
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
        || cast
            != &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
                .as_copy()
                .without_paying_mana_cost()
    {
        return None;
    }
    let mut residual = choose.filter.clone();
    residual.tagged_constraints.clear();
    residual.zone = None;
    if residual != ObjectFilter::default() {
        return None;
    }
    let aggregate_cap = match &choose.aggregate_constraint {
        None => String::new(),
        Some(constraint)
            if constraint.metric == crate::effect::ChoiceAggregateMetric::ManaValue
                && constraint.minimum.is_none()
                && matches!(constraint.maximum.unhinted(), Value::Fixed(_)) =>
        {
            format!(
                " with total mana value {} or less",
                describe_value(&constraint.maximum)
            )
        }
        _ => return None,
    };
    Some(format!(
        "Copy those cards. You may cast any number of spells{aggregate_cap} from among the copies without paying their mana costs"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_retained_cast(effects: &[Effect]) -> Option<String> {
        (0..effects.len().saturating_sub(1)).find_map(|index| {
            let prefix = effects[..index].iter().collect::<Vec<_>>();
            describe_prior_exiled_collection_cast(
                &prefix,
                &effects[index],
                &effects[index + 1],
            )
        })
    }

    const ORACLE: &str = "Whenever you cast a spell from your hand, reveal the top X cards of your library, where X is that spell's mana value. You may cast a spell with mana value X or less from among cards revealed this way without paying its mana cost. Put the rest on the bottom of your library in a random order.";

    #[test]
    fn revealed_dynamic_cap_cast_retains_collection_provenance() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Revealed cast")
            .card_types(vec![CardType::Enchantment])
            .parse_text(ORACLE)
            .unwrap();
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected a triggered ability");
        };
        let effects = &triggered.effects.segments[0].default_effects;
        let window = effects
            .windows(5)
            .find(|window| {
                structural_unwrap_render_wrappers(&window[0])
                    .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
                    .is_some()
            })
            .expect("look/reveal/choice/cast/remainder sequence");
        let refs = window.iter().collect::<Vec<_>>();
        assert!(describe_revealed_collection_free_cast(&refs).is_some());
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), ORACLE);

        let mut altered = window.to_vec();
        let mut choose = structural_unwrap_render_wrappers(&altered[2])
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .unwrap()
            .clone();
        choose.filter.tagged_constraints[0].tag = crate::TagKey::from("unrelated_reveal");
        altered[2] = Effect::new(choose);
        let refs = altered.iter().collect::<Vec<_>>();
        assert!(describe_revealed_collection_free_cast(&refs).is_none());

        let mut altered = window.to_vec();
        let mut choose = structural_unwrap_render_wrappers(&altered[2])
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .unwrap()
            .clone();
        choose.filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(4));
        altered[2] = Effect::new(choose);
        let refs = altered.iter().collect::<Vec<_>>();
        assert!(describe_revealed_collection_free_cast(&refs).is_none());
    }

    #[test]
    fn exiled_top_and_consult_collections_share_the_cast_surface() {
        let combat = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Combat cast")
            .card_types(vec![CardType::Creature])
            .parse_text("Whenever this creature deals combat damage to a player, exile the top X cards of their library, where X is the amount of damage dealt. You may cast any number of spells with mana value X or less from among them without paying their mana costs.")
            .unwrap();
        let crate::ability::AbilityKind::Triggered(triggered) = &combat.abilities[0].kind else {
            panic!("expected a triggered ability");
        };
        assert_eq!(
            find_retained_cast(&triggered.effects.segments[0].default_effects).as_deref(),
            Some("You may cast any number of spells with mana value X or less from among them without paying their mana costs")
        );

        let consult = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Consult cast")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Exile cards from the top of your library until you exile cards with total mana value 4 or greater. You may cast any number of spells from among them without paying their mana costs.")
            .unwrap();
        let effects = &consult.spell_effect.as_ref().unwrap().segments[0].default_effects;
        assert_eq!(
            find_retained_cast(effects).as_deref(),
            Some("You may cast any number of spells from among them without paying their mana costs")
        );
        let isolated = effects[1..].iter().collect::<Vec<_>>();
        assert!(describe_prior_exiled_collection_cast(&[], isolated[0], isolated[1]).is_none());
    }

    #[test]
    fn delegated_consult_cast_requires_the_same_excluded_choice() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Delegated cast")
            .card_types(vec![CardType::Creature])
            .parse_text("At the beginning of your upkeep, each player exiles cards from the top of their library until they exile a nonland card. An opponent chooses a nonland card exiled this way. You may cast up to two spells from among the other cards exiled this way without paying their mana costs.")
            .unwrap();
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected a triggered ability");
        };
        let effects = &triggered.effects.segments[0].default_effects;
        assert_eq!(
            find_retained_cast(effects).as_deref(),
            Some("You may cast up to two spells from among the other cards exiled this way without paying their mana costs")
        );
        let cast_index = effects.iter().position(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                .is_some_and(|choice| choice.count == crate::effect::ChoiceCount::up_to(2))
        }).unwrap();
        assert!(describe_prior_exiled_collection_cast(
            &effects[..cast_index].iter().filter(|effect| {
                structural_unwrap_render_wrappers(effect)
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_none()
            }).collect::<Vec<_>>(),
            &effects[cast_index],
            &effects[cast_index + 1],
        ).is_none());
    }

    #[test]
    fn exiled_top_collection_retains_aggregate_mana_cap() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Aggregate cast")
            .card_types(vec![CardType::Creature])
            .parse_text("When this creature leaves the battlefield, exile the top ten cards of your library. You may cast any number of spells with total mana value 10 or less from among them without paying their mana costs.")
            .unwrap();
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected a triggered ability");
        };
        let effects = &triggered.effects.segments[0].default_effects;
        assert_eq!(
            find_retained_cast(effects).as_deref(),
            Some("You may cast any number of spells with total mana value 10 or less from among them without paying their mana costs")
        );
        let choose_index = effects.iter().position(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                .is_some_and(|choice| choice.aggregate_constraint.is_some())
        }).unwrap();
        let mut wrong = structural_unwrap_render_wrappers(&effects[choose_index])
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .unwrap()
            .clone();
        wrong.aggregate_constraint.as_mut().unwrap().metric = crate::effect::ChoiceAggregateMetric::Power;
        assert!(describe_prior_exiled_collection_cast(
            &effects[..choose_index].iter().collect::<Vec<_>>(),
            &Effect::new(wrong),
            &effects[choose_index + 1],
        ).is_none());
    }
}

/// A conditional replacement of a cast count from one revealed set. Both
/// branches must consume the same set and chosen tag; the later remainder
/// excludes exactly that chosen set by stable object identity.
pub(super) fn describe_revealed_cast_count_override(effects: &[&Effect]) -> Option<String> {
    let [look_effect, reveal_effect, conditional_effect, remainder_effect] = effects else {
        return None;
    };
    let look = structural_unwrap_render_wrappers(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let reveal = structural_unwrap_render_wrappers(reveal_effect)
        .downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if look.reveal
        || look.player != PlayerFilter::target_opponent()
        || look.viewer != PlayerFilter::You
        || reveal.tag != look.tag
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || conditional.capture_condition_result
        || remainder.tag != look.tag
        || remainder.controller_at_last_blocked_by.is_some()
    {
        return None;
    }
    let Value::Fixed(top_count) = look.count.unhinted() else {
        return None;
    };
    let top_count = u32::try_from(*top_count).ok()?;
    let Condition::ValueComparison { left, operator, right } = &conditional.condition else {
        return None;
    };
    if *operator != crate::effect::ValueComparisonOperator::GreaterThanOrEqual
        || right.unhinted() != &Value::Fixed(2)
    {
        return None;
    }
    let Value::Count(graveyard_filter) = left.unhinted() else {
        return None;
    };
    let mut graveyard_filter = graveyard_filter.clone();
    if graveyard_filter.zone != Some(Zone::Graveyard)
        || graveyard_filter.owner != Some(PlayerFilter::You)
        || graveyard_filter.card_types.as_slice() != [CardType::Instant, CardType::Sorcery]
    {
        return None;
    }
    graveyard_filter.zone = None;
    graveyard_filter.owner = None;
    graveyard_filter.card_types.clear();
    graveyard_filter.union_surface = Default::default();
    if graveyard_filter != ObjectFilter::default() {
        return None;
    }

    let cast_branch = |effects: &[Effect], expected_count| -> Option<crate::effects::ChooseObjectsEffect> {
        let [choose_effect, for_each_effect] = effects else {
            return None;
        };
        let choose = structural_unwrap_render_wrappers(choose_effect)
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let for_each = structural_unwrap_render_wrappers(for_each_effect)
            .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        let [cast_effect] = for_each.effects.as_slice() else {
            return None;
        };
        let cast = structural_unwrap_render_wrappers(cast_effect)
            .downcast_ref::<crate::effects::CastTaggedEffect>()?;
        if choose.count != crate::effect::ChoiceCount::up_to(expected_count)
            || choose.chooser != PlayerFilter::You
            || choose.zone != Some(Zone::Library)
            || choose.filter.zone != Some(Zone::Library)
            || for_each.tag != choose.tag
            || for_each.controller_at_last_blocked_by.is_some()
            || cast != &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
                .without_paying_mana_cost()
            || choose != &crate::effects::ChooseObjectsEffect::new(
                choose.filter.clone(), choose.count, PlayerFilter::You, choose.tag.clone(),
            ).in_zone(Zone::Library)
        {
            return None;
        }
        let [membership] = choose.filter.tagged_constraints.as_slice() else {
            return None;
        };
        if membership.tag != look.tag
            || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
        {
            return None;
        }
        Some(choose.clone())
    };
    let larger = cast_branch(&conditional.if_true, 2)?;
    let smaller = cast_branch(&conditional.if_false, 1)?;
    if larger.tag != smaller.tag || larger.filter != smaller.filter {
        return None;
    }
    let mut filter = larger.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.clear();
    if filter.excluded_card_types.as_slice() != [CardType::Land] {
        return None;
    }
    filter.excluded_card_types.clear();
    if filter.card_types.as_slice() == [CardType::Instant, CardType::Sorcery]
        && filter.any_of.is_empty()
    {
        filter.card_types.clear();
    } else {
        let branches = std::mem::take(&mut filter.any_of);
        if branches.len() != 2 || !filter.card_types.is_empty() {
            return None;
        }
        for (branch, card_type) in branches.iter().zip([CardType::Instant, CardType::Sorcery]) {
            let mut branch = branch.clone();
            if branch.card_types.as_slice() != [card_type] || branch.stack_kind.is_some() {
                return None;
            }
            branch.card_types.clear();
            branch.union_surface = Default::default();
            if branch != ObjectFilter::default() {
                return None;
            }
        }
    }
    filter.union_surface = Default::default();
    if filter != ObjectFilter::default() {
        return None;
    }

    let [remainder_effect] = remainder.effects.as_slice() else {
        return None;
    };
    let remainder_conditional = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let Condition::TaggedObjectMatches(iterated, remainder_filter) = &remainder_conditional.condition else {
        return None;
    };
    if iterated.as_str() != "__it__"
        || !remainder_conditional.if_true.is_empty()
        || remainder_conditional.if_false.len() != 1
        || remainder_conditional.capture_condition_result
    {
        return None;
    }
    let [membership] = remainder_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if membership.tag != larger.tag
        || membership.relation != crate::filter::TaggedOpbjectRelation::SameStableId
    {
        return None;
    }
    let mut rest_filter = remainder_filter.clone();
    rest_filter.tagged_constraints.clear();
    if rest_filter != ObjectFilter::default() {
        return None;
    }
    let moved = structural_unwrap_render_wrappers(&remainder_conditional.if_false[0])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if moved != &crate::effects::MoveToZoneEffect::new(ChooseSpec::Iterated, Zone::Graveyard, false)
        .with_remainder_surface(ironsmith_core::LibraryRemainderSurface::SentenceLeadingThenRest)
    {
        return None;
    }
    let seven = small_number_word(top_count).unwrap_or_else(|| top_count.to_string());
    Some(format!(
        "Target opponent reveals the top {seven} cards of their library. You may cast an instant or sorcery spell from among them without paying its mana cost. Then that player puts the rest into their graveyard.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may cast up to two instant and/or sorcery spells from among the revealed cards instead of one"
    ))
}

#[cfg(test)]
mod revealed_cast_count_override_tests {
    use super::*;

    const TEXT: &str = "Target opponent reveals the top seven cards of their library. You may cast an instant or sorcery spell from among them without paying its mana cost. Then that player puts the rest into their graveyard.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, you may cast up to two instant and/or sorcery spells from among the revealed cards instead of one.";

    #[test]
    fn spell_mastery_replaces_the_cast_limit_for_the_same_revealed_pool() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cast count override")
            .card_types(vec![CardType::Sorcery])
            .parse_text(TEXT)
            .expect("revealed cast count override should compile");
        let effects = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects;
        let window = effects.windows(4).find(|window| {
            structural_unwrap_render_wrappers(&window[0])
                .downcast_ref::<crate::effects::LookAtTopCardsEffect>().is_some()
        }).expect("look, reveal, exclusive branches, remainder");
        let refs = window.iter().collect::<Vec<_>>();
        assert_eq!(describe_revealed_cast_count_override(&refs).as_deref(),
            Some(TEXT.trim_end_matches('.')));
        let conditional = structural_unwrap_render_wrappers(&window[2])
            .downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
        assert_eq!(conditional.if_true.len(), 2);
        assert_eq!(conditional.if_false.len(), 2);
        assert!(crate::compiled_text::compiled_text_lines(&definition).join("\n")
            .contains("instead of one"));

        let mut wrong = window.to_vec();
        let mut conditional = structural_unwrap_render_wrappers(&wrong[2])
            .downcast_ref::<crate::effects::ConditionalEffect>().unwrap().clone();
        conditional.if_false.clear();
        wrong[2] = Effect::new(conditional);
        let refs = wrong.iter().collect::<Vec<_>>();
        assert!(describe_revealed_cast_count_override(&refs).is_none(),
            "an additive or missing default cast branch cannot claim a replacement");

        let mut wrong = window.to_vec();
        let mut remainder = structural_unwrap_render_wrappers(&wrong[3])
            .downcast_ref::<crate::effects::ForEachTaggedEffect>().unwrap().clone();
        remainder.tag = TagKey::from("different_revealed_set");
        wrong[3] = Effect::new(remainder);
        let refs = wrong.iter().collect::<Vec<_>>();
        assert!(describe_revealed_cast_count_override(&refs).is_none());
    }
}
