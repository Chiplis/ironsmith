//! UNVALIDATED implementation-first regressions for written tap/untap costs.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{ManaPaymentContext, SelectObjectsContext, TargetsContext};
use ironsmith::effects::{ChooseObjectsEffect, UntapEffect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{
    CardId, CardType, ColorSet, GameProgress, GameState, ObjectId, PlayerId, Target, Zone,
};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/tap_state_cost_selectors.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let fixture = fixtures()
        .into_iter()
        .find(|card| card["name"] == name)
        .unwrap();
    let text = fixture["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for symbol in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(alice).unwrap().mana_pool.add(symbol, 10);
    }
    let filler = CardBuilder::new(CardId::new(), "Library filler")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..5 {
        game.create_object_from_card(&filler, alice, Zone::Library);
    }
    game
}
fn creature(game: &mut GameState, owner: PlayerId, color: ColorSet, tapped: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Cost creature")
        .card_types(vec![CardType::Creature])
        .color_indicator(color)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
    if tapped {
        game.tap(id);
    }
    id
}
fn chosen_cost_ability(def: &CardDefinition, count: usize, untap: bool) -> usize {
    def.abilities
        .iter()
        .position(|ability| {
            let AbilityKind::Activated(a) = &ability.kind else {
                return false;
            };
            a.mana_cost
                .costs()
                .iter()
                .filter_map(|cost| cost.effect_ref())
                .filter_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
                .any(|choose| {
                    choose.count.min == count
                        && if untap {
                            choose.filter.tapped
                        } else {
                            choose.filter.untapped
                        }
                })
        })
        .expect("a typed written tap/untap choice cost")
}
#[derive(Default)]
struct Choices {
    objects: Vec<ObjectId>,
    target: Option<Target>,
    excluded_target: Option<Target>,
    cancel_mana: bool,
    saw_cancel: bool,
}
impl DecisionMaker for Choices {
    fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if self.objects.is_empty() {
            return vec![];
        }
        assert_eq!(ctx.min, self.objects.len());
        assert_eq!(ctx.max, Some(self.objects.len()));
        for id in &self.objects {
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
                .all(|candidate| self.objects.contains(&candidate.id)),
            "the offered cost objects must retain exact type, state, control, and attachment scope"
        );
        self.objects.clone()
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(excluded) = self.excluded_target {
            assert!(
                !ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&excluded))
            );
        }
        if let Some(target) = self.target {
            assert!(
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(&target))
            );
            vec![target]
        } else {
            SelectFirstDecisionMaker.decide_targets(game, ctx)
        }
    }
    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &ManaPaymentContext,
    ) -> ManaPaymentResponse {
        if self.cancel_mana {
            self.saw_cancel = true;
            ManaPaymentResponse::Cancel
        } else {
            SelectFirstDecisionMaker.decide_mana_payment(game, ctx)
        }
    }
}
fn activate(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    assert!(
        compute_legal_actions(game, PlayerId::from_index(0))
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
        dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_mana_ability.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("unfinished payment: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none() && state.pending_mana_ability.is_none());
}

#[test]
fn frozen_selector_family_strictly_compiles_and_transports_real_costs() {
    let cards = fixtures();
    assert_eq!(cards.len(), 9);
    assert_eq!(
        cards
            .iter()
            .filter(|c| c["proposed_coverage"] == "complete")
            .count(),
        9
    );
    for card in cards {
        for definition in definitions(card["name"].as_str().unwrap()) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            if ["Benthic Explorers", "Crackleburr", "Halo Fountain"]
                .contains(&definition.card.name.as_str())
            {
                assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
                    AbilityKind::Activated(a) if a.mana_cost.costs().iter().any(|cost|
                        cost.effect_ref().is_some_and(|effect| effect.downcast_ref::<UntapEffect>().is_some())))));
            }
        }
    }
}

#[test]
fn benthic_explorers_untaps_an_opponents_land_and_uses_that_cost_antecedent_for_mana() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Benthic Explorers") {
        let mut game = game();
        game.player_mut(alice).unwrap().mana_pool.empty();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let land_def =
            compile_to_runtime_definition("Mana fixture", "Type: Land\n{T}: Add {R}.", false)
                .unwrap();
        let theirs = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);
        let yours = game.create_object_from_definition(&land_def, alice, Zone::Battlefield);
        game.tap(yours);
        let index = chosen_cost_ability(&definition, 1, true);
        let action = LegalAction::ActivateManaAbility {
            source,
            ability_index: index,
        };
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action)
        );
        game.tap(theirs);
        let mut dm = Choices {
            objects: vec![theirs],
            ..Default::default()
        };
        activate(&mut game, action, &mut dm);
        assert!(
            game.stack_is_empty(),
            "mana ability resolves during activation"
        );
        assert!(game.is_tapped(source));
        assert!(!game.is_tapped(theirs));
        assert!(game.is_tapped(yours));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 1);
        assert_eq!(game.player(alice).unwrap().mana_pool.red, 1);
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn crackleburr_q_reserves_its_own_tapped_state_and_untaps_two_other_blue_creatures() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Crackleburr") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.tap(source);
        let one = creature(&mut game, alice, ColorSet::BLUE, true);
        let index = chosen_cost_ability(&definition, 2, true);
        let action = LegalAction::ActivateAbility {
            source,
            ability_index: index,
        };
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action)
        );
        game.remove_summoning_sickness(source);
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action),
            "{{Q}} and chosen untap cannot spend one object's tapped state twice"
        );
        let two = creature(&mut game, alice, ColorSet::BLUE, true);
        let wrong_color = creature(&mut game, alice, ColorSet::RED, true);
        let foreign = creature(&mut game, bob, ColorSet::BLUE, true);
        let target = creature(&mut game, bob, ColorSet::RED, false);
        let mut dm = Choices {
            objects: vec![one, two],
            target: Some(Target::Object(target)),
            ..Default::default()
        };
        activate(&mut game, action, &mut dm);
        for id in [source, one, two] {
            assert!(!game.is_tapped(id));
        }
        for id in [wrong_color, foreign] {
            assert!(game.is_tapped(id));
        }
        assert_eq!(game.object(target).unwrap().zone, Zone::Battlefield);
        resolve_stack_entry(&mut game).unwrap();
        assert!(
            game.player(bob)
                .unwrap()
                .hand
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Cost creature")
        );
    }
}

#[test]
fn halo_fountain_exact_fifteen_cost_is_paid_before_the_win_effect_and_can_be_cancelled() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Halo Fountain") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut payers: Vec<_> = (0..14)
            .map(|_| creature(&mut game, alice, ColorSet::GREEN, true))
            .collect();
        let action = LegalAction::ActivateAbility {
            source,
            ability_index: chosen_cost_ability(&definition, 15, true),
        };
        let mana_before = game.player(alice).unwrap().mana_pool.total();
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action)
        );
        assert!(payers.iter().all(|id| game.is_tapped(*id)));
        assert!(!game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), mana_before);
        payers.push(creature(&mut game, alice, ColorSet::GREEN, true));
        // These creatures are summoning sick, but the written untap cost does not use {Q}.
        let mut cancelled = Choices {
            objects: payers.clone(),
            cancel_mana: true,
            ..Default::default()
        };
        activate(&mut game, action.clone(), &mut cancelled);
        assert!(cancelled.saw_cancel);
        assert!(game.stack_is_empty());
        assert!(!game.is_tapped(source));
        assert!(payers.iter().all(|id| game.is_tapped(*id)));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), mana_before);
        let mut dm = Choices {
            objects: payers.clone(),
            ..Default::default()
        };
        activate(&mut game, action, &mut dm);
        assert!(game.is_tapped(source));
        assert!(payers.iter().all(|id| !game.is_tapped(*id)));
        assert!(game.player(bob).unwrap().is_in_game());
        resolve_stack_entry(&mut game).unwrap();
        assert!(game.player(alice).unwrap().is_in_game());
        assert!(!game.player(bob).unwrap().is_in_game());
    }
}

#[test]
fn natures_chosen_taps_only_its_enchanted_creature_without_tap_symbol_sickness() {
    use ironsmith::object::AttachmentTarget;
    let alice = PlayerId::from_index(0);
    for definition in definitions("Nature's Chosen") {
        let mut game = game();
        let host = creature(&mut game, alice, ColorSet::WHITE, false);
        let target = creature(&mut game, alice, ColorSet::WHITE, true);
        let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(host)));
        game.refresh_continuous_state().unwrap();
        let action = LegalAction::ActivateAbility {
            source: aura,
            ability_index: chosen_cost_ability(&definition, 1, false),
        };
        let mut dm = Choices {
            objects: vec![host],
            target: Some(Target::Object(target)),
            excluded_target: None,
            ..Default::default()
        };
        activate(&mut game, action.clone(), &mut dm);
        assert!(game.is_tapped(host));
        assert!(!game.is_tapped(aura));
        assert!(game.is_tapped(target));
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action)
        );
        game.untap(host);
        assert!(
            !compute_legal_actions(&game, alice)
                .unwrap()
                .contains(&action),
            "the once-each-turn cap is separate from tap availability"
        );
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        assert!(
            !game.is_tapped(target),
            "target untaps when the ability resolves after Aura departure"
        );
    }
}

#[test]
fn tagged_tap_state_cost_preflight_is_fail_closed_and_does_not_change_state() {
    use ironsmith::costs::{Cost, CostContext};
    use ironsmith::effect::Effect;
    use ironsmith::effects::TapEffect;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::target::ChooseSpec;
    let alice = PlayerId::from_index(0);
    let mut game = game();
    let source = creature(&mut game, alice, ColorSet::BLUE, false);
    let chosen = creature(&mut game, alice, ColorSet::BLUE, false);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = CostContext::new(source, alice, &mut dm);
    let untap = Cost::try_effect(Effect::new(UntapEffect::with_spec(ChooseSpec::Tagged(
        "cost_objects".into(),
    ))))
    .unwrap();
    let tap = Cost::try_effect(Effect::new(TapEffect::with_spec(ChooseSpec::Tagged(
        "cost_objects".into(),
    ))))
    .unwrap();
    let life = game.player(alice).unwrap().life;
    let mana = game.player(alice).unwrap().mana_pool.total();
    assert!(
        untap.can_pay(&game, &ctx).is_err(),
        "unbound tag is not an empty paid choice"
    );
    ctx.tagged_objects.insert(
        "cost_objects".into(),
        vec![ObjectSnapshot::from_object(
            game.object(chosen).unwrap(),
            &game,
        )],
    );
    assert!(untap.can_pay(&game, &ctx).is_err());
    assert!(tap.can_pay(&game, &ctx).is_ok());
    assert!(!game.is_tapped(chosen));
    assert_eq!(game.player(alice).unwrap().life, life);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), mana);
    game.tap(chosen);
    assert!(untap.can_pay(&game, &ctx).is_ok());
    assert!(tap.can_pay(&game, &ctx).is_err());
    assert!(game.is_tapped(chosen));
    let moved = game.move_object_by_effect(chosen, Zone::Graveyard).unwrap();
    let returned = game
        .move_object_by_effect(moved, Zone::Battlefield)
        .unwrap();
    game.tap(returned);
    assert!(
        untap.can_pay(&game, &ctx).is_err(),
        "a later incarnation does not pay the earlier chosen-object cost"
    );
    assert!(game.is_tapped(returned));
}

#[test]
fn krovikan_plague_cost_and_counter_rider_name_the_same_enchanted_creature() {
    use ironsmith::object::{AttachmentTarget, CounterType};
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Krovikan Plague") {
        let mut game = game();
        let host = creature(&mut game, alice, ColorSet::WHITE, false);
        let bystander = creature(&mut game, alice, ColorSet::WHITE, false);
        let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(host)));
        game.refresh_continuous_state().unwrap();
        let mut dm = Choices {
            objects: vec![host],
            target: Some(Target::Player(bob)),
            ..Default::default()
        };
        let action = LegalAction::ActivateAbility {
            source: aura,
            ability_index: chosen_cost_ability(&definition, 1, false),
        };
        activate(&mut game, action, &mut dm);
        assert!(game.is_tapped(host));
        assert!(!game.is_tapped(bystander));
        assert!(!game.is_tapped(aura));
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 19);
        assert_eq!(game.counter_count(host, CounterType::MinusZeroMinusOne), 1);
        assert_eq!(
            game.counter_count(bystander, CounterType::MinusZeroMinusOne),
            0
        );
        assert_eq!(game.current_toughness(host), Some(1));
    }
}
