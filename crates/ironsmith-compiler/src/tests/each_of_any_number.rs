use super::*;

#[test]
fn counter_removal_each_of_any_number_keeps_zero_and_unbounded_selection() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Chandra, Legacy of Fire")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .parse_text(
            "0: Remove a loyalty counter from each of any number of permanents you control.",
        )
        .expect("the any-number counter-removal ability should parse");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("the loyalty line should produce an activated ability");
    let effects = activated.effects.flattened_default_effects();
    let choose = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::ChooseObjectsEffect>(effect))
        .expect("the ability should choose the affected subset");

    // "Any number" must permit the empty subset.
    assert_eq!(choose.count.min, 0);
    // It must also permit selecting more than any fixed upper bound.
    assert_eq!(choose.count.max, None);
    assert_eq!(choose.count, ChoiceCount::any_number());

    let for_each = effects
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::ForEachTaggedEffect<crate::effect::Effect>>(
                effect,
            )
        })
        .expect("the ability should iterate exactly the chosen subset");
    assert_eq!(for_each.tag, choose.tag);
    let removal = for_each
        .effects
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::RemoveCountersEffect>(effect)
        })
        .expect("each selected permanent should lose one loyalty counter");
    assert_eq!(removal.target, crate::target::ChooseSpec::Iterated);
}

#[test]
fn repeated_counter_placements_preserve_each_any_number_target_cardinality() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Filigree Vector")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "When this creature enters, put a +1/+1 counter on each of any number of target creatures and a charge counter on each of any number of target artifacts.",
        )
        .expect("the paired any-number target placements should parse");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("the enters line should produce a triggered ability");
    let flattened = triggered.effects.flattened_default_effects();
    let puts = flattened
        .iter()
        .filter_map(|effect| super::find_nested_effect::<crate::effects::PutCountersEffect>(effect))
        .collect::<Vec<_>>();

    assert_eq!(puts.len(), 2, "{:#?}", triggered.effects);
    for put in puts {
        assert_eq!(put.target_count, Some(ChoiceCount::any_number()));
        assert!(matches!(
            &put.target,
            crate::target::ChooseSpec::WithCount(_, count)
                if *count == ChoiceCount::any_number()
        ));
    }
}

#[test]
fn repeated_each_counter_placements_apply_to_both_complete_sets() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Set Counter Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .parse_text(
            "−2: Put a +1/+1 counter on each creature you control and a loyalty counter on each other planeswalker you control.",
        )
        .expect("the paired each-set counter placements should parse");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("the loyalty line should produce an activated ability");
    let effects = activated.effects.flattened_default_effects();
    let fanouts = effects
        .iter()
        .filter_map(|effect| super::find_nested_effect::<crate::effects::ForEachObject>(effect))
        .collect::<Vec<_>>();

    assert_eq!(fanouts.len(), 2, "{:#?}", activated.effects);
    assert_eq!(fanouts[0].filter.card_types, vec![CardType::Creature]);
    assert_eq!(
        fanouts[0].filter.controller,
        Some(crate::target::PlayerFilter::You)
    );
    assert!(!fanouts[0].filter.other);
    assert_eq!(fanouts[1].filter.card_types, vec![CardType::Planeswalker]);
    assert_eq!(
        fanouts[1].filter.controller,
        Some(crate::target::PlayerFilter::You)
    );
    assert!(fanouts[1].filter.other);
    assert!(
        fanouts.iter().all(|fanout| {
            fanout.effects.iter().any(|effect| {
                super::find_nested_effect::<crate::effects::PutCountersEffect>(effect)
                    .is_some_and(|put| matches!(&put.target, crate::target::ChooseSpec::Iterated))
            })
        }),
        "each placement must iterate its complete filtered set: {fanouts:#?}"
    );
}

#[test]
fn typed_counter_removed_followup_counts_counter_actions_not_permanents() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Garnet, Princess of Alexandria")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever Garnet attacks, you may remove a lore counter from each of any number of Sagas you control. Put a +1/+1 counter on Garnet for each lore counter removed this way.",
        )
        .expect("the typed removed-counter follow-up should parse");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("the attacks line should produce a triggered ability");
    let put = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::PutCountersEffect>(effect))
        .expect("the follow-up should put a +1/+1 counter");
    let crate::effect::Value::PriorEffectMetric { query, .. } = put.amount.unhinted() else {
        panic!(
            "the follow-up must count the exact prior removal action: {:#?}",
            put.amount
        );
    };

    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Removed)
    );
    assert_eq!(query.source, ironsmith_core::EffectMetricSource::Outcome);
    assert_eq!(query.counter_type, Some(crate::object::CounterType::Lore));
    assert!(query.filter.is_none());
}

#[test]
fn target_player_each_binds_their_library_to_the_iterated_player() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Singularity Rupture")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy all creatures, then any number of target players each mill half their library, rounded down.",
        )
        .expect("the any-number target-player mill should parse");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("the sorcery should have a resolution program")
        .flattened_default_effects();
    let target_players = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::TargetOnlyEffect>(effect))
        .expect("the target-player iterator should declare its shared targets");
    assert_eq!(
        target_players.target.count(),
        crate::effect::ChoiceCount::any_number(),
        "the authored unbounded optional target cardinality must survive lowering"
    );
    let mill = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::MillEffect>(effect))
        .expect("each selected player should mill");

    assert_eq!(mill.player, crate::target::PlayerFilter::IteratedPlayer);
    let crate::effect::Value::HalfRoundedDown(library_count) = mill.count.unhinted() else {
        panic!(
            "expected a rounded-down half-library count: {:#?}",
            mill.count
        );
    };
    assert_eq!(
        library_count.unhinted(),
        &crate::effect::Value::CardsInLibrary(crate::target::PlayerFilter::IteratedPlayer)
    );
}

#[test]
fn counter_removed_from_activation_cost_scales_each_pt_bonus() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Blademane Baku")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you cast a Spirit or Arcane spell, you may put a ki counter on this creature.\n{1}, Remove X ki counters from this creature: For each counter removed, this creature gets +2/+0 until end of turn.",
        )
        .expect("the activation-cost counter result should parse");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("the counter-removal line should produce an activated ability");
    let apply = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::ApplyContinuousEffect>(effect)
        })
        .expect("the ability should produce a runtime-scaled power modifier");
    let runtime_pt = apply
        .runtime_modifications
        .iter()
        .find_map(|modification| match modification {
            crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } => Some((power, toughness)),
            _ => None,
        })
        .expect("the continuous effect should modify power and toughness");

    assert_eq!(runtime_pt.1.unhinted(), &crate::effect::Value::Fixed(0));
    let crate::effect::Value::Scaled(basis, 2) = runtime_pt.0.unhinted() else {
        panic!(
            "each removed counter must contribute +2 power: {:#?}",
            runtime_pt.0
        );
    };
    assert_eq!(basis.unhinted(), &crate::effect::Value::X);
    assert!(
        basis.has_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemoved),
        "{basis:#?}"
    );
}

#[test]
fn twice_x_create_count_remains_dynamic_and_outside_token_name() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Pest Infestation")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy up to X target artifacts and/or enchantments. Create twice X 1/1 black and green Pest creature tokens with \"When this token dies, you gain 1 life.\"",
        )
        .expect("twice-X token creation should parse");
    let create = definition
        .spell_effect
        .as_ref()
        .expect("the sorcery should have a resolution program")
        .flattened_default_effects()
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::CreateTokenEffect>(effect))
        .expect("the second sentence should create Pest tokens");

    assert_eq!(create.count.unhinted(), &crate::effect::Value::XTimes(2));
    assert_eq!(create.token.card.name, "Pest");
    assert_ne!(create.token.card.name, "X");
}

#[test]
fn counted_sacrifice_reflexive_keeps_typed_gate_and_one_shared_count() {
    fn referenced_effect_id(value: &crate::effect::Value) -> Option<crate::effect::EffectId> {
        match value.unhinted() {
            crate::effect::Value::EffectValue(id)
            | crate::effect::Value::EffectValueOffset(id, _) => Some(*id),
            crate::effect::Value::EffectMetric { effect_id, .. }
            | crate::effect::Value::EffectMetricOffset { effect_id, .. } => Some(*effect_id),
            _ => None,
        }
    }

    let source_text = "Whenever this creature attacks, sacrifice any number of artifacts. \
         When you sacrifice one or more artifacts this way, tap up to that many target creatures and draw that many cards.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Counted Sacrifice Reflexive")
        .card_types(vec![CardType::Creature])
        .parse_text(source_text)
        .expect("the counted sacrifice reflexive trigger should parse");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("the attacks line should produce a triggered ability");
    let effects = triggered.effects.flattened_default_effects();

    let choose = effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::ChooseObjectsEffect>(effect))
        .expect("the ability should choose the artifacts to sacrifice");
    assert_eq!(choose.count, ChoiceCount::any_number());
    assert_eq!(choose.filter.card_types, vec![CardType::Artifact]);

    let (sacrifice_id, sacrifice) = effects
        .iter()
        .find_map(|effect| {
            let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
            let sacrifice = super::find_nested_effect::<crate::effects::SacrificePlayerEffect>(
                &with_id.effect,
            )?;
            Some((with_id.id, sacrifice))
        })
        .expect("the selected artifacts should be sacrificed by an identified effect");
    assert_eq!(sacrifice.player, crate::target::PlayerFilter::You);
    assert!(
        sacrifice
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == choose.tag),
        "the sacrifice must consume exactly the chosen artifact set: {sacrifice:#?}"
    );

    let reflexive = effects
        .iter()
        .find_map(|effect| {
            super::find_nested_effect::<crate::effects::ReflexiveTriggerEffect>(effect)
        })
        .expect("the this-way clause should lower as a reflexive trigger");
    assert_eq!(reflexive.condition, sacrifice_id);
    let crate::effect::EffectPredicate::PriorEffectResult(surface) = &reflexive.predicate else {
        panic!("the reflexive gate must retain its typed sacrifice predicate");
    };
    assert_eq!(
        surface.action,
        ironsmith_core::PriorEffectAction::Sacrificed
    );
    assert_eq!(surface.actor, ironsmith_core::PriorEffectResultActor::You);
    assert_eq!(
        surface.quantifier,
        ironsmith_core::PriorEffectResultQuantifier::OneOrMore
    );
    assert_eq!(surface.filter.card_types, vec![CardType::Artifact]);

    let tap = reflexive
        .effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::TapEffect>(effect))
        .expect("the reflexive branch should tap creatures");
    let tap_count = tap
        .target
        .count_value()
        .and_then(referenced_effect_id)
        .expect("the tap target count should reference the sacrifice result");
    let draw = reflexive
        .effects
        .iter()
        .find_map(|effect| super::find_nested_effect::<crate::effects::DrawCardsEffect>(effect))
        .expect("the reflexive branch should draw cards");
    let draw_count = referenced_effect_id(&draw.count)
        .expect("the draw count should reference the sacrifice result");

    assert_eq!(tap_count, sacrifice_id);
    assert_eq!(draw_count, sacrifice_id, "{:#?}", triggered.effects);
}
