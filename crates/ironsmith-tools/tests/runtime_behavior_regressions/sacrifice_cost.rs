//! Strict artifact, paid-cast sacrifice discount probes; stop before spell resolution.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{
    DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, DecisionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, PowerToughness, Zone,
};
use serde_json::{Value, json};
fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn setup() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = alice();
    g.turn.priority_player = Some(alice());
    g.turn.phase = ironsmith::Phase::FirstMain;
    g.turn.step = None;
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        g.player_mut(alice()).unwrap().mana_pool.add(color, 30);
    }
    g
}

struct ProbeDm {
    wanted: usize,
    witness_ids: Vec<ObjectId>,
    choices: Vec<Value>,
}
impl DecisionMaker for ProbeDm {
    fn answers_player_choices(&self) -> bool {
        true
    }
    fn decide_boolean(&mut self, _: &GameState, c: &BooleanContext) -> bool {
        let answer = self.wanted > 0;
        self.choices
            .push(json!({"kind":"boolean","description":c.description,"answer":answer}));
        answer
    }
    fn decide_objects(&mut self, g: &GameState, c: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected: Vec<_> = c
            .candidates
            .iter()
            .filter(|o| self.witness_ids.contains(&o.id))
            .take(self.wanted)
            .map(|o| o.id)
            .collect();
        let answer = if selected.len() >= c.min {
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(g, c)
        };
        self.choices.push(json!({"kind":"objects","description":c.description,"min":c.min,"max":c.max,"selected":answer.iter().map(|id|format!("{id:?}")).collect::<Vec<_>>() }));
        answer
    }
    fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(g, c)
    }
    fn decide_targets(&mut self, g: &GameState, c: &TargetsContext) -> Vec<Target> {
        SelectFirstDecisionMaker.decide_targets(g, c)
    }
}
fn perform(
    game: &mut GameState,
    source: ObjectId,
    ability: Option<usize>,
    dm: &mut ProbeDm,
) -> Result<(TriggerQueue, u32), String> {
    game.turn.priority_player = Some(alice());
    let action = compute_legal_actions(game, alice())
        .expect("fixture has complete replacement state")
        .into_iter()
        .find(|a| match a {
            LegalAction::CastSpell { spell_id, .. } => ability.is_none() && *spell_id == source,
            LegalAction::ActivateAbility {
                source: id,
                ability_index,
            } => *id == source && Some(*ability_index) == ability,
            _ => false,
        })
        .ok_or("intended legal action unavailable")?;
    let mana = game.player(alice()).unwrap().mana_pool.total();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .map_err(|e| e.to_string())?;
    for _ in 0..24 {
        if !game.stack.is_empty() {
            return Ok((
                queue,
                mana - game.player(alice()).unwrap().mana_pool.total(),
            ));
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(DecisionContext::SelectOptions(ref c))
                if c.description.starts_with("Choose optional costs") =>
            {
                apply_priority_response_with_dm(
                    game,
                    &mut queue,
                    &mut state,
                    &PriorityResponse::OptionalCosts(if dm.wanted == 0 {
                        vec![]
                    } else {
                        vec![(0, 1)]
                    }),
                    dm,
                )
                .map_err(|e| e.to_string())?
            }
            GameProgress::NeedsDecisionCtx(ref c) if !matches!(c, DecisionContext::Priority(_)) => {
                apply_decision_context_with_dm(game, &mut queue, &mut state, c, dm)
                    .map_err(|e| e.to_string())?
            }
            other => return Err(format!("fixture action failed to reach stack: {other:?}")),
        };
    }
    Err("fixture action decision bound exceeded".into())
}

fn probe(def: &CardDefinition, available: usize, wanted: usize) -> Value {
    probe_with_budget(def, available, wanted, None)
}
fn probe_with_budget(
    def: &CardDefinition,
    available: usize,
    wanted: usize,
    budget: Option<(u32, u32, u32)>,
) -> Value {
    let mut game = setup();
    if let Some((generic, red, black)) = budget {
        let pool = &mut game.player_mut(alice()).unwrap().mana_pool;
        *pool = Default::default();
        pool.add(ManaSymbol::Colorless, generic);
        pool.add(ManaSymbol::Red, red);
        pool.add(ManaSymbol::Black, black);
    }
    let witness = CardDefinitionBuilder::new(CardId::new(), "Sacrifice audit witness")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let witness_ids = (0..available)
        .map(|_| game.create_object_from_definition(&witness, alice(), Zone::Battlefield))
        .collect();
    let source = game.create_object_from_definition(def, alice(), Zone::Hand);
    let mut dm = ProbeDm {
        wanted,
        witness_ids,
        choices: vec![],
    };
    let before = game.player(alice()).unwrap().mana_pool.total();
    let error = perform(&mut game, source, None, &mut dm).err();
    let count = |zone| {
        game.objects_in_deterministic_order()
            .into_iter()
            .filter(|o| o.name == "Sacrifice audit witness" && o.zone == zone)
            .count()
    };
    json!({"actual":{"cast_error":error,"reached_stack":game.stack.iter().any(|entry| game.object(entry.object_id).is_some_and(|o| o.name == def.name())),"mana_paid":before-game.player(alice()).unwrap().mana_pool.total(),"sacrificed":count(Zone::Graveyard),"witnesses_remaining":count(Zone::Battlefield)},"choices":dm.choices})
}

fn assert_discount(name: &str, base: usize, discount: usize, colored: usize) {
    let def = super::load(name);
    for (available, wanted) in [(0, 0), (2, 0), (2, 1), (2, 2), (8, 8)] {
        let result = probe(&def, available, wanted);
        let expected = json!({"cast_error":null,"reached_stack":true,"mana_paid":base.saturating_sub(discount*wanted).max(colored),"sacrificed":wanted,"witnesses_remaining":available-wanted});
        assert_eq!(
            result["actual"], expected,
            "{name}: {wanted} sacrifices; {result:#}"
        );
    }
}
#[test]
fn dargo_sacrifices_reduce_actual_payment() {
    assert_discount("Dargo, the Shipwrecker", 7, 2, 1);
}
#[test]
fn viper_sacrifices_reduce_actual_payment() {
    assert_discount("Rottenmouth Viper", 6, 1, 1);
}
#[test]
fn blood_avatar_sacrifices_reduce_actual_payment() {
    assert_discount("Awaken the Blood Avatar", 8, 2, 2);
}

#[test]
fn sacrifice_discount_allows_cast_with_only_discounted_mana_available() {
    let mut failures = vec![];
    for (name, budget, paid) in [
        ("Dargo, the Shipwrecker", (2, 1, 0), 3),
        ("Rottenmouth Viper", (3, 0, 1), 4),
        ("Awaken the Blood Avatar", (2, 1, 1), 4),
    ] {
        let result = probe_with_budget(&super::load(name), 2, 2, Some(budget));
        let expected = json!({"cast_error":null,"reached_stack":true,"mana_paid":paid,"sacrificed":2,"witnesses_remaining":0});
        if result["actual"] != expected {
            failures.push(json!({"card":name,"expected":expected,"observed":result}));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
