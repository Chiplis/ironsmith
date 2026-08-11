#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::PriorityResponse;
use crate::Step;
use crate::decision::DecisionMaker;
use crate::game_loop::{PriorityLoopState, apply_priority_response_with_dm};
use crate::game_state::{Phase, StackEntry, Target};
use crate::triggers::{TriggerEvent, TriggerQueue};

fn zone_by_stable_id(game: &crate::GameState, stable_id: StableId) -> Option<Zone> {
    game.find_object_by_stable_id(stable_id)
        .and_then(|id| game.object(id))
        .map(|object| object.zone)
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn creature_with_abilities(
    name: &str,
    abilities: impl IntoIterator<Item = crate::static_abilities::StaticAbility>,
) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    for ability in abilities {
        builder = builder.with_ability(Ability::static_ability(ability));
    }
    builder.build()
}

#[derive(Default)]
struct FeralEncounterDecisions {
    target_contexts: Vec<crate::decisions::context::TargetsContext>,
}

impl DecisionMaker for FeralEncounterDecisions {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .find(|candidate| candidate.name == "Chosen Feral Creature")
            .map(|candidate| vec![candidate.id])
            .unwrap_or_default()
    }

    fn decide_targets(
        &mut self,
        game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.target_contexts.push(ctx.clone());
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            let desired = requirement.legal_targets.iter().find(|target| {
                let Target::Object(object_id) = target else {
                    return false;
                };
                game.object(*object_id).is_some_and(|object| {
                    object.name == "Feral Damage Source" || object.name == "Feral Damage Target"
                })
            });
            if let Some(target) = desired {
                selected.push(target.clone());
            } else if requirement.min_targets > 0
                && let Some(target) = requirement.legal_targets.first()
            {
                selected.push(target.clone());
            }
        }
        selected
    }
}

#[test]
fn feral_encounter_preserves_the_looked_card_permission_and_delayed_damage_targets() {
    use crate::alternative_cast::CastingMethod;
    use crate::decision::{LegalAction, compute_legal_actions};

    let definition = parse_oracle_card_definition("Feral Encounter");
    let spell_effect = definition
        .spell_effect
        .as_ref()
        .expect("Feral Encounter should have a spell effect");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let filler = CardDefinitionBuilder::new(CardId::new(), "Feral Looked Filler")
        .card_types(vec![CardType::Sorcery])
        .build();
    let outside_top_five = game.create_object_from_definition(
        &creature("Outside Feral Top Five", 2, 2),
        alice,
        Zone::Library,
    );
    let looked_fillers = (0..4)
        .map(|_| game.create_object_from_definition(&filler, alice, Zone::Library))
        .collect::<Vec<_>>();
    let chosen = game.create_object_from_definition(
        &creature("Chosen Feral Creature", 2, 2),
        alice,
        Zone::Library,
    );
    let chosen_stable = game.object(chosen).expect("chosen card").stable_id;
    let unlinked_exile = game.create_object_from_definition(
        &creature("Unlinked Exiled Creature", 2, 2),
        alice,
        Zone::Exile,
    );

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, alice));
    let mut decisions = FeralEncounterDecisions::default();
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Feral Encounter should resolve");

    assert_eq!(zone_by_stable_id(&game, chosen_stable), Some(Zone::Exile));
    assert_eq!(
        game.object(outside_top_five).expect("outside card").zone,
        Zone::Library
    );
    assert!(looked_fillers.iter().all(|id| {
        game.object(*id)
            .is_some_and(|object| object.zone == Zone::Library)
    }));
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    let actions = compute_legal_actions(&game, alice);
    let chosen_now = game
        .find_object_by_stable_id(chosen_stable)
        .expect("chosen card should retain stable identity in exile");
    let can_cast_from_exile = |candidate| {
        actions.iter().any(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::PlayFrom { .. },
                } if *spell_id == candidate
            )
        })
    };
    assert!(can_cast_from_exile(chosen_now));
    assert!(!can_cast_from_exile(unlinked_exile));

    let cast_action = actions
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::CastSpell {
                    spell_id,
                    from_zone: Zone::Exile,
                    casting_method: CastingMethod::PlayFrom { .. },
                } if *spell_id == chosen_now
            )
        })
        .expect("the selected creature should be castable this turn");
    let mut trigger_queue = TriggerQueue::new();
    let mut priority_state = PriorityLoopState::new(game.players_in_game());
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut priority_state,
        &PriorityResponse::PriorityAction(cast_action),
        &mut decisions,
    )
    .expect("the exiled creature cast should complete");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("the exiled creature should resolve");
    assert_eq!(
        zone_by_stable_id(&game, chosen_stable),
        Some(Zone::Battlefield)
    );

    let damage_source = game.create_object_from_definition(
        &creature("Feral Damage Source", 4, 4),
        alice,
        Zone::Battlefield,
    );
    let own_decoy = game.create_object_from_definition(
        &creature("Feral Own Recipient Decoy", 1, 10),
        alice,
        Zone::Battlefield,
    );
    let damage_target = game.create_object_from_definition(
        &creature("Feral Damage Target", 1, 10),
        bob,
        Zone::Battlefield,
    );
    let enemy_source_decoy = game.create_object_from_definition(
        &creature("Feral Enemy Source Decoy", 7, 7),
        bob,
        Zone::Battlefield,
    );

    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::BeginCombat);
    let event = TriggerEvent::new_with_provenance(
        crate::events::BeginningOfCombatEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let entries = crate::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(
        entries.len(),
        1,
        "the next combat should consume one delayed trigger"
    );
    let mut queue = TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Feral Encounter's delayed trigger should accept its targets");

    let target_context = decisions
        .target_contexts
        .last()
        .expect("the delayed trigger should request targets");
    let source_requirement = target_context
        .requirements
        .iter()
        .find(|requirement| {
            requirement
                .legal_targets
                .contains(&Target::Object(damage_source))
        })
        .expect("the controlled damage-source requirement");
    assert!(
        !source_requirement
            .legal_targets
            .contains(&Target::Object(enemy_source_decoy))
    );
    let recipient_requirement = target_context
        .requirements
        .iter()
        .find(|requirement| {
            requirement
                .legal_targets
                .contains(&Target::Object(damage_target))
        })
        .expect("the optional not-controlled recipient requirement");
    assert_eq!(
        (
            recipient_requirement.min_targets,
            recipient_requirement.max_targets
        ),
        (0, Some(1))
    );
    assert!(
        !recipient_requirement
            .legal_targets
            .contains(&Target::Object(own_decoy))
    );

    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Feral Encounter's delayed trigger should resolve");
    assert_eq!(game.damage_on(damage_target), 4);
    assert_eq!(game.damage_on(own_decoy), 0);
    assert_eq!(game.damage_on(enemy_source_decoy), 0);
    assert!(game.effect_store.delayed_triggers.is_empty());

    // Keep the parsed program live in this test even if stack setup changes.
    assert!(!spell_effect.flattened_default_effects().is_empty());
}

struct SelectiveAdaptationDecisions {
    refuse_all: bool,
}

impl DecisionMaker for SelectiveAdaptationDecisions {
    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if self.refuse_all {
            return Vec::new();
        }

        for name in ["Flying and First Strike Pick", "First Strike Pick"] {
            if let Some(candidate) = ctx
                .candidates
                .iter()
                .find(|candidate| candidate.name == name)
            {
                return vec![candidate.id];
            }
        }
        Vec::new()
    }
}

fn resolve_selective_adaptation(
    game: &mut crate::GameState,
    controller: PlayerId,
    decisions: &mut dyn DecisionMaker,
) {
    let definition = parse_oracle_card_definition("Selective Adaptation");
    let spell = game.create_object_from_definition(&definition, controller, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, controller));
    crate::game_loop::resolve_stack_entry_with(game, decisions)
        .expect("Selective Adaptation should resolve");
}

#[test]
fn selective_adaptation_uses_distinct_keyword_slots_then_partitions_the_revealed_set() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let outside = game.create_object_from_definition(
        &creature_with_abilities(
            "Outside Selective Top Seven",
            [crate::static_abilities::StaticAbility::flying()],
        ),
        alice,
        Zone::Library,
    );
    let filler = CardDefinitionBuilder::new(CardId::new(), "Selective Filler")
        .card_types(vec![CardType::Land])
        .build();
    let fillers = (0..5)
        .map(|_| game.create_object_from_definition(&filler, alice, Zone::Library))
        .collect::<Vec<_>>();
    let filler_stables = fillers
        .iter()
        .map(|id| game.object(*id).expect("filler").stable_id)
        .collect::<Vec<_>>();
    let first_strike = game.create_object_from_definition(
        &creature_with_abilities(
            "First Strike Pick",
            [crate::static_abilities::StaticAbility::first_strike()],
        ),
        alice,
        Zone::Library,
    );
    let flying_and_first = game.create_object_from_definition(
        &creature_with_abilities(
            "Flying and First Strike Pick",
            [
                crate::static_abilities::StaticAbility::flying(),
                crate::static_abilities::StaticAbility::first_strike(),
            ],
        ),
        alice,
        Zone::Library,
    );
    let first_stable = game
        .object(first_strike)
        .expect("first strike card")
        .stable_id;
    let flying_stable = game
        .object(flying_and_first)
        .expect("multikeyword card")
        .stable_id;

    let mut decisions = SelectiveAdaptationDecisions { refuse_all: false };
    resolve_selective_adaptation(&mut game, alice, &mut decisions);

    assert_eq!(
        zone_by_stable_id(&game, flying_stable),
        Some(Zone::Battlefield)
    );
    assert_eq!(zone_by_stable_id(&game, first_stable), Some(Zone::Hand));
    assert!(
        filler_stables
            .iter()
            .all(|stable| zone_by_stable_id(&game, *stable) == Some(Zone::Graveyard))
    );
    assert_eq!(
        game.object(outside).expect("outside card").zone,
        Zone::Library
    );
}

#[test]
fn selective_adaptation_does_not_allow_declining_a_available_keyword_or_battlefield_choice() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Mandatory Choice Filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..6 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let flying = game.create_object_from_definition(
        &creature_with_abilities(
            "Mandatory Flying Pick",
            [crate::static_abilities::StaticAbility::flying()],
        ),
        alice,
        Zone::Library,
    );
    let flying_stable = game.object(flying).expect("flying card").stable_id;

    let mut decisions = SelectiveAdaptationDecisions { refuse_all: true };
    resolve_selective_adaptation(&mut game, alice, &mut decisions);

    assert_eq!(
        zone_by_stable_id(&game, flying_stable),
        Some(Zone::Battlefield),
        "a mandatory public-zone choice must be filled even if the decision maker submits none"
    );
}
