#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::game_state::Target;

fn three_player_game() -> crate::GameState {
    crate::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    )
}

fn creature(
    name: &str,
    power: i32,
    toughness: i32,
    subtypes: impl IntoIterator<Item = Subtype>,
) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(subtypes.into_iter().collect())
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn permanent(name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![card_type])
        .build()
}

fn flash_spell(name: &str, has_flash: bool) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1));
    if has_flash {
        builder
            .parse_text("Flash")
            .expect("the flash test spell should parse")
    } else {
        builder.build()
    }
}

fn trigger_event<E: crate::events::GameEventType + 'static>(
    event: E,
) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        event,
        crate::provenance::ProvNodeId::default(),
    )
}

fn spell_cast_event(
    game: &crate::GameState,
    spell: ObjectId,
    caster: PlayerId,
) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell).expect("the cast spell should exist"),
        game,
    );
    trigger_event(crate::events::spells::SpellCastEvent::new_with_snapshot(
        spell,
        caster,
        Zone::Hand,
        snapshot,
    ))
}

fn matching_entries(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn resolve_entries_with(
    game: &mut crate::GameState,
    entries: Vec<crate::triggers::TriggeredAbilityEntry>,
    decisions: &mut impl DecisionMaker,
) {
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, decisions)
        .expect("the matching trigger should go on the stack");
    while !game.stack_is_empty() {
        crate::game_loop::resolve_stack_entry_with(game, decisions)
            .expect("the matching trigger should resolve");
    }
}

fn finish_pending_costs(
    game: &mut crate::GameState,
    queue: &mut crate::triggers::TriggerQueue,
    state: &mut crate::game_loop::PriorityLoopState,
    mut progress: crate::decision::GameProgress,
    decisions: &mut impl DecisionMaker,
    preferred_object: Option<ObjectId>,
) {
    for _ in 0..16 {
        if !game.stack_is_empty() {
            return;
        }
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option = ctx
                    .options
                    .iter()
                    .find(|option| option.legal)
                    .expect("cost payment should offer a legal option")
                    .index;
                assert!(
                    ctx.description
                        .to_ascii_lowercase()
                        .starts_with("choose the next cost to pay")
                );
                let response = crate::game_loop::PriorityResponse::NextCostChoice(option);
                crate::game_loop::apply_priority_response_with_dm(
                    game, queue, state, &response, decisions,
                )
                .expect("cost option should be accepted")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::ManaPayment(ctx),
            ) => crate::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                state,
                &crate::game_loop::PriorityResponse::ManaPaymentPlan(
                    crate::mana_payment::ManaPaymentResponse::Confirm {
                        plan_id: ctx.plan.id,
                        request_hash: ctx.plan.request_hash,
                    },
                ),
                decisions,
            )
            .expect("mana plan should be accepted"),
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(ctx),
            ) => {
                let object = preferred_object
                    .filter(|preferred| {
                        ctx.candidates
                            .iter()
                            .any(|candidate| candidate.legal && candidate.id == *preferred)
                    })
                    .unwrap_or_else(|| {
                        ctx.candidates
                            .iter()
                            .find(|candidate| candidate.legal)
                            .expect("cost payment should offer a legal object")
                            .id
                    });
                let response = if ctx.description.to_ascii_lowercase().contains("sacrifice") {
                    crate::game_loop::PriorityResponse::SacrificeTarget(object)
                } else {
                    crate::game_loop::PriorityResponse::CardCostChoice(object)
                };
                crate::game_loop::apply_priority_response_with_dm(
                    game, queue, state, &response, decisions,
                )
                .expect("cost object should be accepted")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            )
            | crate::decision::GameProgress::Continue
            | crate::decision::GameProgress::StackResolved => return,
            other => panic!("unexpected cost-payment state: {other:?}"),
        };
    }
    panic!("cost payment did not finish after repeated decisions");
}

#[derive(Default)]
struct NamedDecisionMaker {
    option: Option<&'static str>,
    target: Option<ObjectId>,
    accept_optional: bool,
    target_contexts: Vec<crate::decisions::context::TargetsContext>,
}

impl DecisionMaker for NamedDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept_optional
    }

    fn decide_options(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(desired) = self.option
            && let Some(option) = ctx.options.iter().find(|option| {
                option.legal
                    && option
                        .description
                        .to_ascii_lowercase()
                        .contains(&desired.to_ascii_lowercase())
            })
        {
            return vec![option.index];
        }
        if let Some(desired) = self.option
            && ["Food", "Treasure"]
                .iter()
                .any(|mode| desired.eq_ignore_ascii_case(mode))
            && ctx.options.iter().filter(|option| option.legal).count() == 2
        {
            let position = usize::from(desired.eq_ignore_ascii_case("Treasure"));
            if let Some(option) = ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .nth(position)
            {
                return vec![option.index];
            }
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }

    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if let Some(target) = self.target
            && ctx
                .candidates
                .iter()
                .any(|candidate| candidate.legal && candidate.id == target)
        {
            return vec![target];
        }
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }

    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.target_contexts.push(ctx.clone());
        ctx.requirements
            .iter()
            .filter_map(|requirement| {
                self.target
                    .map(Target::Object)
                    .filter(|target| requirement.legal_targets.contains(target))
                    .or_else(|| requirement.legal_targets.first().copied())
            })
            .collect()
    }
}

#[test]
fn slitherwisp_requires_another_flash_spell_cast_by_its_controller_and_hits_each_opponent() {
    let definition = parse_oracle_card_definition("Slitherwisp");
    let mut game = three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let self_event = spell_cast_event(&game, source, alice);
    assert!(
        matching_entries(&game, source, &self_event).is_empty(),
        "another must exclude Slitherwisp itself"
    );

    let nonflash = game.create_object_from_definition(
        &flash_spell("Controller Nonflash Spell", false),
        alice,
        Zone::Stack,
    );
    assert!(
        matching_entries(&game, source, &spell_cast_event(&game, nonflash, alice)).is_empty(),
        "the controller's spell still has to have flash"
    );

    let opponent_flash = game.create_object_from_definition(
        &flash_spell("Opponent Flash Spell", true),
        bob,
        Zone::Stack,
    );
    assert!(
        matching_entries(&game, source, &spell_cast_event(&game, opponent_flash, bob)).is_empty(),
        "an opponent casting a flash spell must not trigger Slitherwisp"
    );

    let draw_card = game.create_object_from_definition(
        &permanent("Slitherwisp Draw", CardType::Land),
        alice,
        Zone::Library,
    );
    let draw_stable = game.object(draw_card).expect("draw card").stable_id;
    let controlled_flash = game.create_object_from_definition(
        &flash_spell("Controller Flash Spell", true),
        alice,
        Zone::Stack,
    );
    let entries = matching_entries(
        &game,
        source,
        &spell_cast_event(&game, controlled_flash, alice),
    );
    assert_eq!(entries.len(), 1);
    resolve_entries_with(
        &mut game,
        entries,
        &mut crate::decision::SelectFirstDecisionMaker,
    );

    assert_eq!(game.life_total(alice), 20);
    assert_eq!(game.life_total(bob), 19);
    assert_eq!(game.life_total(charlie), 19);
    assert_eq!(
        game.find_object_by_stable_id(draw_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Hand),
        "Slitherwisp's controller should draw exactly the prepared card"
    );
}

fn run_tireless_provisioner_mode(mode: &'static str) {
    let definition = parse_oracle_card_definition("Tireless Provisioner");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let nonland = game.create_object_from_definition(
        &permanent("Provisioner Nonland", CardType::Artifact),
        alice,
        Zone::Hand,
    );
    game.move_object_with_etb_processing(nonland, Zone::Battlefield)
        .expect("the controlled nonland should enter");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert!(
        queue.entries.iter().all(|entry| entry.source != source),
        "a controlled nonland must not produce landfall"
    );

    let opponent_land = game.create_object_from_definition(
        &permanent("Opponent Provisioner Land", CardType::Land),
        bob,
        Zone::Hand,
    );
    game.move_object_with_etb_processing(opponent_land, Zone::Battlefield)
        .expect("the opponent land should enter");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    assert!(
        queue.entries.iter().all(|entry| entry.source != source),
        "an opponent's land must not produce this landfall trigger"
    );

    let controlled_land = game.create_object_from_definition(
        &permanent("Controlled Provisioner Land", CardType::Land),
        alice,
        Zone::Hand,
    );
    game.move_object_with_etb_processing(controlled_land, Zone::Battlefield)
        .expect("the controlled land should enter");
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    queue.entries.retain(|entry| entry.source == source);
    assert_eq!(queue.entries.len(), 1);
    let mut decisions = NamedDecisionMaker {
        option: Some(mode),
        ..Default::default()
    };
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Provisioner's landfall trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Provisioner's chosen token mode should resolve");

    let tokens = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == crate::object::ObjectKind::Token
                    && object.name == mode
                    && game.controller_of(object) == alice
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        tokens.len(),
        1,
        "the selected {mode} mode must create one token"
    );
    let token = game.object(tokens[0]).expect("created token should exist");
    assert!(token.card_types.contains(&CardType::Artifact));
    let activated = token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the created {mode} must retain its activated ability"));
    let ability_debug = format!("{:#?}", token.abilities);
    assert!(
        activated
            .mana_cost
            .costs()
            .iter()
            .any(crate::costs::Cost::is_sacrifice_self),
        "the created {mode} must retain its sacrifice ability: {ability_debug}"
    );
    if mode == "Treasure" {
        assert!(ability_debug.contains("AddManaOfAnyColorEffect"));
    } else {
        assert!(ability_debug.contains("GainLifeEffect"));
    }
    let other_mode = if mode == "Food" { "Treasure" } else { "Food" };
    assert!(
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .all(|id| {
                game.object(id).is_none_or(|object| {
                    object.kind != crate::object::ObjectKind::Token || object.name != other_mode
                })
            })
    );

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    if mode == "Food" {
        game.player_mut(alice)
            .expect("Alice")
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
    }
    let token_id = tokens[0];
    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| match action {
            crate::decision::LegalAction::ActivateAbility { source, .. }
                if mode == "Food" && *source == token_id =>
            {
                true
            }
            crate::decision::LegalAction::ActivateManaAbility { source, .. }
                if mode == "Treasure" && *source == token_id =>
            {
                true
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("the created {mode}'s intrinsic ability should be legal"));
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut activation_decisions = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut activation_decisions,
    )
    .unwrap_or_else(|error| panic!("the created {mode}'s intrinsic ability should start: {error}"));
    finish_pending_costs(
        &mut game,
        &mut queue,
        &mut state,
        progress,
        &mut activation_decisions,
        Some(token_id),
    );
    assert!(
        !game.battlefield.contains(&token_id),
        "the created {mode} should be sacrificed as an activation cost"
    );
    if mode == "Food" {
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut activation_decisions)
            .expect("Food's life-gain ability should resolve");
        assert_eq!(game.life_total(alice), 23);
        assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);
    } else {
        assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 1);
    }
}

#[test]
fn tireless_provisioner_landfall_offers_and_executes_each_exact_token_mode() {
    run_tireless_provisioner_mode("Food");
    run_tireless_provisioner_mode("Treasure");
}

#[test]
fn tribal_forcemage_uses_the_source_face_up_event_and_pumps_all_creatures_of_the_chosen_type() {
    let mut definition = parse_oracle_card_definition("Tribal Forcemage");
    definition.card.power_toughness = Some(PowerToughness::fixed(1, 1));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let own_elf = game.create_object_from_definition(
        &creature("Own Elf", 2, 2, [Subtype::Elf]),
        alice,
        Zone::Battlefield,
    );
    let opponent_elf = game.create_object_from_definition(
        &creature("Opponent Elf", 2, 2, [Subtype::Elf]),
        bob,
        Zone::Battlefield,
    );
    let goblin = game.create_object_from_definition(
        &creature("Unchosen Goblin", 2, 2, [Subtype::Goblin]),
        alice,
        Zone::Battlefield,
    );
    let other = game.create_object_from_definition(
        &creature("Other Face-Up Creature", 2, 2, []),
        alice,
        Zone::Battlefield,
    );

    let other_event = trigger_event(crate::events::TurnedFaceUpEvent::new(other, alice));
    assert!(matching_entries(&game, source, &other_event).is_empty());

    let event = trigger_event(crate::events::TurnedFaceUpEvent::new(source, alice));
    let entries = matching_entries(&game, source, &event);
    assert_eq!(entries.len(), 1);
    let mut decisions = NamedDecisionMaker {
        option: Some("Elf"),
        ..Default::default()
    };
    resolve_entries_with(&mut game, entries, &mut decisions);
    game.refresh_continuous_state();

    assert_eq!(game.current_power(source), Some(3));
    assert_eq!(game.current_power(own_elf), Some(4));
    assert_eq!(game.current_power(opponent_elf), Some(4));
    assert_eq!(game.current_power(goblin), Some(2));
    for elf in [source, own_elf, opponent_elf] {
        assert!(game.object_has_static_ability_id(elf, StaticAbilityId::Trample));
    }
    assert!(!game.object_has_static_ability_id(goblin, StaticAbilityId::Trample));

    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(game.current_power(source), Some(1));
    assert_eq!(game.current_power(own_elf), Some(2));
    assert_eq!(game.current_power(opponent_elf), Some(2));
    assert!(!game.object_has_static_ability_id(own_elf, StaticAbilityId::Trample));
}

fn unassuming_sage_case(accept_optional: bool, available_mana: u32) {
    let mut definition = parse_oracle_card_definition("Unassuming Sage");
    definition.card.power_toughness = Some(PowerToughness::fixed(2, 2));
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, available_mana);
    let in_hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let entered = game
        .move_object_with_etb_processing(in_hand, Zone::Battlefield)
        .expect("Unassuming Sage should enter")
        .new_id;
    let mut queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
    queue.entries.retain(|entry| entry.source == entered);
    assert_eq!(queue.entries.len(), 1);
    let mut decisions = NamedDecisionMaker {
        accept_optional,
        ..Default::default()
    };
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Sage's ETB trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Sage's ETB trigger should resolve");

    let roles = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Sorcerer Role")
        })
        .collect::<Vec<_>>();
    let should_create = accept_optional && available_mana >= 2;
    assert_eq!(roles.len(), usize::from(should_create));
    if should_create {
        let role = game.object(roles[0]).expect("Sorcerer Role should exist");
        assert_eq!(game.controller_of(role), alice);
        assert_eq!(
            role.attached_to,
            Some(crate::object::AttachmentTarget::Object(entered)),
            "the Role must attach to the exact Sage that triggered"
        );
        game.refresh_continuous_state();
        assert_eq!(game.current_power(entered), Some(3));
        assert_eq!(game.current_toughness(entered), Some(3));
        assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 0);

        let bob = PlayerId::from_index(1);
        game.create_object_from_definition(
            &permanent("Sorcerer Role Scry Card", CardType::Land),
            alice,
            Zone::Library,
        );
        let attack_event = trigger_event(crate::events::combat::CreatureAttackedEvent::new(
            entered,
            crate::events::combat::AttackEventTarget::Player(bob),
        ));
        let entries = matching_entries(&game, entered, &attack_event);
        assert_eq!(
            entries.len(),
            1,
            "the attached Sorcerer Role should grant Sage one attack trigger"
        );
        assert!(format!("{:#?}", entries[0].ability.effects).contains("ScryEffect"));
        resolve_entries_with(
            &mut game,
            entries,
            &mut crate::decision::SelectFirstDecisionMaker,
        );
    } else {
        assert_eq!(game.current_power(entered), Some(2));
        assert_eq!(
            game.player(alice).expect("Alice").mana_pool.total(),
            available_mana
        );
    }
}

#[test]
fn unassuming_sage_pays_only_when_accepted_and_attaches_its_role_to_the_triggering_source() {
    unassuming_sage_case(false, 2);
    unassuming_sage_case(true, 0);
    unassuming_sage_case(true, 2);
}

#[test]
fn wall_of_corpses_can_target_only_the_attacker_it_blocks_and_resolves_from_lki() {
    let definition = parse_oracle_card_definition("Wall of Corpses");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareBlockers);
    game.player_mut(alice)
        .expect("Alice")
        .mana_pool
        .add(ManaSymbol::Black, 1);

    let wall = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let wall_stable = game.object(wall).expect("Wall").stable_id;
    let blocked_attacker = game.create_object_from_definition(
        &creature("Wall-Blocked Attacker", 3, 3, []),
        bob,
        Zone::Battlefield,
    );
    let blocked_stable = game.object(blocked_attacker).expect("attacker").stable_id;
    let unrelated_attacker = game.create_object_from_definition(
        &creature("Unrelated Attacker", 3, 3, []),
        bob,
        Zone::Battlefield,
    );
    let other_blocker = game.create_object_from_definition(
        &creature("Other Blocker", 2, 2, []),
        alice,
        Zone::Battlefield,
    );
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![
            crate::combat_state::AttackerInfo {
                creature: blocked_attacker,
                target: crate::combat_state::AttackTarget::Player(alice),
            },
            crate::combat_state::AttackerInfo {
                creature: unrelated_attacker,
                target: crate::combat_state::AttackTarget::Player(alice),
            },
        ],
        blockers: std::collections::HashMap::from([
            (blocked_attacker, vec![wall]),
            (unrelated_attacker, vec![other_blocker]),
        ]),
        ..Default::default()
    });

    let ability_index = game
        .object(wall)
        .expect("Wall")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Wall should have an activated ability");
    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source,
                    ability_index: index,
                } if *source == wall && *index == ability_index
            )
        })
        .expect("Wall's activation should be legal while it blocks a creature");
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut decisions,
    )
    .expect("Wall's activation should ask for its target");
    let crate::decision::GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Targets(targets),
    ) = progress
    else {
        panic!("Wall should request one target: {progress:?}");
    };
    let legal = &targets.requirements[0].legal_targets;
    assert!(legal.contains(&Target::Object(blocked_attacker)));
    assert!(!legal.contains(&Target::Object(unrelated_attacker)));
    assert!(!legal.contains(&Target::Object(other_blocker)));

    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::Targets(vec![Target::Object(blocked_attacker)]),
        &mut decisions,
    )
    .expect("Wall's exact blocked target should be accepted");
    finish_pending_costs(
        &mut game,
        &mut queue,
        &mut state,
        progress,
        &mut decisions,
        Some(wall),
    );
    assert_eq!(
        game.find_object_by_stable_id(wall_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Graveyard),
        "sacrificing the Wall is an activation cost"
    );
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("the ability should resolve using the sacrificed Wall's LKI");
    assert_eq!(
        game.find_object_by_stable_id(blocked_stable)
            .and_then(|id| game.object(id))
            .map(|object| object.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(
        game.object(unrelated_attacker).expect("unrelated").zone,
        Zone::Battlefield
    );
}

fn tezzeret_definition() -> CardDefinition {
    let mut definition = parse_oracle_card_definition("Tezzeret the Schemer");
    definition.card.loyalty = Some(5);
    definition
}

fn main_phase_game() -> (crate::GameState, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    (game, alice)
}

fn activate_tezzeret(
    game: &mut crate::GameState,
    controller: PlayerId,
    source: ObjectId,
    ability_index: usize,
    target: Option<ObjectId>,
) -> Vec<Target> {
    let action = crate::decision::compute_legal_actions(game, controller)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility {
                    source: candidate,
                    ability_index: candidate_index,
                } if *candidate == source && *candidate_index == ability_index
            )
        })
        .unwrap_or_else(|| panic!("Tezzeret loyalty ability {ability_index} should be legal"));
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut decisions = crate::decision::AutoPassDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut decisions,
    )
    .expect("Tezzeret's loyalty activation should start");
    let mut legal_targets = Vec::new();
    if let Some(target) = target {
        let crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(ctx),
        ) = progress
        else {
            panic!("targeted Tezzeret ability should request a target: {progress:?}");
        };
        legal_targets = ctx.requirements[0].legal_targets.clone();
        crate::game_loop::apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &crate::game_loop::PriorityResponse::Targets(vec![Target::Object(target)]),
            &mut decisions,
        )
        .expect("Tezzeret's target should be accepted");
    }
    crate::game_loop::resolve_stack_entry_with(game, &mut decisions)
        .expect("Tezzeret's loyalty ability should resolve");
    legal_targets
}

#[test]
fn tezzeret_plus_one_creates_an_executable_etherium_cell_mana_token() {
    let definition = tezzeret_definition();
    let (mut game, alice) = main_phase_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    activate_tezzeret(&mut game, alice, source, 0, None);
    assert_eq!(game.counter_count(source, CounterType::Loyalty), 6);

    let cell = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Etherium Cell")
        })
        .expect("Tezzeret's +1 should create an Etherium Cell");
    let cell_object = game.object(cell).expect("Etherium Cell");
    assert_eq!(cell_object.kind, crate::object::ObjectKind::Token);
    assert_eq!(game.controller_of(cell_object), alice);
    assert!(cell_object.card_types.contains(&CardType::Artifact));

    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateManaAbility { source, .. }
                    if *source == cell
            )
        })
        .expect("Etherium Cell's tap-and-sacrifice mana ability should be legal");
    let mut queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(action),
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Etherium Cell's mana ability should resolve immediately");
    assert!(!game.battlefield.contains(&cell));
    assert_eq!(game.player(alice).expect("Alice").mana_pool.total(), 1);
}

#[test]
fn tezzeret_minus_two_counts_only_its_controllers_artifacts_for_the_exact_target() {
    let definition = tezzeret_definition();
    let (mut game, alice) = main_phase_game();
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    for index in 0..3 {
        game.create_object_from_definition(
            &permanent(&format!("Controlled Artifact {index}"), CardType::Artifact),
            alice,
            Zone::Battlefield,
        );
    }
    game.create_object_from_definition(
        &permanent("Opponent Artifact", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let target = game.create_object_from_definition(
        &creature("Tezzeret Animation Target", 2, 10, []),
        bob,
        Zone::Battlefield,
    );
    let legal = activate_tezzeret(&mut game, alice, source, 1, Some(target));
    assert!(legal.contains(&Target::Object(target)));
    assert_eq!(game.current_power(target), Some(5));
    assert_eq!(game.current_toughness(target), Some(7));
    assert_eq!(game.counter_count(source, CounterType::Loyalty), 3);
}

#[test]
fn tezzeret_minus_seven_emblem_triggers_only_on_its_turn_and_animates_only_a_controlled_artifact() {
    let definition = tezzeret_definition();
    let (mut game, alice) = main_phase_game();
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.add_counters(source, CounterType::Loyalty, 2);
    activate_tezzeret(&mut game, alice, source, 2, None);

    let emblem = game
        .objects_in_zone(Zone::Command)
        .into_iter()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.kind == crate::object::ObjectKind::Emblem)
        })
        .expect("Tezzeret's ultimate should create an emblem in the command zone");
    assert_eq!(game.current_controller(emblem), Some(alice));

    let controlled_artifact = game.create_object_from_definition(
        &permanent("Controlled Emblem Artifact", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );
    let opponent_artifact = game.create_object_from_definition(
        &permanent("Opponent Emblem Artifact", CardType::Artifact),
        bob,
        Zone::Battlefield,
    );
    let controlled_nonartifact = game.create_object_from_definition(
        &creature("Controlled Emblem Nonartifact", 2, 2, []),
        alice,
        Zone::Battlefield,
    );

    let opponent_combat = trigger_event(crate::events::BeginningOfCombatEvent::new(bob));
    assert!(matching_entries(&game, emblem, &opponent_combat).is_empty());
    let own_combat = trigger_event(crate::events::BeginningOfCombatEvent::new(alice));
    let entries = matching_entries(&game, emblem, &own_combat);
    assert_eq!(entries.len(), 1);
    let mut decisions = NamedDecisionMaker {
        target: Some(controlled_artifact),
        ..Default::default()
    };
    resolve_entries_with(&mut game, entries, &mut decisions);
    let target_context = decisions
        .target_contexts
        .last()
        .expect("the emblem should request its artifact target");
    let legal = &target_context.requirements[0].legal_targets;
    assert!(legal.contains(&Target::Object(controlled_artifact)));
    assert!(!legal.contains(&Target::Object(opponent_artifact)));
    assert!(!legal.contains(&Target::Object(controlled_nonartifact)));

    game.refresh_continuous_state();
    assert!(game.current_has_card_type(controlled_artifact, CardType::Artifact));
    assert!(game.current_has_card_type(controlled_artifact, CardType::Creature));
    assert_eq!(game.current_power(controlled_artifact), Some(5));
    assert_eq!(game.current_toughness(controlled_artifact), Some(5));
    assert!(!game.current_has_card_type(opponent_artifact, CardType::Creature));
}
