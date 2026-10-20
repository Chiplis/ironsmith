//! Regressions for mechanisms interrupted in the Claude handoff.
#[path = "cf8_p08/support.rs"]
mod support;
#[path = "cf8_p08/play.rs"]
mod play;

#[test]
fn copied_abilities_retain_source_qualities() {
    use ironsmith::ability::AbilityKind;
    for (name, body, required) in [
        ("Abstruse Archaic", "Mana cost: {4}\nType: Creature — Avatar\nPower/Toughness: 3/4\nVigilance\n{1}, {T}: Copy target activated or triggered ability you control from a colorless source. You may choose new targets for the copy. (Mana abilities can't be targeted.)", vec!["colorless: true"]),
        ("The Peregrine Dynamo", "Mana cost: {3}\nType: Legendary Artifact Creature — Construct\nPower/Toughness: 1/5\nHaste\n{1}, {T}: Copy target activated or triggered ability you control from another legendary source that's not a commander. You may choose new targets for the copy. (Mana abilities can't be targeted.)", vec!["Legendary", "noncommander: true", "other: true"]),
    ] {
        for definition in support::definitions(name, body) {
            let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated), _ => None,
            }).unwrap();
            let debug = format!("{activated:?}");
            for qualifier in &required { assert!(debug.contains(qualifier), "{name}: missing {qualifier}: {debug}"); }
        }
    }
}

#[test]
fn delayed_death_checks_the_referenced_creatures_last_controller() {
    use ironsmith::target::PlayerFilter;
    let oracle = "Target creature gets +1/-1 until end of turn. When it dies under your control this turn, draw two cards.";
    let body = format!("Mana cost: {{B}}\nType: Instant\n{oracle}");
    for definition in support::definitions("Delayed Death Probe", &body) {
        let program = definition.spell_effect.as_ref().expect("spell program");
        let delayed = program.flattened_default_effects().iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>())
            .expect("delayed death watch");
        assert!(delayed.target_tag.is_some(), "the modified target must be captured");
        assert!(delayed.target_filter.is_none(), "control must not be checked on resolution");
        let death = delayed.trigger.downcast_ref::<ironsmith::triggers::ZoneChangeTrigger>()
            .expect("typed death trigger");
        assert!(death.this_object, "only the captured creature may satisfy the watch");
        assert_eq!(death.object_filter.controller, Some(PlayerFilter::You));
        assert_eq!(support::rendered(&definition), oracle);
    }

    // Omitting the controller condition keeps the same captured-object death
    // watch, but must not silently gain a controller restriction.
    let plain = "Target creature gets +1/-1 until end of turn. When it dies this turn, draw two cards.";
    let body = format!("Mana cost: {{B}}\nType: Instant\n{plain}");
    for definition in support::definitions("Unqualified Death Probe", &body) {
        let program = definition.spell_effect.as_ref().expect("spell program");
        let delayed = program.flattened_default_effects().iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>())
            .expect("delayed death watch");
        assert!(delayed.target_filter.is_none());
        let death = delayed.trigger.downcast_ref::<ironsmith::triggers::ZoneChangeTrigger>()
            .expect("typed death trigger");
        assert!(death.this_object);
        assert_eq!(death.object_filter.controller, None);
    }
}

#[test]
fn delayed_death_uses_control_at_death_in_live_games() {
    use ironsmith::game_loop::put_triggers_on_stack_with_dm;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{Target, Zone};

    let oracle = "Target creature gets +1/-1 until end of turn. When it dies under your control this turn, draw two cards.";
    let body = format!("Mana cost: {{B}}\nType: Instant\n{oracle}");
    for definition in support::definitions("Delayed Death Scenario", &body) {
        for (initial_controller, death_controller, expected_cards) in [
            (play::B, play::A, 2),
            (play::A, play::B, 0),
            (play::A, play::A, 2),
        ] {
            let mut game = play::game();
            play::give_mana(&mut game, play::A, ManaSymbol::Black, 1);
            let witness = play::vanilla("Death witness", "{1}", "Human", 4, 4);
            let target = game.create_object_from_definition(
                &witness, initial_controller, Zone::Battlefield,
            );
            for index in 0..3 {
                let card = play::vanilla(&format!("Draw {index}"), "{1}", "Human", 1, 1);
                game.create_object_from_definition(&card, play::A, Zone::Library);
            }
            let mut decisions = play::Script {
                targets: vec![Target::Object(target)],
                ..Default::default()
            };
            play::cast(&mut game, play::A, &definition, &mut decisions);
            play::resolve_all(&mut game, &mut decisions);
            assert!(game.player(play::A).unwrap().hand.is_empty());

            // Changing control after the spell resolves must change whether
            // the captured target satisfies the delayed death condition.
            game.set_current_controller(target, death_controller).unwrap();
            game.move_object_by_game_rule(target, Zone::Graveyard).unwrap();
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions).unwrap();
            assert_eq!(game.stack.len(), usize::from(expected_cards > 0));
            play::resolve_all(&mut game, &mut decisions);
            assert_eq!(game.player(play::A).unwrap().hand.len(), expected_cards);
        }
    }
}

#[test]
fn target_controller_pays_that_targets_toughness_as_life() {
    use ironsmith::effect::Value;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::{Target, Zone};

    let oracle = "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated.";
    let body = format!("Mana cost: {{1}}{{U}}{{B}}\nType: Instant\n{oracle}");
    for definition in support::definitions("Target Toughness Payment Probe", &body) {
        let program = definition.spell_effect.as_ref().expect("spell program");
        let unless = program.flattened_default_effects().iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::UnlessPaysEffect>())
            .expect("controller may pay life");
        let [effect] = unless.cost.costs() else { panic!("one payment effect"); };
        let payment = effect.effect_ref()
            .and_then(|effect| effect.downcast_ref::<ironsmith::effects::PayLifeEffect>())
            .expect("dynamic life payment");
        let Value::ToughnessOf(basis) = payment.amount.unhinted() else {
            panic!("payment must read target toughness: {:?}", payment.amount);
        };
        let destroy = unless.effects[0].downcast_ref::<ironsmith::effects::TaggedEffect>().expect("destroy outcome tag")
            .effect.downcast_ref::<ironsmith::effects::DestroyNoRegenerationEffect>()
            .expect("unregenerable destruction");
        assert_eq!(basis.unhinted(), destroy.spec.unhinted());
        assert_eq!(support::rendered(&definition), oracle);

        for toughness in [2, 5] {
            let mut game = play::game();
            play::give_mana(&mut game, play::A, ManaSymbol::Colorless, 1);
            play::give_mana(&mut game, play::A, ManaSymbol::Blue, 1);
            play::give_mana(&mut game, play::A, ManaSymbol::Black, 1);
            let witness = play::vanilla("Payment witness", "{1}", "Human", 4, toughness);
            let target = game.create_object_from_definition(&witness, play::B, Zone::Battlefield);
            let mut decisions = play::Script {
                targets: vec![Target::Object(target)],
                ..Default::default()
            };
            play::cast(&mut game, play::A, &definition, &mut decisions);
            play::resolve_all(&mut game, &mut decisions);
            assert!(game.object(target).is_some(), "the controller paid to keep the target");
            assert_eq!(game.player(play::B).unwrap().life, 20 - toughness);
        }
    }
}

#[test]
fn craft_linked_copy_survives_direct_and_artifact_compilation() {
    use ironsmith::ability::AbilityKind;
    use ironsmith_core::static_ability_model::StaticAbilityPayload;
    let oracle = "Locus of Enlightenment has each activated ability of the exiled cards used to craft it. You may activate each of those abilities only once each turn.";
    for definition in support::definitions("Locus of Enlightenment", &format!("Type: Artifact\n{oracle}")) {
        let copy = definition.abilities.iter().find_map(|ability| {
            let AbilityKind::Static(ability) = &ability.kind else { return None; };
            let model = ability.canonical_model()?;
            match model.payload {
                StaticAbilityPayload::CopyActivatedAbilities(copy) => Some(copy),
                _ => None,
            }
        }).expect("craft-linked static copy");
        assert!(copy.force_once_each_turn);
        assert_eq!(copy.filter.zone, Some(ironsmith::Zone::Exile));
        assert!(copy.filter.tagged_constraints.iter().any(|constraint|
            constraint.tag.as_str() == "__source_exiled__"
                && constraint.relation == ironsmith::filter::TaggedOpbjectRelation::IsTaggedObject));
        let rendered = support::rendered(&definition);
        assert!(rendered.contains("the exiled cards used to craft it"), "{rendered}");
        assert!(rendered.contains("only once each turn"), "{rendered}");
    }
}

#[test]
fn aura_grant_keeps_the_named_granting_source_through_artifact_compilation() {
    use ironsmith::ability::AbilityKind;
    use ironsmith_core::static_ability_model::StaticAbilityPayload;
    let oracle = "Enchant creature\nEnchanted creature can't attack or block and has \"{7}: Hold for Ransom's controller sacrifices it and draws a card. Activate only as a sorcery.\"";
    for definition in support::definitions("Hold for Ransom", &format!("Type: Enchantment — Aura\n{oracle}")) {
        let grant = definition.abilities.iter().find_map(|ability| {
            let AbilityKind::Static(ability) = &ability.kind else { return None; };
            let model = ability.canonical_model()?;
            match model.payload {
                StaticAbilityPayload::AttachedAbilityGrant(grant) => Some(grant),
                _ => None,
            }
        }).expect("attached ability grant");
        let ironsmith_core::AbilityKind::Activated(activated) = &grant.ability.kind else {
            panic!("the Aura grants an activated ability");
        };
        let body = format!("{activated:?}");
        assert!(body.contains("SacrificeTarget"), "{body}");
        assert!(body.contains("SourceReference"), "{body}");
        assert!(body.contains("sacrificed_0") && body.contains("ControllerOf"), "{body}");
        assert!(body.contains("SorcerySpeed"), "{body}");
        let rendered = support::rendered(&definition).to_ascii_lowercase();
        assert!(rendered.contains("enchanted creature can't attack or block"), "{rendered}");
        assert!(rendered.contains("controller sacrifices it and draws a card"), "{rendered}");
        assert!(rendered.contains("activate only as a sorcery"), "{rendered}");
    }
}
