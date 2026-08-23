#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::DecisionMaker;
use crate::game_state::GameState;

const RIPPLE_LINE: &str = "Proliferate, then choose any number of permanents you control that had a counter put on them this way. Those permanents phase out.";

struct RipplesDecisions {
    proliferate_permanents: Vec<ObjectId>,
    phase_candidates: Vec<ObjectId>,
}

impl DecisionMaker for RipplesDecisions {
    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        crate::decisions::specs::ProliferateResponse {
            permanents: self.proliferate_permanents.clone(),
            players: Vec::new(),
        }
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.phase_candidates = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        self.phase_candidates.clone()
    }
}

#[test]
fn ripples_of_potential_phases_only_your_permanents_proliferated_this_way() {
    let definition = parse_oracle_card_definition("Ripples of Potential");
    let spell_effect = definition
        .spell_effect
        .as_ref()
        .expect("Ripples of Potential should have a spell program");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let permanent_definition = CardDefinitionBuilder::new(CardId::new(), "Ripple Counter Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let selected =
        game.create_object_from_definition(&permanent_definition, alice, Zone::Battlefield);
    let skipped =
        game.create_object_from_definition(&permanent_definition, alice, Zone::Battlefield);
    let prevented =
        game.create_object_from_definition(&permanent_definition, alice, Zone::Battlefield);
    let opponent =
        game.create_object_from_definition(&permanent_definition, bob, Zone::Battlefield);
    for permanent in [selected, skipped, prevented, opponent] {
        game.add_counters(permanent, CounterType::PlusOnePlusOne, 1)
            .expect("counter probe should receive an initial counter");
    }
    game.effect_store
        .cant_effects
        .cant_have_counters_placed
        .insert(prevented);

    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = RipplesDecisions {
        proliferate_permanents: vec![selected, prevented, opponent],
        phase_candidates: Vec::new(),
    };
    let mut ctx = crate::effects::ExecutionContext::new(spell, alice, &mut decisions);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell,
        spell_effect,
        None,
        &[],
    )
    .expect("Ripples of Potential should resolve");

    assert_eq!(
        decisions.phase_candidates,
        vec![selected],
        "the later choice must be the intersection of your permanents and this proliferate action's affected set"
    );
    assert_eq!(game.counter_count(selected, CounterType::PlusOnePlusOne), 2);
    assert!(game.is_phased_out(selected));
    assert_eq!(game.counter_count(skipped, CounterType::PlusOnePlusOne), 1);
    assert!(!game.is_phased_out(skipped));
    assert_eq!(
        game.counter_count(prevented, CounterType::PlusOnePlusOne),
        1,
        "proliferate must honor counter-placement prevention"
    );
    assert!(
        !game.is_phased_out(prevented),
        "a permanent that received no counter this way must not enter Ripples' later choice"
    );
    assert_eq!(game.counter_count(opponent, CounterType::PlusOnePlusOne), 2);
    assert!(!game.is_phased_out(opponent));

    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered.iter().any(|line| line == RIPPLE_LINE),
        "Ripples must retain its proliferated-this-way selection surface: {rendered:#?}"
    );
}
