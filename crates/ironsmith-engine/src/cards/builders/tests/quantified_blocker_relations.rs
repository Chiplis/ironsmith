#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn creature_with_flanking(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .parse_text("Flanking")
        .expect("the blocker fixture's printed flanking should parse")
}

fn became_blocked_event(
    game: &crate::GameState,
    attacker: ObjectId,
    blockers: &[ObjectId],
) -> crate::triggers::TriggerEvent {
    let attacker_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).expect("attacker exists"),
            game,
        );
    let blocker_snapshots = blockers
        .iter()
        .map(|blocker| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(*blocker).expect("blocker exists"),
                game,
            )
        })
        .collect();
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureBecameBlockedEvent::with_target_and_blockers(
            attacker,
            blockers.to_vec(),
            None,
            Some(attacker_snapshot),
            blocker_snapshots,
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn install_two_blocked_attackers(
    game: &mut crate::GameState,
    source_attacker: ObjectId,
    source_blockers: Vec<ObjectId>,
    decoy_attacker: ObjectId,
    decoy_blocker: ObjectId,
) {
    game.combat = Some(crate::combat_state::CombatState {
        blockers: std::collections::HashMap::from([
            (source_attacker, source_blockers),
            (decoy_attacker, vec![decoy_blocker]),
        ]),
        ..Default::default()
    });
}

fn this_becomes_blocked_ability(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::ThisBecomesBlockedTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("the card should retain its this-becomes-blocked ability")
}

fn pump_filter(program: &crate::resolution::ResolutionProgram) -> &ObjectFilter {
    fn find(effect: &Effect) -> Option<&ObjectFilter> {
        if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
            && let crate::continuous::EffectTarget::Filter(filter) = &apply.target
            && matches!(
                apply.runtime_modifications.as_slice(),
                [crate::effects::RuntimeModification::ModifyPowerToughness { .. }]
            )
        {
            return Some(filter);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return find(&tagged.effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return find(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return find(&with_id.effect);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(find);
        }
        None
    }

    program
        .flattened_default_effects()
        .iter()
        .find_map(find)
        .expect("the ability should contain one set-based P/T modification")
}

fn assert_directional_blocker_filter(filter: &ObjectFilter) {
    assert!(
        filter.blocking,
        "the recipient must be a blocker: {filter:#?}"
    );
    assert!(
        filter.in_combat_with_source,
        "the recipient must block the relevant attacker, not any attacker: {filter:#?}"
    );
}

fn assert_high_semantic_score(name: &str, definition: &CardDefinition) {
    let compiled = unprocessed_compiled_lines(definition);
    let (_oracle_coverage, _compiled_coverage, similarity, delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            &oracle_text_by_name()[name],
            &compiled,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        !mismatch && similarity >= 0.99,
        "{name} should retain its misleadingly high text score after the behavioral repair: similarity={similarity}, delta={delta}, compiled={compiled:#?}"
    );
}

fn resolve_program(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    program: &crate::resolution::ResolutionProgram,
    event: Option<crate::triggers::TriggerEvent>,
) -> Vec<crate::triggers::TriggerEvent> {
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, controller, &mut decisions);
    if let Some(event) = event {
        context = context.with_triggering_event(event);
    }
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        program,
        None,
        &[],
    )
    .expect("the parser-backed ability should resolve")
}

#[test]
fn baneblade_scoundrel_and_plague_wight_shrink_only_their_own_blockers() {
    for name in ["Baneblade Scoundrel", "Plague Wight"] {
        let mut definition = parse_oracle_card_definition(name);
        definition.card.power_toughness = Some(PowerToughness::fixed(4, 4));
        let triggered = this_becomes_blocked_ability(&definition);
        assert_directional_blocker_filter(pump_filter(&triggered.effects));

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let first_blocker = game.create_object_from_definition(
            &creature("Relevant Blocker A", 3, 3),
            bob,
            Zone::Battlefield,
        );
        let second_blocker = game.create_object_from_definition(
            &creature("Relevant Blocker B", 3, 3),
            bob,
            Zone::Battlefield,
        );
        let decoy_attacker = game.create_object_from_definition(
            &creature("Decoy Attacker", 3, 3),
            alice,
            Zone::Battlefield,
        );
        let decoy_blocker = game.create_object_from_definition(
            &creature("Unrelated Blocker", 3, 3),
            bob,
            Zone::Battlefield,
        );
        install_two_blocked_attackers(
            &mut game,
            source,
            vec![first_blocker, second_blocker],
            decoy_attacker,
            decoy_blocker,
        );
        let event = became_blocked_event(&game, source, &[first_blocker, second_blocker]);
        resolve_program(&mut game, source, alice, &triggered.effects, Some(event));

        assert_eq!(game.current_power(first_blocker), Some(2), "{name}");
        assert_eq!(game.current_toughness(first_blocker), Some(2), "{name}");
        assert_eq!(game.current_power(second_blocker), Some(2), "{name}");
        assert_eq!(game.current_toughness(second_blocker), Some(2), "{name}");
        assert_eq!(
            game.current_power(source),
            Some(4),
            "{name} must not shrink itself once per blocker"
        );
        assert_eq!(
            game.current_power(decoy_blocker),
            Some(3),
            "{name} must not affect a blocker in an unrelated combat pair"
        );
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert!(
            compiled.contains("each creature blocking this creature gets -1/-1 until end of turn"),
            "{name}: {compiled}"
        );
        assert_high_semantic_score(name, &definition);
    }
}

#[test]
fn knight_of_valor_shrinks_only_nonflanking_creatures_blocking_it() {
    let mut definition = parse_oracle_card_definition("Knight of Valor");
    definition.card.power_toughness = Some(PowerToughness::fixed(4, 4));
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Knight of Valor should retain its activated ability");
    assert_eq!(
        activated.timing,
        crate::ability::ActivationTiming::OncePerTurn
    );
    let filter = pump_filter(&activated.effects);
    assert_directional_blocker_filter(filter);
    assert_eq!(
        filter.excluded_static_abilities,
        [StaticAbilityId::Flanking]
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let affected = game.create_object_from_definition(
        &creature("Relevant Nonflanking Blocker", 3, 3),
        bob,
        Zone::Battlefield,
    );
    let excluded = game.create_object_from_definition(
        &creature_with_flanking("Relevant Flanking Blocker", 3, 3),
        bob,
        Zone::Battlefield,
    );
    let decoy_attacker = game.create_object_from_definition(
        &creature("Decoy Attacker", 3, 3),
        alice,
        Zone::Battlefield,
    );
    let decoy_blocker = game.create_object_from_definition(
        &creature("Unrelated Nonflanking Blocker", 3, 3),
        bob,
        Zone::Battlefield,
    );
    install_two_blocked_attackers(
        &mut game,
        source,
        vec![affected, excluded],
        decoy_attacker,
        decoy_blocker,
    );
    resolve_program(&mut game, source, alice, &activated.effects, None);

    assert_eq!(game.current_power(affected), Some(2));
    assert_eq!(game.current_toughness(affected), Some(2));
    assert_eq!(game.current_power(excluded), Some(3));
    assert_eq!(game.current_power(decoy_blocker), Some(3));
    assert_eq!(game.current_power(source), Some(4));
    let compiled = canonical_compiled_lines(&definition).join("\n");
    assert!(
        compiled.contains(
            "Each creature blocking this creature without flanking gets -1/-1 until end of turn"
        ),
        "{compiled}"
    );
    assert_high_semantic_score("Knight of Valor", &definition);
}

#[test]
fn trailblazers_torch_uses_the_equipped_creature_as_source_and_its_blockers_as_recipients() {
    let definition = parse_oracle_card_definition("Trailblazer's Torch");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::BecomesBlockedTrigger>()
                    .is_some() =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Trailblazer's Torch should retain its equipped-creature block trigger");
    let trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::BecomesBlockedTrigger>()
        .unwrap();
    assert!(trigger.filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "equipped"
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));

    let execute = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>())
        .expect("recipient enumeration must execute with the equipped creature as source");
    assert!(
        matches!(execute.source.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"),
        "the block-event attacker must be the explicit damage source: {execute:#?}"
    );
    let for_each = execute
        .effect
        .downcast_ref::<crate::effects::ForEachObject>()
        .expect("the explicit source should wrap the blocker fanout");
    assert_directional_blocker_filter(&for_each.filter);
    let [damage_effect] = for_each.effects.as_slice() else {
        panic!("the fanout should contain one damage action: {for_each:#?}");
    };
    let damage = damage_effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .expect("the inner action should deal damage with the rebound source");
    assert!(matches!(damage.target.base(), ChooseSpec::Iterated));

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let equipment = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let equipped_creature = game.create_object_from_definition(
        &creature("Equipped Attacker", 4, 4),
        alice,
        Zone::Battlefield,
    );
    let first_blocker = game.create_object_from_definition(
        &creature("Relevant Blocker A", 3, 4),
        bob,
        Zone::Battlefield,
    );
    let second_blocker = game.create_object_from_definition(
        &creature("Relevant Blocker B", 3, 4),
        bob,
        Zone::Battlefield,
    );
    let decoy_attacker = game.create_object_from_definition(
        &creature("Decoy Attacker", 3, 3),
        alice,
        Zone::Battlefield,
    );
    let decoy_blocker = game.create_object_from_definition(
        &creature("Unrelated Blocker", 3, 4),
        bob,
        Zone::Battlefield,
    );
    install_two_blocked_attackers(
        &mut game,
        equipped_creature,
        vec![first_blocker, second_blocker],
        decoy_attacker,
        decoy_blocker,
    );
    let event = became_blocked_event(&game, equipped_creature, &[first_blocker, second_blocker]);
    let events = resolve_program(&mut game, equipment, alice, &triggered.effects, Some(event));
    let damage_events = events
        .iter()
        .filter_map(|event| event.downcast::<crate::events::DamageEvent>())
        .map(|damage| (damage.source, damage.target, damage.amount))
        .collect::<Vec<_>>();

    assert_eq!(game.damage_on(first_blocker), 2);
    assert_eq!(game.damage_on(second_blocker), 2);
    assert_eq!(game.damage_on(decoy_blocker), 0);
    assert_eq!(game.damage_on(equipment), 0);
    assert_eq!(damage_events.len(), 2, "{damage_events:#?}");
    assert!(damage_events.iter().all(|(source, target, amount)| {
        *source == equipped_creature
            && *amount == 2
            && matches!(
                target,
                crate::events::DamageTarget::Object(id)
                    if *id == first_blocker || *id == second_blocker
            )
    }));
    let compiled = canonical_compiled_lines(&definition).join("\n");
    assert!(
        compiled.contains("deals 2 damage to each creature blocking this creature"),
        "{compiled}"
    );
    assert_high_semantic_score("Trailblazer's Torch", &definition);
}
