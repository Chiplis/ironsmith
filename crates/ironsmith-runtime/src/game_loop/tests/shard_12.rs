#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn howl_of_the_horde_without_raid_schedules_one_one_shot_copy_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    execute_howl_of_the_horde_spell_effect(&mut game, false);
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        1,
        "Howl should schedule only the base next-spell trigger without raid"
    );
    assert!(
        game.effect_store.delayed_triggers[0].one_shot,
        "Howl's next-spell trigger should be one-shot"
    );

    let spell_id = create_howl_target_spell_on_stack(&mut game);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "base Howl trigger should fire once"
    );
    assert!(
        game.effect_store.delayed_triggers.is_empty(),
        "one-shot Howl trigger should be consumed after the next cast"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("put Howl trigger on stack");
    resolve_stack_entry(&mut game).expect("resolve Howl delayed copy trigger");
    assert_eq!(
        game.stack.len(),
        2,
        "resolving the base Howl trigger should leave the original spell plus one copy on the stack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn howl_of_the_horde_with_raid_schedules_two_one_shot_copy_triggers() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    execute_howl_of_the_horde_spell_effect(&mut game, true);
    assert_eq!(
        game.effect_store.delayed_triggers.len(),
        2,
        "Howl should schedule the base and raid next-spell triggers after attacking"
    );
    assert!(
        game.effect_store
            .delayed_triggers
            .iter()
            .all(|trigger| trigger.one_shot),
        "both Howl delayed triggers should be one-shot"
    );

    let spell_id = create_howl_target_spell_on_stack(&mut game);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        2,
        "base and raid Howl triggers should both fire on the next instant or sorcery"
    );
    assert!(
        game.effect_store.delayed_triggers.is_empty(),
        "both one-shot Howl triggers should be consumed after the next cast"
    );
}

#[test]
pub(super) fn test_prototyped_spell_on_stack_snapshots_with_prototype_mana_value() {
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 3);

    let prototype_def = CardDefinitionBuilder::new(CardId::new(), "Prototype Runtime Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(6, 4))
        .parse_text("Prototype {2}{R} — 3/2\nHaste")
        .expect("prototype text should parse");
    let prototype_id = game.create_object_from_definition(&prototype_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: prototype_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("prototype cast should succeed");

    let stack_id = game
        .stack
        .last()
        .expect("prototype spell should be on the stack")
        .object_id;
    let stack_object = game
        .object(stack_id)
        .expect("prototype stack object should exist");
    assert_eq!(
        stack_object.mana_cost.as_deref().map(ManaCost::mana_value),
        Some(3),
        "prototyped spell object should have its prototype mana cost on the stack"
    );
    let snapshot = ObjectSnapshot::from_object(stack_object, &game);
    assert_eq!(
        snapshot.mana_value(),
        3,
        "stack LKI snapshot should use the prototype mana value"
    );

    let tagged_counter =
        Effect::counter(crate::target::ChooseSpec::target_spell()).tag("countered_0");
    let mut ctx = crate::effects::ExecutionContext::new_default(stack_id, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(stack_id)]);
    crate::effects::execute_effect(&mut game, &tagged_counter, &mut ctx)
        .expect("tagged counter should resolve");
    let tagged_mana_value = crate::effects::helpers::resolve_value(
        &game,
        &Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(
            crate::TagKey::from("countered_0"),
        ))),
        &ctx,
    )
    .expect("countered tag mana value should resolve");
    assert_eq!(
        tagged_mana_value, 3,
        "countered prototyped spell LKI should keep the prototype mana value"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_dash_cost_reduction_applies_only_to_dash_casts() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let warbringer_def = CardDefinitionBuilder::new(CardId::new(), "Warbringer Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Dash costs you pay cost {2} less (as long as this creature is on the battlefield).\nDash {2}{R}",
        )
        .expect("Warbringer-style text should parse");
    game.create_object_from_definition(&warbringer_def, alice, Zone::Battlefield);

    let dash_probe = CardDefinitionBuilder::new(CardId::new(), "Dash Discount Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .dash(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .build();
    let dash_probe_id = game.create_object_from_definition(&dash_probe, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0),
            } if *spell_id == dash_probe_id
        )),
        "expected dash cast to be legal with only the reduced mana available"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == dash_probe_id
        )),
        "normal cast should still be unaffordable"
    );

    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: dash_probe_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("reduced dash cast should succeed");
    resolve_stack_entry(&mut game).expect("reduced dash spell should resolve");

    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Dash Discount Probe")
        }),
        "expected dashed creature to resolve onto the battlefield"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.total(),
        0,
        "the reduced dash cast should spend only the single red mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_auriok_steelshaper_reduces_only_your_equip_costs() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let alice_creature = create_creature(&mut game, "Alice Target", alice, 2, 2);

    let equipment_def = CardDefinitionBuilder::new(CardId::new(), "Equip Probe")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text("Equip {1}")
        .expect("equip probe should parse");
    let alice_equipment =
        game.create_object_from_definition(&equipment_def, alice, Zone::Battlefield);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let steelshaper_def = CardDefinitionBuilder::new(CardId::new(), "Auriok Steelshaper")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Equip costs you pay cost {1} less.\nAs long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1.",
        )
        .expect("Auriok Steelshaper should parse");
    game.create_object_from_definition(&steelshaper_def, alice, Zone::Battlefield);

    let activate_with = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == alice_equipment
            )
        })
        .expect("equip action should be legal with Auriok Steelshaper in play");
    assert!(matches!(activate_with, LegalAction::ActivateAbility { .. }));

    assert!(game.object(alice_creature).is_some());
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_auriok_steelshaper_anthem_requires_being_equipped_and_only_buffs_soldiers_or_knights()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let steelshaper_def = CardDefinitionBuilder::new(CardId::new(), "Auriok Steelshaper")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Equip costs you pay cost {1} less.\nAs long as this creature is equipped, each creature you control that's a Soldier or a Knight gets +1/+1.",
        )
        .expect("Auriok Steelshaper should parse");
    let steelshaper_id =
        game.create_object_from_definition(&steelshaper_def, alice, Zone::Battlefield);

    let soldier_id = CardDefinitionBuilder::new(CardId::new(), "Soldier Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let soldier_id = game.create_object_from_definition(&soldier_id, alice, Zone::Battlefield);

    let knight_id = CardDefinitionBuilder::new(CardId::new(), "Knight Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Knight])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let knight_id = game.create_object_from_definition(&knight_id, alice, Zone::Battlefield);

    let bear_id = create_creature(&mut game, "Bear Probe", alice, 2, 2);

    let before_soldier = game
        .calculated_characteristics(soldier_id)
        .expect("soldier should have calculated characteristics");
    let before_knight = game
        .calculated_characteristics(knight_id)
        .expect("knight should have calculated characteristics");
    let before_bear = game
        .calculated_characteristics(bear_id)
        .expect("bear should have calculated characteristics");
    assert_eq!(
        (before_soldier.power, before_soldier.toughness),
        (Some(2), Some(2))
    );
    assert_eq!(
        (before_knight.power, before_knight.toughness),
        (Some(2), Some(2))
    );
    assert_eq!(
        (before_bear.power, before_bear.toughness),
        (Some(2), Some(2))
    );

    let equipment_def = CardDefinitionBuilder::new(CardId::new(), "Steelshaper Gear")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text("Equip {0}")
        .expect("equipment should parse");
    let equipment_id = game.create_object_from_definition(&equipment_def, alice, Zone::Battlefield);

    if let Some(equipment) = game.object_mut(equipment_id) {
        equipment.attached_to = Some(crate::object::AttachmentTarget::Object(steelshaper_id));
    }
    if let Some(steelshaper) = game.object_mut(steelshaper_id) {
        steelshaper.attachments.push(equipment_id);
    }

    let after_steelshaper = game
        .calculated_characteristics(steelshaper_id)
        .expect("steelshaper should have calculated characteristics while equipped");
    let after_soldier = game
        .calculated_characteristics(soldier_id)
        .expect("soldier should have calculated characteristics while steelshaper is equipped");
    let after_knight = game
        .calculated_characteristics(knight_id)
        .expect("knight should have calculated characteristics while steelshaper is equipped");
    let after_bear = game
        .calculated_characteristics(bear_id)
        .expect("bear should have calculated characteristics while steelshaper is equipped");

    assert_eq!(
        (after_steelshaper.power, after_steelshaper.toughness),
        (Some(2), Some(2)),
        "equipped Auriok Steelshaper is a Soldier and should buff itself"
    );
    assert_eq!(
        (after_soldier.power, after_soldier.toughness),
        (Some(3), Some(3)),
        "Soldiers should get +1/+1 once Auriok Steelshaper is equipped"
    );
    assert_eq!(
        (after_knight.power, after_knight.toughness),
        (Some(3), Some(3)),
        "Knights should get +1/+1 once Auriok Steelshaper is equipped"
    );
    assert_eq!(
        (after_bear.power, after_bear.toughness),
        (Some(2), Some(2)),
        "non-Soldier and non-Knight creatures should not get the anthem"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_robe_of_the_archmagi_equip_branches_and_damage_trigger() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let robe_def = CardDefinitionBuilder::new(CardId::new(), "Robe of the Archmagi")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Whenever equipped creature deals combat damage to a player, you draw that many cards.\n\
             Equip {4}\n\
             Equip Shaman, Warlock, or Wizard {1}",
        )
        .expect("Robe of the Archmagi should parse");
    let robe_id = game.create_object_from_definition(&robe_def, alice, Zone::Battlefield);

    let bear_id = create_creature(&mut game, "Vanilla Bear", alice, 2, 2);
    let wizard_id = CardBuilder::new(CardId::from_raw(88_001), "Apprentice Wizard")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let wizard_id = game.create_object_from_card(&wizard_id, alice, Zone::Battlefield);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 1);

    let one_mana_actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        one_mana_actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::ActivateAbility {
                source,
                ability_index: idx
            } if *source == robe_id && *idx == 2
        )),
        "expected subtype-qualified equip ability to be legal with one mana when a Wizard is present"
    );
    if let Some(wizard) = game.object_mut(wizard_id) {
        wizard.subtypes.clear();
    }
    let no_wizard_actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        !no_wizard_actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::ActivateAbility {
                source,
                ability_index: idx
            } if *source == robe_id && *idx == 2
        )),
        "expected subtype-qualified equip ability to be unavailable without a Shaman, Warlock, or Wizard"
    );

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    let four_mana_actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        four_mana_actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::ActivateAbility {
                source,
                ability_index: idx
            } if *source == robe_id && *idx == 1
        )),
        "expected base Equip {{4}} ability to be legal with four mana"
    );

    let triggered = robe_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Robe should have its combat-damage trigger");

    for id in [88_010, 88_011, 88_012] {
        let draw_card = CardBuilder::new(CardId::from_raw(id), "Draw Fodder")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&draw_card, alice, Zone::Library);
    }
    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let library_before = game.player(alice).expect("alice exists").library.len();

    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            bear_id,
            crate::events::DamageTarget::Player(bob),
            3,
            true,
            crate::events::cause::EventCause::combat_damage(bear_id),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let mut dm = AutoPassDecisionMaker;
    let mut ctx = ExecutionContext::new_default(robe_id, alice)
        .with_decision_maker(&mut dm)
        .with_triggering_event(damage_event);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Robe trigger should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 3,
        "Robe trigger should draw cards equal to combat damage dealt"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before - 3,
        "Robe trigger should move the same number of cards from library to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_gargoyle_sentinel_gains_flying_only_for_itself_until_end_of_turn() {
    use crate::PriorityResponse;
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::{LegalAction, compute_legal_actions};
    use crate::game_loop::{
        PriorityLoopState, apply_priority_response_with_dm, resolve_stack_entry,
    };

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 3);

    let gargoyle_def = CardDefinitionBuilder::new(CardId::new(), "Gargoyle Sentinel")
        .parse_text(
            "Mana cost: {3}\n\
             Type: Artifact Creature — Gargoyle\n\
             Power/Toughness: 3/3\n\
             Defender (This creature can't attack.)\n\
             {3}: Until end of turn, this creature loses defender and gains flying.",
        )
        .expect("Gargoyle Sentinel should parse");
    let gargoyle_id = game.create_object_from_definition(&gargoyle_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(gargoyle_id);

    let other_creature_id = create_creature(&mut game, "Training Bear", alice, 2, 2);
    game.remove_summoning_sickness(other_creature_id);

    assert!(
        !crate::rules::combat::can_attack(
            game.object(gargoyle_id).expect("gargoyle exists"),
            &game
        ),
        "defender should stop Gargoyle Sentinel from attacking before its ability resolves"
    );
    assert!(
        !game.object_has_ability(gargoyle_id, &StaticAbility::flying()),
        "Gargoyle Sentinel should not start with flying"
    );
    assert!(
        !game.object_has_ability(other_creature_id, &StaticAbility::flying()),
        "the nearby creature should not start with flying"
    );

    let ability_index = game
        .object(gargoyle_id)
        .expect("gargoyle sentinel exists")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Gargoyle Sentinel should have an activated ability");
    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == gargoyle_id && *idx == ability_index
            )
        })
        .expect("Gargoyle Sentinel's activated ability should be legal");

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Gargoyle Sentinel activation should succeed");
    resolve_stack_entry(&mut game).expect("Gargoyle Sentinel ability should resolve");

    assert!(
        !game.object_has_ability(gargoyle_id, &StaticAbility::defender()),
        "defender should be removed until end of turn"
    );
    assert!(
        crate::rules::combat::can_attack(game.object(gargoyle_id).expect("gargoyle exists"), &game),
        "the sentinel should be able to attack after losing defender"
    );
    assert!(
        game.object_has_ability(gargoyle_id, &StaticAbility::flying()),
        "the sentinel should gain flying"
    );
    assert!(
        !game.object_has_ability(other_creature_id, &StaticAbility::flying()),
        "the activated ability should not grant flying to other creatures"
    );

    crate::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    game.next_turn();

    assert!(
        !game.object_has_ability(gargoyle_id, &StaticAbility::flying()),
        "flying should expire at end of turn"
    );
    assert!(
        !crate::rules::combat::can_attack(
            game.object(gargoyle_id).expect("gargoyle exists"),
            &game
        ),
        "defender should come back after end of turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_sacellum_godspeaker_reveals_hand_creatures_and_adds_green_mana() {
    use crate::PriorityResponse;
    use crate::decision::{DecisionMaker, compute_legal_actions};
    use crate::game_loop::{
        PriorityLoopState, apply_priority_response_with_dm, resolve_stack_entry,
    };

    struct ChooseAllLegalObjectsDm;

    impl DecisionMaker for ChooseAllLegalObjectsDm {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let sacellum_def = CardDefinitionBuilder::new(CardId::from_raw(99031), "Sacellum Godspeaker")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Elf, crate::types::Subtype::Druid])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "{T}: Reveal any number of creature cards with power 5 or greater from your hand. Add {G} for each card revealed this way.",
        )
        .expect("Sacellum Godspeaker should parse");
    let sacellum_id = game.create_object_from_definition(&sacellum_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(sacellum_id);

    let qualifying_one = CardBuilder::new(CardId::from_raw(99032), "Hill Giant Hand Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    game.create_object_from_card(&qualifying_one, alice, Zone::Hand);

    let qualifying_two = CardBuilder::new(CardId::from_raw(99033), "Force of Nature Hand Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(8, 8))
        .build();
    game.create_object_from_card(&qualifying_two, alice, Zone::Hand);

    let nonqualifying = CardBuilder::new(CardId::from_raw(99034), "Small Hand Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_card(&nonqualifying, alice, Zone::Hand);

    let ability_index = game
        .object(sacellum_id)
        .expect("Sacellum Godspeaker exists")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Sacellum Godspeaker should have an activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateManaAbility {
                    source,
                    ability_index: idx,
                } if *source == sacellum_id && *idx == ability_index
            )
        })
        .expect("Sacellum Godspeaker activation should be legal");

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ChooseAllLegalObjectsDm;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Sacellum Godspeaker activation should succeed");

    assert!(
        game.is_tapped(sacellum_id),
        "Sacellum Godspeaker should tap as part of its activation cost"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before,
        "revealing cards should not move them out of hand"
    );
    assert_eq!(
        game.stack.len(),
        0,
        "Sacellum Godspeaker is a mana ability and should not use the stack"
    );

    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.green,
        2,
        "Sacellum Godspeaker should add one green mana for each qualifying revealed card"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.total(),
        2,
        "the mana pool should only contain the two green mana from Sacellum Godspeaker"
    );
}

#[test]
pub(super) fn test_nested_mana_effect_without_mana_output_is_mana_ability_action() {
    use crate::decision::compute_legal_actions;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let definition = CardDefinitionBuilder::new(CardId::from_raw(99035), "Nested Mana Probe")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::activated(
            crate::cost::TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            vec![Effect::for_players(
                PlayerFilter::Any,
                vec![Effect::for_each(
                    crate::filter::ObjectFilter::default(),
                    vec![Effect::add_mana(vec![ManaSymbol::Green])],
                )],
            )],
        ))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let actions = compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateManaAbility {
                source: action_source,
                ability_index: 0,
            } if *action_source == source
        )),
        "nested mana-producing effects should be offered as mana abilities: {actions:?}"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::ActivateAbility {
                source: action_source,
                ability_index: 0,
                ..
            } if *action_source == source
        )),
        "nested mana-producing effects should not be offered as responseable activated abilities: {actions:?}"
    );
}

#[test]
pub(super) fn test_suspend_creature_gains_haste_until_control_changes() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let suspend_creature =
        CardDefinitionBuilder::new(CardId::from_raw(99003), "Suspend Creature Probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .suspend(1, ManaCost::new())
            .build();
    let card_id = game.create_object_from_definition(&suspend_creature, alice, Zone::Hand);

    let mut dm = SelectFirstDecisionMaker;
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::Suspend { card_id },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("suspend special action should resolve");

    let mut trigger_queue = TriggerQueue::new();
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("upkeep trigger should go on the stack");

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("suspend upkeep trigger should resolve");
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("last-counter trigger should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("suspend cast trigger should resolve");
    assert_eq!(
        game.stack.len(),
        1,
        "suspend should cast the creature spell"
    );

    resolve_stack_entry(&mut game).expect("suspended creature spell should resolve");

    let creature_id = *game
        .battlefield
        .iter()
        .find(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Suspend Creature Probe")
        })
        .expect("suspended creature should resolve onto the battlefield");

    let has_haste = game
        .current_abilities(creature_id)
        .expect("creature should exist")
        .iter()
        .any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.has_haste())
        });
    assert!(has_haste, "suspended creature should gain haste");

    game.set_current_controller(creature_id, bob);

    let has_haste_after_control_change = game
        .current_abilities(creature_id)
        .expect("creature should still exist")
        .iter()
        .any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.has_haste())
        });
    assert!(
        !has_haste_after_control_change,
        "suspend haste should end once you no longer control the permanent"
    );
}

#[test]
pub(super) fn test_warp_exiles_at_next_end_step_and_grants_play_from_exile() {
    use crate::cards::CardDefinitionBuilder;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 3);

    let warp_def = CardDefinitionBuilder::new(CardId::new(), "Warp Runtime Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .warp(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .build();
    let warp_id = game.create_object_from_definition(&warp_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: warp_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("warp cast should succeed");
    resolve_stack_entry(&mut game).expect("warp spell should resolve");

    let warped_id = *game
        .battlefield
        .iter()
        .find(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Warp Runtime Probe")
        })
        .expect("warped creature should be on battlefield");

    let end_step_event = TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfEndStepEvent::new(game.turn.active_player),
        crate::provenance::ProvNodeId::default(),
    );
    for trigger in crate::triggers::check_delayed_triggers(&mut game, &end_step_event) {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put warp delayed trigger on stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve warp delayed trigger");
    }

    assert!(
        !game.battlefield.contains(&warped_id),
        "warped creature should leave the battlefield at the next end step"
    );
    let exiled_id = *game
        .exile
        .iter()
        .find(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Warp Runtime Probe")
        })
        .expect("warped creature should be exiled");
    assert!(
        !game
            .effect_store
            .grant_registry
            .granted_play_from_for_card(&game, exiled_id, Zone::Exile, alice)
            .is_empty(),
        "warp should grant play permission from exile"
    );
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 3);

    let legal_actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        legal_actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Exile,
                casting_method: CastingMethod::PlayFrom { .. },
            } if *spell_id == exiled_id
        )),
        "warped card should be castable from exile after being exiled"
    );
}

#[test]
pub(super) fn test_next_matching_spell_cost_reduction_is_consumed_by_first_match_only() {
    use crate::cards::CardDefinitionBuilder;
    use crate::triggers::TriggerQueue;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source = game.new_object_id();
    game.add_temporary_spell_cost_reduction(
        alice,
        source,
        crate::target::ObjectFilter::default().with_type(CardType::Creature),
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
        1,
    );

    let instant_def = CardDefinitionBuilder::new(CardId::new(), "Instant Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::draw(1)])
        .build();
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Creature Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let second_creature_def = CardDefinitionBuilder::new(CardId::new(), "Second Creature Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let instant_id = game.create_object_from_definition(&instant_def, alice, Zone::Hand);
    let creature_id = game.create_object_from_definition(&creature_def, alice, Zone::Hand);
    let second_creature_id =
        game.create_object_from_definition(&second_creature_def, alice, Zone::Hand);

    let instant = game.object(instant_id).expect("instant exists");
    let instant_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        instant,
        instant.mana_cost.as_ref().expect("instant mana cost"),
    );
    assert_eq!(
        instant_cost.to_oracle(),
        "{1}",
        "nonmatching spell should not be reduced"
    );

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 6);
    let mut state = PriorityLoopState::new(2);
    let mut trigger_queue = TriggerQueue::new();

    let instant_cast = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: instant_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &instant_cast)
        .expect("instant cast should succeed");
    resolve_stack_entry(&mut game).expect("instant should resolve");

    let creature = game.object(creature_id).expect("creature exists");
    let reduced_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        creature,
        creature.mana_cost.as_ref().expect("creature mana cost"),
    );
    assert_eq!(
        reduced_cost.to_oracle(),
        "{1}",
        "first matching creature spell should be reduced"
    );

    let creature_cast = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: creature_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &creature_cast)
        .expect("creature cast should succeed");
    resolve_stack_entry(&mut game).expect("creature should resolve");

    let second_creature = game
        .object(second_creature_id)
        .expect("second creature exists");
    let full_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        second_creature,
        second_creature
            .mana_cost
            .as_ref()
            .expect("second creature mana cost"),
    );
    assert_eq!(
        full_cost.to_oracle(),
        "{4}",
        "temporary reduction should be consumed by the first matching spell"
    );
}

pub(super) fn defiler_of_instinct_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(466_119), "Defiler of Instinct")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Phyrexian, Subtype::Kavu])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "First strike\n\
             As an additional cost to cast red permanent spells, you may pay 2 life. Those spells cost {R} less to cast if you paid life this way. This effect reduces only the amount of red mana you pay.\n\
             Whenever you cast a red permanent spell, this creature deals 1 damage to any target.",
        )
        .expect("Defiler of Instinct should parse for runtime tests")
}

pub(super) fn one_red_creature_definition(name: &str) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

pub(super) fn start_defiler_red_creature_cast(
    game: &mut GameState,
    creature_id: ObjectId,
) -> (
    TriggerQueue,
    PriorityLoopState,
    crate::decisions::context::SelectOptionsContext,
) {
    let alice = PlayerId::from_index(0);
    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(crate::decision::LegalAction::CastSpell {
            spell_id: creature_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        &mut dm,
    )
    .expect("red creature cast should start");

    let ctx = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ) => ctx,
        other => panic!("expected Defiler optional-cost prompt, got {other:?}"),
    };
    assert_eq!(ctx.player, alice);
    (trigger_queue, state, ctx)
}

#[test]
pub(super) fn defiler_of_instinct_optional_life_cost_reduces_red_permanent_spell_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let defiler = defiler_of_instinct_definition();
    game.create_object_from_definition(&defiler, alice, Zone::Battlefield);
    let red_creature = one_red_creature_definition("Defiler of Instinct Red Creature Probe");
    let red_creature_id = game.create_object_from_definition(&red_creature, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let (mut trigger_queue, mut state, optional_ctx) =
        start_defiler_red_creature_cast(&mut game, red_creature_id);
    assert!(
        optional_ctx.options.iter().any(|option| {
            option
                .description
                .to_ascii_lowercase()
                .contains("as an additional cost to cast red permanent spells, you may pay 2 life")
        }),
        "Defiler should offer its optional life additional cost, got {:?}",
        optional_ctx.options
    );

    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::OptionalCosts(vec![(0, 1)]),
        &mut dm,
    )
    .expect("paying Defiler optional life cost should continue casting");

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        18,
        "paying the Defiler additional cost should cost 2 life"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.total(),
        1,
        "paying life should reduce the red pip and leave the red mana unspent"
    );
}

#[test]
pub(super) fn defiler_of_instinct_declining_life_cost_does_not_reduce_red_permanent_spell_cost() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let defiler = defiler_of_instinct_definition();
    game.create_object_from_definition(&defiler, alice, Zone::Battlefield);
    let red_creature = one_red_creature_definition("Defiler of Instinct Decline Probe");
    let red_creature_id = game.create_object_from_definition(&red_creature, alice, Zone::Hand);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    let (mut trigger_queue, mut state, _optional_ctx) =
        start_defiler_red_creature_cast(&mut game, red_creature_id);
    let mut dm = SelectFirstDecisionMaker;
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::OptionalCosts(vec![]),
        &mut dm,
    )
    .expect("declining Defiler optional life cost should continue casting");

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        20,
        "declining the Defiler additional cost should not cost life"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.total(),
        0,
        "declining life should leave the red pip unreduced and spend the red mana"
    );
}

#[test]
pub(super) fn defiler_of_instinct_life_cost_makes_red_permanent_spell_legal_without_red_mana() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let defiler = defiler_of_instinct_definition();
    game.create_object_from_definition(&defiler, alice, Zone::Battlefield);
    let red_creature = one_red_creature_definition("Defiler of Instinct Legal Action Probe");
    let red_creature_id = game.create_object_from_definition(&red_creature, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            crate::decision::LegalAction::CastSpell { spell_id, from_zone: Zone::Hand, .. }
                if *spell_id == red_creature_id
        )),
        "Defiler's optional life additional cost should make a one-red permanent spell castable with no red mana, got {actions:?}"
    );
}

#[test]
pub(super) fn commander_liara_portyr_runtime_reduces_each_exiled_spell_by_attacked_players() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: game.new_object_id(),
        target: AttackTarget::Player(bob),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: game.new_object_id(),
        target: AttackTarget::Player(charlie),
    });
    game.combat = Some(combat);

    let source = game.new_object_id();
    let grant_reduction = Effect::new(
        crate::effects::GrantNextSpellCostReductionEffect::all_matching_this_turn(
            PlayerFilter::You,
            crate::target::ObjectFilter::default()
                .in_zone(Zone::Exile)
                .cast_by_you(),
            Value::PlayersBeingAttacked,
        ),
    );
    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_effect(&mut game, &grant_reduction, &mut ctx)
        .expect("Commander Liara Portyr reduction grant should resolve");
    game.combat = None;

    let first_exiled =
        CardDefinitionBuilder::new(CardId::new(), "Commander Liara Portyr Exiled Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::draw(1)])
            .build();
    let second_exiled =
        CardDefinitionBuilder::new(CardId::new(), "Commander Liara Portyr Second Exiled Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::draw(1)])
            .build();
    let hand_spell = CardDefinitionBuilder::new(CardId::new(), "Commander Liara Portyr Hand Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::draw(1)])
        .build();

    let first_id = game.create_object_from_definition(&first_exiled, alice, Zone::Exile);
    let second_id = game.create_object_from_definition(&second_exiled, alice, Zone::Exile);
    let hand_id = game.create_object_from_definition(&hand_spell, alice, Zone::Hand);
    let play_from_exile = CastingMethod::PlayFrom {
        source,
        zone: Zone::Exile,
        use_alternative: None,
    };

    let first = game.object(first_id).expect("first exiled spell exists");
    let first_cost = crate::decision::calculate_effective_mana_cost_for_casting_method(
        &game,
        alice,
        first,
        first.mana_cost.as_ref().expect("first spell mana cost"),
        &play_from_exile,
    );
    assert_eq!(
        first_cost.to_oracle(),
        "{2}",
        "two defending players should reduce the first exiled spell by two generic mana"
    );

    let second = game.object(second_id).expect("second exiled spell exists");
    let second_cost = crate::decision::calculate_effective_mana_cost_for_casting_method(
        &game,
        alice,
        second,
        second.mana_cost.as_ref().expect("second spell mana cost"),
        &play_from_exile,
    );
    assert_eq!(
        second_cost.to_oracle(),
        "{3}",
        "Commander Liara Portyr's reduction applies to every matching exiled spell this turn"
    );

    let hand = game.object(hand_id).expect("hand spell exists");
    let hand_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        hand,
        hand.mana_cost.as_ref().expect("hand spell mana cost"),
    );
    assert_eq!(
        hand_cost.to_oracle(),
        "{4}",
        "Commander Liara Portyr should not reduce spells cast from hand"
    );

    for idx in 0..3 {
        let library_card = CardDefinitionBuilder::new(
            CardId::new(),
            format!("Commander Liara Portyr Library Probe {idx}"),
        )
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::draw(1)])
        .build();
        game.create_object_from_definition(&library_card, alice, Zone::Library);
    }
    let library_before = game.player(alice).expect("alice exists").library.len();
    let exile_before = game.exile.len();
    let mut combat = CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: game.new_object_id(),
        target: AttackTarget::Player(bob),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: game.new_object_id(),
        target: AttackTarget::Player(charlie),
    });
    game.combat = Some(combat);
    let exile_top =
        Effect::exile_top_of_library_player(Value::PlayersBeingAttacked, PlayerFilter::You);
    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_effect(&mut game, &exile_top, &mut ctx)
        .expect("Commander Liara Portyr exile-top effect should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before - 2,
        "two attacked players should make Commander Liara Portyr exile the top two cards"
    );
    assert_eq!(
        game.exile.len(),
        exile_before + 2,
        "Commander Liara Portyr should exile one top card per attacked player"
    );
}

#[test]
pub(super) fn commander_liara_portyr_runtime_has_no_reduction_without_attacked_players() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.new_object_id();
    let grant_reduction = Effect::new(
        crate::effects::GrantNextSpellCostReductionEffect::all_matching_this_turn(
            PlayerFilter::You,
            crate::target::ObjectFilter::default()
                .in_zone(Zone::Exile)
                .cast_by_you(),
            Value::PlayersBeingAttacked,
        ),
    );
    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_effect(&mut game, &grant_reduction, &mut ctx)
        .expect("Commander Liara Portyr zero-count reduction grant should resolve");

    let exiled_spell =
        CardDefinitionBuilder::new(CardId::new(), "Commander Liara Portyr No Attack Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::draw(1)])
            .build();
    let spell_id = game.create_object_from_definition(&exiled_spell, alice, Zone::Exile);
    let spell = game.object(spell_id).expect("exiled spell exists");
    let cost = crate::decision::calculate_effective_mana_cost_for_casting_method(
        &game,
        alice,
        spell,
        spell.mana_cost.as_ref().expect("spell mana cost"),
        &CastingMethod::PlayFrom {
            source,
            zone: Zone::Exile,
            use_alternative: None,
        },
    );

    assert_eq!(
        cost.to_oracle(),
        "{4}",
        "without current attacked players, Commander Liara Portyr's dynamic reduction is zero"
    );

    let library_card = CardDefinitionBuilder::new(
        CardId::new(),
        "Commander Liara Portyr No Attack Library Probe",
    )
    .card_types(vec![CardType::Sorcery])
    .with_spell_effect(vec![Effect::draw(1)])
    .build();
    game.create_object_from_definition(&library_card, alice, Zone::Library);
    let library_before = game.player(alice).expect("alice exists").library.len();
    let exile_before = game.exile.len();
    let exile_top =
        Effect::exile_top_of_library_player(Value::PlayersBeingAttacked, PlayerFilter::You);
    let mut ctx = ExecutionContext::new_default(source, alice);
    execute_effect(&mut game, &exile_top, &mut ctx)
        .expect("Commander Liara Portyr zero-count exile-top effect should resolve");
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before,
        "without attacked players, Commander Liara Portyr should exile no library cards"
    );
    assert_eq!(
        game.exile.len(),
        exile_before,
        "without attacked players, Commander Liara Portyr should leave exile unchanged"
    );
}

#[test]
pub(super) fn will_kenrith_plus_two_sets_base_pt_and_removes_abilities_until_next_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let will_def = will_kenrith_definition();
    let will = game.create_object_from_definition(&will_def, alice, Zone::Battlefield);

    let creature_with_keywords = |name: &str, power: i32, toughness: i32| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .parse_text("Flying\nTrample")
            .expect("keyword creature should parse")
    };
    let first = game.create_object_from_definition(
        &creature_with_keywords("Will Kenrith First Target", 4, 4),
        bob,
        Zone::Battlefield,
    );
    let second = game.create_object_from_definition(
        &creature_with_keywords("Will Kenrith Second Target", 6, 6),
        bob,
        Zone::Battlefield,
    );
    let untouched = game.create_object_from_definition(
        &creature_with_keywords("Will Kenrith Untouched Creature", 5, 5),
        bob,
        Zone::Battlefield,
    );

    resolve_will_kenrith_loyalty_ability(
        &mut game,
        will,
        alice,
        &["PutCountersEffect"],
        vec![
            crate::effects::ResolvedTarget::Object(first),
            crate::effects::ResolvedTarget::Object(second),
        ],
    );

    for target in [first, second] {
        let characteristics = game
            .calculated_characteristics(target)
            .expect("targeted creature should have calculated characteristics");
        assert_eq!(
            (characteristics.power, characteristics.toughness),
            (Some(0), Some(3)),
            "Will Kenrith +2 should set each targeted creature's base power/toughness to 0/3"
        );
        assert!(
            !game.object_has_ability(target, &StaticAbility::flying())
                && !game.object_has_ability(target, &StaticAbility::trample()),
            "Will Kenrith +2 should remove all abilities from each targeted creature"
        );
    }

    assert_eq!(
        (
            game.calculated_power(untouched),
            game.calculated_toughness(untouched)
        ),
        (Some(5), Some(5)),
        "Will Kenrith +2 should not affect creatures beyond the two selected targets"
    );
    assert!(
        game.object_has_ability(untouched, &StaticAbility::flying())
            && game.object_has_ability(untouched, &StaticAbility::trample()),
        "unselected creature should keep its abilities"
    );

    game.turn.turn_number += 1;
    game.turn.active_player = bob;
    assert_eq!(
        (
            game.calculated_power(first),
            game.calculated_toughness(first)
        ),
        (Some(0), Some(3)),
        "Will Kenrith +2 should last through other players' turns"
    );

    game.turn.turn_number += 1;
    game.turn.active_player = alice;
    assert_eq!(
        (
            game.calculated_power(first),
            game.calculated_toughness(first)
        ),
        (Some(4), Some(4)),
        "Will Kenrith +2 should expire when Will's controller reaches their next turn"
    );
    assert!(
        game.object_has_ability(first, &StaticAbility::flying())
            && game.object_has_ability(first, &StaticAbility::trample()),
        "targeted creature should regain printed abilities after Will Kenrith +2 expires"
    );
}

#[test]
pub(super) fn will_kenrith_minus_two_draws_and_reduces_only_target_players_matching_spells_until_next_turn()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let will_def = will_kenrith_definition();
    let will = game.create_object_from_definition(&will_def, alice, Zone::Battlefield);

    for idx in 0..2 {
        let draw_card =
            CardDefinitionBuilder::new(CardId::new(), format!("Will Kenrith Bob Draw Probe {idx}"))
                .card_types(vec![CardType::Instant])
                .build();
        game.create_object_from_definition(&draw_card, bob, Zone::Library);
    }
    let bob_hand_before = game.player(bob).expect("bob exists").hand.len();

    resolve_will_kenrith_loyalty_ability(
        &mut game,
        will,
        alice,
        &["RemoveCountersEffect", "Fixed(2)"],
        vec![crate::effects::ResolvedTarget::Player(bob)],
    );

    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        bob_hand_before + 2,
        "Will Kenrith -2 should make the targeted player draw two cards"
    );

    let costed_spell = |name: &str, card_type: CardType| {
        let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![card_type]);
        if card_type == CardType::Creature {
            builder = builder.power_toughness(PowerToughness::fixed(2, 2));
        }
        if card_type == CardType::Planeswalker {
            builder = builder.loyalty(3);
        }
        builder.build()
    };
    let bob_instant = game.create_object_from_definition(
        &costed_spell("Will Kenrith Bob Instant Probe", CardType::Instant),
        bob,
        Zone::Hand,
    );
    let bob_sorcery = game.create_object_from_definition(
        &costed_spell("Will Kenrith Bob Sorcery Probe", CardType::Sorcery),
        bob,
        Zone::Hand,
    );
    let bob_planeswalker = game.create_object_from_definition(
        &costed_spell(
            "Will Kenrith Bob Planeswalker Probe",
            CardType::Planeswalker,
        ),
        bob,
        Zone::Hand,
    );
    let bob_creature = game.create_object_from_definition(
        &costed_spell("Will Kenrith Bob Creature Probe", CardType::Creature),
        bob,
        Zone::Hand,
    );
    let alice_instant = game.create_object_from_definition(
        &costed_spell("Will Kenrith Alice Instant Probe", CardType::Instant),
        alice,
        Zone::Hand,
    );

    for (spell_id, expected, message) in [
        (
            bob_instant,
            "{2}",
            "target player's instant should be reduced",
        ),
        (
            bob_sorcery,
            "{2}",
            "target player's sorcery should be reduced",
        ),
        (
            bob_planeswalker,
            "{2}",
            "target player's planeswalker should be reduced",
        ),
        (
            bob_creature,
            "{4}",
            "target player's creature should not be reduced",
        ),
        (
            alice_instant,
            "{4}",
            "non-target player's instant should not be reduced",
        ),
    ] {
        let spell = game.object(spell_id).expect("spell exists");
        let caster = game.controller_of(spell);
        let cost = crate::decision::calculate_effective_mana_cost(
            &game,
            caster,
            spell,
            spell
                .mana_cost
                .as_ref()
                .expect("spell should have mana cost"),
        );
        assert_eq!(cost.to_oracle(), expected, "{message}");
    }

    game.turn.turn_number += 1;
    game.turn.active_player = bob;
    let spell = game.object(bob_instant).expect("bob instant exists");
    let cost = crate::decision::calculate_effective_mana_cost(
        &game,
        bob,
        spell,
        spell.mana_cost.as_ref().expect("bob instant mana cost"),
    );
    assert_eq!(
        cost.to_oracle(),
        "{2}",
        "Will Kenrith -2 reduction should last through the target player's next turn"
    );

    game.turn.turn_number += 1;
    game.turn.active_player = alice;
    let spell = game.object(bob_instant).expect("bob instant exists");
    let cost = crate::decision::calculate_effective_mana_cost(
        &game,
        bob,
        spell,
        spell.mana_cost.as_ref().expect("bob instant mana cost"),
    );
    assert_eq!(
        cost.to_oracle(),
        "{4}",
        "Will Kenrith -2 reduction should expire on Will controller's next turn"
    );
}

#[test]
pub(super) fn will_kenrith_minus_eight_gives_emblem_to_target_player() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let will_def = will_kenrith_definition();
    let will = game.create_object_from_definition(&will_def, alice, Zone::Battlefield);
    let command_zone_before = game.command_zone.len();

    resolve_will_kenrith_loyalty_ability(
        &mut game,
        will,
        alice,
        &["RemoveCountersEffect", "Fixed(8)"],
        vec![crate::effects::ResolvedTarget::Player(bob)],
    );

    assert_eq!(
        game.command_zone.len(),
        command_zone_before + 1,
        "Will Kenrith -8 should create one emblem"
    );
    let emblem_id = *game
        .command_zone
        .last()
        .expect("emblem should be in command zone");
    let emblem = game.object(emblem_id).expect("emblem object should exist");
    assert_eq!(emblem.kind, ObjectKind::Emblem);
    assert_eq!(
        emblem.owner, bob,
        "Will Kenrith -8 should give the emblem to the targeted player, not Will's controller"
    );
    let emblem_debug = format!("{:#?}", emblem.abilities);
    assert!(
        emblem_debug.contains("SpellCastTrigger")
            && emblem_debug.contains("CopySpellEffect")
            && emblem_debug.contains("RetargetStackObjectEffect"),
        "Will Kenrith emblem should keep its instant/sorcery copy trigger, got {emblem_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cheering_fanatic_runtime_reduces_only_spells_with_chosen_name_this_turn() {
    struct ChooseLightningBoltName;

    impl DecisionMaker for ChooseLightningBoltName {
        fn decide_text(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::TextInputContext,
        ) -> String {
            "Lightning Bolt".to_string()
        }
    }

    fn generic_spell(name: &str) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::draw(1)])
            .build()
    }

    fn effective_cost_text(game: &GameState, player: PlayerId, spell_id: ObjectId) -> String {
        let spell = game.object(spell_id).expect("spell should exist");
        crate::decision::calculate_effective_mana_cost(
            game,
            player,
            spell,
            spell.mana_cost.as_ref().expect("spell mana cost"),
        )
        .to_oracle()
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cheering = CardDefinitionBuilder::new(CardId::from_raw(56_400), "Cheering Fanatic")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Whenever this creature attacks, choose a card name. \
             Spells with the chosen name cost {1} less to cast this turn.",
        )
        .expect("Cheering Fanatic should parse for runtime regression");
    let triggered = cheering
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cheering Fanatic should have an attack trigger");
    let cheering_id = game.create_object_from_definition(&cheering, alice, Zone::Battlefield);

    let lightning = generic_spell("Lightning Bolt");
    let other_spell = generic_spell("Giant Growth");
    let alice_lightning = game.create_object_from_definition(&lightning, alice, Zone::Hand);
    let bob_lightning = game.create_object_from_definition(&lightning, bob, Zone::Hand);
    let alice_other = game.create_object_from_definition(&other_spell, alice, Zone::Hand);

    let mut decision_maker = ChooseLightningBoltName;
    let mut ctx = crate::effects::ExecutionContext::new_default(cheering_id, alice)
        .with_decision_maker(&mut decision_maker);
    for effect in triggered.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Cheering Fanatic attack trigger effect should resolve");
    }

    assert_eq!(
        effective_cost_text(&game, alice, alice_lightning),
        "{1}",
        "the chosen Lightning Bolt should cost one less for Cheering Fanatic's controller"
    );
    assert_eq!(
        effective_cost_text(&game, bob, bob_lightning),
        "{1}",
        "Cheering Fanatic's unqualified cost reduction should apply to other players too"
    );
    assert_eq!(
        effective_cost_text(&game, alice, alice_other),
        "{2}",
        "spells without the chosen name should not be reduced"
    );

    game.turn.turn_number += 1;
    game.cleanup_temporary_spell_cost_reductions_end_of_turn();
    assert_eq!(
        effective_cost_text(&game, alice, alice_lightning),
        "{2}",
        "Cheering Fanatic's reduction should expire after this turn"
    );
}

#[test]
pub(super) fn test_face_down_cast_matches_panoptic_filter_and_enters_battlefield_face_down() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let source = game.new_object_id();
    game.add_temporary_spell_cost_reduction(
        alice,
        source,
        crate::target::ObjectFilter::default()
            .with_type(CardType::Creature)
            .face_down(),
        ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]),
        1,
    );

    let normal_creature = CardDefinitionBuilder::new(CardId::new(), "Normal Creature Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let normal_creature_id =
        game.create_object_from_definition(&normal_creature, alice, Zone::Hand);
    let normal_creature_obj = game
        .object(normal_creature_id)
        .expect("normal creature should exist");
    let normal_cost = crate::decision::calculate_effective_mana_cost(
        &game,
        alice,
        normal_creature_obj,
        normal_creature_obj
            .mana_cost
            .as_ref()
            .expect("normal creature mana cost"),
    );
    assert_eq!(
        normal_cost.mana_value(),
        4,
        "face-down-only reducer should not affect normal creature spells in hand"
    );

    let morph_card = CardBuilder::new(CardId::from_raw(1200), "Morph Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(6)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let morph_id = game.create_object_from_card(&morph_card, alice, Zone::Hand);
    game.object_mut(morph_id)
        .expect("morph card should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::morph(
            crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Green]])),
        )));

    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        morph_id,
        Zone::Hand,
        alice,
        &CastingMethod::FaceDown,
    )
    .expect("face-down cast should move spell to stack");
    assert!(
        game.is_face_down(stack_id),
        "face-down cast should mark the spell object as face down on the stack"
    );

    let stack_obj = game.object(stack_id).expect("stack spell should exist");
    let face_down_cost = crate::decision::spell_mana_cost_for_cast(
        &game,
        alice,
        stack_obj,
        &CastingMethod::FaceDown,
        Zone::Hand,
    )
    .expect("face-down cast should use the shared {3} cost");
    let reduced_cost =
        crate::decision::calculate_effective_mana_cost(&game, alice, stack_obj, &face_down_cost);
    assert_eq!(
        reduced_cost.mana_value(),
        0,
        "Panoptic-style reducer should apply once the creature spell is actually being cast face down"
    );

    let battlefield_id = game
        .move_object_by_effect(stack_id, Zone::Battlefield)
        .expect("face-down spell should resolve onto the battlefield");
    assert!(
        game.is_face_down(battlefield_id),
        "stack-to-battlefield move should preserve face-down state for a face-down cast"
    );
    let permanent = game
        .object(battlefield_id)
        .expect("resolved permanent should exist");
    assert_eq!(permanent.power(), Some(2));
    assert_eq!(permanent.toughness(), Some(2));

    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Green, 1);
    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::TurnFaceUp { creature_id, .. } if *creature_id == battlefield_id
        )),
        "resolved face-down morph permanent should still be turnable face up"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn aquamorph_entity_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(386300), "Aquamorph Entity")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Shapeshifter])
        .power_toughness(PowerToughness::new(PtValue::Star, PtValue::Star))
        .parse_text(
            "As this creature enters or is turned face up, it becomes your choice of 5/1 or 1/5.\nMorph {2}{U}",
        )
        .expect("Aquamorph Entity should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseAquamorphPowerToughness {
    pub(super) option_index: usize,
    pub(super) choices_seen: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseAquamorphPowerToughness {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.options.iter().any(|option| option.description == "5/1")
            && ctx.options.iter().any(|option| option.description == "1/5")
        {
            self.choices_seen += 1;
            return vec![self.option_index];
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aquamorph_entity_enters_with_chosen_power_toughness() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let aquamorph = aquamorph_entity_definition();
    let hand_id = game.create_object_from_definition(&aquamorph, alice, Zone::Hand);

    let mut dm = ChooseAquamorphPowerToughness {
        option_index: 1,
        choices_seen: 0,
    };
    let result = game
        .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut dm)
        .expect("Aquamorph Entity should enter the battlefield");
    let object = game
        .object(result.new_id)
        .expect("Aquamorph Entity should exist on the battlefield");
    assert_eq!(object.power(), Some(1));
    assert_eq!(object.toughness(), Some(5));
    assert_eq!(dm.choices_seen, 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aquamorph_entity_turns_face_up_with_chosen_power_toughness() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let aquamorph = aquamorph_entity_definition();
    let hand_id = game.create_object_from_definition(&aquamorph, alice, Zone::Hand);
    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        hand_id,
        Zone::Hand,
        alice,
        &CastingMethod::FaceDown,
    )
    .expect("Aquamorph Entity should be castable face down");
    let battlefield_id = game
        .move_object_by_effect(stack_id, Zone::Battlefield)
        .expect("face-down Aquamorph Entity should resolve");
    let face_down = game
        .object(battlefield_id)
        .expect("face-down Aquamorph Entity should exist");
    assert!(game.is_face_down(battlefield_id));
    assert_eq!(face_down.power(), Some(2));
    assert_eq!(face_down.toughness(), Some(2));

    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::TurnFaceUp { creature_id, .. }
                    if *creature_id == battlefield_id
            )
        })
        .expect("Aquamorph Entity should be turnable face up for its morph cost");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ChooseAquamorphPowerToughness {
        option_index: 0,
        choices_seen: 0,
    };
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .expect("turning Aquamorph Entity face up should succeed");

    let face_up = game
        .object(battlefield_id)
        .expect("Aquamorph Entity should remain on the battlefield");
    assert!(!game.is_face_down(battlefield_id));
    assert_eq!(face_up.power(), Some(5));
    assert_eq!(face_up.toughness(), Some(1));
    assert_eq!(dm.choices_seen, 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn primal_plasma_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(509445), "Primal Plasma")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Shapeshifter])
        .power_toughness(PowerToughness::new(PtValue::Star, PtValue::Star))
        .parse_text(
            "As this creature enters, it becomes your choice of a 3/3 creature, a 2/2 creature with flying, or a 1/6 creature with defender.",
        )
        .expect("Primal Plasma should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChoosePrimalPlasmaCharacteristics {
    pub(super) option_index: usize,
    pub(super) choices_seen: usize,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChoosePrimalPlasmaCharacteristics {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.options.iter().any(|option| option.description == "3/3")
            && ctx.options.iter().any(|option| option.description == "2/2")
            && ctx.options.iter().any(|option| option.description == "1/6")
        {
            self.choices_seen += 1;
            return vec![self.option_index];
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn primal_plasma_enters_with_each_chosen_characteristic_set() {
    let cases = [
        (0, 3, 3, false, false),
        (1, 2, 2, true, false),
        (2, 1, 6, false, true),
    ];

    for (option_index, expected_power, expected_toughness, has_flying, has_defender) in cases {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let primal_plasma = primal_plasma_definition();
        let hand_id = game.create_object_from_definition(&primal_plasma, alice, Zone::Hand);

        let mut dm = ChoosePrimalPlasmaCharacteristics {
            option_index,
            choices_seen: 0,
        };
        let result = game
            .move_object_with_etb_processing_with_dm(hand_id, Zone::Battlefield, &mut dm)
            .expect("Primal Plasma should enter the battlefield");
        let object = game
            .object(result.new_id)
            .expect("Primal Plasma should exist on the battlefield");

        assert_eq!(object.power(), Some(expected_power));
        assert_eq!(object.toughness(), Some(expected_toughness));
        assert_eq!(
            game.object_has_ability(result.new_id, &StaticAbility::flying()),
            has_flying,
            "flying grant should match option {option_index}"
        );
        assert_eq!(
            game.object_has_ability(result.new_id, &StaticAbility::defender()),
            has_defender,
            "defender grant should match option {option_index}"
        );
        assert_eq!(dm.choices_seen, 1);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_bestow_cast_enters_as_aura_and_reverts_when_unattached() {
    use crate::cards::CardDefinitionBuilder;
    use crate::decision::compute_legal_actions;
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let host_id = create_creature(&mut game, "Bestow Host", alice, 2, 2);
    game.remove_summoning_sickness(host_id);

    let bestow_def = CardDefinitionBuilder::new(CardId::new(), "Bestow Probe Runtime")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Bestow {0}\nLifelink\nEnchanted creature gets +1/+1 and has lifelink.")
        .expect("bestow probe should parse");

    let bestow_in_hand = game.create_object_from_definition(&bestow_def, alice, Zone::Hand);

    let actions = compute_legal_actions(&game, alice);
    let can_cast_bestow = actions.iter().any(|action| {
        matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(_),
            } if *spell_id == bestow_in_hand
        )
    });
    assert!(
        can_cast_bestow,
        "bestow cast option should be available from hand when a creature target exists"
    );

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();

    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: bestow_in_hand,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Alternative(0),
    });
    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
            .expect("bestow cast should start successfully");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
        ),
        "bestow cast should require choosing an Aura target"
    );

    let stack_bestow_id = state
        .pending_cast
        .as_ref()
        .map(|pending| pending.spell_id)
        .expect("bestow cast should still be pending on stack");
    let stack_bestow = game
        .object(stack_bestow_id)
        .expect("bestow spell should exist on stack");
    assert!(
        stack_bestow.subtypes.contains(&crate::types::Subtype::Aura),
        "bestow cast should be an Aura spell on stack"
    );
    assert!(
        !stack_bestow.card_types.contains(&CardType::Creature),
        "bestow cast should not be a creature spell on stack"
    );

    let target_response = PriorityResponse::Targets(vec![Target::Object(host_id)]);
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
        .expect("choosing bestow target should complete cast");

    assert_eq!(game.stack.len(), 1, "bestow spell should be on stack");
    resolve_stack_entry(&mut game).expect("bestow spell should resolve");

    let bestowed_id = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Bestow Probe Runtime")
                .unwrap_or(false)
        })
        .expect("bestowed permanent should be on battlefield");

    let bestowed = game.object(bestowed_id).expect("bestowed permanent exists");
    assert!(
        bestowed.subtypes.contains(&crate::types::Subtype::Aura),
        "bestowed permanent should enter as an Aura"
    );
    assert!(
        !bestowed.card_types.contains(&CardType::Creature),
        "bestowed permanent should not be a creature while attached"
    );
    assert_eq!(
        bestowed.attached_to,
        Some(crate::object::AttachmentTarget::Object(host_id)),
        "bestowed permanent should be attached to the chosen creature"
    );

    game.move_object_by_effect(host_id, Zone::Graveyard)
        .expect("host creature should move to graveyard");
    check_and_apply_sbas(&mut game, &mut trigger_queue)
        .expect("state-based actions should process unattached bestow");

    let reverted = game
        .object(bestowed_id)
        .expect("bestow permanent should remain on battlefield after host leaves");
    assert_eq!(reverted.zone, Zone::Battlefield);
    assert!(
        reverted.card_types.contains(&CardType::Creature),
        "bestow permanent should revert to creature form when unattached"
    );
    assert!(
        !reverted.subtypes.contains(&crate::types::Subtype::Aura),
        "bestow permanent should no longer be an Aura after reverting"
    );
    assert!(
        reverted.attached_to.is_none(),
        "reverted bestow permanent should no longer be attached"
    );
}

#[test]
pub(super) fn test_bestowed_control_aura_controls_attached_creature() {
    use crate::cards::CardDefinitionBuilder;
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let host_id = create_creature(&mut game, "Silvercoat Lion", bob, 2, 2);

    let bestow_def = CardDefinitionBuilder::new(CardId::new(), "Hypnotic Siren Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Bestow {0}\nFlying\nYou control enchanted creature.\nEnchanted creature gets +1/+1 and has flying.",
        )
        .expect("bestow control probe should parse");

    let bestow_in_hand = game.create_object_from_definition(&bestow_def, alice, Zone::Hand);
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: bestow_in_hand,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Alternative(0),
        }),
    )
    .expect("bestow cast should start");
    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(host_id)]),
    )
    .expect("bestow target should be accepted");
    resolve_stack_entry(&mut game).expect("bestow spell should resolve");

    game.refresh_continuous_state();
    assert_eq!(
        game.current_controller(host_id),
        Some(alice),
        "bestowed control Aura should change control of the attached creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_curse_aura_attaches_to_player_and_triggers_on_enchanted_players_upkeep() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let curse_def = CardDefinitionBuilder::new(CardId::new(), "Curse Runtime Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura])
        .parse_text(
            "Enchant player\nAt the beginning of enchanted player's upkeep, that player loses 1 life.",
        )
        .expect("curse text should parse");
    let curse_in_hand = game.create_object_from_definition(&curse_def, alice, Zone::Hand);

    let mut state = PriorityLoopState::new(game.players_in_game());
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: curse_in_hand,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
            .expect("curse cast should start successfully");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
        ),
        "enchant player Aura should ask for a player target"
    );

    let target_response = PriorityResponse::Targets(vec![Target::Player(bob)]);
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
        .expect("choosing enchanted player should complete cast");
    resolve_stack_entry(&mut game).expect("curse should resolve");

    let curse_id = game
        .battlefield
        .iter()
        .copied()
        .find(|&id| {
            game.object(id)
                .map(|obj| obj.name == "Curse Runtime Variant")
                .unwrap_or(false)
        })
        .expect("resolved curse should be on the battlefield");

    assert_eq!(
        game.object(curse_id).and_then(|object| object.attached_to),
        Some(crate::object::AttachmentTarget::Player(bob)),
        "the curse should enchant the chosen player"
    );
    assert!(
        game.player(bob)
            .expect("bob should exist")
            .attachments
            .contains(&curse_id),
        "the enchanted player should track the attached Aura"
    );

    let life_before = game.player(bob).expect("bob should exist").life;
    game.turn.turn_number += 1;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Upkeep);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "the curse should trigger on the enchanted player's upkeep"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("curse upkeep trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("curse upkeep trigger should resolve");

    assert_eq!(
        game.player(bob).expect("bob should exist").life,
        life_before - 1,
        "the enchanted player should lose life when the curse trigger resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn grievous_wound_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Grievous Wound")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura])
        .parse_text(
            "Enchant player\nEnchanted player can't gain life.\nWhenever enchanted player is dealt damage, they lose half their life, rounded up.",
        )
        .expect("Grievous Wound should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_grievous_wound_stops_enchanted_player_life_gain_and_triggers_on_damage() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let grievous_wound = grievous_wound_definition();
    let wound_id = game.create_object_from_definition(&grievous_wound, alice, Zone::Battlefield);
    game.object_mut(wound_id)
        .expect("Grievous Wound should exist")
        .attached_to = Some(crate::object::AttachmentTarget::Player(bob));
    game.player_mut(bob)
        .expect("bob should exist")
        .attachments
        .push(wound_id);
    game.refresh_continuous_state();

    assert!(
        game.can_gain_life(alice),
        "Grievous Wound should not stop other players from gaining life"
    );
    assert!(
        !game.can_gain_life(bob),
        "Grievous Wound should stop the enchanted player from gaining life"
    );

    game.player_mut(bob).expect("bob should exist").life = 17;
    let mut gain_ctx = crate::effects::ExecutionContext::new_default(wound_id, alice);
    crate::effects::execute_effect(
        &mut game,
        &Effect::gain_life_player(3, crate::target::ChooseSpec::SpecificPlayer(bob)),
        &mut gain_ctx,
    )
    .expect("life gain prevention should still let the effect resolve");
    assert_eq!(
        game.player(bob).expect("bob should exist").life,
        17,
        "the enchanted player should not gain life"
    );

    let alice_life_before = game.player(alice).expect("alice should exist").life;
    let mut gain_ctx = crate::effects::ExecutionContext::new_default(wound_id, alice);
    crate::effects::execute_effect(
        &mut game,
        &Effect::gain_life_player(3, crate::target::ChooseSpec::SpecificPlayer(alice)),
        &mut gain_ctx,
    )
    .expect("other players should still gain life");
    assert_eq!(
        game.player(alice).expect("alice should exist").life,
        alice_life_before + 3,
        "non-enchanted players should still gain life"
    );

    let alice_damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            wound_id,
            crate::events::DamageTarget::Player(alice),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        check_triggers(&game, &alice_damage_event).is_empty(),
        "Grievous Wound should not trigger when a non-enchanted player is dealt damage"
    );

    let bob_damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            wound_id,
            crate::events::DamageTarget::Player(bob),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    for trigger in check_triggers(&game, &bob_damage_event) {
        trigger_queue.add(trigger);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Grievous Wound should trigger when the enchanted player is dealt damage"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Grievous Wound trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Grievous Wound trigger should resolve");
    assert_eq!(
        game.player(bob).expect("bob should exist").life,
        8,
        "the enchanted player should lose half their life rounded up from 17"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_kitsune_mystic_requires_two_attached_auras_for_end_step_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::Ending;
    game.turn.step = Some(crate::game_state::Step::End);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let kitsune_def = CardDefinitionBuilder::new(CardId::new(), "Kitsune Mystic Runtime Variant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "At the beginning of the end step, if this creature is enchanted by two or more Auras, flip it.",
        )
        .expect("Kitsune Mystic text should parse");
    let kitsune_id = game.create_object_from_definition(&kitsune_def, alice, Zone::Battlefield);

    let aura_def = CardDefinitionBuilder::new(CardId::new(), "Runtime Aura Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura])
        .build();

    let first_aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
    if let Some(aura) = game.object_mut(first_aura_id) {
        aura.attached_to = Some(crate::object::AttachmentTarget::Object(kitsune_id));
    }
    if let Some(kitsune) = game.object_mut(kitsune_id) {
        kitsune.attachments.push(first_aura_id);
    }

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        0,
        "Kitsune Mystic should not trigger with only one attached Aura"
    );

    let second_aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
    if let Some(aura) = game.object_mut(second_aura_id) {
        aura.attached_to = Some(crate::object::AttachmentTarget::Object(kitsune_id));
    }
    if let Some(kitsune) = game.object_mut(kitsune_id) {
        kitsune.attachments.push(second_aura_id);
    }

    generate_and_queue_step_triggers(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Kitsune Mystic should trigger once two attached Auras are present"
    );
    assert_eq!(
        trigger_queue.entries[0].source_name.as_str(),
        "Kitsune Mystic Runtime Variant",
        "the end-step trigger should come from Kitsune Mystic"
    );
}

#[test]
pub(super) fn test_illegal_equipment_becomes_unattached_instead_of_dying() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let creature = create_creature(&mut game, "Bearer", alice, 2, 2);
    let equipment_card = CardBuilder::new(CardId::new(), "Test Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![crate::types::Subtype::Equipment])
        .build();
    let equipment = game.create_object_from_card(&equipment_card, alice, Zone::Battlefield);

    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment,
            crate::object::AttachmentTarget::Object(creature),
        )
    );

    game.object_mut(creature)
        .expect("equipped creature should exist")
        .card_types = vec![CardType::Land].into();

    crate::rules::state_based::apply_state_based_actions(&mut game);

    assert_eq!(
        game.object(equipment).map(|object| object.zone),
        Some(Zone::Battlefield),
        "Equipment should remain on the battlefield when it falls off"
    );
    assert!(
        game.object(equipment)
            .expect("equipment should exist")
            .attached_to
            .is_none(),
        "Equipment should become unattached when its bearer stops being a creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_strip_bare_destroys_attached_auras_and_equipment_only() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 1);

    let strip_bare_def = CardDefinitionBuilder::new(CardId::new(), "Strip Bare Runtime Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy all Auras and Equipment attached to target creature.")
        .expect("Strip Bare should parse");
    let strip_bare_id = game.create_object_from_definition(&strip_bare_def, alice, Zone::Hand);

    let target_id = create_creature(&mut game, "Strip Bare Target", bob, 2, 2);

    let aura_def = CardDefinitionBuilder::new(CardId::new(), "Strip Bare Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![crate::types::Subtype::Aura])
        .parse_text("Enchant creature")
        .expect("aura should parse");
    let aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            aura_id,
            crate::object::AttachmentTarget::Object(target_id),
        ),
        "aura should attach to the target creature"
    );

    let equipment_def = CardBuilder::new(CardId::new(), "Strip Bare Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![crate::types::Subtype::Equipment])
        .build();
    let equipment_id = game.create_object_from_card(&equipment_def, alice, Zone::Battlefield);
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment_id,
            crate::object::AttachmentTarget::Object(target_id),
        ),
        "equipment should attach to the target creature"
    );

    let mut state = PriorityLoopState::new(game.players_in_game());
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: strip_bare_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
            .expect("Strip Bare cast should start successfully");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
        ),
        "Strip Bare should ask for a target creature"
    );

    let target_response = PriorityResponse::Targets(vec![Target::Object(target_id)]);
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &target_response)
        .expect("choosing the target creature should complete Strip Bare");

    assert_eq!(game.stack.len(), 1, "Strip Bare should be on the stack");
    resolve_stack_entry(&mut game).expect("Strip Bare should resolve");

    let alice_graveyard = game
        .player(alice)
        .expect("alice should exist")
        .graveyard
        .clone();

    assert_eq!(
        game.object(target_id).map(|object| object.zone),
        Some(Zone::Battlefield),
        "the target creature should survive Strip Bare"
    );
    assert!(
        alice_graveyard
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|object| object.name == "Strip Bare Aura" && object.zone == Zone::Graveyard),
        "the attached Aura should be destroyed"
    );
    assert!(
        alice_graveyard
            .iter()
            .filter_map(|&id| game.object(id))
            .any(|object| object.name == "Strip Bare Equipment" && object.zone == Zone::Graveyard),
        "the attached Equipment should be destroyed"
    );
    assert!(
        game.object(target_id)
            .expect("target creature should exist")
            .attachments
            .is_empty(),
        "Strip Bare should clear all attachments from the target creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn soul_nova_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(48_101), "Soul Nova")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target attacking creature and all Equipment attached to it.")
        .expect("Soul Nova should parse strictly for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_soul_nova_exiles_attacking_creature_and_attached_equipment_only() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 5);

    let soul_nova = soul_nova_definition();
    let soul_nova_id = game.create_object_from_definition(&soul_nova, alice, Zone::Hand);
    let attacker_id = create_creature(&mut game, "Soul Nova Attacker", bob, 3, 3);
    let other_creature_id = create_creature(&mut game, "Soul Nova Bystander", bob, 2, 2);
    game.remove_summoning_sickness(attacker_id);

    let mut combat = CombatState::default();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        }],
    )
    .expect("Soul Nova target creature should be able to attack");

    let equipment_def = CardBuilder::new(CardId::new(), "Soul Nova Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let equipment_id = game.create_object_from_card(&equipment_def, bob, Zone::Battlefield);
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            equipment_id,
            crate::object::AttachmentTarget::Object(attacker_id),
        ),
        "equipment should attach to the attacking creature"
    );

    let second_equipment_def = CardBuilder::new(CardId::new(), "Soul Nova Other Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let other_equipment_id =
        game.create_object_from_card(&second_equipment_def, bob, Zone::Battlefield);
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            other_equipment_id,
            crate::object::AttachmentTarget::Object(other_creature_id),
        ),
        "other equipment should attach to the non-target creature"
    );

    let aura_def = CardDefinitionBuilder::new(CardId::new(), "Soul Nova Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Enchant creature")
        .expect("aura should parse");
    let aura_id = game.create_object_from_definition(&aura_def, bob, Zone::Battlefield);
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            aura_id,
            crate::object::AttachmentTarget::Object(attacker_id),
        ),
        "aura should attach to the attacking creature"
    );

    let mut state = PriorityLoopState::new(game.players_in_game());
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: soul_nova_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
            .expect("Soul Nova cast should start successfully");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
        ),
        "Soul Nova should ask for one attacking creature target"
    );

    apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(attacker_id)]),
    )
    .expect("choosing the attacking creature should complete Soul Nova");
    assert_eq!(game.stack.len(), 1, "Soul Nova should be on the stack");
    resolve_stack_entry(&mut game).expect("Soul Nova should resolve");

    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Soul Nova Attacker"),
        1,
        "Soul Nova should exile the target attacking creature"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Soul Nova Equipment"),
        1,
        "Soul Nova should exile Equipment attached to the target creature"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Battlefield, "Soul Nova Other Equipment"),
        1,
        "Soul Nova should not exile Equipment attached to a different creature"
    );
    assert_eq!(
        count_named_objects_in_zone(&game, Zone::Exile, "Soul Nova Aura"),
        0,
        "Soul Nova should not exile non-Equipment attachments"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_soul_nova_targets_only_attacking_creatures() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareAttackers);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::White, 5);

    let soul_nova = soul_nova_definition();
    let soul_nova_id = game.create_object_from_definition(&soul_nova, alice, Zone::Hand);
    let attacker_id = create_creature(&mut game, "Soul Nova Legal Attacker", bob, 3, 3);
    let nonattacker_id = create_creature(&mut game, "Soul Nova Illegal Bystander", bob, 2, 2);
    game.remove_summoning_sickness(attacker_id);

    let mut combat = CombatState::default();
    apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &[AttackerDeclaration {
            creature: attacker_id,
            target: AttackTarget::Player(alice),
        }],
    )
    .expect("attacker should be legal");

    let mut state = PriorityLoopState::new(game.players_in_game());
    let progress = apply_priority_response(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id: soul_nova_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
    )
    .expect("Soul Nova cast should ask for targets");

    let targets_ctx = match progress {
        GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(
            ctx,
        )) => ctx,
        other => panic!("expected Soul Nova target prompt, got {other:?}"),
    };
    let legal_targets = &targets_ctx.requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(attacker_id)),
        "Soul Nova should be able to target the attacking creature"
    );
    assert!(
        !legal_targets.contains(&Target::Object(nonattacker_id)),
        "Soul Nova should not be able to target a nonattacking creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_asinine_antics_flash_extra_cost_is_available_at_instant_timing_only_as_alternative()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 6);

    let asinine_antics_def = CardDefinitionBuilder::new(CardId::new(), "Asinine Antics")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You may cast this spell as though it had flash if you pay {2} more to cast it.\n\
             For each creature your opponents control, create a Cursed Role token attached to that creature.",
        )
        .expect("Asinine Antics should parse");
    let asinine_antics_id =
        game.create_object_from_definition(&asinine_antics_def, alice, Zone::Hand);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(_),
            } if *spell_id == asinine_antics_id
        )),
        "Asinine Antics should be castable on another player's turn through its flash extra-cost method"
    );
    assert!(
        !actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method: CastingMethod::Normal,
            } if *spell_id == asinine_antics_id
        )),
        "the normal sorcery-speed cast should stay unavailable at instant timing"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_asinine_antics_attaches_cursed_roles_to_each_opponent_creature() {
    #[derive(Default)]
    struct AsinineAnticsDecisionMaker {
        attach_aura_prompts: usize,
    }

    impl DecisionMaker for AsinineAnticsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx.description == "Attach Aura to" {
                self.attach_aura_prompts += 1;
            }
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Blue, 4);

    let asinine_antics_def = CardDefinitionBuilder::new(CardId::new(), "Asinine Antics")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You may cast this spell as though it had flash if you pay {2} more to cast it.\n\
             For each creature your opponents control, create a Cursed Role token attached to that creature.",
        )
        .expect("Asinine Antics should parse");
    let asinine_antics_id =
        game.create_object_from_definition(&asinine_antics_def, alice, Zone::Hand);

    let bob_creature_a = create_creature(&mut game, "First Opponent Creature", bob, 2, 2);
    let bob_creature_b = create_creature(&mut game, "Second Opponent Creature", bob, 3, 3);
    let alice_creature = create_creature(&mut game, "Friendly Creature", alice, 2, 2);
    let target_requirements = super::targeting::extract_target_requirements(
        &game,
        asinine_antics_def
            .spell_effect
            .as_ref()
            .expect("Asinine Antics should have a spell effect")
            .flattened_default_effects(),
        alice,
        Some(asinine_antics_id),
    );
    assert!(
        target_requirements.is_empty(),
        "Asinine Antics should not ask for attachment targets; each Role attaches to the iterated creature, got {target_requirements:?}"
    );

    let mut state = PriorityLoopState::new(game.players_in_game());
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: asinine_antics_id,
        from_zone: Zone::Hand,
        casting_method: CastingMethod::Normal,
    });
    apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
        .expect("Asinine Antics cast should start successfully");
    assert_eq!(game.stack.len(), 1, "Asinine Antics should be on the stack");

    let mut dm = AsinineAnticsDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut dm).expect("Asinine Antics should resolve");
    assert_eq!(
        dm.attach_aura_prompts, 0,
        "attached Role token creation should not ask the player to choose what the Aura attaches to"
    );

    let cursed_roles: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|object| (id, object)))
        .filter(|(_, object)| object.name == "Cursed Role")
        .collect();
    assert_eq!(
        cursed_roles.len(),
        2,
        "one Cursed Role should be created for each opponent creature only"
    );

    let attached_targets: Vec<_> = cursed_roles
        .iter()
        .map(|(_, object)| object.attached_to)
        .collect();
    assert!(
        attached_targets.contains(&Some(crate::object::AttachmentTarget::Object(
            bob_creature_a
        ))),
        "one Cursed Role should attach to the first opponent creature"
    );
    assert!(
        attached_targets.contains(&Some(crate::object::AttachmentTarget::Object(
            bob_creature_b
        ))),
        "one Cursed Role should attach to the second opponent creature"
    );
    assert!(
        !attached_targets.contains(&Some(crate::object::AttachmentTarget::Object(
            alice_creature
        ))),
        "Asinine Antics should not create a Role for your own creature"
    );

    game.refresh_continuous_state();
    for creature_id in [bob_creature_a, bob_creature_b] {
        let characteristics = game
            .calculated_characteristics(creature_id)
            .expect("opponent creature should have calculated characteristics");
        assert_eq!(
            (characteristics.power, characteristics.toughness),
            (Some(1), Some(1)),
            "Cursed Role should set each enchanted opponent creature's base power and toughness to 1/1"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_disturb_cast_uses_back_face_characteristics_on_stack() {
    use crate::cards::basic_forest;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::Subtype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let registry = crate::cards::CardRegistry::with_builtin_cards_for_names(["Squirrel Nest"]);
    let back_face = registry
        .get("Squirrel Nest")
        .expect("Squirrel Nest should exist in explicit test registry");
    let mut disturb_def = CardDefinitionBuilder::new(CardId::new(), "Disturb Runtime Front")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(2, 2))
        .disturb(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
    disturb_def.card.other_face_name = Some("Squirrel Nest".to_string());
    disturb_def.card.other_face = Some(back_face.card.id);

    let host_id = game.create_object_from_definition(&basic_forest(), alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let disturb_id = game.create_object_from_definition(&disturb_def, alice, Zone::Graveyard);

    let actions = crate::decision::compute_legal_actions(&game, alice);
    assert!(
        actions.iter().any(|action| matches!(
            action,
            LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Graveyard,
                casting_method: CastingMethod::Alternative(_),
            } if *spell_id == disturb_id
        )),
        "disturb cast should be available from graveyard when the back-face Aura has a legal target; alt_casts={:?} other_face={:?} actions={actions:?}",
        disturb_def.alternative_casts,
        disturb_def.card.other_face
    );

    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut trigger_queue = TriggerQueue::new();
    let cast_response = PriorityResponse::PriorityAction(LegalAction::CastSpell {
        spell_id: disturb_id,
        from_zone: Zone::Graveyard,
        casting_method: CastingMethod::Alternative(0),
    });
    let progress =
        apply_priority_response(&mut game, &mut trigger_queue, &mut state, &cast_response)
            .expect("disturb cast should start successfully");
    assert!(
        matches!(
            progress,
            GameProgress::NeedsDecisionCtx(crate::decisions::context::DecisionContext::Targets(_))
        ),
        "disturb cast should require choosing an Aura target from the back face"
    );

    let stack_id = state
        .pending_cast
        .as_ref()
        .map(|pending| pending.spell_id)
        .expect("disturb cast should still be pending on stack");

    let stack_obj = game
        .object(stack_id)
        .expect("disturbed spell should be on stack");
    assert_eq!(stack_obj.name, "Squirrel Nest");
    assert!(
        stack_obj.subtypes.contains(&crate::types::Subtype::Aura),
        "disturbed spell should use back-face Aura subtype on stack"
    );
    assert!(
        !stack_obj.card_types.contains(&CardType::Creature),
        "disturbed spell should not remain a creature spell on stack"
    );
    assert!(
        matches!(
            &stack_obj.cast_alternative_method,
            Some(method) if matches!(method.as_ref(), crate::alternative_cast::AlternativeCastingMethod::Disturb { .. })
        ),
        "disturbed spell should retain the selected alternative method after applying the back face"
    );
    let stack_cost = crate::decision::spell_mana_cost_for_cast(
        &game,
        alice,
        stack_obj,
        &CastingMethod::Alternative(0),
        Zone::Graveyard,
    )
    .expect("disturbed spell should still use its front-face disturb cost");
    assert_eq!(
        stack_cost,
        ManaCost::from_pips(vec![vec![ManaSymbol::Green]]),
        "disturbed spell should not become free after the back-face overlay"
    );

    let requirements = super::targeting::extract_target_requirements(
        &game,
        stack_obj
            .spell_effect
            .as_deref()
            .map_or(&[][..], |program| program.flattened_default_effects()),
        alice,
        Some(stack_id),
    );
    assert_eq!(
        requirements.len(),
        1,
        "disturbed Aura should require one target"
    );
    assert!(
        requirements[0]
            .legal_targets
            .iter()
            .any(|target| *target == Target::Object(host_id)),
        "disturbed Aura should target the host land from its back face"
    );
}

#[test]
pub(super) fn test_gift_promise_updates_cast_time_target_requirements() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(9100), "Gift Target Runtime")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, alice, Zone::Stack);

    let artifact = CardBuilder::new(CardId::from_raw(9101), "Opponent Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_card(&artifact, bob, Zone::Battlefield);

    let default_target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::creature().opponent_controls(),
    ));
    let gift_target = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
        crate::target::ObjectFilter::nonland_permanent().opponent_controls(),
    ));
    let program =
        crate::resolution::ResolutionProgram::new(vec![crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::move_to_zone(default_target, Zone::Hand, false)],
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                crate::effect::Condition::ThisSpellPaidLabel("Gift".into()),
                vec![Effect::move_to_zone(gift_target, Zone::Hand, false)],
            )],
        }]);

    if let Some(obj) = game.object_mut(spell_id) {
        obj.spell_effect = Some(program.into());
        obj.optional_costs = vec![crate::cost::OptionalCost::custom(
            "Gift a card",
            crate::cost::TotalCost::free(),
        )]
        .into();
        obj.optional_costs_paid = crate::cost::OptionalCostsPaid::from_costs(&obj.optional_costs);
    }

    let program = game
        .object(spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .expect("test spell should have a resolution program");

    assert!(
        !spell_program_has_legal_targets_with_modes(&game, program, alice, Some(spell_id), None),
        "unpromised Gift branch should still require a creature target"
    );
    assert!(
        extract_target_requirements_from_program_with_modes(
            &game,
            program,
            alice,
            Some(spell_id),
            None,
        )
        .is_empty(),
        "unpromised Gift branch should expose no legal target requirements in this setup"
    );

    game.object_mut(spell_id)
        .expect("test spell should still exist")
        .optional_costs_paid
        .pay_times(0, 1);

    let program = game
        .object(spell_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .expect("test spell should still have a resolution program");
    let requirements = extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(spell_id),
        None,
    );

    assert!(
        spell_program_has_legal_targets_with_modes(&game, program, alice, Some(spell_id), None),
        "promised Gift branch should switch to the replacement targets"
    );
    assert_eq!(requirements.len(), 1, "expected one promised-branch target");
    assert_eq!(
        requirements[0].legal_targets,
        vec![Target::Object(artifact_id)],
        "expected the promised Gift branch to target the opponent's nonland permanent"
    );
}
