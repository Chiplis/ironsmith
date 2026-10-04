//! UNVALIDATED: authored during the implementation-first card campaign.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    ManaPaymentContext, NumberContext, SelectObjectsContext, TargetsContext,
};
use ironsmith::effects::ChooseObjectsEffect;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/x_tap_costs.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let card = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    let text = card["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}

fn x_ability(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return false;
            };
            activated.mana_cost.costs().iter().any(|cost| {
                cost.effect_ref()
                    .and_then(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
                    .is_some_and(|choose| choose.count.dynamic_x)
            })
        })
        .expect("printed activation has a typed X object-choice cost")
}

fn setup(
    definition: &CardDefinition,
    mana: u32,
) -> (GameState, ObjectId, Vec<ObjectId>, Vec<ObjectId>) {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, mana);
    let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
    game.remove_summoning_sickness(source);
    let artifact = CardBuilder::new(CardId::new(), "Eligible artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let creature = CardBuilder::new(CardId::new(), "Wrong type creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let eligible = (0..2)
        .map(|_| game.create_object_from_card(&artifact, alice, Zone::Battlefield))
        .collect();
    let foreign = game.create_object_from_card(&artifact, bob, Zone::Battlefield);
    let wrong_type = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let tapped = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
    game.tap(tapped);
    (game, source, eligible, vec![foreign, wrong_type, tapped])
}

struct Choices {
    x: u32,
    expected_max: u32,
    target: Target,
    eligible: Vec<ObjectId>,
    saw_x: bool,
    selected: Vec<ObjectId>,
    cancel_mana: bool,
    saw_cancel: bool,
}
impl DecisionMaker for Choices {
    fn decide_number(&mut self, _game: &GameState, ctx: &NumberContext) -> u32 {
        assert!(ctx.is_x_value);
        assert_eq!(ctx.min, 0);
        assert_eq!(
            ctx.max, self.expected_max,
            "X is bounded by distinct eligible objects and payable mana"
        );
        assert!(self.x <= ctx.max);
        self.saw_x = true;
        self.x
    }
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert!(
            ctx.requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&self.target))
        );
        vec![self.target]
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_eq!(ctx.min, self.x as usize);
        assert_eq!(ctx.max, Some(self.x as usize));
        let selected = self
            .eligible
            .iter()
            .copied()
            .take(self.x as usize)
            .collect::<Vec<_>>();
        for id in &selected {
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == *id && candidate.legal)
            );
        }
        assert!(
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .all(|candidate| self.eligible.contains(&candidate.id)),
            "no source, tapped, wrong-type, or foreign card may pay"
        );
        self.selected = selected.clone();
        selected
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        if self.cancel_mana {
            self.saw_cancel = true;
            return ManaPaymentResponse::Cancel;
        }
        SelectFirstDecisionMaker.decide_mana_payment(game, ctx)
    }
}

fn activate(
    game: &mut GameState,
    definition: &CardDefinition,
    source: ObjectId,
    choices: &mut Choices,
) {
    let alice = PlayerId::from_index(0);
    let action = LegalAction::ActivateAbility {
        source,
        ability_index: x_ability(definition),
    };
    assert!(
        compute_legal_actions(game, alice)
            .unwrap()
            .contains(&action)
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        choices,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished activation: {progress:?}");
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, choices).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert!(choices.saw_x, "X is announced before payment");
}

#[test]
fn all_eight_frozen_x_tap_cards_strict_compile_and_keep_typed_costs_after_transport() {
    let cards = fixtures();
    assert_eq!(cards.len(), 8);
    for card in cards {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let AbilityKind::Activated(activated) =
                &definition.abilities[x_ability(&definition)].kind
            else {
                unreachable!()
            };
            let choices = activated
                .mana_cost
                .costs()
                .iter()
                .filter_map(|cost| cost.effect_ref())
                .filter_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
                .collect::<Vec<_>>();
            assert_eq!(choices.len(), 1);
            assert_eq!(
                choices[0].count,
                ironsmith::effect::ChoiceCount::dynamic_x()
            );
            assert!(choices[0].filter.untapped);
            assert_eq!(
                choices[0].filter.controller,
                Some(ironsmith::target::PlayerFilter::You)
            );
            assert_eq!(choices[0].filter.zone, Some(Zone::Battlefield));
            // Fixed-count sibling on Belisarius remains exactly two; existing
            // artifact count encoding is reused unchanged, not migrated.
            if definition.card.name == "Belisarius Cawl" {
                assert!(definition.abilities.iter().any(|ability| {
                    let AbilityKind::Activated(activated) = &ability.kind else {
                        return false;
                    };
                    activated
                        .mana_cost
                        .costs()
                        .iter()
                        .filter_map(|cost| cost.effect_ref())
                        .filter_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
                        .any(|choose| choose.count == ironsmith::effect::ChoiceCount::exactly(2))
                }));
            }
        }
    }
}

#[test]
fn necron_overlord_pays_exact_x_and_resolves_announced_amount_after_source_and_tap_state_change() {
    let bob = PlayerId::from_index(1);
    for definition in definitions("Necron Overlord") {
        for (mana, x) in [(5, 0), (5, 1), (5, 2), (1, 1)] {
            let (mut game, source, eligible, excluded) = setup(&definition, mana);
            let mut choices = Choices {
                x,
                expected_max: mana.min(2),
                target: Target::Player(bob),
                eligible: eligible.clone(),
                saw_x: false,
                selected: vec![],
                cancel_mana: false,
                saw_cancel: false,
            };
            activate(&mut game, &definition, source, &mut choices);
            assert_eq!(game.stack.len(), 1);
            assert!(game.is_tapped(source));
            assert_eq!(
                game.player(PlayerId::from_index(0))
                    .unwrap()
                    .mana_pool
                    .total(),
                mana - x
            );
            assert_eq!(
                game.player(bob).unwrap().life,
                20,
                "effect has not resolved during payment"
            );
            for (index, id) in eligible.iter().enumerate() {
                assert_eq!(game.is_tapped(*id), index < x as usize);
            }
            assert!(!game.is_tapped(excluded[0]));
            assert!(!game.is_tapped(excluded[1]));
            assert!(game.is_tapped(excluded[2]));
            for id in &eligible {
                game.untap(*id);
            }
            game.move_object_by_effect(source, Zone::Exile).unwrap();
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(
                game.player(bob).unwrap().life,
                20 - x as i32,
                "resolution uses announced X, independent of current tapped count or source zone"
            );
        }
    }
}

#[test]
fn secluded_starforge_nonmana_x_is_not_limited_by_its_fixed_mana_cost_and_expires() {
    for definition in definitions("Secluded Starforge") {
        let (mut game, source, eligible, excluded) = setup(&definition, 2);
        let target = excluded[1];
        let mut choices = Choices {
            x: 2,
            expected_max: 2,
            target: Target::Object(target),
            eligible,
            saw_x: false,
            selected: vec![],
            cancel_mana: false,
            saw_cancel: false,
        };
        activate(&mut game, &definition, source, &mut choices);
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.current_power(target), Some(5));
        assert_eq!(game.current_toughness(target), Some(3));
        assert_eq!(
            game.player(PlayerId::from_index(0))
                .unwrap()
                .mana_pool
                .total(),
            0
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_power(target), Some(3));
    }
}

#[test]
fn cancelling_x_tap_payment_restores_mana_taps_and_pending_activation() {
    for definition in definitions("Necron Overlord") {
        let (mut game, source, eligible, _) = setup(&definition, 5);
        let mut choices = Choices {
            x: 2,
            expected_max: 2,
            target: Target::Player(PlayerId::from_index(1)),
            eligible: eligible.clone(),
            saw_x: false,
            selected: vec![],
            cancel_mana: true,
            saw_cancel: false,
        };
        activate(&mut game, &definition, source, &mut choices);
        assert!(
            choices.saw_cancel,
            "cancel the actual mana payment after announcing X"
        );
        assert!(game.stack_is_empty());
        assert!(!game.is_tapped(source));
        assert!(eligible.iter().all(|id| !game.is_tapped(*id)));
        assert_eq!(
            game.player(PlayerId::from_index(0))
                .unwrap()
                .mana_pool
                .total(),
            5
        );
        assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 20);
        let action = LegalAction::ActivateAbility {
            source,
            ability_index: x_ability(&definition),
        };
        assert!(
            compute_legal_actions(&game, PlayerId::from_index(0))
                .unwrap()
                .contains(&action)
        );
    }
}
