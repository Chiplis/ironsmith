#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Flying\nWhen this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.";

struct EvidenceDecisions {
    accept: bool,
}

impl crate::decision::DecisionMaker for EvidenceDecisions {
    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.accept
    }

    fn decide_objects(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect()
    }
}

fn evidence_card(name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mana_value),
        ]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

fn resolve_death_trigger(accept: bool) -> (crate::GameState, StableId, Vec<StableId>) {
    let definition = parse_oracle_card_definition("Lamplight Phoenix");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let source_stable = game.object(source).expect("source exists").stable_id;
    let evidence = [
        game.create_object_from_definition(&evidence_card("Clue One", 2), alice, Zone::Graveyard),
        game.create_object_from_definition(&evidence_card("Clue Two", 2), alice, Zone::Graveyard),
    ];
    let evidence_stable = evidence
        .iter()
        .map(|id| game.object(*id).expect("evidence exists").stable_id)
        .collect::<Vec<_>>();

    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source).expect("source exists"),
        &game,
    );
    game.move_object_by_effect(source, Zone::Graveyard)
        .expect("source should die");
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            source,
            Zone::Battlefield,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot.clone()),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(vec![snapshot]);
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|trigger| trigger.source == source)
    {
        queue.add(trigger);
    }
    assert_eq!(queue.entries.len(), 1, "the Phoenix should trigger once");

    let mut decisions = EvidenceDecisions { accept };
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("death trigger should go on the stack");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("death trigger should resolve");

    (game, source_stable, evidence_stable)
}

#[test]
fn lamplight_phoenix_keeps_collect_evidence_and_exile_return_provenance() {
    let definition = parse_oracle_card_definition("Lamplight Phoenix");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);
    let debug = format!("{definition:#?}");
    assert!(debug.contains("minimum"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
}

#[test]
fn accepted_collect_evidence_exiles_other_cards_and_returns_source_tapped() {
    let (game, source_stable, evidence_stable) = resolve_death_trigger(true);
    let source_id = game
        .find_object_by_stable_id(source_stable)
        .expect("source remains identifiable");
    let source = game.object(source_id).expect("source remains identifiable");
    assert_eq!(source.zone, Zone::Battlefield);
    assert!(game.is_tapped(source_id));
    for stable in evidence_stable {
        let evidence = game
            .find_object_by_stable_id(stable)
            .and_then(|id| game.object(id))
            .expect("evidence remains identifiable");
        assert_eq!(evidence.zone, Zone::Exile);
    }
}

#[test]
fn declining_collect_evidence_leaves_source_and_cards_in_graveyard() {
    let (game, source_stable, evidence_stable) = resolve_death_trigger(false);
    let source = game
        .find_object_by_stable_id(source_stable)
        .and_then(|id| game.object(id))
        .expect("source remains identifiable");
    assert_eq!(source.zone, Zone::Graveyard);
    for stable in evidence_stable {
        let evidence = game
            .find_object_by_stable_id(stable)
            .and_then(|id| game.object(id))
            .expect("evidence remains identifiable");
        assert_eq!(evidence.zone, Zone::Graveyard);
    }
}
