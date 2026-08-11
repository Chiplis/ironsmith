#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::game_state::Target;
use crate::mana::{ManaCost, ManaSymbol};

fn vanilla_permanent(name: &str, card_type: CardType) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![card_type]);
    if card_type == CardType::Creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

fn card_with_mana_value(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn joshua_phoenix_pair() -> (CardDefinition, CardDefinition) {
    let mut joshua = parse_oracle_card_definition("Joshua, Phoenix's Dominant");
    let mut phoenix = parse_oracle_card_definition("Phoenix, Warden of Fire");
    let joshua_id = CardId::from_raw(9_870_001);
    let phoenix_id = CardId::from_raw(9_870_002);

    joshua.card.id = joshua_id;
    joshua.card.other_face = Some(phoenix_id);
    joshua.card.other_face_name = Some("Phoenix, Warden of Fire".to_string());
    joshua.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;

    phoenix.card.id = phoenix_id;
    phoenix.card.other_face = Some(joshua_id);
    phoenix.card.other_face_name = Some("Joshua, Phoenix's Dominant".to_string());
    phoenix.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;

    (joshua, phoenix)
}

fn chapter(definition: &CardDefinition, number: u32) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .trigger
                    .downcast_ref::<crate::triggers::SagaChapterTrigger>()
                    .is_some_and(|chapter| chapter.chapters.contains(&number)) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing chapter {number} in {definition:#?}"))
}

fn target_contexts(
    requirements: &[crate::decision::TargetRequirement],
) -> Vec<crate::decisions::context::TargetRequirementContext> {
    requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                aggregate_constraint: requirement.aggregate_constraint.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
                distinct_player_group: requirement.distinct_player_group,
            },
        )
        .collect()
}

fn resolve_program_with_targets(
    game: &mut crate::GameState,
    source: ObjectId,
    controller: PlayerId,
    program: &crate::resolution::ResolutionProgram,
    targets: Vec<Target>,
) {
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        program,
        controller,
        Some(source),
        None,
    );
    let assignments = super::shard_17::target_assignments_for_requirements(&requirements, &targets);
    let resolved = targets
        .iter()
        .copied()
        .map(|target| match target {
            Target::Object(object) => crate::effects::ResolvedTarget::Object(object),
            Target::Player(player) => crate::effects::ResolvedTarget::Player(player),
        })
        .collect::<Vec<_>>();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(source, controller, &mut decisions)
        .with_targets(resolved)
        .with_target_assignments(assignments.clone());
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        program,
        None,
        &assignments,
    )
    .expect("exact parser-backed program should resolve");
}

fn resolve_trigger_entries(
    game: &mut crate::GameState,
    entries: Vec<crate::triggers::TriggeredAbilityEntry>,
) {
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut decisions)
        .expect("exact parser-backed triggers should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
            .expect("exact parser-backed trigger should resolve");
    }
}

#[test]
fn all_remaining_named_faces_parse_as_their_own_oracle_definitions() {
    let expected = [
        ("Mirage Phalanx", "Soulbond"),
        ("Joshua, Phoenix's Dominant", "When Joshua enters"),
        ("Phoenix, Warden of Fire", "I, II"),
        ("Tergrid, God of Fright", "Whenever an opponent sacrifices"),
        ("Tergrid's Lantern", "Target player loses 3 life unless"),
        ("Oath of the Grey Host", "You and target opponent each"),
    ];
    for (name, surface) in expected {
        let definition = parse_oracle_card_definition(name);
        let rendered = canonical_compiled_lines(&definition).join("\n");
        assert!(
            rendered.contains(surface),
            "{name} should compile its own face text, got {rendered}"
        );
    }
}

#[test]
fn mirage_phalanx_grants_both_paired_creatures_copy_triggers_only_on_its_combat() {
    let definition = parse_oracle_card_definition("Mirage Phalanx");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let phalanx = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let partner = game.create_object_from_definition(
        &vanilla_permanent("Mirage Partner", CardType::Creature),
        alice,
        Zone::Battlefield,
    );

    let combat_event = |player| {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::BeginningOfCombatEvent::new(player),
            crate::provenance::ProvNodeId::default(),
        )
    };
    assert!(
        crate::triggers::check_triggers(&game, &combat_event(alice)).is_empty(),
        "an unpaired Mirage Phalanx must not receive the shared combat trigger"
    );
    game.set_soulbond_pair(phalanx, partner);
    assert!(
        crate::triggers::check_triggers(&game, &combat_event(bob)).is_empty(),
        "the shared trigger must not fire at an opponent's combat"
    );
    let entries = crate::triggers::check_triggers(&game, &combat_event(alice));
    assert_eq!(entries.len(), 2, "each paired creature should trigger once");
    assert!(entries.iter().any(|entry| entry.source == phalanx));
    assert!(entries.iter().any(|entry| entry.source == partner));
    resolve_trigger_entries(&mut game, entries);

    let token_ids = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.kind == crate::object::ObjectKind::Token)
        })
        .collect::<Vec<_>>();
    assert_eq!(token_ids.len(), 2);
    let mut token_names = token_ids
        .iter()
        .map(|id| game.object(*id).expect("token exists").name.clone())
        .collect::<Vec<_>>();
    token_names.sort();
    assert_eq!(token_names, ["Mirage Partner", "Mirage Phalanx"]);
    for token in &token_ids {
        let ability_debug = format!("{:?}", game.object(*token).expect("token exists").abilities);
        assert!(
            ability_debug.contains("Haste"),
            "copy should have haste: {ability_debug}"
        );
        assert!(
            !ability_debug.contains("SoulbondPairEffect"),
            "copy must lose soulbond: {ability_debug}"
        );
    }
    assert_eq!(game.effect_store.delayed_triggers.len(), 2);

    let end_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::EndOfCombatEvent::new(),
        crate::provenance::ProvNodeId::default(),
    );
    let delayed = crate::triggers::check_delayed_triggers(&mut game, &end_event);
    resolve_trigger_entries(&mut game, delayed);
    assert!(token_ids.iter().all(|id| !game.battlefield.contains(id)));
}

fn resolve_joshua_etb(select_cards: bool) -> (usize, usize, usize) {
    let definition = parse_oracle_card_definition("Joshua, Phoenix's Dominant");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Joshua should have an enters trigger");
    let alice = PlayerId::from_index(0);
    let mut game = crate::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for index in 0..3 {
        game.create_object_from_definition(
            &vanilla_permanent(&format!("Hand Card {index}"), CardType::Sorcery),
            alice,
            Zone::Hand,
        );
        game.create_object_from_definition(
            &vanilla_permanent(&format!("Library Card {index}"), CardType::Instant),
            alice,
            Zone::Library,
        );
    }
    if select_cards {
        let mut decisions = crate::decision::SelectFirstDecisionMaker;
        let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut context,
            alice,
            source,
            &triggered.effects,
            None,
            &[],
        )
        .expect("Joshua's enters trigger should resolve");
    } else {
        let mut decisions = crate::decision::AutoPassDecisionMaker;
        let mut context = crate::effects::ExecutionContext::new(source, alice, &mut decisions);
        crate::game_loop::execute_resolution_program(
            &mut game,
            &mut context,
            alice,
            source,
            &triggered.effects,
            None,
            &[],
        )
        .expect("Joshua's declined discard should resolve");
    }
    let player = game.player(alice).expect("Alice exists");
    (
        player.hand.len(),
        player.graveyard.len(),
        player.library.len(),
    )
}

#[test]
fn joshua_front_discards_up_to_two_then_draws_exactly_that_many() {
    assert_eq!(resolve_joshua_etb(true), (3, 2, 1));
    assert_eq!(resolve_joshua_etb(false), (3, 0, 3));

    let (definition, phoenix) = joshua_phoenix_pair();
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Joshua should have a transform activation");
    assert_eq!(
        activated.timing,
        crate::ability::ActivationTiming::SorcerySpeed,
        "Joshua's transform activation must be illegal outside sorcery timing"
    );

    let alice = PlayerId::from_index(0);
    let mut game = crate::GameState::new(vec!["Alice".to_string()], 20);
    game.register_linked_face_definition(&definition);
    game.register_linked_face_definition(&phoenix);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let stable_id = game.object(source).expect("Joshua exists").stable_id;
    resolve_program_with_targets(&mut game, source, alice, &activated.effects, Vec::new());
    let transformed = game
        .find_object_by_stable_id(stable_id)
        .expect("Joshua should remain identifiable after exile and return");
    let transformed = game.object(transformed).expect("returned face exists");
    assert_eq!(transformed.zone, Zone::Battlefield);
    assert_eq!(transformed.name, "Phoenix, Warden of Fire");
    assert_eq!(game.controller_of(transformed), alice);
}

#[test]
fn a_card_without_a_transforming_back_face_cannot_enter_transformed() {
    let alice = PlayerId::from_index(0);
    let mut game = crate::GameState::new(vec!["Alice".to_string()], 20);
    let ordinary = game.create_object_from_definition(
        &vanilla_permanent("Ordinary Exile Card", CardType::Creature),
        alice,
        Zone::Exile,
    );
    let move_transformed = crate::effect::Effect::new(
        crate::effects::MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(ordinary),
            Zone::Battlefield,
            false,
        )
        .transformed(),
    );
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut context = crate::effects::ExecutionContext::new(ordinary, alice, &mut decisions);

    let outcome = crate::effects::execute_effect(&mut game, &move_transformed, &mut context)
        .expect("the invalid transformed entry should resolve as prevented");

    assert_eq!(outcome.status, crate::effect::OutcomeStatus::Prevented);
    assert_eq!(
        game.object(ordinary).expect("ordinary card remains").zone,
        Zone::Exile
    );
    assert!(game.battlefield.is_empty());
}

#[test]
fn phoenix_back_deals_two_to_each_opponent_and_enforces_one_aggregate_six_mana_budget() {
    let definition = parse_oracle_card_definition("Phoenix, Warden of Fire");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let damage_chapter = chapter(&definition, 1);
    resolve_program_with_targets(
        &mut game,
        source,
        alice,
        &damage_chapter.effects,
        Vec::new(),
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        24,
        "Phoenix has lifelink and should gain four life from damaging two opponents"
    );
    assert_eq!(game.player(bob).expect("Bob exists").life, 18);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 18);

    let four = game.create_object_from_definition(
        &card_with_mana_value("Phoenix Four-Drop", 4),
        alice,
        Zone::Graveyard,
    );
    let three = game.create_object_from_definition(
        &card_with_mana_value("Phoenix Three-Drop", 3),
        alice,
        Zone::Graveyard,
    );
    let two = game.create_object_from_definition(
        &card_with_mana_value("Phoenix Two-Drop", 2),
        alice,
        Zone::Graveyard,
    );
    let final_chapter = chapter(&definition, 3);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &final_chapter.effects,
        alice,
        Some(source),
        None,
    );
    let [requirement] = requirements.as_slice() else {
        panic!("Phoenix chapter III should announce one target set: {requirements:#?}");
    };
    assert_eq!(requirement.min_targets, 0);
    assert_eq!(requirement.max_targets, None);
    let aggregate = requirement
        .aggregate_constraint
        .as_ref()
        .expect("Phoenix chapter III should carry a total mana-value constraint");
    assert_eq!(aggregate.maximum, 6);
    let contexts = target_contexts(&requirements);
    assert!(
        !crate::targeting::validate_flat_target_assignment(
            &contexts,
            &[Target::Object(four), Target::Object(three)],
        ),
        "4+3 must be rejected even though both cards are individually under six"
    );
    assert!(crate::targeting::validate_flat_target_assignment(
        &contexts,
        &[Target::Object(four), Target::Object(two)],
    ));
}

fn resolve_tergrid_event(
    game: &mut crate::GameState,
    source: ObjectId,
    event: crate::triggers::TriggerEvent,
) {
    let entries = crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "Tergrid should trigger exactly once");
    resolve_trigger_entries(game, entries);
}

#[test]
fn tergrid_front_puts_the_opponents_discarded_and_sacrificed_nontoken_permanents_under_your_control()
 {
    let definition = parse_oracle_card_definition("Tergrid, God of Fright");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mut discard_game = crate::tests::test_helpers::setup_two_player_game();
    let discard_source =
        discard_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let discarded = discard_game.create_object_from_definition(
        &vanilla_permanent("Tergrid Discarded Relic", CardType::Artifact),
        bob,
        Zone::Hand,
    );
    let discarded_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        discard_game
            .object(discarded)
            .expect("discarded card exists"),
        &discard_game,
    );
    let discarded_stable = discarded_snapshot.stable_id;
    discard_game
        .move_object_by_effect(discarded, Zone::Graveyard)
        .expect("discarded permanent should enter the graveyard");
    let discard_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::CardDiscardedEvent::new(bob, discarded).with_snapshot(discarded_snapshot),
        crate::provenance::ProvNodeId::default(),
    );
    resolve_tergrid_event(&mut discard_game, discard_source, discard_event);
    let returned = discard_game
        .find_object_by_stable_id(discarded_stable)
        .expect("discarded permanent should remain identifiable");
    assert_eq!(
        discard_game.object(returned).expect("returned card").zone,
        Zone::Battlefield
    );
    assert_eq!(discard_game.controller_of_id(returned), Some(alice));

    let mut sacrifice_game = crate::tests::test_helpers::setup_two_player_game();
    let sacrifice_source =
        sacrifice_game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let sacrificed = sacrifice_game.create_object_from_definition(
        &vanilla_permanent("Tergrid Sacrificed Relic", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let sacrificed_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        sacrifice_game
            .object(sacrificed)
            .expect("sacrificed permanent exists"),
        &sacrifice_game,
    );
    let sacrificed_stable = sacrificed_snapshot.stable_id;
    sacrifice_game
        .move_object_by_effect(sacrificed, Zone::Graveyard)
        .expect("sacrificed permanent should enter the graveyard");
    let sacrifice_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::permanents::SacrificeEvent::new(sacrificed, None)
            .with_snapshot(Some(sacrificed_snapshot), Some(bob)),
        crate::provenance::ProvNodeId::default(),
    );
    resolve_tergrid_event(&mut sacrifice_game, sacrifice_source, sacrifice_event);
    let returned = sacrifice_game
        .find_object_by_stable_id(sacrificed_stable)
        .expect("sacrificed permanent should remain identifiable");
    assert_eq!(
        sacrifice_game.object(returned).expect("returned card").zone,
        Zone::Battlefield
    );
    assert_eq!(sacrifice_game.controller_of_id(returned), Some(alice));

    let token = sacrifice_game.create_object_from_definition(
        &vanilla_permanent("Tergrid Sacrificed Token", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    sacrifice_game.object_mut(token).expect("token exists").kind = crate::object::ObjectKind::Token;
    let token_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        sacrifice_game.object(token).expect("token exists"),
        &sacrifice_game,
    );
    let token_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::permanents::SacrificeEvent::new(token, None)
            .with_snapshot(Some(token_snapshot), Some(bob)),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&sacrifice_game, &token_event)
            .into_iter()
            .all(|entry| entry.source != sacrifice_source),
        "Tergrid must reject an opponent's token sacrifice"
    );
}

fn lantern_lose_life_ability(definition: &CardDefinition) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if format!("{:?}", activated.effects).contains("LoseLife") =>
            {
                Some(activated)
            }
            _ => None,
        })
        .expect("Tergrid's Lantern should have its target-player activation")
}

fn resolve_lantern(permanent: Option<CardType>, hand_card: bool) -> (i32, usize, usize) {
    let definition = parse_oracle_card_definition("Tergrid's Lantern");
    let activated = lantern_lose_life_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    if let Some(card_type) = permanent {
        game.create_object_from_definition(
            &vanilla_permanent("Lantern Permanent", card_type),
            bob,
            Zone::Battlefield,
        );
    }
    if hand_card {
        game.create_object_from_definition(
            &vanilla_permanent("Lantern Hand Card", CardType::Instant),
            bob,
            Zone::Hand,
        );
    }
    resolve_program_with_targets(
        &mut game,
        source,
        alice,
        &activated.effects,
        vec![Target::Player(bob)],
    );
    let bob_state = game.player(bob).expect("Bob exists");
    (
        bob_state.life,
        bob_state.graveyard.len(),
        game.battlefield
            .iter()
            .filter(|id| game.controller_of_id(**id) == Some(bob))
            .count(),
    )
}

#[test]
fn tergrids_lantern_honors_both_alternatives_and_untaps_only_itself() {
    assert_eq!(
        resolve_lantern(Some(CardType::Land), false),
        (17, 0, 1),
        "a land alone cannot satisfy the nonland sacrifice alternative"
    );
    assert_eq!(
        resolve_lantern(Some(CardType::Artifact), false),
        (20, 1, 0),
        "sacrificing a chosen nonland permanent should prevent the life loss"
    );
    assert_eq!(
        resolve_lantern(None, true),
        (20, 1, 0),
        "discarding a card should also prevent the life loss"
    );

    let definition = parse_oracle_card_definition("Tergrid's Lantern");
    let untap = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| format!("{:?}", activated.effects).contains("Untap"))
        .expect("Lantern should have a self-untap activation");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other = game.create_object_from_definition(
        &vanilla_permanent("Other Tapped Artifact", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    game.tap(source);
    game.tap(other);
    resolve_program_with_targets(&mut game, source, alice, &untap.effects, Vec::new());
    assert!(!game.is_tapped(source));
    assert!(game.is_tapped(other));
}

fn token_controllers(game: &crate::GameState, name: &str) -> Vec<PlayerId> {
    game.battlefield
        .iter()
        .filter_map(|id| {
            game.object(*id).and_then(|object| {
                (object.kind == crate::object::ObjectKind::Token && object.name == name)
                    .then(|| game.controller_of(object))
            })
        })
        .collect()
}

#[test]
fn coordinated_target_scoping_shares_only_a_synthetic_prelude() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let source = game.create_object_from_definition(
        &vanilla_permanent("Coordinated Target Source", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    let target_opponent = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Player(
        crate::target::PlayerFilter::Opponent,
    ));

    let shared =
        crate::resolution::ResolutionProgram::from_effects(vec![crate::effect::Effect::new(
            crate::effects::SequenceEffect::coordinated(vec![
                crate::effect::Effect::new(crate::effects::TargetOnlyEffect::new(
                    target_opponent.clone(),
                )),
                crate::effect::Effect::new(crate::effects::LoseLifeEffect::new(
                    3,
                    target_opponent.clone(),
                )),
            ]),
        )]);
    let shared_requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &shared,
        alice,
        Some(source),
        None,
    );
    assert_eq!(
        shared_requirements.len(),
        1,
        "a lowering-only prelude and its consumer announce one target"
    );
    resolve_program_with_targets(&mut game, source, alice, &shared, vec![Target::Player(bob)]);
    assert_eq!(game.player(bob).expect("Bob exists").life, 17);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 20);

    let independent =
        crate::resolution::ResolutionProgram::from_effects(vec![crate::effect::Effect::new(
            crate::effects::SequenceEffect::coordinated(vec![
                crate::effect::Effect::new(crate::effects::LoseLifeEffect::new(
                    1,
                    target_opponent.clone(),
                )),
                crate::effect::Effect::new(crate::effects::LoseLifeEffect::new(2, target_opponent)),
            ]),
        )]);
    let independent_requirements =
        crate::game_loop::extract_target_requirements_from_program_with_modes(
            &game,
            &independent,
            alice,
            Some(source),
            None,
        );
    assert_eq!(
        independent_requirements.len(),
        2,
        "ordinary equal-looking coordinated targets remain independent"
    );
    resolve_program_with_targets(
        &mut game,
        source,
        alice,
        &independent,
        vec![Target::Player(bob), Target::Player(charlie)],
    );
    assert_eq!(game.player(bob).expect("Bob exists").life, 16);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 18);
}

#[test]
fn oath_of_the_grey_host_chapters_keep_their_exact_players_counts_and_tapped_state() {
    let definition = parse_oracle_card_definition("Oath of the Grey Host");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let first = chapter(&definition, 1);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        &first.effects,
        alice,
        Some(source),
        None,
    );
    let [requirement] = requirements.as_slice() else {
        panic!("Oath chapter I should have one target opponent: {requirements:#?}");
    };
    assert!(requirement.legal_targets.contains(&Target::Player(bob)));
    assert!(requirement.legal_targets.contains(&Target::Player(charlie)));
    assert!(!requirement.legal_targets.contains(&Target::Player(alice)));
    resolve_program_with_targets(
        &mut game,
        source,
        alice,
        &first.effects,
        vec![Target::Player(bob)],
    );
    let mut foods = token_controllers(&game, "Food");
    foods.sort();
    assert_eq!(foods, [alice, bob]);
    assert!(
        !foods.contains(&charlie),
        "the unchosen opponent gets no Food"
    );

    resolve_program_with_targets(
        &mut game,
        source,
        alice,
        &chapter(&definition, 2).effects,
        Vec::new(),
    );
    assert_eq!(game.player(alice).expect("Alice exists").life, 20);
    assert_eq!(game.player(bob).expect("Bob exists").life, 17);
    assert_eq!(game.player(charlie).expect("Charlie exists").life, 17);
    assert_eq!(token_controllers(&game, "Treasure"), [alice]);

    resolve_program_with_targets(
        &mut game,
        source,
        alice,
        &chapter(&definition, 3).effects,
        Vec::new(),
    );
    let spirits = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token && object.name == "Spirit"
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(spirits.len(), 3);
    assert!(spirits.iter().all(|spirit| game.is_tapped(*spirit)));
    assert!(
        spirits
            .iter()
            .all(|spirit| game.controller_of_id(*spirit) == Some(alice))
    );
    assert!(
        spirits.iter().all(
            |spirit| format!("{:?}", game.object(*spirit).unwrap().abilities).contains("Flying")
        )
    );
}
