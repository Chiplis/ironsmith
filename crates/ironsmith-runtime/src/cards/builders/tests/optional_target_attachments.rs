#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[derive(Default)]
struct TargetProbe {
    proposed: Vec<crate::game_state::Target>,
    contexts: Vec<crate::decisions::context::TargetsContext>,
}

impl TargetProbe {
    fn choosing(proposed: Vec<crate::game_state::Target>) -> Self {
        Self {
            proposed,
            contexts: Vec::new(),
        }
    }
}

impl crate::decision::DecisionMaker for TargetProbe {
    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        self.contexts.push(ctx.clone());
        self.proposed.clone()
    }
}

fn combat_game() -> (crate::GameState, PlayerId, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::BeginCombat);
    (game, alice, bob)
}

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn equipment(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build()
}

fn record_entry_this_turn(game: &mut crate::GameState, object: ObjectId) {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object).expect("entered permanent should exist"),
        game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            object,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

fn etb_event(game: &crate::GameState, object: ObjectId) -> crate::triggers::TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object)
            .expect("entering permanent should exist"),
        game,
    );
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            object,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
}

fn combat_event(player: PlayerId) -> crate::triggers::TriggerEvent {
    crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfCombatEvent::new(player),
        crate::provenance::ProvNodeId::default(),
    )
}

fn source_trigger_entries(
    game: &crate::GameState,
    source: ObjectId,
    event: &crate::triggers::TriggerEvent,
) -> Vec<crate::triggers::TriggeredAbilityEntry> {
    crate::triggers::check_triggers(game, event)
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect()
}

fn announce_source_trigger(
    game: &mut crate::GameState,
    source: ObjectId,
    event: crate::triggers::TriggerEvent,
    proposed: Vec<crate::game_state::Target>,
) -> (Vec<crate::game_state::Target>, TargetProbe) {
    let entries = source_trigger_entries(game, source, &event);
    assert_eq!(entries.len(), 1, "expected exactly one source trigger");
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    let mut probe = TargetProbe::choosing(proposed);
    crate::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut probe)
        .expect("trigger target announcement should succeed");
    let targets = game
        .stack
        .last()
        .expect("trigger should be on the stack")
        .targets
        .clone();
    (targets, probe)
}

fn resolve_trigger(game: &mut crate::GameState, probe: &mut TargetProbe) {
    crate::game_loop::resolve_stack_entry_with(game, probe).expect("trigger should resolve");
}

fn attachment_target(game: &crate::GameState, attachment: ObjectId) -> Option<ObjectId> {
    match game
        .object(attachment)
        .and_then(|object| object.attached_to)
    {
        Some(crate::object::AttachmentTarget::Object(target)) => Some(target),
        _ => None,
    }
}

fn optional_creature_requirement(
    ctx: &crate::decisions::context::TargetsContext,
    legal_creature: ObjectId,
) -> &crate::decisions::context::TargetRequirementContext {
    ctx.requirements
        .iter()
        .find(|requirement| {
            requirement
                .legal_targets
                .contains(&crate::game_state::Target::Object(legal_creature))
                && requirement.min_targets == 0
                && requirement.max_targets == Some(1)
        })
        .expect("expected an optional creature target requirement")
}

fn named_attach_effect(name: &str) -> (CardDefinition, crate::effects::AttachObjectsEffect) {
    let definition = parse_oracle_card_definition(name);
    let attach = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<crate::effects::AttachObjectsEffect>())
                .cloned(),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{name} should contain a triggered attachment effect"));
    (definition, attach)
}

#[test]
fn named_optional_attachment_cards_preserve_zero_or_one_destination_structure() {
    for name in [
        "Assassin Gauntlet",
        "Bespoke Battlegarb",
        "Blacksmith's Talent",
    ] {
        let (definition, attach) = named_attach_effect(name);
        assert!(attach.target.is_target(), "{name}: {attach:#?}");
        assert_eq!(
            attach.target.count(),
            ChoiceCount::up_to(1),
            "{name} must preserve its optional destination count: {attach:#?}"
        );
        assert!(
            canonical_compiled_lines(&definition)
                .join(" ")
                .to_ascii_lowercase()
                .contains("up to one target creature you control"),
            "{name} compiled text must retain the optional target"
        );
    }
}

#[test]
fn assassin_gauntlet_decline_still_targets_an_opponent_and_taps_their_creatures() {
    let definition = parse_oracle_card_definition("Assassin Gauntlet");
    let (mut game, alice, bob) = combat_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally =
        game.create_object_from_definition(&creature("Legal Ally"), alice, Zone::Battlefield);
    let enemy = game.create_object_from_definition(&creature("Enemy"), bob, Zone::Battlefield);

    let event = etb_event(&game, source);
    let (targets, mut probe) = announce_source_trigger(
        &mut game,
        source,
        event,
        vec![crate::game_state::Target::Player(bob)],
    );
    assert_eq!(targets, vec![crate::game_state::Target::Player(bob)]);
    let ctx = probe
        .contexts
        .first()
        .expect("target context should be captured");
    let optional = optional_creature_requirement(ctx, ally);
    assert!(
        !optional
            .legal_targets
            .contains(&crate::game_state::Target::Object(enemy))
    );
    let opponent = ctx
        .requirements
        .iter()
        .find(|requirement| {
            requirement
                .legal_targets
                .contains(&crate::game_state::Target::Player(bob))
                && requirement.min_targets == 1
        })
        .expect("Assassin Gauntlet must still require a target opponent");
    assert_eq!(opponent.max_targets, Some(1));

    resolve_trigger(&mut game, &mut probe);
    assert_eq!(attachment_target(&game, source), None);
    assert!(game.is_tapped(enemy));
    assert!(!game.is_tapped(ally));
}

#[test]
fn assassin_gauntlet_attaches_to_a_legal_ally_and_rejects_enemy_creatures() {
    let definition = parse_oracle_card_definition("Assassin Gauntlet");
    let (mut game, alice, bob) = combat_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let ally =
        game.create_object_from_definition(&creature("Chosen Ally"), alice, Zone::Battlefield);
    let enemy =
        game.create_object_from_definition(&creature("Illegal Enemy"), bob, Zone::Battlefield);

    let event = etb_event(&game, source);
    let (targets, mut probe) = announce_source_trigger(
        &mut game,
        source,
        event,
        vec![
            crate::game_state::Target::Object(ally),
            crate::game_state::Target::Player(bob),
        ],
    );
    assert_eq!(
        targets,
        vec![
            crate::game_state::Target::Object(ally),
            crate::game_state::Target::Player(bob),
        ]
    );
    let optional = optional_creature_requirement(&probe.contexts[0], ally);
    assert!(
        !optional
            .legal_targets
            .contains(&crate::game_state::Target::Object(enemy))
    );

    resolve_trigger(&mut game, &mut probe);
    assert_eq!(attachment_target(&game, source), Some(ally));
    assert!(game.is_tapped(enemy));
}

#[test]
fn bespoke_battlegarb_requires_celebration_then_allows_decline_or_legal_attachment() {
    for choose_target in [false, true] {
        let definition = parse_oracle_card_definition("Bespoke Battlegarb");
        let (mut game, alice, bob) = combat_game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let ally =
            game.create_object_from_definition(&creature("Legal Ally"), alice, Zone::Battlefield);
        let enemy =
            game.create_object_from_definition(&creature("Illegal Enemy"), bob, Zone::Battlefield);
        let celebrant_a = game.create_object_from_definition(
            &equipment("Celebration A"),
            alice,
            Zone::Battlefield,
        );
        let celebrant_b = game.create_object_from_definition(
            &equipment("Celebration B"),
            alice,
            Zone::Battlefield,
        );
        record_entry_this_turn(&mut game, celebrant_a);
        record_entry_this_turn(&mut game, celebrant_b);

        let proposed = choose_target
            .then_some(crate::game_state::Target::Object(ally))
            .into_iter()
            .collect();
        let (targets, mut probe) =
            announce_source_trigger(&mut game, source, combat_event(alice), proposed);
        let optional = optional_creature_requirement(&probe.contexts[0], ally);
        assert!(
            !optional
                .legal_targets
                .contains(&crate::game_state::Target::Object(enemy))
        );
        assert_eq!(targets.len(), usize::from(choose_target));

        resolve_trigger(&mut game, &mut probe);
        assert_eq!(
            attachment_target(&game, source),
            choose_target.then_some(ally)
        );
    }

    let definition = parse_oracle_card_definition("Bespoke Battlegarb");
    let (mut game, alice, _bob) = combat_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let only_entry =
        game.create_object_from_definition(&equipment("Only Entry"), alice, Zone::Battlefield);
    record_entry_this_turn(&mut game, only_entry);
    assert!(
        source_trigger_entries(&game, source, &combat_event(alice)).is_empty(),
        "Bespoke Battlegarb must not trigger below the celebration threshold"
    );
}

#[test]
fn blacksmiths_talent_level_two_requires_own_equipment_but_allows_zero_creatures() {
    for choose_creature in [false, true] {
        let definition = parse_oracle_card_definition("Blacksmith's Talent");
        let (mut game, alice, bob) = combat_game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.add_counters(source, crate::object::CounterType::Level, 1);
        let own_equipment = game.create_object_from_definition(
            &equipment("Own Equipment"),
            alice,
            Zone::Battlefield,
        );
        let enemy_equipment = game.create_object_from_definition(
            &equipment("Enemy Equipment"),
            bob,
            Zone::Battlefield,
        );
        let ally =
            game.create_object_from_definition(&creature("Legal Ally"), alice, Zone::Battlefield);
        let enemy =
            game.create_object_from_definition(&creature("Illegal Enemy"), bob, Zone::Battlefield);

        let mut proposed = vec![crate::game_state::Target::Object(own_equipment)];
        if choose_creature {
            proposed.push(crate::game_state::Target::Object(ally));
        }
        let (targets, mut probe) =
            announce_source_trigger(&mut game, source, combat_event(alice), proposed);
        let ctx = &probe.contexts[0];
        let equipment_requirement = ctx
            .requirements
            .iter()
            .find(|requirement| {
                requirement
                    .legal_targets
                    .contains(&crate::game_state::Target::Object(own_equipment))
                    && requirement.min_targets == 1
            })
            .expect("Blacksmith's Talent must require one Equipment target");
        assert_eq!(equipment_requirement.max_targets, Some(1));
        assert!(
            !equipment_requirement
                .legal_targets
                .contains(&crate::game_state::Target::Object(enemy_equipment))
        );
        let creature_requirement = optional_creature_requirement(ctx, ally);
        assert!(
            !creature_requirement
                .legal_targets
                .contains(&crate::game_state::Target::Object(enemy))
        );
        assert_eq!(targets.len(), 1 + usize::from(choose_creature));

        resolve_trigger(&mut game, &mut probe);
        assert_eq!(
            attachment_target(&game, own_equipment),
            choose_creature.then_some(ally)
        );
    }

    let definition = parse_oracle_card_definition("Blacksmith's Talent");
    let (mut game, alice, _bob) = combat_game();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(
        source_trigger_entries(&game, source, &combat_event(alice)).is_empty(),
        "Blacksmith's Talent combat ability must remain locked before level 2"
    );
}
