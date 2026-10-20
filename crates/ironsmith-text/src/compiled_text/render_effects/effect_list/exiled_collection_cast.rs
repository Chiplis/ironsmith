use super::*;

/// A common captured exile pool can grant one independent free cast for
/// every nonland card type. The separate grants are the executable budgets;
/// this folds only their repeated wording after proving all typed grants.
pub(super) fn describe_per_card_type_exiled_cast(effects: &[Effect]) -> Option<String> {
    let [exile_effect, land_effect, grants @ ..] = effects else { return None; };
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let [pool] = exile.moved_tags.as_slice() else { return None; };
    let Value::Fixed(count) = exile.count else { return None; };
    if count < 1 || *exile != crate::effects::ExileTopOfLibraryEffect::new(
        Value::Fixed(count), PlayerFilter::You,
    ).tag_moved(pool.clone()) { return None; }

    let may = land_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_effect, for_each_effect] = may.effects.as_slice() else { return None; };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_land_filter = ObjectFilter::default()
        .with_type(CardType::Land)
        .in_zone(Zone::Exile)
        .match_tagged(pool.clone(), crate::filter::TaggedOpbjectRelation::IsTaggedObject);
    let expected_choose = crate::effects::ChooseObjectsEffect::new(
        expected_land_filter, crate::effect::ChoiceCount::exactly(1),
        PlayerFilter::You, choose.tag.clone(),
    ).in_zone(Zone::Exile);
    if *choose != expected_choose { return None; }
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let expected_move = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Iterated, Zone::Battlefield, false,
    );
    let [move_effect] = for_each.effects.as_slice() else { return None; };
    let moved = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
        || *moved != expected_move
        || may.decider != Some(PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || may.pay_as_cost
    { return None; }

    const TYPES: [CardType; 14] = [
        CardType::Creature, CardType::Artifact, CardType::Enchantment,
        CardType::Planeswalker, CardType::Instant, CardType::Sorcery,
        CardType::Battle, CardType::Kindred, CardType::Plane,
        CardType::Phenomenon, CardType::Vanguard, CardType::Scheme,
        CardType::Conspiracy, CardType::Dungeon,
    ];
    if grants.len() != TYPES.len() * 2 { return None; }
    for (pair, card_type) in grants.chunks_exact(2).zip(TYPES) {
        let grant = pair[0].downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
        let free = pair[1].downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()?;
        let mut expected = crate::effects::GrantPlayTaggedEffect::new(
            pool.clone(), PlayerFilter::You,
            crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
            false, false,
        ).with_max_plays(Some(1)).cast_pool_is_plural(true);
        expected.spell_filter = Some(ObjectFilter::default().with_type(card_type));
        if *grant != expected
            || *free != crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                pool.clone(), PlayerFilter::You,
            )
        { return None; }
    }
    let count = small_number_word(u32::try_from(count).ok()?)
        .unwrap_or_else(|| count.to_string());
    Some(format!(
        "Exile the top {count} cards of your library. You may put a land card from among them onto the battlefield. Until end of turn, for each nonland card type, you may cast a spell of that type from among the exiled cards without paying its mana cost"
    ))
}

/// An optional one-card exile can repeat until declined. A later total-value
/// gate and free-cast selector consume exactly the accumulated exile tag.
pub(super) fn describe_repeated_exile_capped_cast(effects: &[Effect]) -> Option<String> {
    let [shuffle_effect, repeat_effect, conditional_effect] = effects else { return None; };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if *shuffle != crate::effects::ShuffleLibraryEffect::you() { return None; }
    let repeat = repeat_effect.downcast_ref::<crate::effects::RepeatProcessEffect>()?;
    let [with_id_effect] = repeat.effects.as_slice() else { return None; };
    let with_id = with_id_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    if repeat.condition != with_id.id
        || repeat.predicate != crate::effect::EffectPredicate::Happened
        || !repeat.choice_history.is_empty()
        || repeat.max_iterations.is_some()
    { return None; }
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [exile_effect] = may.effects.as_slice() else { return None; };
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let ([moved_tag], [accumulated_tag]) =
        (exile.moved_tags.as_slice(), exile.accumulated_tags.as_slice()) else { return None; };
    if may.decider != Some(PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || may.pay_as_cost
        || !crate::cards::is_sentence_helper_tag(moved_tag.as_str(), "exiled")
        || exile.player != PlayerFilter::You
        || exile.count != Value::Fixed(1)
        || exile.surface.is_some()
        || exile.face_down
    { return None; }
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::ValueComparison { left, operator, right } = &conditional.condition else {
        return None;
    };
    let Value::TotalManaValue(total_filter) = left else { return None; };
    let (crate::effect::ValueComparisonOperator::LessThanOrEqual, Value::Fixed(cap)) =
        (operator, right) else { return None; };
    let expected_total = ObjectFilter::tagged(accumulated_tag.clone());
    if total_filter != &expected_total
        || *cap < 0
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || conditional.capture_condition_result
        || !conditional.if_false.is_empty()
    { return None; }
    let [choose_effect, for_each_effect] = conditional.if_true.as_slice() else { return None; };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_choose = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::tagged(accumulated_tag.clone())
            .without_type(CardType::Land)
            .in_zone(Zone::Exile)
            .match_tagged(
                accumulated_tag.clone(),
                crate::filter::TaggedOpbjectRelation::SameObjectId,
            ),
        crate::effect::ChoiceCount::any_number(), PlayerFilter::You, choose.tag.clone(),
    ).in_zone(Zone::Exile);
    if *choose != expected_choose { return None; }
    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [cast_effect] = for_each.effects.as_slice() else { return None; };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
        || *cast != crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
            .without_paying_mana_cost()
    { return None; }
    Some(format!(
        "Shuffle your library. As many times as you choose, you may exile the top card of your library. If the total mana value of the cards exiled this way is {cap} or less, you may cast any number of spells from among those cards without paying their mana costs"
    ))
}

/// A source-linked exile pool is a persistent collection, not the preceding
/// resolution's temporary exile result. The typed source surface retains the
/// authored reference; the SourceExiled membership proves its runtime scope.
pub(super) fn describe_source_linked_collection_cast(effects: &[&Effect]) -> Option<String> {
    let [choose_effect, for_each_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let surface = choose.collection_source_surface.as_ref()?;
    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [cast_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if choose.is_search
        || choose.optional_search_zones
        || choose.search_participant_tag.is_some()
        || choose.chooser != PlayerFilter::You
        || choose.count_value.is_some()
        || choose.description != "Choose"
        || choose.reveal
        || choose.reveal_is_presentation_only != Some(false)
        || choose.search_mode != crate::effect::SearchSelectionMode::Exact
        || choose.search_reveal_reference_surface.is_some()
        || choose.search_result_reference_surface.is_some()
        || choose.search_top_in_any_order_surface.is_some()
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || choose.remember_as_chosen_object
        || choose.public_selection
        || choose.zone != Some(Zone::Exile)
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::Exile)
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
    {
        return None;
    }
    let paid_cast = crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You);
    // "..., and mana of any type can be spent to cast that spell" (Summon:
    // Esper Valigarmanda) is the cast's own spending rider.
    let any_type_mana =
        cast.mana_spend_mode == ironsmith_core::value_model::ManaSpendMode::AnyType;
    let mut normalized_cast = cast.clone();
    normalized_cast.mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::Normal;
    let free = normalized_cast == paid_cast.clone().without_paying_mana_cost();
    if !free && normalized_cast != paid_cast {
        return None;
    }
    let [membership] = choose.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if membership.tag.as_str() != "__source_exiled__"
        || membership.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.tagged_constraints.clear();
    let owned = match filter.owner.take() {
        None => false,
        Some(PlayerFilter::You) => true,
        _ => return None,
    };
    filter.excluded_card_types.dedup();
    if filter.excluded_card_types.as_slice() != [CardType::Land] {
        return None;
    }
    filter.excluded_card_types.clear();
    // An alternative pool ("a Hero or noncreature spell") is described as
    // one cast noun; its branches carry no extra choice semantics.
    let alternative_spell = if filter.any_of.is_empty() {
        None
    } else if !surface.from_among {
        // "You may cast an instant or sorcery card exiled with this Saga"
        // (Summon: Esper Valigarmanda): the imperative form keeps the card
        // noun of the alternative pool.
        if filter.mana_value.is_some() {
            return None;
        }
        let description = filter.description();
        let card = strip_indefinite_article(&description).trim().to_string();
        if !card.ends_with(" card") {
            return None;
        }
        filter.any_of.clear();
        filter.union_surface = ObjectFilter::default().union_surface;
        Some(card)
    } else {
        if filter.mana_value.is_some() {
            return None;
        }
        let mut spell =
            describe_cast_spell_filter(&filter, CastSpellFilterContext::EnclosingPermission);
        if spell.contains("card") || !spell.ends_with("spell") {
            // "a Hero or noncreature spell" (Scarlet Witch, Chaotic
            // Avenger): describe each alternative as a spell noun and share
            // the trailing "spell".
            let mut residual = filter.clone();
            residual.any_of.clear();
            residual.union_surface = ObjectFilter::default().union_surface;
            if residual != ObjectFilter::default() {
                return None;
            }
            let branches = filter
                .any_of
                .iter()
                .map(|branch| {
                    let text = describe_cast_spell_filter(
                        branch,
                        CastSpellFilterContext::EnclosingPermission,
                    );
                    text.strip_suffix(" spell")
                        .filter(|noun| !noun.is_empty() && !noun.contains("card"))
                        .map(str::to_string)
                })
                .collect::<Option<Vec<_>>>()?;
            if branches.len() < 2 {
                return None;
            }
            spell = format!("{} spell", branches.join(" or "));
        }
        filter.any_of.clear();
        filter.union_surface = ObjectFilter::default().union_surface;
        Some(spell)
    };
    let card_types = std::mem::take(&mut filter.card_types);
    let subtypes = std::mem::take(&mut filter.subtypes);
    let generic_spell = card_types.is_empty() && subtypes.is_empty();
    let mana_cap = match filter.mana_value.take() {
        None => String::new(),
        Some(crate::filter::Comparison::LessThanOrEqual(value)) =>
            format!(" with mana value {value} or less"),
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) =>
            format!(" with mana value {} or less", describe_value(&value)),
        Some(crate::filter::Comparison::Equal(value)) =>
            format!(" with mana value {value}"),
        Some(crate::filter::Comparison::EqualExpr(value)) =>
            format!(" with mana value {}", describe_value(&value)),
        _ => return None,
    };
    if filter != ObjectFilter::default() {
        return None;
    }
    let aggregate_cap = match &choose.aggregate_constraint {
        None => String::new(),
        Some(constraint) if constraint.metric == crate::effect::ChoiceAggregateMetric::ManaValue
            && constraint.minimum.is_none() => {
            let maximum = describe_value(&constraint.maximum);
            if matches!(constraint.maximum.unhinted(), Value::Fixed(_) | Value::X) {
                format!(" with total mana value {maximum} or less")
            } else {
                format!(" with total mana value less than or equal to {maximum}")
            }
        }
        _ => return None,
    };
    if !mana_cap.is_empty() && !aggregate_cap.is_empty() {
        return None;
    }
    let count = choose.count;
    let (quantifier, plural) = if count == crate::effect::ChoiceCount::any_number() {
        ("any number of".to_string(), true)
    } else if count == crate::effect::ChoiceCount::up_to(1) {
        ("a".to_string(), false)
    } else if count.min == 0 && !count.dynamic_x && !count.up_to_x
        && !count.random && !count.explicit_exactly && count.max.is_some_and(|max| max > 1) {
        let maximum = count.max?;
        (format!("up to {}", small_number_word(u32::try_from(maximum).ok()?)
            .unwrap_or_else(|| maximum.to_string())), true)
    } else {
        return None;
    };
    let source = surface.source.display_text();
    let cards = if owned { "cards you own exiled" } else { "cards exiled" };
    let costs = if plural { "their mana costs" } else { "its mana cost" };
    let payment = if free {
        format!(" without paying {costs}")
    } else {
        String::new()
    };
    if surface.from_among {
        // Without its own "may", the optional choice belongs to an
        // enclosing "you may" wrapper that renders this as its body.
        let verb = if surface.may_cast { "You may cast" } else { "Cast" };
        let spell = match &alternative_spell {
            Some(spell) if plural => pluralize_cast_spell_description(spell),
            Some(spell) => spell.clone(),
            None => spell_subject(generic_spell, &card_types, &subtypes, plural)?,
        };
        let quantified = if quantifier == "a" {
            with_indefinite_article(&spell)
        } else {
            format!("{quantifier} {spell}")
        };
        let mana_rider = if !any_type_mana {
            String::new()
        } else if plural {
            ", and mana of any type can be spent to cast those spells".to_string()
        } else {
            ", and mana of any type can be spent to cast that spell".to_string()
        };
        Some(format!(
            "{verb} {quantified}{mana_cap} from among {cards} with {source}{aggregate_cap}{payment}{mana_rider}"
        ))
    } else {
        if let Some(card) = &alternative_spell {
            if !mana_cap.is_empty()
                || !aggregate_cap.is_empty()
                || owned
                || !generic_spell
                || plural
                || quantifier != "a"
            {
                return None;
            }
            let verb = if surface.may_cast { "You may cast" } else { "Cast" };
            let mana_rider = if any_type_mana {
                ", and mana of any type can be spent to cast that spell"
            } else {
                ""
            };
            return Some(format!(
                "{verb} {} exiled with {source}{payment}{mana_rider}",
                with_indefinite_article(card)
            ));
        }
        if !mana_cap.is_empty()
            || !aggregate_cap.is_empty()
            || owned
            || !generic_spell
            || !free
            || any_type_mana
        {
            return None;
        }
        let verb = if surface.may_cast { "You may cast" } else { "Cast" };
        let card = if plural { "cards" } else { "card" };
        Some(format!("{verb} {quantifier} {card} exiled with {source} without paying {costs}"))
    }
}

#[cfg(test)]
mod source_linked_cast_tests {
    use super::*;

    fn linked_cast(
        source_noun: &str,
        from_among: bool,
        may_cast: bool,
        aggregate: bool,
    ) -> Vec<Effect> {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.excluded_card_types.push(CardType::Land);
        filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
            tag: crate::tag::TagKey::from("__source_exiled__"),
            relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
        });
        let mut choose = crate::effects::ChooseObjectsEffect::new(
            filter,
            crate::effect::ChoiceCount::any_number(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Exile)
        .with_collection_source_surface(ironsmith_core::effect::SourceLinkedCollectionCastSurface {
            source: crate::target::SourceReferenceSurface::ThisPermanentType(source_noun.to_string()),
            from_among,
            may_cast,
        });
        if aggregate {
            choose = choose.with_aggregate_constraint(
                crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(Value::X),
            );
        }
        vec![
            Effect::new(choose),
            Effect::new(crate::effects::ForEachTaggedEffect::new(
                "chosen",
                vec![Effect::new(
                    crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
                        .without_paying_mana_cost(),
                )],
            )),
        ]
    }

    #[test]
    fn linked_source_and_aggregate_cap_survive_cast_compaction() {
        let effects = linked_cast("this artifact", true, true, true);
        let refs = effects.iter().collect::<Vec<_>>();
        assert_eq!(
            describe_source_linked_collection_cast(&refs).as_deref(),
            Some("You may cast any number of spells from among cards exiled with this artifact with total mana value X or less without paying their mana costs")
        );
        let mut wrong = effects.clone();
        let mut choice = wrong[0].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap().clone();
        choice.filter.tagged_constraints[0].tag = crate::tag::TagKey::from("unrelated_exile");
        wrong[0] = Effect::new(choice);
        let refs = wrong.iter().collect::<Vec<_>>();
        assert!(describe_source_linked_collection_cast(&refs).is_none());
    }

    #[test]
    fn imperative_linked_cast_keeps_card_noun() {
        let effects = linked_cast("this creature", false, false, false);
        let refs = effects.iter().collect::<Vec<_>>();
        assert_eq!(
            describe_source_linked_collection_cast(&refs).as_deref(),
            Some("Cast any number of cards exiled with this creature without paying their mana costs")
        );
    }

    #[test]
    fn linked_cast_does_not_hide_persistent_or_participant_choice_semantics() {
        let effects = linked_cast("this artifact", true, true, false);
        for change in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let mut changed = effects.clone();
            let mut choice = changed[0]
                .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                .unwrap()
                .clone();
            choice.remember_as_chosen_object = change.0;
            choice.optional_search_zones = change.1;
            if change.2 {
                choice.search_participant_tag = Some(crate::tag::TagKey::from("searching_player"));
            }
            changed[0] = Effect::new(choice);
            let refs = changed.iter().collect::<Vec<_>>();
            assert!(describe_source_linked_collection_cast(&refs).is_none());
        }
    }
}

fn sentence_helper_membership(
    filter: &ObjectFilter,
) -> Option<(&crate::tag::TagKey, &crate::tag::TagKey)> {
    let [exiled, selected] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    if exiled.relation != crate::target::TaggedOpbjectRelation::IsTaggedObject
        || selected.relation != crate::target::TaggedOpbjectRelation::IsNotTaggedObject
        || !crate::cards::is_sentence_helper_tag(exiled.tag.as_str(), "exiled")
        || !crate::cards::is_sentence_helper_tag(
            selected.tag.as_str(),
            "cast_from_exiled_collection",
        )
    {
        return None;
    }
    Some((&exiled.tag, &selected.tag))
}

fn exact_cast_choice_filter(
    filter: &ObjectFilter,
) -> Option<(bool, Vec<CardType>, Vec<crate::types::Subtype>, String)> {
    if filter.zone != Some(Zone::Exile) {
        return None;
    }
    let [membership] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    if membership.relation != crate::target::TaggedOpbjectRelation::IsTaggedObject
        || !crate::cards::is_sentence_helper_tag(membership.tag.as_str(), "exiled")
    {
        return None;
    }

    let card_types = if filter.any_of.is_empty() {
        filter.card_types.clone()
    } else {
        let [instant, sorcery] = filter.any_of.as_slice() else {
            return None;
        };
        let exact_plain_type_branch = |branch: &ObjectFilter, expected: CardType| {
            if branch.card_types.as_slice() != [expected] {
                return false;
            }
            let mut residual = branch.clone();
            residual.card_types.clear();
            residual.set_explicit_card_type_noun(None);
            residual == ObjectFilter::default()
        };
        if !filter.card_types.is_empty()
            || !exact_plain_type_branch(instant, CardType::Instant)
            || !exact_plain_type_branch(sorcery, CardType::Sorcery)
        {
            return None;
        }
        vec![CardType::Instant, CardType::Sorcery]
    };

    let generic_spell = card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_card_types.as_slice() == [CardType::Land];
    if !generic_spell && !filter.excluded_card_types.is_empty() {
        return None;
    }
    if card_types.is_empty() && filter.subtypes.is_empty() && !generic_spell {
        return None;
    }

    let cap = match &filter.mana_value {
        None => String::new(),
        Some(crate::filter::Comparison::LessThanOrEqual(value)) => {
            format!(" with mana value {value} or less")
        }
        Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) => {
            format!(" with mana value {} or less", describe_value(value))
        }
        _ => return None,
    };

    let mut residual = filter.clone();
    residual.zone = None;
    residual.card_types.clear();
    residual.subtypes.clear();
    residual.mana_value = None;
    residual.tagged_constraints.clear();
    residual.any_of.clear();
    if generic_spell {
        residual.excluded_card_types.clear();
    }
    if residual != ObjectFilter::default() {
        return None;
    }

    Some((generic_spell, card_types, filter.subtypes.clone(), cap))
}

fn spell_subject(
    generic_spell: bool,
    card_types: &[CardType],
    subtypes: &[crate::types::Subtype],
    plural: bool,
) -> Option<String> {
    if generic_spell {
        return Some(if plural { "spells" } else { "spell" }.to_string());
    }
    if !card_types.is_empty() && !subtypes.is_empty() {
        return None;
    }
    if !card_types.is_empty() {
        let names = card_types
            .iter()
            .map(|card_type| card_type.name())
            .collect::<Vec<_>>();
        let joined = match names.as_slice() {
            [one] => (*one).to_string(),
            [first, second] => format!("{first} {} {second}", if plural { "and" } else { "or" }),
            _ => return None,
        };
        return Some(format!("{joined} spell{}", if plural { "s" } else { "" }));
    }
    let [subtype] = subtypes else {
        return None;
    };
    Some(format!("{subtype} spell{}", if plural { "s" } else { "" }))
}

fn with_indefinite_article(subject: &str) -> String {
    let article = if subject
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        "an"
    } else {
        "a"
    };
    format!("{article} {subject}")
}

/// Restores the optional collective cast instruction from the executable
/// choose-set/iterate-set representation. The helper requires the exact
/// sentence-helper exile membership and selected-set tag chain, so unrelated
/// choose-and-cast procedures cannot acquire the "from among them" surface.
pub(super) fn describe_exiled_collection_cast_choice(effects: &[Effect]) -> Option<String> {
    let effects = if let [effect] = effects
        && let Some(sequence) = structural_unwrap_render_wrappers(effect)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
            return None;
        }
        sequence.effects.as_slice()
    } else {
        effects
    };
    let [choose_effect, for_each_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.chooser != PlayerFilter::You
        || choose.zone != Some(Zone::Exile)
        || !choose.additional_zones.is_empty()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.replace_tagged_objects
        || !crate::cards::is_sentence_helper_tag(choose.tag.as_str(), "cast_from_exiled_collection")
    {
        return None;
    }
    let (generic_spell, card_types, subtypes, cap) = exact_cast_choice_filter(&choose.filter)?;

    let for_each = structural_unwrap_render_wrappers(for_each_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [cast_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>().filter(|permission| permission.alternative_cost.is_none())?;
    if for_each.tag != choose.tag
        || cast.tag.as_str() != "__it__"
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.cost_reduction.is_some()
    {
        return None;
    }

    let count = choose.count;
    if count.dynamic_x || count.up_to_x || count.random || count.explicit_exactly {
        return None;
    }
    let (selection, plural) = match (count.min, count.max) {
        (0, None) if generic_spell => (
            format!(
                "any number of {}",
                spell_subject(true, &card_types, &subtypes, true)?
            ),
            true,
        ),
        (0, None) => (spell_subject(false, &card_types, &subtypes, true)?, true),
        (0, Some(1)) => (
            with_indefinite_article(&spell_subject(
                generic_spell,
                &card_types,
                &subtypes,
                false,
            )?),
            false,
        ),
        (0, Some(maximum)) if maximum > 1 => (
            format!(
                "up to {} {}",
                number_word(maximum as i32).unwrap_or_else(|| maximum.to_string()),
                spell_subject(generic_spell, &card_types, &subtypes, true)?
            ),
            true,
        ),
        _ => return None,
    };

    Some(format!(
        "You may cast {selection}{cap} from among them without paying {} mana cost{}",
        if plural { "their" } else { "its" },
        if plural { "s" } else { "" }
    ))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RemainingKind {
    All,
    InstantOrSorcery,
}

fn remaining_filter_kind(
    filter: &ObjectFilter,
) -> Option<(RemainingKind, crate::tag::TagKey, crate::tag::TagKey)> {
    if filter.zone != Some(Zone::Exile) {
        return None;
    }
    let (exiled_tag, selected_tag) = sentence_helper_membership(filter)?;
    let kind = match filter.card_types.as_slice() {
        [] => RemainingKind::All,
        [CardType::Instant, CardType::Sorcery] => RemainingKind::InstantOrSorcery,
        _ => return None,
    };
    let mut residual = filter.clone();
    residual.zone = None;
    residual.card_types.clear();
    residual.tagged_constraints.clear();
    if kind == RemainingKind::InstantOrSorcery {
        let has_any_typed_noun_surface = residual.has_conjunctive_set_surface()
            || residual.has_explicit_card_noun()
            || residual.explicit_card_type_noun().is_some();
        if has_any_typed_noun_surface
            && (!residual.has_conjunctive_set_surface()
                || !residual.has_explicit_card_noun()
                || residual.explicit_card_type_noun() != Some(CardType::Sorcery))
        {
            return None;
        }
        residual.set_conjunctive_set_surface(false);
        residual.set_explicit_card_noun(false);
        residual.set_explicit_card_type_noun(None);
    }
    if residual != ObjectFilter::default() {
        return None;
    }
    Some((kind, exiled_tag.clone(), selected_tag.clone()))
}

fn exact_remaining_move(
    effect: &Effect,
    zone: Zone,
    random_bottom: bool,
) -> Option<(RemainingKind, crate::tag::TagKey, crate::tag::TagKey)> {
    let moved = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::All(filter) = moved.target.base() else {
        return None;
    };
    let expected = crate::effects::MoveToZoneEffect::new(moved.target.clone(), zone, false)
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .with_target_plural_surface()
        .with_destination_player_surface(PlayerFilter::You);
    let expected = if random_bottom {
        expected.with_library_order(crate::effects::LibraryPlacementOrder::Random)
    } else {
        expected
    };
    if moved != &expected {
        return None;
    }
    remaining_filter_kind(filter)
}

/// Renders the exact complement of the selected cast set. Source-leading
/// `Then` provenance distinguishes the terse bare-rest form from the explicit
/// non-cast complement, while the two-move shape proves Muse-style typed/rest
/// partitioning without consulting source text or card identity.
fn exiled_collection_partition_view(
    effects: &[Effect],
) -> Option<(String, crate::tag::TagKey, crate::tag::TagKey)> {
    let (effects, leading_then) = if let [effect] = effects {
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen {
                return None;
            }
            (sequence.effects.as_slice(), true)
        } else {
            (effects, false)
        }
    } else {
        (effects, false)
    };

    if let [move_effect] = effects {
        if let Some((RemainingKind::All, exiled_tag, selected_tag)) =
            exact_remaining_move(move_effect, Zone::Graveyard, false)
        {
            return leading_then.then(|| {
                (
                    "Then put all cards exiled this way that weren't cast into your graveyard"
                        .to_string(),
                    exiled_tag,
                    selected_tag,
                )
            });
        }
        let (kind, exiled_tag, selected_tag) =
            exact_remaining_move(move_effect, Zone::Library, true)?;
        if kind != RemainingKind::All {
            return None;
        }
        return Some((
            if leading_then {
                "Then put the rest on the bottom of your library in a random order".to_string()
            } else {
                "Put the exiled cards not cast this way on the bottom of your library in a random order"
                    .to_string()
            },
            exiled_tag,
            selected_tag,
        ));
    }

    let [hand_effect, bottom_effect] = effects else {
        return None;
    };
    let (hand_kind, hand_exiled, hand_selected) =
        exact_remaining_move(hand_effect, Zone::Hand, false)?;
    let (bottom_kind, bottom_exiled, bottom_selected) =
        exact_remaining_move(bottom_effect, Zone::Library, true)?;
    if !leading_then
        || hand_kind != RemainingKind::InstantOrSorcery
        || bottom_kind != RemainingKind::All
        || hand_exiled != bottom_exiled
        || hand_selected != bottom_selected
    {
        return None;
    }
    Some((
        "Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order"
            .to_string(),
        hand_exiled,
        hand_selected,
    ))
}

pub(super) fn describe_exiled_collection_partition(effects: &[Effect]) -> Option<String> {
    exiled_collection_partition_view(effects).map(|(surface, _, _)| surface)
}

/// Rejoins the migrated flat executable form of an authored exile/collective
/// cast/remainder program. Every boundary is proved by the sentence-helper
/// tags: the exile producer feeds the choice, the choice feeds the iterator,
/// and the remainder excludes that exact selected set.
pub(super) fn describe_exiled_collection_program(effects: &[Effect]) -> Option<String> {
    let [exile_effect, choose_effect, _for_each_effect, tail @ ..] = effects else {
        return None;
    };
    if tail.len() > 2 {
        return None;
    }

    let exile = structural_unwrap_render_wrappers(exile_effect)
        .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let [exiled_tag] = exile.moved_tags.as_slice() else {
        return None;
    };
    if exile.face_down || !exile.accumulated_tags.is_empty() {
        return None;
    }

    let choice = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let [choice_membership] = choice.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if choice_membership.relation != crate::target::TaggedOpbjectRelation::IsTaggedObject
        || choice_membership.tag != *exiled_tag
    {
        return None;
    }
    let cast_surface = describe_exiled_collection_cast_choice(&effects[1..3])?;

    let mut parts = vec![
        describe_effect(exile_effect)
            .trim()
            .trim_end_matches('.')
            .to_string(),
        cast_surface,
    ];
    if !tail.is_empty() {
        let (mut remainder_surface, remainder_exiled, remainder_selected) = if let Some(view) =
            exiled_collection_partition_view(tail)
        {
            view
        } else {
            match tail {
                [remainder] => {
                    let (kind, remainder_exiled, remainder_selected) =
                        exact_remaining_move(remainder, Zone::Graveyard, false)?;
                    if kind != RemainingKind::All {
                        return None;
                    }
                    (
                        "Then put all cards exiled this way that weren't cast into your graveyard"
                            .to_string(),
                        remainder_exiled,
                        remainder_selected,
                    )
                }
                [hand_effect, bottom_effect] => {
                    let (hand_kind, hand_exiled, hand_selected) =
                        exact_remaining_move(hand_effect, Zone::Hand, false)?;
                    let (bottom_kind, bottom_exiled, bottom_selected) =
                        exact_remaining_move(bottom_effect, Zone::Library, true)?;
                    if hand_kind != RemainingKind::InstantOrSorcery
                        || bottom_kind != RemainingKind::All
                        || hand_exiled != bottom_exiled
                        || hand_selected != bottom_selected
                    {
                        return None;
                    }
                    (
                        "Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order"
                            .to_string(),
                        hand_exiled,
                        hand_selected,
                    )
                }
                _ => return None,
            }
        };
        if remainder_exiled != *exiled_tag || remainder_selected != choice.tag {
            return None;
        }
        // With a single optional selection, the exact tagged complement is
        // naturally "the rest". Multi-card casting keeps the explicit set.
        if choice.count.max == Some(1)
            && !choice.count.dynamic_x
            && remainder_surface
                == "Put the exiled cards not cast this way on the bottom of your library in a random order"
        {
            remainder_surface =
                "Then put the rest on the bottom of your library in a random order".to_string();
        }
        parts.push(remainder_surface);
    }
    Some(parts.join(". "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collection_filter(
        card_types: Vec<CardType>,
        subtypes: Vec<crate::types::Subtype>,
        count: crate::effect::ChoiceCount,
        with_cap: bool,
    ) -> Vec<Effect> {
        let exiled_tag = crate::tag::TagKey::from("__sentence_helper_exiled_l0_s0_e10");
        let chosen_tag =
            crate::tag::TagKey::from("__sentence_helper_cast_from_exiled_collection_l0_s11_e30");
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.card_types = card_types;
        filter.subtypes = subtypes;
        if filter.card_types.is_empty() && filter.subtypes.is_empty() {
            filter.excluded_card_types.push(CardType::Land);
        }
        if with_cap {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(3));
        }
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: exiled_tag,
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        vec![
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    filter,
                    count,
                    PlayerFilter::You,
                    chosen_tag.clone(),
                )
                .in_zone(Zone::Exile),
            ),
            Effect::new(crate::effects::ForEachTaggedEffect::new(
                chosen_tag,
                vec![Effect::new(
                    crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
                        .without_paying_mana_cost(),
                )],
            )),
        ]
    }

    #[test]
    fn exiled_collection_cast_choice_preserves_count_and_spell_subject() {
        let any_spells = collection_filter(
            vec![],
            vec![],
            crate::effect::ChoiceCount::any_number(),
            true,
        );
        assert_eq!(
            describe_exiled_collection_cast_choice(&any_spells).as_deref(),
            Some(
                "You may cast any number of spells with mana value 3 or less from among them without paying their mana costs"
            )
        );

        let two_sorceries = collection_filter(
            vec![CardType::Sorcery],
            vec![],
            crate::effect::ChoiceCount::up_to(2),
            true,
        );
        assert_eq!(
            describe_exiled_collection_cast_choice(&two_sorceries).as_deref(),
            Some(
                "You may cast up to two sorcery spells with mana value 3 or less from among them without paying their mana costs"
            )
        );

        let one_instant_or_sorcery = collection_filter(
            vec![CardType::Instant, CardType::Sorcery],
            vec![],
            crate::effect::ChoiceCount::up_to(1),
            true,
        );
        assert_eq!(
            describe_exiled_collection_cast_choice(&one_instant_or_sorcery).as_deref(),
            Some(
                "You may cast an instant or sorcery spell with mana value 3 or less from among them without paying its mana cost"
            )
        );

        let one_aura = collection_filter(
            vec![],
            vec![crate::types::Subtype::Aura],
            crate::effect::ChoiceCount::up_to(1),
            false,
        );
        assert_eq!(
            describe_exiled_collection_cast_choice(&one_aura).as_deref(),
            Some("You may cast an Aura spell from among them without paying its mana cost")
        );
    }

    #[test]
    fn coordinated_type_union_collection_cast_preserves_global_any_number_scope() {
        let mut effects = collection_filter(
            vec![],
            vec![],
            crate::effect::ChoiceCount::any_number(),
            true,
        );
        let choose = effects[0]
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .expect("fixture should start with a choice")
            .clone();
        let mut union_filter = choose.filter.clone();
        union_filter.excluded_card_types.clear();
        let mut instant = ObjectFilter::default();
        instant.card_types.push(CardType::Instant);
        instant.set_explicit_card_type_noun(Some(CardType::Instant));
        let mut sorcery = ObjectFilter::default();
        sorcery.card_types.push(CardType::Sorcery);
        sorcery.set_explicit_card_type_noun(Some(CardType::Sorcery));
        union_filter.any_of = vec![instant, sorcery];

        let mut mismatched_filter = union_filter.clone();
        mismatched_filter.any_of[1].tapped = true;
        let mismatched = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    mismatched_filter,
                    choose.count,
                    choose.chooser.clone(),
                    choose.tag.clone(),
                )
                .in_zone(Zone::Exile),
            ),
            effects[1].clone(),
        ]));
        assert!(describe_exiled_collection_cast_choice(&[mismatched]).is_none());

        effects[0] = Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                union_filter,
                choose.count,
                choose.chooser.clone(),
                choose.tag.clone(),
            )
            .in_zone(Zone::Exile),
        );
        let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(effects));

        assert_eq!(
            describe_exiled_collection_cast_choice(&[coordinated]).as_deref(),
            Some(
                "You may cast instant and sorcery spells with mana value 3 or less from among them without paying their mana costs"
            )
        );
    }

    #[test]
    fn parsed_collection_programs_reach_the_exact_specialist_surfaces() {
        let cases = [
            (
                "Epic Collection Probe",
                "Exile the top X cards of your library. You may cast instant and sorcery spells with mana value X or less from among them without paying their mana costs. Then put all cards exiled this way that weren't cast into your graveyard.",
            ),
            (
                "Collected Collection Probe",
                "Exile the top six cards of your library. You may cast up to two sorcery spells with mana value 3 or less from among them without paying their mana costs. Put the exiled cards not cast this way on the bottom of your library in a random order.",
            ),
            (
                "Muse Collection Probe",
                "Exile the top X cards of your library. You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost. Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order.",
            ),
            (
                "Villainous Collection Probe",
                "Target opponent exiles the top X cards of their library. You may cast any number of spells with mana value X or less from among them without paying their mana costs.",
            ),
        ];

        for (name, oracle) in cases {
            let definition = crate::compiler_test_support::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                name,
            )
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .expect("typed exile/cast/complement collection should compile");

            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition),
                vec![oracle],
                "{name}",
            );
        }
    }

    fn remaining_move(card_types: Vec<CardType>, zone: Zone, random_bottom: bool) -> Effect {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.card_types = card_types;
        if filter.card_types.as_slice() == [CardType::Instant, CardType::Sorcery] {
            filter.set_conjunctive_set_surface(true);
            filter.set_explicit_card_noun(true);
            filter.set_explicit_card_type_noun(Some(CardType::Sorcery));
        }
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: crate::tag::TagKey::from("__sentence_helper_exiled_l0_s0_e10"),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: crate::tag::TagKey::from(
                    "__sentence_helper_cast_from_exiled_collection_l0_s11_e30",
                ),
                relation: crate::target::TaggedOpbjectRelation::IsNotTaggedObject,
            });
        let moved = crate::effects::MoveToZoneEffect::new(ChooseSpec::All(filter), zone, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .with_target_plural_surface()
            .with_destination_player_surface(PlayerFilter::You);
        let moved = if random_bottom {
            moved.with_library_order(crate::effects::LibraryPlacementOrder::Random)
        } else {
            moved
        };
        Effect::new(moved)
    }

    #[test]
    fn exiled_collection_partition_preserves_exact_remainder_wording() {
        let collected = remaining_move(vec![], Zone::Library, true);
        assert_eq!(
            describe_exiled_collection_partition(&[collected]).as_deref(),
            Some(
                "Put the exiled cards not cast this way on the bottom of your library in a random order"
            )
        );

        let muse = Effect::new(crate::effects::SequenceEffect::sentence_leading_then(vec![
            remaining_move(
                vec![CardType::Instant, CardType::Sorcery],
                Zone::Hand,
                false,
            ),
            remaining_move(vec![], Zone::Library, true),
        ]));
        assert_eq!(
            describe_exiled_collection_partition(&[muse]).as_deref(),
            Some(
                "Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order"
            )
        );
    }
}

/// One-shot cast from the caster's hand or graveyard with a replacement life
/// price. The choice carries ownership and both zones; the cost is attached
/// to the cast itself, so no separate payment effect may be inferred here.
pub(super) fn describe_paid_cast_from_owned_zones(effects: &[&Effect]) -> Option<String> {
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
    if choose.chooser != PlayerFilter::You
        || choose.count != crate::effect::ChoiceCount::up_to(1)
        || choose.zone != Some(Zone::Graveyard)
        || choose.additional_zones.as_slice() != [Zone::Hand]
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.filter.card_types.len() != 1
        || choose.filter.excluded_card_types.as_slice() != [CardType::Land]
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
        || choose != &crate::effects::ChooseObjectsEffect::new(
            choose.filter.clone(), choose.count, PlayerFilter::You, choose.tag.clone(),
        ).in_zones(vec![Zone::Graveyard, Zone::Hand])
    {
        return None;
    }
    let cost = cast.alternative_cost.as_ref()?;
    let [component] = cost.as_all()? else {
        return None;
    };
    let payment = component.effect_ref()?.downcast_ref::<crate::effects::PayLifeEffect>()?;
    if payment.player != ChooseSpec::Player(PlayerFilter::You)
        || !matches!(payment.amount.unhinted(), Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Source))
        || cast != &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
            .with_alternative_cost(cost.clone())
    {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.owner = None;
    let card_type = filter.card_types.pop()?;
    filter.excluded_card_types.clear();
    filter.union_surface = Default::default();
    if filter != ObjectFilter::default() {
        return None;
    }
    let kind = card_type.to_string().to_lowercase();
    let spell = with_indefinite_article(&format!("{kind} spell"));
    Some(format!(
        "You may cast {spell} from your hand or graveyard by paying life equal to its mana value rather than paying its mana cost"
    ))
}

/// "Cast any number of <cards> from your graveyard without paying their mana
/// costs": an unbounded optional selection from the caster's own graveyard
/// iterated by a free cast. The land exclusion is the castability guard that
/// the spell-only instruction implies, so it is not repeated in the surface.
/// "Cast any number of Eldrazi spells from among cards you own outside the
/// game without paying their mana costs" (Spawnsire of Ulamog).
pub(super) fn describe_free_cast_any_number_from_outside_game(
    effects: &[&Effect],
) -> Option<String> {
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
    let mut excluded = choose.filter.excluded_card_types.clone();
    excluded.dedup();
    if choose.chooser != PlayerFilter::You
        || choose.is_search
        || !choose.count.is_any_number()
        || choose.zone != Some(Zone::OutsideGame)
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::OutsideGame)
        || choose.filter.owner != Some(PlayerFilter::You)
        || excluded.as_slice() != [CardType::Land]
        || (choose.filter.card_types.is_empty() && choose.filter.subtypes.is_empty())
        || !choose.filter.tagged_constraints.is_empty()
        || !choose.filter.any_of.is_empty()
        || choose.aggregate_constraint.is_some()
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
        || cast != &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
            .without_paying_mana_cost()
    {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.excluded_card_types.clear();
    let spell = describe_cast_spell_filter(&filter, CastSpellFilterContext::EnclosingPermission);
    if spell.contains("card") || !spell.ends_with("spell") {
        return None;
    }
    let spells = pluralize_cast_spell_description(&spell);
    Some(format!(
        "Cast any number of {spells} from among cards you own outside the game without paying their mana costs"
    ))
}

pub(super) fn describe_free_cast_any_number_from_your_graveyard(
    effects: &[&Effect],
) -> Option<String> {
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
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_any_number()
        || choose.zone != Some(Zone::Graveyard)
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::Graveyard)
        || choose.filter.owner != Some(PlayerFilter::You)
        || choose.filter.excluded_card_types.as_slice() != [CardType::Land]
        || choose.filter.card_types.is_empty()
        || !choose.filter.tagged_constraints.is_empty()
        || !choose.filter.any_of.is_empty()
        || for_each.tag != choose.tag
        || for_each.controller_at_last_blocked_by.is_some()
        || choose != &crate::effects::ChooseObjectsEffect::new(
            choose.filter.clone(), choose.count, PlayerFilter::You, choose.tag.clone(),
        ).in_zone(Zone::Graveyard)
        || cast != &crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
            .without_paying_mana_cost()
    {
        return None;
    }
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.excluded_card_types.clear();
    let description = filter.description();
    let mut noun = strip_leading_article(description.trim()).to_string();
    if !noun.ends_with(" card") {
        return None;
    }
    noun = pluralize_noun_phrase(&noun);
    Some(format!(
        "Cast any number of {noun} from your graveyard without paying their mana costs"
    ))
}

/// "You may cast an instant or sorcery spell from your graveyard with mana
/// value 4 or less without paying its mana cost": one card of the caster's
/// own graveyard is selected and the optional free cast consumes exactly that
/// selection.
pub(super) fn describe_chosen_graveyard_card_may_free_cast(
    effects: &[&Effect],
) -> Option<String> {
    let [choose_effect, may_effect] = effects else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) {
        return None;
    }
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if choose.chooser != PlayerFilter::You
        || !choose.count.is_single()
        || choose.zone != Some(Zone::Graveyard)
        || !choose.additional_zones.is_empty()
        || choose.filter.zone != Some(Zone::Graveyard)
        || !choose.filter.tagged_constraints.is_empty()
        || !choose.filter.any_of.is_empty()
        || choose != &crate::effects::ChooseObjectsEffect::new(
            choose.filter.clone(), choose.count, PlayerFilter::You, choose.tag.clone(),
        ).in_zone(Zone::Graveyard)
        || cast != &crate::effects::CastTaggedEffect::new(choose.tag.clone(), PlayerFilter::You)
            .without_paying_mana_cost()
    {
        return None;
    }
    // "from that player's graveyard" (Sorcerous Squall) names the graveyard
    // of the player the instruction already targeted.
    let graveyard = match &choose.filter.owner {
        Some(PlayerFilter::You) => "your graveyard",
        Some(PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_)) => {
            "that player's graveyard"
        }
        _ => return None,
    };
    let mut filter = choose.filter.clone();
    filter.zone = None;
    filter.owner = None;
    filter.excluded_card_types.retain(|card_type| *card_type != CardType::Land);
    if !filter.excluded_card_types.is_empty() {
        return None;
    }
    let card_types = std::mem::take(&mut filter.card_types);
    let subtypes = std::mem::take(&mut filter.subtypes);
    let mana_cap = match filter.mana_value.take() {
        None => String::new(),
        Some(crate::filter::Comparison::LessThanOrEqual(value)) => {
            format!(" with mana value {value} or less")
        }
        _ => return None,
    };
    filter.union_surface = Default::default();
    if filter != ObjectFilter::default() {
        return None;
    }
    let generic = card_types.is_empty() && subtypes.is_empty();
    let spell = spell_subject(generic, &card_types, &subtypes, false)?;
    Some(format!(
        "You may cast {} from {graveyard}{mana_cap} without paying its mana cost",
        with_indefinite_article(&spell)
    ))
}

#[cfg(test)]
mod paid_collection_cast_tests {
    use super::*;

    #[test]
    fn priced_owned_collection_keeps_its_zone_owner_and_payment_guards() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Paid cast")
            .card_types(vec![CardType::Creature])
            .parse_text("Whenever this creature attacks, you may cast an artifact spell from your hand or graveyard by paying life equal to its mana value rather than paying its mana cost.")
            .expect("paid collection cast should compile");
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected attacking trigger");
        };
        let effects = &triggered.effects.segments[0].default_effects;
        let window = effects.windows(2).find(|window| {
            structural_unwrap_render_wrappers(&window[0])
                .downcast_ref::<crate::effects::ChooseObjectsEffect>().is_some()
        }).expect("choice and priced cast");
        let refs = window.iter().collect::<Vec<_>>();
        assert_eq!(describe_paid_cast_from_owned_zones(&refs).as_deref(),
            Some("You may cast an artifact spell from your hand or graveyard by paying life equal to its mana value rather than paying its mana cost"));

        let mut wrong = window.to_vec();
        let mut choice = structural_unwrap_render_wrappers(&wrong[0])
            .downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap().clone();
        choice.filter.owner = None;
        wrong[0] = Effect::new(choice);
        let refs = wrong.iter().collect::<Vec<_>>();
        assert!(describe_paid_cast_from_owned_zones(&refs).is_none());

        let mut wrong = window.to_vec();
        let mut choice = structural_unwrap_render_wrappers(&wrong[0])
            .downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap().clone();
        choice.additional_zones.clear();
        wrong[0] = Effect::new(choice);
        let refs = wrong.iter().collect::<Vec<_>>();
        assert!(describe_paid_cast_from_owned_zones(&refs).is_none());

        let mut wrong = window.to_vec();
        let mut for_each = structural_unwrap_render_wrappers(&wrong[1])
            .downcast_ref::<crate::effects::ForEachTaggedEffect>().unwrap().clone();
        for_each.effects[0] = Effect::new(
            crate::effects::CastTaggedEffect::new("__it__", PlayerFilter::You)
                .without_paying_mana_cost(),
        );
        wrong[1] = Effect::new(for_each);
        let refs = wrong.iter().collect::<Vec<_>>();
        assert!(describe_paid_cast_from_owned_zones(&refs).is_none(),
            "a free cast cannot render as a life-priced cast");
    }
}

#[cfg(test)]
mod per_card_type_exiled_cast_tests {
    use super::*;

    #[test]
    fn independent_type_budgets_share_one_authored_permission() {
        let oracle = "Exile the top eight cards of your library. You may put a land card from among them onto the battlefield. Until end of turn, for each nonland card type, you may cast a spell of that type from among the exiled cards without paying its mana cost.";
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Type budget probe")
            .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
        assert_eq!(definition.spell_effect.as_ref().unwrap().segments.len(), 1);
        let effects = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects;
        assert!(describe_per_card_type_exiled_cast(effects).is_some());
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
        let mut altered = effects.clone();
        let grant = altered[2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>().unwrap();
        let mut grant = grant.clone();
        grant.max_plays = Some(2);
        altered[2] = Effect::new(grant);
        assert!(describe_per_card_type_exiled_cast(&altered).is_none());
        let mut altered = effects.clone();
        let grant = altered[2].downcast_ref::<crate::effects::GrantPlayTaggedEffect>().unwrap();
        let mut grant = grant.clone();
        grant.tag = crate::TagKey::from("unrelated_pool");
        altered[2] = Effect::new(grant);
        assert!(describe_per_card_type_exiled_cast(&altered).is_none());
    }
}

#[cfg(test)]
mod repeated_exile_capped_cast_tests {
    use super::*;

    #[test]
    fn repeated_exile_total_and_cast_use_one_accumulated_tag() {
        let oracle = "Shuffle your library. As many times as you choose, you may exile the top card of your library. If the total mana value of the cards exiled this way is 13 or less, you may cast any number of spells from among those cards without paying their mana costs.";
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Repeated exile probe")
            .card_types(vec![CardType::Sorcery]).parse_text(oracle).unwrap();
        assert_eq!(definition.spell_effect.as_ref().unwrap().segments.len(), 1);
        let effects = &definition.spell_effect.as_ref().unwrap().segments[0].default_effects;
        assert!(describe_repeated_exile_capped_cast(effects).is_some());
        assert_eq!(crate::compiled_text::compiled_text_lines(&definition).join("\n"), oracle);
        let conditional = effects[2].downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
        let choice = conditional.if_true[0].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap();
        assert_eq!(choice.zone, Some(Zone::Exile));
        assert!(choice.additional_zones.is_empty());
        assert!(choice.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameObjectId
        }));
        let mut altered = effects.clone();
        let conditional = altered[2].downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
        let mut conditional = conditional.clone();
        let choose = conditional.if_true[0].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap();
        let mut choose = choose.clone();
        choose.filter.tagged_constraints[0].tag = crate::TagKey::from("unrelated_exile");
        conditional.if_true[0] = Effect::new(choose);
        altered[2] = Effect::new(conditional);
        assert!(describe_repeated_exile_capped_cast(&altered).is_none());
        let mut altered = effects.clone();
        let conditional = altered[2].downcast_ref::<crate::effects::ConditionalEffect>().unwrap();
        let mut conditional = conditional.clone();
        let choose = conditional.if_true[0].downcast_ref::<crate::effects::ChooseObjectsEffect>().unwrap();
        let mut choose = choose.clone();
        choose.filter.tagged_constraints.retain(|constraint| {
            constraint.relation != crate::filter::TaggedOpbjectRelation::SameObjectId
        });
        conditional.if_true[0] = Effect::new(choose);
        altered[2] = Effect::new(conditional);
        assert!(describe_repeated_exile_capped_cast(&altered).is_none(),
            "a stable-card-only exile match cannot use the original-object surface");
    }
}
