//! Actual full Swords Dance and Magnitude spell lifecycles, direct/artifact.
#[path = "custom_final_next_wave_support.rs"]
mod support;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::{CardDefinition, CounterType, GameProgress, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/revolution-six-family-whole.json"
    ))
    .unwrap();
    let row = data["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "rev-harness-whole-0")
        .unwrap();
    let card = row["representative_cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|card| card["name"] == name)
        .unwrap();
    support::definitions(card).unwrap()
}
struct Choice {
    pay: bool,
    resource: ObjectId,
    target: ObjectId,
}
impl DecisionMaker for Choice {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        if c.description.starts_with("Choose optional costs") {
            return c
                .options
                .iter()
                .filter(|option| option.legal && self.pay)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        if c.candidates
            .iter()
            .any(|candidate| candidate.id == self.resource)
        {
            vec![self.resource]
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        }
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<ironsmith::Target> {
        if c.requirements.iter().any(|r| {
            r.legal_targets
                .contains(&ironsmith::Target::Object(self.target))
        }) {
            vec![ironsmith::Target::Object(self.target)]
        } else {
            SelectFirstDecisionMaker.decide_targets(g, c)
        }
    }
    fn decide_number(&mut self, g: &GameState, c: &NumberContext) -> u32 {
        if c.is_x_value {
            1
        } else {
            SelectFirstDecisionMaker.decide_number(g, c)
        }
    }
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["A".into(), "B".into()], 20);
    g.turn.active_player = A;
    g.turn.priority_player = Some(A);
    g.turn.phase = ironsmith::game_state::Phase::FirstMain;
    g.turn.step = None;
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::Red, 2);
    g.player_mut(A)
        .unwrap()
        .mana_pool
        .add(ironsmith::ManaSymbol::Colorless, 2);
    g
}
fn body(g: &mut GameState, flying: bool) -> ObjectId {
    let text = format!(
        "Type: Creature — Human\nPower/Toughness: 2/5\n{}",
        if flying { "Flying" } else { "" }
    );
    let d = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Independent payment body",
        &text,
        false,
    )
    .unwrap();
    g.create_object_from_definition(&d, A, Zone::Battlefield)
}
fn reenter(g: &mut GameState, id: ObjectId) -> ObjectId {
    let stable = g.object(id).unwrap().stable_id;
    g.move_object_by_effect(id, Zone::Exile).unwrap();
    let exiled = g.find_object_by_stable_id(stable).unwrap();
    g.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
    g.find_object_by_stable_id(stable).unwrap()
}
fn announce(game: &mut GameState, spell: ObjectId, dm: &mut Choice) {
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .expect("actual source spell castable");
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..96 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("casting did not finish");
}

#[test]
fn actual_swords_dance_adds_only_to_the_original_paid_donor_after_the_primary_target() {
    for definition in definitions("Swords Dance") {
        for (pay, again) in [(false, false), (true, false), (true, true)] {
            let mut g = game();
            let donor = body(&mut g, false);
            let target = body(&mut g, false);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choice {
                pay,
                resource: donor,
                target,
            };
            announce(&mut g, spell, &mut dm);
            assert_eq!(g.is_tapped(donor), pay);
            let current = if again { reenter(&mut g, donor) } else { donor };
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(g.counter_count(target, CounterType::PlusOnePlusOne), 2);
            assert_eq!(
                g.counter_count(current, CounterType::PlusOnePlusOne),
                u32::from(pay && !again)
            );
        }
    }
}
#[test]
fn actual_magnitude_excludes_only_exact_paid_donor_and_preserves_player_and_flying_branches() {
    for definition in definitions("Magnitude") {
        for (pay, again) in [(false, false), (true, false), (true, true)] {
            let mut g = game();
            let donor = body(&mut g, false);
            let other = body(&mut g, false);
            let flying = body(&mut g, true);
            let spell = g.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choice {
                pay,
                resource: donor,
                target: other,
            };
            announce(&mut g, spell, &mut dm);
            let current = if again { reenter(&mut g, donor) } else { donor };
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut dm).unwrap();
            assert_eq!(g.player(A).unwrap().life, 19);
            assert_eq!(g.player(B).unwrap().life, 19);
            assert_eq!(g.damage_on(other), 1);
            assert_eq!(g.damage_on(flying), 0);
            assert_eq!(g.damage_on(current), u32::from(!pay || again));
        }
    }
}
