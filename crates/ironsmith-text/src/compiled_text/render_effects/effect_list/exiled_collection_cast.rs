use super::*;

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
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
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
        let (remainder_surface, remainder_exiled, remainder_selected) = if let Some(view) =
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
                vec![oracle.to_string()],
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
