#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::game_state::{GameState, StackEntry, Target, TargetAssignment};
use crate::mana::{ManaCost, ManaSymbol};
use crate::types::{CardType, Subtype};

fn test_spell(
    name: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    mana_value: u8,
) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .subtypes(subtypes);
    if mana_value > 0 {
        builder = builder.mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
            mana_value,
        )]));
    }
    builder.build()
}

fn instant(name: &str, mana_value: u8) -> CardDefinition {
    test_spell(name, vec![CardType::Instant], Vec::new(), mana_value)
}

fn add_stack_spell(
    game: &mut GameState,
    definition: &CardDefinition,
    controller: PlayerId,
) -> (ObjectId, StableId) {
    let id = game.create_object_from_definition(definition, controller, Zone::Stack);
    let stable_id = game.object(id).expect("stack spell should exist").stable_id;
    game.push_to_stack(StackEntry::new(id, controller));
    (id, stable_id)
}

fn zone_of_stable(game: &GameState, stable_id: StableId) -> Zone {
    let id = game
        .find_object_by_stable_id(stable_id)
        .expect("the moved card should remain findable by stable id");
    game.object(id).expect("the moved card should exist").zone
}

fn push_actual_counter(
    game: &mut GameState,
    card_name: &str,
    controller: PlayerId,
    targets: Vec<Target>,
    chosen_modes: Option<Vec<usize>>,
    x_value: Option<u32>,
    target_ranges: Option<Vec<std::ops::Range<usize>>>,
) -> (ObjectId, StableId) {
    let definition = parse_oracle_card_definition(card_name);
    let source = game.create_object_from_definition(&definition, controller, Zone::Stack);
    let stable_id = game
        .object(source)
        .expect("counterspell should exist")
        .stable_id;
    if let Some(x) = x_value {
        game.object_mut(source)
            .expect("counterspell should exist")
            .x_value = Some(x);
    }
    let mut entry = StackEntry::new(source, controller)
        .with_targets(targets)
        .with_chosen_modes(chosen_modes.clone());
    if let Some(x) = x_value {
        entry = entry.with_x(x);
    }
    if let Some(ranges) = target_ranges {
        let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
            game,
            definition
                .spell_effect
                .as_ref()
                .expect("counterspell should have a spell program"),
            controller,
            Some(source),
            chosen_modes.as_deref(),
        );
        assert_eq!(requirements.len(), ranges.len());
        entry = entry.with_target_assignments(
            requirements
                .into_iter()
                .zip(ranges)
                .map(|(requirement, range)| TargetAssignment {
                    spec: requirement.spec,
                    range,
                })
                .collect(),
        );
    }
    game.push_to_stack(entry);
    (source, stable_id)
}

fn resolve_actual_counter_to_zone(
    card_name: &str,
    target_definition: &CardDefinition,
    expected_zone: Zone,
    chosen_modes: Option<Vec<usize>>,
    x_value: Option<u32>,
) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (target, target_stable_id) = add_stack_spell(&mut game, target_definition, bob);
    push_actual_counter(
        &mut game,
        card_name,
        alice,
        vec![Target::Object(target)],
        chosen_modes,
        x_value,
        None,
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .unwrap_or_else(|error| panic!("{card_name} should resolve: {error}"));
    assert_eq!(
        zone_of_stable(&game, target_stable_id),
        expected_zone,
        "{card_name} should put the countered spell in {expected_zone:?}"
    );
}

fn actual_legal_targets(
    game: &GameState,
    card_name: &str,
    source: ObjectId,
    controller: PlayerId,
    chosen_modes: Option<&[usize]>,
) -> Vec<Target> {
    let definition = parse_oracle_card_definition(card_name);
    crate::game_loop::extract_target_requirements_from_program_with_modes(
        game,
        definition
            .spell_effect
            .as_ref()
            .expect("counterspell should have a spell program"),
        controller,
        Some(source),
        chosen_modes,
    )
    .into_iter()
    .flat_map(|requirement| requirement.legal_targets)
    .collect()
}

fn assert_target_matrix(
    card_name: &str,
    chosen_modes: Option<&[usize]>,
    legal_definitions: Vec<CardDefinition>,
    illegal_definitions: Vec<CardDefinition>,
) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let legal = legal_definitions
        .iter()
        .map(|definition| add_stack_spell(&mut game, definition, bob).0)
        .collect::<Vec<_>>();
    let illegal = illegal_definitions
        .iter()
        .map(|definition| add_stack_spell(&mut game, definition, bob).0)
        .collect::<Vec<_>>();
    let counter = parse_oracle_card_definition(card_name);
    let source = game.create_object_from_definition(&counter, alice, Zone::Stack);
    let candidates = actual_legal_targets(&game, card_name, source, alice, chosen_modes);
    for id in legal {
        assert!(
            candidates.contains(&Target::Object(id)),
            "{card_name} should accept the positive target {id:?}"
        );
    }
    for id in illegal {
        assert!(
            !candidates.contains(&Target::Object(id)),
            "{card_name} should reject the negative target {id:?}"
        );
    }
}

#[test]
fn assert_authority_dissipate_and_void_shatter_exile_spells_but_not_abilities() {
    for card_name in ["Assert Authority", "Dissipate", "Void Shatter"] {
        resolve_actual_counter_to_zone(
            card_name,
            &instant("Ordinary Target", 2),
            Zone::Exile,
            None,
            None,
        );

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_definition =
            test_spell("Ability Source", vec![CardType::Artifact], Vec::new(), 2);
        let ability_source =
            game.create_object_from_definition(&source_definition, bob, Zone::Battlefield);
        game.push_to_stack(StackEntry::ability(
            ability_source,
            bob,
            vec![Effect::gain_life(1)],
        ));
        let counter = parse_oracle_card_definition(card_name);
        let counter_source = game.create_object_from_definition(&counter, alice, Zone::Stack);
        assert!(
            !actual_legal_targets(&game, card_name, counter_source, alice, None)
                .contains(&Target::Object(ability_source)),
            "{card_name} targets spells, not activated or triggered abilities"
        );
    }
}

#[test]
fn force_of_negation_targets_noncreatures_exiles_them_and_gates_its_blue_pitch_cost() {
    let noncreature = instant("Force Target", 2);
    assert_target_matrix(
        "Force of Negation",
        None,
        vec![noncreature.clone()],
        vec![test_spell(
            "Creature Decoy",
            vec![CardType::Creature],
            Vec::new(),
            2,
        )],
    );
    resolve_actual_counter_to_zone("Force of Negation", &noncreature, Zone::Exile, None, None);

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    let (opponent_spell, opponent_spell_stable_id) =
        add_stack_spell(&mut game, &instant("Opponent Noncreature Spell", 2), bob);
    let force = parse_oracle_card_definition("Force of Negation");
    let force_id = game.create_object_from_definition(&force, alice, Zone::Hand);
    let blue_card = CardDefinitionBuilder::new(CardId::new(), "Blue Pitch Card")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .card_types(vec![CardType::Instant])
        .build();
    let blue_card_id = game.create_object_from_definition(&blue_card, alice, Zone::Hand);
    let blue_card_stable_id = game
        .object(blue_card_id)
        .expect("the blue pitch card should exist")
        .stable_id;
    let has_pitch_action = |game: &GameState| {
        crate::decision::compute_legal_actions(game, alice)
            .iter()
            .any(|action| {
                matches!(
                    action,
                    crate::decision::LegalAction::CastSpell {
                        spell_id,
                        from_zone: Zone::Hand,
                        casting_method: crate::alternative_cast::CastingMethod::Alternative(0),
                    } if *spell_id == force_id
                )
            })
    };
    assert!(
        has_pitch_action(&game),
        "on Bob's turn, another blue hand card should make the alternative cost legal"
    );
    game.turn.active_player = alice;
    assert!(
        !has_pitch_action(&game),
        "Force of Negation's alternative cost must be unavailable during its controller's turn"
    );
    game.turn.active_player = bob;

    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let cast = crate::game_loop::PriorityResponse::PriorityAction(
        crate::decision::LegalAction::CastSpell {
            spell_id: force_id,
            from_zone: Zone::Hand,
            casting_method: crate::alternative_cast::CastingMethod::Alternative(0),
        },
    );
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &cast,
        &mut decisions,
    )
    .expect("the alternative-cost cast should begin");
    assert!(
        matches!(
            progress,
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Targets(_)
            )
        ),
        "the legal pitch cast should advance to target selection"
    );
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::Targets(vec![Target::Object(opponent_spell)]),
        &mut decisions,
    )
    .expect("the noncreature target should be accepted");
    let progress = match progress {
        direct @ crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_),
        ) => direct,
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(context),
        ) => {
            let cost_index = context
                .options
                .iter()
                .find(|option| {
                    option.legal && option.description.to_ascii_lowercase().contains("exile")
                })
                .map(|option| option.index)
                .expect("the alternative cost should expose its exile component");
            crate::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &crate::game_loop::PriorityResponse::NextCostChoice(cost_index),
                &mut decisions,
            )
            .expect("the exile cost should be selected")
        }
        other => panic!("expected an exile-card cost choice, got {other:?}"),
    };
    assert!(matches!(
        progress,
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectObjects(_)
        )
    ));
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::CardCostChoice(blue_card_id),
        &mut decisions,
    )
    .expect("exiling the other blue card should finish the alternative cost");
    assert_eq!(
        zone_of_stable(&game, blue_card_stable_id),
        Zone::Exile,
        "the actual alternative-cost cast should exile the other blue card"
    );
    assert!(
        game.stack.iter().any(|entry| {
            game.object(entry.object_id)
                .is_some_and(|object| object.name == "Force of Negation")
        }),
        "Force of Negation should be on the stack after its alternative cost is paid"
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the alternatively cast Force of Negation should resolve");
    assert_eq!(zone_of_stable(&game, opponent_spell_stable_id), Zone::Exile);
}

#[test]
fn dissipate_does_not_exile_a_spell_that_was_not_countered() {
    let uncounterable = CardDefinitionBuilder::new(CardId::new(), "Uncounterable Target")
        .card_types(vec![CardType::Instant])
        .parse_text("This spell can't be countered.")
        .expect("the uncounterable target should parse");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (target, target_stable_id) = add_stack_spell(&mut game, &uncounterable, bob);
    push_actual_counter(
        &mut game,
        "Dissipate",
        alice,
        vec![Target::Object(target)],
        None,
        None,
        None,
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Dissipate should resolve");
    assert_eq!(
        zone_of_stable(&game, target_stable_id),
        Zone::Stack,
        "an uncounterable spell must remain on the stack"
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the uncounterable target should resolve normally");
    assert_eq!(
        zone_of_stable(&game, target_stable_id),
        Zone::Graveyard,
        "the conditional exile replacement must not outlive a failed counter"
    );
}

#[test]
fn lapse_hinder_remand_and_crumple_destinations_require_a_successful_counter() {
    for card_name in [
        "Lapse of Certainty",
        "Memory Lapse",
        "Hinder",
        "Remand",
        "Spell Crumple",
    ] {
        let uncounterable = CardDefinitionBuilder::new(CardId::new(), "Uncounterable Target")
            .card_types(vec![CardType::Instant])
            .parse_text("This spell can't be countered.")
            .expect("the uncounterable target should parse");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.create_object_from_definition(&instant("Draw Sentinel", 1), alice, Zone::Library);
        let (target, target_stable_id) = add_stack_spell(&mut game, &uncounterable, bob);
        push_actual_counter(
            &mut game,
            card_name,
            alice,
            vec![Target::Object(target)],
            None,
            None,
            None,
        );
        let mut decisions = crate::decision::SelectFirstDecisionMaker;
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
            .unwrap_or_else(|error| panic!("{card_name} should resolve: {error}"));
        assert_eq!(
            zone_of_stable(&game, target_stable_id),
            Zone::Stack,
            "{card_name} must not move a spell it failed to counter"
        );
        crate::game_loop::resolve_stack_entry(&mut game)
            .expect("the uncounterable target should resolve normally");
        assert_eq!(
            zone_of_stable(&game, target_stable_id),
            Zone::Graveyard,
            "{card_name}'s alternate destination must not survive the failed counter"
        );
    }
}

#[test]
fn defabricate_exiles_artifact_or_enchantment_spells_and_plain_counters_an_ability() {
    assert_target_matrix(
        "Defabricate",
        Some(&[0]),
        vec![
            test_spell("Artifact Spell", vec![CardType::Artifact], Vec::new(), 2),
            test_spell(
                "Enchantment Spell",
                vec![CardType::Enchantment],
                Vec::new(),
                2,
            ),
        ],
        vec![test_spell(
            "Creature Spell",
            vec![CardType::Creature],
            Vec::new(),
            2,
        )],
    );
    resolve_actual_counter_to_zone(
        "Defabricate",
        &test_spell("Artifact Target", vec![CardType::Artifact], Vec::new(), 2),
        Zone::Exile,
        Some(vec![0]),
        None,
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let permanent = game.create_object_from_definition(
        &test_spell(
            "Activated Ability Source",
            vec![CardType::Artifact],
            Vec::new(),
            2,
        ),
        bob,
        Zone::Battlefield,
    );
    game.push_to_stack(StackEntry::ability(
        permanent,
        bob,
        vec![Effect::gain_life(3)],
    ));
    push_actual_counter(
        &mut game,
        "Defabricate",
        alice,
        vec![Target::Object(permanent)],
        Some(vec![1]),
        None,
        None,
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Defabricate's second mode should resolve");
    assert!(
        !game
            .stack
            .iter()
            .any(|entry| entry.is_ability && entry.object_id == permanent),
        "the targeted ability should be countered"
    );
    assert_eq!(
        game.object(permanent).expect("source should remain").zone,
        Zone::Battlefield,
        "countering the ability must not exile its source permanent"
    );
}

#[test]
fn deny_existence_and_deny_the_divine_enforce_types_before_exiling() {
    assert_target_matrix(
        "Deny Existence",
        None,
        vec![test_spell(
            "Creature Target",
            vec![CardType::Creature],
            Vec::new(),
            3,
        )],
        vec![test_spell(
            "Enchantment Decoy",
            vec![CardType::Enchantment],
            Vec::new(),
            3,
        )],
    );
    resolve_actual_counter_to_zone(
        "Deny Existence",
        &test_spell("Creature Target", vec![CardType::Creature], Vec::new(), 3),
        Zone::Exile,
        None,
        None,
    );

    assert_target_matrix(
        "Deny the Divine",
        None,
        vec![
            test_spell("Creature Target", vec![CardType::Creature], Vec::new(), 3),
            test_spell(
                "Enchantment Target",
                vec![CardType::Enchantment],
                Vec::new(),
                3,
            ),
        ],
        vec![instant("Instant Decoy", 3)],
    );
    for definition in [
        test_spell("Creature Target", vec![CardType::Creature], Vec::new(), 3),
        test_spell(
            "Enchantment Target",
            vec![CardType::Enchantment],
            Vec::new(),
            3,
        ),
    ] {
        resolve_actual_counter_to_zone("Deny the Divine", &definition, Zone::Exile, None, None);
    }
}

#[test]
fn reject_targets_creatures_or_planeswalkers_and_exiles_only_after_an_unpaid_successful_counter() {
    let creature = CardDefinitionBuilder::new(CardId::new(), "Reject Creature Target")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    assert_target_matrix(
        "Reject",
        None,
        vec![
            creature.clone(),
            test_spell(
                "Reject Planeswalker Target",
                vec![CardType::Planeswalker],
                Vec::new(),
                3,
            ),
        ],
        vec![instant("Reject Instant Decoy", 3)],
    );
    assert_unless_counter_paid_and_unpaid("Reject", 3, None, &creature, Zone::Battlefield);

    let uncounterable_creature =
        CardDefinitionBuilder::new(CardId::new(), "Uncounterable Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .parse_text("This spell can't be countered.")
            .expect("the uncounterable creature should parse");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (target, target_stable_id) = add_stack_spell(&mut game, &uncounterable_creature, bob);
    push_actual_counter(
        &mut game,
        "Reject",
        alice,
        vec![Target::Object(target)],
        None,
        None,
        None,
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Reject should resolve");
    assert_eq!(zone_of_stable(&game, target_stable_id), Zone::Stack);
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("the uncounterable creature should resolve");
    assert_eq!(
        zone_of_stable(&game, target_stable_id),
        Zone::Battlefield,
        "Reject's exile replacement must not apply when the counter failed"
    );
}

#[test]
fn faerie_trickery_rejects_faeries_and_exiles_a_nonfaerie_spell() {
    let nonfaerie = test_spell(
        "Wizard Target",
        vec![CardType::Creature],
        vec![Subtype::Wizard],
        2,
    );
    let faerie = test_spell(
        "Faerie Decoy",
        vec![CardType::Creature],
        vec![Subtype::Faerie],
        2,
    );
    assert_target_matrix(
        "Faerie Trickery",
        None,
        vec![nonfaerie.clone()],
        vec![faerie],
    );
    resolve_actual_counter_to_zone("Faerie Trickery", &nonfaerie, Zone::Exile, None, None);
}

#[test]
fn horribly_awry_and_liquify_apply_their_mana_value_limits() {
    let mv_four_creature = test_spell(
        "Four Mana Creature",
        vec![CardType::Creature],
        Vec::new(),
        4,
    );
    assert_target_matrix(
        "Horribly Awry",
        None,
        vec![mv_four_creature.clone()],
        vec![
            test_spell(
                "Five Mana Creature",
                vec![CardType::Creature],
                Vec::new(),
                5,
            ),
            instant("Four Mana Noncreature", 4),
        ],
    );
    resolve_actual_counter_to_zone("Horribly Awry", &mv_four_creature, Zone::Exile, None, None);

    let mv_three = instant("Three Mana Spell", 3);
    assert_target_matrix(
        "Liquify",
        None,
        vec![mv_three.clone()],
        vec![instant("Four Mana Spell", 4)],
    );
    resolve_actual_counter_to_zone("Liquify", &mv_three, Zone::Exile, None, None);
}

#[derive(Default)]
struct BottomScryDecisionMaker;

impl DecisionMaker for BottomScryDecisionMaker {
    fn decide_partition(
        &mut self,
        _game: &GameState,
        context: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        context.cards.iter().map(|(id, _)| *id).collect()
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        context: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        context.items.iter().map(|(id, _)| *id).collect()
    }
}

#[test]
fn no_escape_targets_only_creatures_or_planeswalkers_exiles_then_scries() {
    assert_target_matrix(
        "No Escape",
        None,
        vec![
            test_spell("Creature Target", vec![CardType::Creature], Vec::new(), 3),
            test_spell(
                "Planeswalker Target",
                vec![CardType::Planeswalker],
                Vec::new(),
                3,
            ),
        ],
        vec![instant("Instant Decoy", 3)],
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let old_top =
        game.create_object_from_definition(&instant("Old Library Card", 1), alice, Zone::Library);
    let scry_card =
        game.create_object_from_definition(&instant("Scry Card", 1), alice, Zone::Library);
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .library
            .last(),
        Some(&scry_card)
    );
    let (target, target_stable_id) = add_stack_spell(
        &mut game,
        &test_spell("Creature Target", vec![CardType::Creature], Vec::new(), 3),
        bob,
    );
    push_actual_counter(
        &mut game,
        "No Escape",
        alice,
        vec![Target::Object(target)],
        None,
        None,
        None,
    );
    let mut decisions = BottomScryDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("No Escape should resolve and scry");
    assert_eq!(zone_of_stable(&game, target_stable_id), Zone::Exile);
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .library
            .last(),
        Some(&old_top),
        "putting the looked-at card on the bottom proves the scry executed"
    );
}

#[test]
fn devious_cover_up_exiles_the_spell_and_shuffles_only_its_controllers_grave_cards() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (target, target_stable_id) = add_stack_spell(&mut game, &instant("Counter Target", 2), bob);
    let mut grave_targets = Vec::new();
    let mut grave_stable_ids = Vec::new();
    for index in 0..4 {
        let id = game.create_object_from_definition(
            &instant(&format!("Grave Target {index}"), 1),
            alice,
            Zone::Graveyard,
        );
        grave_targets.push(Target::Object(id));
        grave_stable_ids.push(game.object(id).expect("grave card should exist").stable_id);
    }
    let opponent_decoy = game.create_object_from_definition(
        &instant("Opponent Graveyard Decoy", 1),
        bob,
        Zone::Graveyard,
    );
    let definition = parse_oracle_card_definition("Devious Cover-Up");
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        definition
            .spell_effect
            .as_ref()
            .expect("Devious Cover-Up should have a spell program"),
        alice,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 2);
    assert!(
        grave_targets
            .iter()
            .all(|target| requirements[1].legal_targets.contains(target))
    );
    assert!(
        !requirements[1]
            .legal_targets
            .contains(&Target::Object(opponent_decoy)),
        "Devious Cover-Up says 'from your graveyard', so the opponent's card is illegal"
    );
    let assignments = requirements
        .into_iter()
        .zip([0..1, 1..5])
        .map(|(requirement, range)| TargetAssignment {
            spec: requirement.spec,
            range,
        })
        .collect();
    let mut targets = vec![Target::Object(target)];
    targets.extend(grave_targets);
    game.push_to_stack(
        StackEntry::new(source, alice)
            .with_targets(targets)
            .with_target_assignments(assignments),
    );
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Devious Cover-Up should resolve");
    assert_eq!(zone_of_stable(&game, target_stable_id), Zone::Exile);
    for stable_id in grave_stable_ids {
        let moved = game
            .find_object_by_stable_id(stable_id)
            .expect("shuffled card should remain findable");
        assert_eq!(
            game.object(moved).expect("shuffled card should exist").zone,
            Zone::Library
        );
        assert!(
            game.player(alice)
                .expect("Alice should exist")
                .library
                .contains(&moved),
            "each selected grave card should enter its owner's library"
        );
    }
    assert_eq!(
        game.object(opponent_decoy)
            .expect("opponent decoy should remain")
            .zone,
        Zone::Graveyard
    );
}

#[test]
fn lapse_of_certainty_and_memory_lapse_put_the_countered_spell_on_top() {
    for card_name in ["Lapse of Certainty", "Memory Lapse"] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let existing = game.create_object_from_definition(
            &instant("Existing Library Card", 1),
            bob,
            Zone::Library,
        );
        let (target, target_stable_id) =
            add_stack_spell(&mut game, &instant("Counter Target", 2), bob);
        push_actual_counter(
            &mut game,
            card_name,
            alice,
            vec![Target::Object(target)],
            None,
            None,
            None,
        );
        crate::game_loop::resolve_stack_entry(&mut game)
            .unwrap_or_else(|error| panic!("{card_name} should resolve: {error}"));
        let moved = game
            .find_object_by_stable_id(target_stable_id)
            .expect("target should move");
        assert_eq!(
            game.object(moved).expect("target should exist").zone,
            Zone::Library
        );
        assert_eq!(
            game.player(bob).expect("Bob should exist").library.last(),
            Some(&moved)
        );
        assert_ne!(moved, existing);
    }
}

struct HinderDecisionMaker {
    choose_bottom: bool,
}

impl DecisionMaker for HinderDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        context: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        let needle = if self.choose_bottom { "bottom" } else { "top" };
        context
            .options
            .iter()
            .find(|option| option.legal && option.description.to_ascii_lowercase().contains(needle))
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

#[test]
fn hinder_lets_the_counterspell_controller_choose_top_or_bottom() {
    for choose_bottom in [false, true] {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let existing = game.create_object_from_definition(
            &instant("Existing Library Card", 1),
            bob,
            Zone::Library,
        );
        let (target, target_stable_id) =
            add_stack_spell(&mut game, &instant("Hinder Target", 2), bob);
        push_actual_counter(
            &mut game,
            "Hinder",
            alice,
            vec![Target::Object(target)],
            None,
            None,
            None,
        );
        let mut decisions = HinderDecisionMaker { choose_bottom };
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
            .expect("Hinder should resolve");
        let moved = game
            .find_object_by_stable_id(target_stable_id)
            .expect("target should move");
        let library = &game.player(bob).expect("Bob should exist").library;
        if choose_bottom {
            assert_eq!(library.first(), Some(&moved));
            assert_eq!(library.last(), Some(&existing));
        } else {
            assert_eq!(library.last(), Some(&moved));
            assert_eq!(library.first(), Some(&existing));
        }
    }
}

#[test]
fn remand_returns_the_spell_to_its_owners_hand_and_draws_for_its_controller() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let draw_card =
        game.create_object_from_definition(&instant("Alice Draw", 1), alice, Zone::Library);
    let draw_stable_id = game
        .object(draw_card)
        .expect("draw card should exist")
        .stable_id;
    let (target, target_stable_id) = add_stack_spell(&mut game, &instant("Remand Target", 2), bob);
    push_actual_counter(
        &mut game,
        "Remand",
        alice,
        vec![Target::Object(target)],
        None,
        None,
        None,
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Remand should resolve");
    let moved = game
        .find_object_by_stable_id(target_stable_id)
        .expect("target should move");
    assert_eq!(
        game.object(moved).expect("target should exist").zone,
        Zone::Hand
    );
    assert!(
        game.player(bob)
            .expect("Bob should exist")
            .hand
            .contains(&moved)
    );
    let drawn = game
        .find_object_by_stable_id(draw_stable_id)
        .expect("drawn card should remain findable");
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .hand
            .contains(&drawn)
    );
}

#[test]
fn spell_crumple_puts_both_the_target_and_itself_on_their_owners_library_bottoms() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&instant("Alice Existing", 1), alice, Zone::Library);
    game.create_object_from_definition(&instant("Bob Existing", 1), bob, Zone::Library);
    let (target, target_stable_id) = add_stack_spell(&mut game, &instant("Crumple Target", 2), bob);
    let (_, crumple_stable_id) = push_actual_counter(
        &mut game,
        "Spell Crumple",
        alice,
        vec![Target::Object(target)],
        None,
        None,
        None,
    );
    crate::game_loop::resolve_stack_entry(&mut game).expect("Spell Crumple should resolve");
    let moved_target = game
        .find_object_by_stable_id(target_stable_id)
        .expect("target should move");
    let moved_crumple = game
        .find_object_by_stable_id(crumple_stable_id)
        .expect("Crumple should move");
    assert_eq!(
        game.object(moved_target).expect("target should exist").zone,
        Zone::Library
    );
    assert_eq!(
        game.object(moved_crumple)
            .expect("Crumple should exist")
            .zone,
        Zone::Library
    );
    assert_eq!(
        game.player(bob).expect("Bob should exist").library.first(),
        Some(&moved_target)
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .library
            .first(),
        Some(&moved_crumple)
    );
}

fn assert_unless_counter_paid_and_unpaid(
    card_name: &str,
    payment: u32,
    x_value: Option<u32>,
    target_definition: &CardDefinition,
    resolved_zone: Zone,
) {
    let mut unpaid_game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let (unpaid_target, unpaid_stable_id) =
        add_stack_spell(&mut unpaid_game, target_definition, bob);
    push_actual_counter(
        &mut unpaid_game,
        card_name,
        alice,
        vec![Target::Object(unpaid_target)],
        None,
        x_value,
        None,
    );
    let mut accept = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut unpaid_game, &mut accept)
        .unwrap_or_else(|error| panic!("unpaid {card_name} should resolve: {error}"));
    assert_eq!(zone_of_stable(&unpaid_game, unpaid_stable_id), Zone::Exile);

    let mut paid_game = crate::tests::test_helpers::setup_two_player_game();
    paid_game
        .player_mut(bob)
        .expect("Bob should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, payment);
    let (paid_target, paid_stable_id) = add_stack_spell(&mut paid_game, target_definition, bob);
    push_actual_counter(
        &mut paid_game,
        card_name,
        alice,
        vec![Target::Object(paid_target)],
        None,
        x_value,
        None,
    );
    let mut accept = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut paid_game, &mut accept)
        .unwrap_or_else(|error| panic!("paid {card_name} should resolve: {error}"));
    assert_eq!(
        zone_of_stable(&paid_game, paid_stable_id),
        Zone::Stack,
        "paying {payment} should prevent {card_name} from countering the target"
    );
    assert_eq!(
        paid_game
            .player(bob)
            .expect("Bob should exist")
            .mana_pool
            .total(),
        0,
        "{card_name} should spend the announced payment"
    );
    crate::game_loop::resolve_stack_entry(&mut paid_game)
        .unwrap_or_else(|error| panic!("the paid-through target should resolve: {error}"));
    assert_eq!(
        zone_of_stable(&paid_game, paid_stable_id),
        resolved_zone,
        "the exile replacement from {card_name} must not survive a paid-through counter attempt"
    );
}

#[test]
fn no_more_lies_spell_shrivel_and_syncopate_exile_only_when_the_payment_is_not_made() {
    let target = instant("Unless-Counter Target", 2);
    assert_unless_counter_paid_and_unpaid("No More Lies", 3, None, &target, Zone::Graveyard);
    assert_unless_counter_paid_and_unpaid("Spell Shrivel", 4, None, &target, Zone::Graveyard);
    assert_unless_counter_paid_and_unpaid("Syncopate", 2, Some(2), &target, Zone::Graveyard);
}
