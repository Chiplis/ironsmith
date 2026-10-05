use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction};
use ironsmith::decisions::context::{BooleanContext, TargetsContext};
use ironsmith::effect::Value;
use ironsmith::effects::PayLifeEffect;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const RAUBAHN: &str = "Mana cost: {1}{R}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 2/2\nWard—Pay life equal to Raubahn's power.\nWhenever Raubahn attacks, attach up to one target Equipment you control to target attacking creature.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let encoded = artifact.to_json().unwrap();
    let decoded = CompiledCardArtifact::from_json(&encoded).unwrap();
    assert_eq!(artifact, decoded);
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    [direct, restored]
}

fn assert_dynamic_ward(definition: &CardDefinition) {
    let ward = definition
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(ability) = &ability.kind else {
                return None;
            };
            (ability.id() == StaticAbilityId::Ward).then(|| ability.ward_cost().unwrap())
        })
        .expect("an executable typed ward ability");
    assert_eq!(ward.costs().len(), 1);
    let payment = ward.costs()[0]
        .effect_ref()
        .unwrap()
        .downcast_ref::<PayLifeEffect>()
        .unwrap();
    assert!(
        matches!(payment.amount.unhinted(), Value::PowerOf(target) if matches!(target.unhinted(), ChooseSpec::Source)),
        "{:?}",
        payment.amount
    );
    assert!(matches!(
        payment.player.unhinted(),
        ChooseSpec::Player(ironsmith::PlayerFilter::You)
    ));
}

#[test]
fn ward_source_power_preserves_typed_cost_metadata_and_artifacts() {
    for (name, text) in [
        ("Raubahn, Bull of Ala Mhigo", RAUBAHN.to_string()),
        ("Synthetic Sentinel", RAUBAHN.replace("Raubahn", "Synthetic Sentinel")),
        ("Explicit Sentinel", "Mana cost: {1}{R}\nType: Creature — Human Warrior\nPower/Toughness: 2/2\nWard—Pay life equal to this creature's power.".to_string()),
    ] {
        for definition in definitions(name, &text) {
            assert_dynamic_ward(&definition);
            assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), "{1}{R}");
            assert_eq!(definition.card.card_types, vec![CardType::Creature]);
            assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(2, 2)));
        }
    }
}

struct WardDecisions {
    target: ObjectId,
    pay: bool,
    ward_prompts: usize,
}
impl DecisionMaker for WardDecisions {
    fn decide_boolean(&mut self, _game: &GameState, _context: &BooleanContext) -> bool {
        self.ward_prompts += 1;
        self.pay
    }
    fn decide_targets(&mut self, _game: &GameState, context: &TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.target);
        assert!(
            context
                .requirements
                .iter()
                .any(|requirement| requirement.legal_targets.contains(&target))
        );
        vec![target]
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn cast_targeting_spell(
    game: &mut GameState,
    uncounterable: bool,
    decisions: &mut WardDecisions,
) -> ObjectId {
    let text = format!(
        "Mana cost: {{U}}\nType: Instant\n{}Tap target creature.",
        if uncounterable {
            "This spell can't be countered.\n"
        } else {
            ""
        }
    );
    let spell = compile_to_runtime_definition("Ward probe", text, false).unwrap();
    let alice = PlayerId::from_index(0);
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Hand);
    game.player_mut(alice)
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Blue, 1);
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(LegalAction::CastSpell {
            spell_id,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        }),
        decisions,
    )
    .unwrap();
    for _ in 0..32 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else {
            break;
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, decisions)
                .unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    assert_eq!(
        game.player(alice).unwrap().mana_pool.total(),
        0,
        "the targeting spell's printed cost is paid separately"
    );
    put_triggers_on_stack_with_dm(game, &mut queue, decisions).unwrap();
    spell_id
}

#[test]
fn ward_source_power_uses_current_and_departure_power_at_payment_time() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Raubahn, Bull of Ala Mhigo", RAUBAHN) {
        for leave in [false, true] {
            let mut game = game();
            let target = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let mut decisions = WardDecisions {
                target,
                pay: true,
                ward_prompts: 0,
            };
            cast_targeting_spell(&mut game, false, &mut decisions);
            assert_eq!(game.stack.len(), 2, "spell and actual ward trigger");
            game.add_counters(target, CounterType::PlusOnePlusOne, 3)
                .unwrap();
            assert_eq!(game.current_power(target), Some(5));
            if leave {
                game.move_object_by_effect(target, Zone::Graveyard).unwrap();
            }
            resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
            assert_eq!(decisions.ward_prompts, 1);
            assert_eq!(
                game.player(alice).unwrap().life,
                15,
                "pay using the protected source's resolution/departure power"
            );
            assert_eq!(
                game.player(bob).unwrap().life,
                20,
                "the opponent pays, never the protected permanent's controller"
            );
            assert_eq!(game.stack.len(), 1, "paying preserves the targeting spell");
            resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
            if !leave {
                assert!(game.is_tapped(target));
            }
            assert!(game.stack_is_empty());
        }
    }
}

#[test]
fn ward_source_power_decline_insufficient_life_and_uncounterable_spell() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Raubahn, Bull of Ala Mhigo", RAUBAHN) {
        for (pay, life, uncounterable, paid, survives) in [
            (false, 20, false, false, false),
            (true, 4, false, false, false),
            (true, 5, false, true, true),
            (false, 20, true, false, true),
        ] {
            let mut game = game();
            game.player_mut(alice).unwrap().life = life;
            let target = game.create_object_from_definition(&definition, bob, Zone::Battlefield);
            let mut decisions = WardDecisions {
                target,
                pay,
                ward_prompts: 0,
            };
            cast_targeting_spell(&mut game, uncounterable, &mut decisions);
            assert_eq!(game.stack.len(), 2);
            game.add_counters(target, CounterType::PlusOnePlusOne, 3)
                .unwrap();
            resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
            assert_eq!(
                game.player(alice).unwrap().life,
                life - if paid { 5 } else { 0 }
            );
            assert_eq!(
                game.stack.len(),
                usize::from(survives),
                "ward counters only when unpaid and counterable"
            );
            assert!(!game.is_tapped(target), "the spell has not resolved yet");
            if survives {
                resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
                assert!(game.is_tapped(target));
            }
        }
    }
}

#[test]
fn ward_source_power_ignores_own_targeting_and_clamps_nonpositive_power() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "Mana cost: {1}{R}\nType: Creature — Human Warrior\nPower/Toughness: 2/8\nWard—Pay life equal to this creature's power.";
    for definition in definitions("Durable Sentinel", text) {
        for (owner, counters) in [(alice, 0), (bob, 2), (bob, 3)] {
            let mut game = game();
            let target = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
            let mut decisions = WardDecisions {
                target,
                pay: true,
                ward_prompts: 0,
            };
            cast_targeting_spell(&mut game, false, &mut decisions);
            assert_eq!(game.stack.len(), if owner == alice { 1 } else { 2 });
            if counters != 0 {
                game.add_counters(target, CounterType::MinusOneMinusOne, counters)
                    .unwrap();
            }
            if owner != alice {
                assert!(game.current_power(target).unwrap() <= 0);
                resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
                assert_eq!(game.stack.len(), 1, "zero life is payable");
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
            assert!(game.is_tapped(target));
            assert_eq!(decisions.ward_prompts, usize::from(owner != alice));
        }
    }
}
